use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

/// The Future trait for Script language async operations
pub trait ScriptFuture {
    type Output;

    /// Poll the future to check if it's ready
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output>;
}

/// A boxed future type for dynamic dispatch
pub type BoxedFuture<T> = Box<dyn ScriptFuture<Output = T> + Send>;

/// Shared result storage for blocking operations
#[derive(Debug)]
pub struct SharedResult<T> {
    /// The result of the async operation
    result: Mutex<Option<T>>,
    /// Condition variable to signal completion
    completion: Condvar,
    /// Flag to indicate if the operation completed
    completed: AtomicBool,
}

impl<T> SharedResult<T> {
    /// Create a new shared result storage
    pub fn new() -> Arc<Self> {
        Arc::new(SharedResult {
            result: Mutex::new(None),
            completion: Condvar::new(),
            completed: AtomicBool::new(false),
        })
    }

    /// Store the result and signal completion
    pub fn set_result(&self, value: T) -> Result<()> {
        let mut result = self.result.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on shared result")
        })?;
        *result = Some(value);
        self.completed.store(true, Ordering::SeqCst);
        self.completion.notify_all();
        Ok(())
    }

    /// Wait for the result and return it
    pub fn wait_for_result(&self) -> Result<T> {
        let mut result = self.result.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on shared result")
        })?;
        while result.is_none() {
            result = self.completion.wait(result).map_err(|_| {
                Error::lock_poisoned("Condition variable wait failed")
            })?;
        }
        result.take().ok_or_else(|| {
            Error::internal("Shared result was empty after wait completed")
        })
    }

    /// Wait for the result with a timeout
    pub fn wait_for_result_timeout(&self, timeout: Duration) -> Result<Option<T>> {
        let mut result = self.result.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on shared result")
        })?;
        let start = Instant::now();
        
        while result.is_none() && start.elapsed() < timeout {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }
            
            let (guard, timeout_result) = self.completion.wait_timeout(result, remaining).map_err(|_| {
                Error::lock_poisoned("Condition variable wait timeout failed")
            })?;
            result = guard;
            
            if timeout_result.timed_out() {
                break;
            }
        }
        
        Ok(result.take())
    }

    /// Check if the operation has completed
    pub fn is_completed(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }
}

/// Task represents an async computation that can be scheduled
pub struct Task {
    id: TaskId,
    future: Mutex<BoxedFuture<()>>,
    waker: Arc<TaskWaker>,
}

/// Unique identifier for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

/// Waker implementation for Script tasks
pub struct TaskWaker {
    task_id: TaskId,
    executor: Arc<ExecutorShared>,
}

impl TaskWaker {
    fn wake(&self) {
        let _ = self.executor.wake_task(self.task_id);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        TaskWaker::wake(&self);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        TaskWaker::wake(self);
    }
}

/// Shared executor state for thread-safe access
pub struct ExecutorShared {
    /// Ready queue protected by mutex
    ready_queue: Mutex<VecDeque<TaskId>>,
    /// Condition variable for waking the executor
    wake_signal: Condvar,
    /// Flag to check if executor should shut down
    shutdown: AtomicBool,
}

/// The async executor that runs tasks
pub struct Executor {
    tasks: Vec<Option<Arc<Task>>>,
    next_id: usize,
    shared: Arc<ExecutorShared>,
}

impl ExecutorShared {
    fn new() -> Self {
        ExecutorShared {
            ready_queue: Mutex::new(VecDeque::new()),
            wake_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
        }
    }

