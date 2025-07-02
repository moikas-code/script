use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

/// The Future trait for Script language async operations
pub trait ScriptFuture {
    type Output;

    /// Poll the future to check if it's ready
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output>;
}

/// A boxed future type for dynamic dispatch
pub type BoxedFuture<T> = Box<dyn ScriptFuture<Output = T> + Send>;

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
        self.executor.wake_task(self.task_id);
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

    fn wake_task(&self, task_id: TaskId) {
        let mut queue = self.ready_queue.lock().unwrap();
        queue.push_back(task_id);
        self.wake_signal.notify_one();
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
    pub fn spawn(executor: Arc<Mutex<Self>>, future: BoxedFuture<()>) -> TaskId {
        let (task_id, shared) = {
            let mut exec = executor.lock().unwrap();
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
        shared.wake_task(task_id);
        task_id
    }

    /// Run the executor until all tasks complete
    pub fn run(executor: Arc<Mutex<Self>>) {
        let shared = executor.lock().unwrap().shared.clone();

        loop {
            // Check for shutdown
            if shared.shutdown.load(Ordering::Relaxed) {
                return;
            }

            let task_info = {
                let mut queue = shared.ready_queue.lock().unwrap();

                // Wait for tasks to be ready
                while queue.is_empty() && !shared.shutdown.load(Ordering::Relaxed) {
                    // Check if we have any tasks at all
                    let has_tasks = {
                        let exec = executor.lock().unwrap();
                        exec.tasks.iter().any(|t| t.is_some())
                    };

                    if !has_tasks {
                        return; // All tasks completed
                    }

                    // Wait for wake signal
                    queue = shared.wake_signal.wait(queue).unwrap();
                }

                if shared.shutdown.load(Ordering::Relaxed) {
                    return;
                }

                // Get next ready task
                if let Some(task_id) = queue.pop_front() {
                    let exec = executor.lock().unwrap();
                    exec.tasks
                        .get(task_id.0)
                        .and_then(|t| t.clone())
                        .map(|task| (task_id, task))
                } else {
                    None
                }
            };

            if let Some((task_id, task)) = task_info {
                let mut future = task.future.lock().unwrap();
                let waker = create_waker(task.waker.clone());

                match future.poll(&waker) {
                    Poll::Ready(()) => {
                        // Task completed, remove it
                        drop(future);
                        let mut exec = executor.lock().unwrap();
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
    pub fn shutdown(executor: Arc<Mutex<Self>>) {
        let shared = executor.lock().unwrap().shared.clone();
        shared.shutdown();
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
            let mut results = Vec::new();
            for result in self.results.drain(..) {
                results.push(result.unwrap());
            }
            Poll::Ready(results)
        } else {
            Poll::Pending
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
        handle.join().unwrap();

        // Task should have executed
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