    fn wake_task(&self, task_id: TaskId) -> Result<()> {
        let mut queue = self.ready_queue.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on ready queue")
        })?;
        queue.push_back(task_id);
        self.wake_signal.notify_one();
        Ok(())
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.wake_signal.notify_all();
    }
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Executor {
            tasks: Vec::new(),
            next_id: 0,
            shared: Arc::new(ExecutorShared::new()),
        }))
    }

    /// Spawn a new task
    pub fn spawn(executor: Arc<Mutex<Self>>, future: BoxedFuture<()>) -> Result<TaskId> {
        let (task_id, shared) = {
            let mut exec = executor.lock().map_err(|_| {
                Error::lock_poisoned("Failed to acquire lock on executor")
            })?;
            let task_id = TaskId(exec.next_id);
            exec.next_id += 1;

            let waker = Arc::new(TaskWaker {
                task_id,
                executor: exec.shared.clone(),
            });

            let task = Arc::new(Task {
                id: task_id,
                future: Mutex::new(future),
                waker,
            });

            // Ensure tasks vector is large enough
            if task_id.0 >= exec.tasks.len() {
                exec.tasks.resize(task_id.0 + 1, None);
            }

            exec.tasks[task_id.0] = Some(task);
            (task_id, exec.shared.clone())
        };

        // Wake the task outside the lock
        shared.wake_task(task_id)?;
        Ok(task_id)
    }

    /// Run the executor until all tasks complete
    pub fn run(executor: Arc<Mutex<Self>>) -> Result<()> {
        let shared = executor.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on executor")
        })?.shared.clone();

        loop {
            // Check for shutdown
            if shared.shutdown.load(Ordering::Relaxed) {
                return;
            }

            let task_info = {
                let mut queue = shared.ready_queue.lock().map_err(|_| {
                    Error::lock_poisoned("Failed to acquire lock on ready queue")
                })?;

                // Wait for tasks to be ready
                while queue.is_empty() && !shared.shutdown.load(Ordering::Relaxed) {
                    // Check if we have any tasks at all
                    let has_tasks = {
                        let exec = executor.lock().map_err(|_| {
                            Error::lock_poisoned("Failed to acquire lock on executor")
                        })?;
                        exec.tasks.iter().any(|t| t.is_some())
                    };

                    if !has_tasks {
                        return Ok(()); // All tasks completed
                    }

                    // Wait for wake signal
                    queue = shared.wake_signal.wait(queue).map_err(|_| {
                        Error::lock_poisoned("Condition variable wait failed")
                    })?;
                }

                if shared.shutdown.load(Ordering::Relaxed) {
                    return Ok(());
                }

                // Get next ready task
                if let Some(task_id) = queue.pop_front() {
                    let exec = executor.lock().map_err(|_| {
                        Error::lock_poisoned("Failed to acquire lock on executor")
                    })?;
                    exec.tasks
                        .get(task_id.0)
                        .and_then(|t| t.clone())
                        .map(|task| (task_id, task))
                } else {
                    None
                }
            };

            if let Some((task_id, task)) = task_info {
                let mut future = task.future.lock().map_err(|_| {
                    Error::lock_poisoned("Failed to acquire lock on task future")
                })?;
                let waker = create_waker(task.waker.clone());

                match future.poll(&waker) {
                    Poll::Ready(()) => {
                        // Task completed, remove it
                        drop(future);
                        let mut exec = executor.lock().map_err(|_| {
                            Error::lock_poisoned("Failed to acquire lock on executor")
                        })?;
                        exec.tasks[task_id.0] = None;
                    }
                    Poll::Pending => {
                        // Task not ready, will be re-queued when woken
                    }
                }
            }
        }
    }

    /// Shutdown the executor
    pub fn shutdown(executor: Arc<Mutex<Self>>) -> Result<()> {
        let shared = executor.lock().map_err(|_| {
            Error::lock_poisoned("Failed to acquire lock on executor")
        })?.shared.clone();
        shared.shutdown();
        Ok(())
    }
}

/// Create a standard Waker from our TaskWaker
fn create_waker(task_waker: Arc<TaskWaker>) -> Waker {
    unsafe {
        Waker::from_raw(std::task::RawWaker::new(
            Arc::into_raw(task_waker) as *const (),
            &WAKER_VTABLE,
        ))
    }
}

// Waker vtable for the executor
static WAKER_VTABLE: std::task::RawWakerVTable =
    std::task::RawWakerVTable::new(clone_waker, wake_waker, wake_by_ref_waker, drop_waker);

unsafe fn clone_waker(data: *const ()) -> std::task::RawWaker {
    let waker = Arc::from_raw(data as *const TaskWaker);
    let cloned = waker.clone();
    let _ = Arc::into_raw(waker); // Convert back to raw pointer without dropping
    std::task::RawWaker::new(Arc::into_raw(cloned) as *const (), &WAKER_VTABLE)
}

unsafe fn wake_waker(data: *const ()) {
    let waker = Arc::from_raw(data as *const TaskWaker);
    waker.wake();
    // Arc is consumed by from_raw and dropped here
}

unsafe fn wake_by_ref_waker(data: *const ()) {
    let waker = Arc::from_raw(data as *const TaskWaker);
    waker.wake();
    let _ = Arc::into_raw(waker); // Convert back to raw pointer without dropping
}

unsafe fn drop_waker(data: *const ()) {
    drop(Arc::from_raw(data as *const TaskWaker));
}

/// Global timer thread for efficient timer handling
static TIMER_THREAD: std::sync::OnceLock<Arc<TimerThread>> = std::sync::OnceLock::new();

struct TimerThread {
    sender: std::sync::mpsc::Sender<(std::time::Instant, Waker)>,
}

impl TimerThread {
    fn get() -> Arc<TimerThread> {
        TIMER_THREAD
            .get_or_init(|| {
                let (sender, receiver) = std::sync::mpsc::channel();

                thread::Builder::new()
                    .name("script-timer-thread".to_string())
                    .spawn(move || {
                        let mut timers: Vec<(std::time::Instant, Waker)> = Vec::new();

                        loop {
                            // Calculate next timeout
                            let timeout = if timers.is_empty() {
                                Duration::from_secs(60) // Long timeout when no timers
                            } else {
                                let now = std::time::Instant::now();
                                timers
                                    .iter()
                                    .map(|(deadline, _)| deadline.saturating_duration_since(now))
                                    .min()
                                    .unwrap_or(Duration::ZERO)
                            };

                            // Wait for new timer or timeout
                            match receiver.recv_timeout(timeout) {
                                Ok((deadline, waker)) => {
                                    timers.push((deadline, waker));
                                    timers.sort_by_key(|(d, _)| *d);
                                }
                                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                    // Check for expired timers
                                    let now = std::time::Instant::now();
                                    let mut i = 0;
                                    while i < timers.len() {
                                        if timers[i].0 <= now {
                                            let (_, waker) = timers.remove(i);
                                            waker.wake();
                                        } else {
                                            break; // Timers are sorted, so we can stop
                                        }
                                    }
                                }
                                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                    break; // Shutdown
                                }
                            }
                        }
                    })
                    .expect("Failed to spawn timer thread");

                Arc::new(TimerThread { sender })
            })
            .clone()
    }

    fn register(&self, deadline: std::time::Instant, waker: Waker) {
        let _ = self.sender.send((deadline, waker));
    }
}

/// A simple async timer future
pub struct Timer {
    deadline: std::time::Instant,
    registered: bool,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Timer {
            deadline: std::time::Instant::now() + duration,
            registered: false,
        }
    }
}

impl ScriptFuture for Timer {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if std::time::Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            if !self.registered {
                // Register with the global timer thread
                TimerThread::get().register(self.deadline, waker.clone());
                self.registered = true;
            }
            Poll::Pending
        }
    }
}

/// Adapter to convert Script futures to Rust futures
pub struct ScriptToRustFuture<F: ScriptFuture> {
    future: F,
}

impl<F: ScriptFuture> StdFuture for ScriptToRustFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We never move the future out of the pin
        let this = unsafe { self.get_unchecked_mut() };
        this.future.poll(cx.waker())
    }
}

/// Join multiple futures
pub struct JoinAll<T> {
    futures: Vec<BoxedFuture<T>>,
    results: Vec<Option<T>>,
}

impl<T> JoinAll<T> {
    pub fn new(futures: Vec<BoxedFuture<T>>) -> Self {
        let len = futures.len();
        let mut results = Vec::with_capacity(len);
        for _ in 0..len {
            results.push(None);
        }
        JoinAll { futures, results }
    }
}

impl<T> ScriptFuture for JoinAll<T> {
    type Output = Vec<T>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        let mut all_ready = true;

        for (i, future) in self.futures.iter_mut().enumerate() {
            if self.results[i].is_none() {
                match future.poll(waker) {
                    Poll::Ready(value) => {
                        self.results[i] = Some(value);
                    }
                    Poll::Pending => {
                        all_ready = false;
                    }
                }
            }
        }

        if all_ready {
            // Collect all results, only proceeding if all are Some
            let results: Option<Vec<T>> = self.results.drain(..).collect();
            match results {
                Some(values) => Poll::Ready(values),
                None => {
                    // This indicates an internal logic error - some result was None
                    // Reset to pending and continue (defensive programming)
                    Poll::Pending
                }
            }
        } else {
            Poll::Pending
        }
    }
}

/// A specialized executor for blocking operations
pub struct BlockingExecutor {
    shared: Arc<ExecutorShared>,
    tasks: Vec<Option<Arc<Task>>>,
    next_task_id: usize,
}

impl BlockingExecutor {
    /// Create a new blocking executor
    pub fn new() -> Arc<Mutex<Self>> {
        let shared = Arc::new(ExecutorShared {
            ready_queue: Mutex::new(VecDeque::new()),
            wake_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
        });

        Arc::new(Mutex::new(BlockingExecutor {
            shared,
            tasks: Vec::new(),
            next_task_id: 0,
        }))
    }

    /// Block on a future until it completes, returning the result
    pub fn block_on<T>(future: BoxedFuture<T>) -> Result<T>
    where
        T: Send + 'static,
    {
        let result_storage = SharedResult::<T>::new();
        let result_clone = result_storage.clone();

        // Create a wrapper future that stores the result
        struct BlockingFuture<T> {
            inner: BoxedFuture<T>,
            result_storage: Arc<SharedResult<T>>,
        }

        impl<T> ScriptFuture for BlockingFuture<T> {
            type Output = ();

            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                match self.inner.poll(waker) {
                    Poll::Ready(value) => {
                        self.result_storage.set_result(value);
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }

        let blocking_future = BlockingFuture {
            inner: future,
            result_storage: result_clone,
        };

        // Create a dedicated executor for this operation
        let executor = Self::new();

        // Spawn the future
        let task_id = {
            let mut exec = executor.lock().map_err(|_| {
                Error::lock_poisoned("Failed to acquire lock on blocking executor")
            })?;
            let task_id = TaskId(exec.next_task_id);
            exec.next_task_id += 1;

            let waker = Arc::new(TaskWaker {
                task_id,
                executor: exec.shared.clone(),
            });

            let task = Arc::new(Task {
                id: task_id,
                future: Mutex::new(Box::new(blocking_future)),
                waker,
            });

            // Ensure tasks vector is large enough
            if task_id.0 >= exec.tasks.len() {
                exec.tasks.resize(task_id.0 + 1, None);
            }

            exec.tasks[task_id.0] = Some(task);
            let _ = exec.shared.wake_task(task_id);
            task_id
        };

        // Run the executor in a separate thread to avoid blocking the current thread completely
        let exec_clone = executor.clone();
        let handle = thread::spawn(move || {
            Self::run_until_complete(exec_clone);
        });

        // Wait for the result
        let result = result_storage.wait_for_result()?;

        // Clean up the executor thread
        {
            let exec = executor.lock().map_err(|_| {
                Error::lock_poisoned("Failed to acquire lock on blocking executor")
            })?;
            exec.shared.shutdown.store(true, Ordering::SeqCst);
            exec.shared.wake_signal.notify_all();
        }

        // Wait for the thread to finish
        handle.join().map_err(|_| {
            Error::async_error("Failed to join executor thread")
        })?;

        Ok(result)
    }

    /// Block on a future with a timeout
    pub fn block_on_timeout<T>(future: BoxedFuture<T>, timeout: Duration) -> Result<Option<T>>
    where
        T: Send + 'static,
    {
        let result_storage = SharedResult::<T>::new();
        let result_clone = result_storage.clone();

        // Create a wrapper future that stores the result
        struct BlockingFuture<T> {
            inner: BoxedFuture<T>,
            result_storage: Arc<SharedResult<T>>,
        }

        impl<T> ScriptFuture for BlockingFuture<T> {
            type Output = ();

            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                match self.inner.poll(waker) {
                    Poll::Ready(value) => {
                        self.result_storage.set_result(value);
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }

        let blocking_future = BlockingFuture {
            inner: future,
            result_storage: result_clone,
        };

        // Create a dedicated executor for this operation
        let executor = Self::new();

        // Spawn the future
        {
            let mut exec = executor.lock().map_err(|_| {
                Error::lock_poisoned("Failed to acquire lock on blocking executor")
            })?;
            let task_id = TaskId(exec.next_task_id);
            exec.next_task_id += 1;

            let waker = Arc::new(TaskWaker {
                task_id,
                executor: exec.shared.clone(),
            });

            let task = Arc::new(Task {
                id: task_id,
                future: Mutex::new(Box::new(blocking_future)),
                waker,
            });

            // Ensure tasks vector is large enough
            if task_id.0 >= exec.tasks.len() {
                exec.tasks.resize(task_id.0 + 1, None);
            }

            exec.tasks[task_id.0] = Some(task);
            let _ = exec.shared.wake_task(task_id);
        }

        // Run the executor in a separate thread
        let exec_clone = executor.clone();
        let handle = thread::spawn(move || {
            Self::run_until_complete(exec_clone);
        });

        // Wait for the result with timeout
        let result = result_storage.wait_for_result_timeout(timeout)?;

        // Clean up the executor thread
        {
            let exec = executor.lock().map_err(|_| {
                Error::lock_poisoned("Failed to acquire lock on blocking executor")
            })?;
            exec.shared.shutdown.store(true, Ordering::SeqCst);
            exec.shared.wake_signal.notify_all();
        }

        // Wait for the thread to finish
        handle.join().map_err(|_| {
            Error::async_error("Failed to join executor thread")
        })?;

        Ok(result)
    }

    /// Run the executor until completion or shutdown
    fn run_until_complete(executor: Arc<Mutex<Self>>) {
        let shared = match executor.lock() {
            Ok(exec) => exec.shared.clone(),
            Err(_) => return, // Exit on lock failure
        };

        loop {
            // Check for shutdown
            if shared.shutdown.load(Ordering::Relaxed) {
                return;
            }

            let task_id = {
                let mut queue = match shared.ready_queue.lock() {
                    Ok(q) => q,
                    Err(_) => return, // Exit on lock failure
                };

                // Wait for tasks to be ready
                while queue.is_empty() && !shared.shutdown.load(Ordering::Relaxed) {
                    // Check if we have any tasks at all
                    let has_tasks = {
                        let exec = match executor.lock() {
                            Ok(e) => e,
                            Err(_) => return, // Exit on lock failure
                        };
                        exec.tasks.iter().any(|t| t.is_some())
                    };

                    if !has_tasks {
                        return; // All tasks completed
                    }

                    // Wait for wake signal
                    queue = match shared.wake_signal.wait(queue) {
                        Ok(q) => q,
                        Err(_) => return, // Exit on condition variable failure
                    };
                }

                if shared.shutdown.load(Ordering::Relaxed) {
                    return;
                }

                queue.pop_front()
            };

            if let Some(task_id) = task_id {
                // Get the task
                let task = {
                    let exec = match executor.lock() {
                        Ok(e) => e,
                        Err(_) => return, // Exit on lock failure
                    };
                    exec.tasks.get(task_id.0).and_then(|t| t.clone())
                };

                if let Some(task) = task {
                    // Poll the task
                    let mut future = match task.future.lock() {
                        Ok(f) => f,
                        Err(_) => return, // Exit on lock failure
                    };
                    let waker = Waker::from(task.waker.clone());

                    match future.poll(&waker) {
                        Poll::Ready(()) => {
                            // Task completed - remove it
                            let mut exec = match executor.lock() {
                                Ok(e) => e,
                                Err(_) => return, // Exit on lock failure
                            };
                            exec.tasks[task_id.0] = None;
                        }
                        Poll::Pending => {
                            // Task is still pending, it will be woken when ready
                        }
                    }
                }
            }
        }
    }
}

impl Default for BlockingExecutor {
    fn default() -> Self {
        // Create a default instance (though this won't be used much)
        let shared = Arc::new(ExecutorShared {
            ready_queue: Mutex::new(VecDeque::new()),
            wake_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
        });

        BlockingExecutor {
            shared,
            tasks: Vec::new(),
            next_task_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_timer() {
        let executor = Executor::new();

        struct TimerTask {
            timer: Timer,
        }

        impl TimerTask {
            fn new() -> Self {
                Self {
                    timer: Timer::new(Duration::from_millis(100)),
                }
            }
        }

        impl ScriptFuture for TimerTask {
            type Output = ();

            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                self.timer.poll(waker)
            }
        }

        let _task_id = Executor::spawn(executor.clone(), Box::new(TimerTask::new()));

        Executor::run(executor);
    }

    #[test]
    fn test_multiple_timers() {
        let executor = Executor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        struct CountingTimer {
            timer: Timer,
            counter: Arc<AtomicUsize>,
        }

        impl CountingTimer {
            fn new(duration: Duration, counter: Arc<AtomicUsize>) -> Self {
                Self {
                    timer: Timer::new(duration),
                    counter,
                }
            }
        }

        impl ScriptFuture for CountingTimer {
            type Output = ();

            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                match self.timer.poll(waker) {
                    Poll::Ready(()) => {
                        self.counter.fetch_add(1, Ordering::Relaxed);
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }

        // Spawn multiple timers with different durations
        for i in 0..5 {
            let duration = Duration::from_millis(50 * (i + 1));
            Executor::spawn(
                executor.clone(),
                Box::new(CountingTimer::new(duration, counter.clone())),
            );
        }

        Executor::run(executor);

        // All 5 timers should have completed
        assert_eq!(counter.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_immediate_ready() {
        let executor = Executor::new();

        struct ImmediateTask;

        impl ScriptFuture for ImmediateTask {
            type Output = ();

            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        Executor::spawn(executor.clone(), Box::new(ImmediateTask));
        Executor::run(executor);
    }

    #[test]
    fn test_executor_shutdown() {
        let executor = Executor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        struct IncrementTask {
            counter: Arc<AtomicUsize>,
            done: bool,
        }

        impl IncrementTask {
            fn new(counter: Arc<AtomicUsize>) -> Self {
                Self {
                    counter,
                    done: false,
                }
            }
        }

        impl ScriptFuture for IncrementTask {
            type Output = ();

            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                if !self.done {
                    self.counter.fetch_add(1, Ordering::Relaxed);
                    self.done = true;
                    Poll::Ready(())
                } else {
                    Poll::Ready(())
                }
            }
        }

        Executor::spawn(
            executor.clone(),
            Box::new(IncrementTask::new(counter.clone())),
        );

        // Run executor in a separate thread
        let executor_clone = executor.clone();
        let handle = thread::spawn(move || {
            Executor::run(executor_clone);
        });

        // Give it time to start
        thread::sleep(Duration::from_millis(50));

        // Shutdown the executor
        Executor::shutdown(executor);

        // Wait for the executor thread to finish
        let _ = handle.join();

        // Task should have executed
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
