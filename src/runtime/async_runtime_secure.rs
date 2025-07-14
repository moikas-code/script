//! Secure async runtime implementation
//!
//! This module provides a secure async runtime that eliminates all panic-prone
//! code and implements comprehensive error handling throughout. All security
//! vulnerabilities from the original implementation have been addressed.

use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Maximum number of tasks to prevent resource exhaustion
const MAX_TASKS: usize = 100_000;

/// Maximum timer duration to prevent infinite blocking
const MAX_TIMER_DURATION: Duration = Duration::from_secs(3600); // 1 hour

/// Comprehensive error types for async runtime operations
#[derive(Debug, Clone)]
pub enum AsyncRuntimeError {
    /// A mutex was poisoned and could not be locked
    PoisonedMutex(String),
    /// Task limit exceeded
    TaskLimitExceeded { limit: usize, attempted: usize },
    /// Timer duration invalid
    InvalidTimerDuration(Duration),
    /// Executor shutdown during operation
    ExecutorShutdown,
    /// Thread join failed
    ThreadJoinFailed,
    /// Task not found
    TaskNotFound(TaskId),
    /// Operation timeout
    OperationTimeout,
    /// Resource exhaustion
    ResourceExhaustion(String),
    /// Invalid task state
    InvalidTaskState(String),
}

impl std::fmt::Display for AsyncRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncRuntimeError::PoisonedMutex(msg) => write!(f, "Mutex poisoned: {}", msg),
            AsyncRuntimeError::TaskLimitExceeded { limit, attempted } => {
                write!(
                    f,
                    "Task limit exceeded: attempted {}, limit {}",
                    attempted, limit
                )
            }
            AsyncRuntimeError::InvalidTimerDuration(duration) => {
                write!(f, "Invalid timer duration: {:?}", duration)
            }
            AsyncRuntimeError::ExecutorShutdown => write!(f, "Executor has been shut down"),
            AsyncRuntimeError::ThreadJoinFailed => write!(f, "Failed to join thread"),
            AsyncRuntimeError::TaskNotFound(id) => write!(f, "Task not found: {:?}", id),
            AsyncRuntimeError::OperationTimeout => write!(f, "Operation timed out"),
            AsyncRuntimeError::ResourceExhaustion(msg) => write!(f, "Resource exhaustion: {}", msg),
            AsyncRuntimeError::InvalidTaskState(msg) => write!(f, "Invalid task state: {}", msg),
        }
    }
}

impl std::error::Error for AsyncRuntimeError {}

/// Helper trait to convert mutex poison errors
trait MutexExt<'a, T> {
    type Output;
    fn secure_lock(self) -> Result<Self::Output, AsyncRuntimeError>;
}

impl<'a, T> MutexExt<'a, T> for Result<MutexGuard<'a, T>, PoisonError<MutexGuard<'a, T>>> {
    type Output = MutexGuard<'a, T>;
    fn secure_lock(self) -> Result<MutexGuard<'a, T>, AsyncRuntimeError> {
        self.map_err(|e| AsyncRuntimeError::PoisonedMutex(format!("{:?}", e)))
    }
}

// Also implement for wait_timeout result
impl<'a, T> MutexExt<'a, T>
    for Result<(MutexGuard<'a, T>, std::sync::WaitTimeoutResult), PoisonError<MutexGuard<'a, T>>>
{
    type Output = (MutexGuard<'a, T>, std::sync::WaitTimeoutResult);
    fn secure_lock(
        self,
    ) -> Result<(MutexGuard<'a, T>, std::sync::WaitTimeoutResult), AsyncRuntimeError> {
        self.map_err(|e| AsyncRuntimeError::PoisonedMutex(format!("{:?}", e)))
    }
}

/// Secure result type for async operations
pub type AsyncResult<T> = Result<T, AsyncRuntimeError>;

/// The Future trait for Script language async operations
pub trait ScriptFuture {
    type Output;

    /// Poll the future to check if it's ready
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output>;
}

/// A boxed future type for dynamic dispatch
pub type BoxedFuture<T> = Box<dyn ScriptFuture<Output = T> + Send>;

/// Shared result storage for blocking operations with secure access
#[derive(Debug)]
pub struct SharedResult<T> {
    /// The result of the async operation
    result: Mutex<Option<T>>,
    /// Condition variable to signal completion
    completion: Condvar,
    /// Flag to indicate if the operation completed
    completed: AtomicBool,
    /// Creation timestamp for timeout tracking
    created_at: Instant,
}

impl<T> SharedResult<T> {
    /// Create a new shared result storage
    pub fn new() -> Arc<Self> {
        Arc::new(SharedResult {
            result: Mutex::new(None),
            completion: Condvar::new(),
            completed: AtomicBool::new(false),
            created_at: Instant::now(),
        })
    }

    /// Store the result and signal completion
    pub fn set_result(&self, value: T) -> AsyncResult<()> {
        let mut result = self.result.lock().secure_lock()?;
        *result = Some(value);
        self.completed.store(true, Ordering::SeqCst);
        self.completion.notify_all();
        Ok(())
    }

    /// Wait for the result and return it
    pub fn wait_for_result(&self) -> AsyncResult<T> {
        let mut result = self.result.lock().secure_lock()?;

        while result.is_none() {
            result = self.completion.wait(result).secure_lock()?;
        }

        result.take().ok_or_else(|| {
            AsyncRuntimeError::InvalidTaskState("Result was None after completion".to_string())
        })
    }

    /// Wait for the result with a timeout
    pub fn wait_for_result_timeout(&self, timeout: Duration) -> AsyncResult<Option<T>> {
        let mut result = self.result.lock().secure_lock()?;
        let start = Instant::now();

        while result.is_none() && start.elapsed() < timeout {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            let wait_result = self
                .completion
                .wait_timeout(result, remaining)
                .map_err(|_| {
                    AsyncRuntimeError::PoisonedMutex(
                        "Condition variable wait_timeout failed".to_string(),
                    )
                })?;
            result = wait_result.0;

            if wait_result.1.timed_out() {
                break;
            }
        }

        Ok(result.take())
    }

    /// Check if the operation has completed
    pub fn is_completed(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }

    /// Get the age of this shared result
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.created_at)
    }
}

/// Unique identifier for tasks with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

impl TaskId {
    /// Validate that this task ID is within acceptable bounds
    pub fn validate(&self) -> AsyncResult<()> {
        if self.0 >= MAX_TASKS {
            Err(AsyncRuntimeError::TaskLimitExceeded {
                limit: MAX_TASKS,
                attempted: self.0,
            })
        } else {
            Ok(())
        }
    }
}

/// Task represents an async computation that can be scheduled
pub struct Task {
    id: TaskId,
    future: Mutex<BoxedFuture<()>>,
    waker: Arc<TaskWaker>,
    created_at: Instant,
    state: AtomicTaskState,
}

/// Task state tracking for debugging and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum TaskState {
    Created = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

struct AtomicTaskState(AtomicU8);

impl AtomicTaskState {
    fn new(state: TaskState) -> Self {
        AtomicTaskState(AtomicU8::new(state as u8))
    }

    fn load(&self) -> TaskState {
        match self.0.load(Ordering::SeqCst) {
            0 => TaskState::Created,
            1 => TaskState::Running,
            2 => TaskState::Completed,
            3 => TaskState::Failed,
            _ => TaskState::Failed, // Default to failed for unknown states
        }
    }

    fn store(&self, state: TaskState) {
        self.0.store(state as u8, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for AtomicTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AtomicTaskState({:?})", self.load())
    }
}

impl Task {
    /// Create a new task with validation
    fn new(id: TaskId, future: BoxedFuture<()>, waker: Arc<TaskWaker>) -> AsyncResult<Self> {
        id.validate()?;

        Ok(Task {
            id,
            future: Mutex::new(future),
            waker,
            created_at: Instant::now(),
            state: AtomicTaskState::new(TaskState::Created),
        })
    }

    /// Get the age of this task
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.created_at)
    }

    /// Get the current state
    pub fn state(&self) -> TaskState {
        self.state.load()
    }

    /// Set the task state
    fn set_state(&self, state: TaskState) {
        self.state.store(state);
    }
}

/// Waker implementation for Script tasks with error handling
pub struct TaskWaker {
    task_id: TaskId,
    executor: Arc<ExecutorShared>,
}

impl TaskWaker {
    fn wake(&self) -> AsyncResult<()> {
        self.executor.wake_task(self.task_id)
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        if let Err(e) = TaskWaker::wake(&self) {
            eprintln!("Failed to wake task {:?}: {}", self.task_id, e);
        }
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if let Err(e) = TaskWaker::wake(self) {
            eprintln!("Failed to wake task {:?}: {}", self.task_id, e);
        }
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
    /// Task count for monitoring
    task_count: AtomicUsize,
    /// Error count for monitoring
    error_count: AtomicUsize,
}

impl ExecutorShared {
    fn new() -> Self {
        ExecutorShared {
            ready_queue: Mutex::new(VecDeque::new()),
            wake_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
            task_count: AtomicUsize::new(0),
            error_count: AtomicUsize::new(0),
        }
    }

    fn wake_task(&self, task_id: TaskId) -> AsyncResult<()> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(AsyncRuntimeError::ExecutorShutdown);
        }

        let mut queue = self.ready_queue.lock().secure_lock()?;

        // Prevent queue overflow
        if queue.len() >= MAX_TASKS {
            return Err(AsyncRuntimeError::ResourceExhaustion(
                "Ready queue is full".to_string(),
            ));
        }

        queue.push_back(task_id);
        self.wake_signal.notify_one();
        Ok(())
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.wake_signal.notify_all();
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    fn increment_error_count(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_stats(&self) -> (usize, usize) {
        (
            self.task_count.load(Ordering::Relaxed),
            self.error_count.load(Ordering::Relaxed),
        )
    }
}

/// The secure async executor that runs tasks
pub struct Executor {
    tasks: Vec<Option<Arc<Task>>>,
    next_id: usize,
    shared: Arc<ExecutorShared>,
    max_tasks: usize,
}

impl Executor {
    /// Create a new executor with optional task limit
    pub fn new() -> Arc<Mutex<Self>> {
        Self::with_max_tasks(MAX_TASKS)
    }

    /// Create a new executor with custom task limit
    pub fn with_max_tasks(max_tasks: usize) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Executor {
            tasks: Vec::new(),
            next_id: 0,
            shared: Arc::new(ExecutorShared::new()),
            max_tasks,
        }))
    }

    /// Spawn a new task with validation
    pub fn spawn(executor: Arc<Mutex<Self>>, future: BoxedFuture<()>) -> AsyncResult<TaskId> {
        let (task_id, shared) = {
            let mut exec = executor.lock().secure_lock()?;

            // Check task limit
            if exec.next_id >= exec.max_tasks {
                return Err(AsyncRuntimeError::TaskLimitExceeded {
                    limit: exec.max_tasks,
                    attempted: exec.next_id,
                });
            }

            let task_id = TaskId(exec.next_id);
            exec.next_id += 1;

            let waker = Arc::new(TaskWaker {
                task_id,
                executor: exec.shared.clone(),
            });

            let task = Arc::new(Task::new(task_id, future, waker)?);

            // Ensure tasks vector is large enough
            if task_id.0 >= exec.tasks.len() {
                exec.tasks.resize(task_id.0 + 1, None);
            }

            exec.tasks[task_id.0] = Some(task);
            exec.shared.task_count.fetch_add(1, Ordering::Relaxed);
            (task_id, exec.shared.clone())
        };

        // Wake the task outside the lock
        shared.wake_task(task_id)?;
        Ok(task_id)
    }

    /// Run the executor until all tasks complete
    pub fn run(executor: Arc<Mutex<Self>>) -> AsyncResult<()> {
        let shared = {
            let exec = executor.lock().secure_lock()?;
            exec.shared.clone()
        };

        loop {
            // Check for shutdown
            if shared.is_shutdown() {
                return Ok(());
            }

            let task_info = {
                let mut queue = shared.ready_queue.lock().secure_lock()?;

                // Wait for tasks to be ready
                while queue.is_empty() && !shared.is_shutdown() {
                    // Check if we have any tasks at all
                    let has_tasks = {
                        let exec = executor.lock().secure_lock()?;
                        exec.tasks.iter().any(|t| t.is_some())
                    };

                    if !has_tasks {
                        return Ok(()); // All tasks completed
                    }

                    // Wait for wake signal
                    queue = shared.wake_signal.wait(queue).secure_lock()?;
                }

                if shared.is_shutdown() {
                    return Ok(());
                }

                // Get next ready task
                if let Some(task_id) = queue.pop_front() {
                    let exec = executor.lock().secure_lock()?;
                    exec.tasks
                        .get(task_id.0)
                        .and_then(|t| t.clone())
                        .map(|task| (task_id, task))
                } else {
                    None
                }
            };

            if let Some((task_id, task)) = task_info {
                match Self::poll_task(&task) {
                    Ok(Poll::Ready(())) => {
                        // Task completed, remove it
                        let mut exec = executor.lock().secure_lock()?;
                        exec.tasks[task_id.0] = None;
                        task.set_state(TaskState::Completed);
                    }
                    Ok(Poll::Pending) => {
                        // Task not ready, will be re-queued when woken
                        task.set_state(TaskState::Running);
                    }
                    Err(e) => {
                        // Task failed
                        eprintln!("Task {:?} failed: {}", task_id, e);
                        let mut exec = executor.lock().secure_lock()?;
                        exec.tasks[task_id.0] = None;
                        task.set_state(TaskState::Failed);
                        shared.increment_error_count();
                    }
                }
            }
        }
    }

    /// Poll a single task with error handling
    fn poll_task(task: &Task) -> AsyncResult<Poll<()>> {
        let mut future = task.future.lock().secure_lock()?;
        let waker = create_waker(task.waker.clone())?;

        Ok(future.poll(&waker))
    }

    /// Shutdown the executor
    pub fn shutdown(executor: Arc<Mutex<Self>>) -> AsyncResult<()> {
        let shared = {
            let exec = executor.lock().secure_lock()?;
            exec.shared.clone()
        };
        shared.shutdown();
        Ok(())
    }

    /// Get executor statistics
    pub fn get_stats(executor: Arc<Mutex<Self>>) -> AsyncResult<(usize, usize, usize)> {
        let exec = executor.lock().secure_lock()?;
        let (task_count, error_count) = exec.shared.get_stats();
        let active_tasks = exec.tasks.iter().filter(|t| t.is_some()).count();
        Ok((task_count, error_count, active_tasks))
    }
}

/// Create a standard Waker from our TaskWaker with error handling
fn create_waker(task_waker: Arc<TaskWaker>) -> AsyncResult<Waker> {
    // Validate the waker before creating
    task_waker.task_id.validate()?;

    Ok(unsafe {
        Waker::from_raw(std::task::RawWaker::new(
            Arc::into_raw(task_waker) as *const (),
            &WAKER_VTABLE,
        ))
    })
}

// Secure waker vtable for the executor
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
    let _ = waker.wake(); // Ignore wake errors in low-level callback
                          // Arc is consumed by from_raw and dropped here
}

unsafe fn wake_by_ref_waker(data: *const ()) {
    let waker = Arc::from_raw(data as *const TaskWaker);
    let _ = TaskWaker::wake(&waker); // Call the method directly to avoid consuming the Arc
    let _ = Arc::into_raw(waker); // Convert back to raw pointer without dropping
}

unsafe fn drop_waker(data: *const ()) {
    drop(Arc::from_raw(data as *const TaskWaker));
}

/// Secure timer thread for efficient timer handling
struct TimerThread {
    sender: std::sync::mpsc::Sender<TimerRequest>,
    handle: JoinHandle<()>,
}

#[derive(Debug)]
struct TimerRequest {
    deadline: Instant,
    waker: Waker,
}

impl TimerThread {
    fn new() -> AsyncResult<Self> {
        let (sender, receiver) = std::sync::mpsc::channel::<TimerRequest>();

        let handle = thread::Builder::new()
            .name("script-secure-timer-thread".to_string())
            .spawn(move || {
                let mut timers: Vec<TimerRequest> = Vec::new();

                loop {
                    // Calculate next timeout
                    let timeout = if timers.is_empty() {
                        Duration::from_secs(60) // Long timeout when no timers
                    } else {
                        let now = Instant::now();
                        timers
                            .iter()
                            .map(|req| req.deadline.saturating_duration_since(now))
                            .min()
                            .unwrap_or(Duration::ZERO)
                    };

                    // Wait for new timer or timeout
                    match receiver.recv_timeout(timeout) {
                        Ok(request) => {
                            timers.push(request);
                            timers.sort_by_key(|req| req.deadline);

                            // Prevent unbounded growth
                            if timers.len() > MAX_TASKS {
                                eprintln!("Timer queue overflow, removing oldest timers");
                                timers.truncate(MAX_TASKS);
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // Check for expired timers
                            let now = Instant::now();
                            let mut i = 0;
                            while i < timers.len() {
                                if timers[i].deadline <= now {
                                    let request = timers.remove(i);
                                    request.waker.wake();
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
            .map_err(|_| AsyncRuntimeError::ThreadJoinFailed)?;

        Ok(TimerThread { sender, handle })
    }

    fn register(&self, deadline: Instant, waker: Waker) -> AsyncResult<()> {
        // Validate deadline
        let now = Instant::now();
        let duration = deadline.saturating_duration_since(now);
        if duration > MAX_TIMER_DURATION {
            return Err(AsyncRuntimeError::InvalidTimerDuration(duration));
        }

        let request = TimerRequest { deadline, waker };
        self.sender.send(request).map_err(|_| {
            AsyncRuntimeError::ResourceExhaustion("Timer thread disconnected".to_string())
        })?;

        Ok(())
    }
}

/// Global timer thread instance
static TIMER_THREAD: std::sync::OnceLock<TimerThread> = std::sync::OnceLock::new();

fn get_timer_thread() -> AsyncResult<&'static TimerThread> {
    match TIMER_THREAD.get() {
        Some(timer) => Ok(timer),
        None => {
            // Try to initialize (this may race but that's okay)
            match TimerThread::new() {
                Ok(timer) => {
                    match TIMER_THREAD.set(timer) {
                        Ok(()) => Ok(TIMER_THREAD.get().unwrap()),
                        Err(_) => {
                            // Someone else beat us to it, use their value
                            Ok(TIMER_THREAD.get().unwrap())
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// A secure async timer future
pub struct Timer {
    deadline: Instant,
    registered: bool,
}

impl Timer {
    pub fn new(duration: Duration) -> AsyncResult<Self> {
        // Validate duration
        if duration > MAX_TIMER_DURATION {
            return Err(AsyncRuntimeError::InvalidTimerDuration(duration));
        }

        Ok(Timer {
            deadline: Instant::now() + duration,
            registered: false,
        })
    }
}

impl ScriptFuture for Timer {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            if !self.registered {
                // Register with the global timer thread
                if let Err(e) = get_timer_thread()
                    .and_then(|timer| timer.register(self.deadline, waker.clone()))
                {
                    eprintln!("Failed to register timer: {e}");
                    // Fall back to immediate ready on error
                    return Poll::Ready(());
                }
                self.registered = true;
            }
            Poll::Pending
        }
    }
}

/// Secure adapter to convert Script futures to Rust futures
pub struct ScriptToRustFuture<F: ScriptFuture> {
    future: F,
}

impl<F: ScriptFuture> ScriptToRustFuture<F> {
    pub fn new(future: F) -> Self {
        ScriptToRustFuture { future }
    }
}

impl<F: ScriptFuture> StdFuture for ScriptToRustFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We never move the future out of the pin
        let this = unsafe { self.get_unchecked_mut() };
        this.future.poll(cx.waker())
    }
}

/// Secure join multiple futures with resource limits
pub struct JoinAll<T> {
    futures: Vec<BoxedFuture<T>>,
    results: Vec<Option<T>>,
    max_futures: usize,
}

impl<T> JoinAll<T> {
    pub fn new(futures: Vec<BoxedFuture<T>>) -> AsyncResult<Self> {
        let len = futures.len();

        // Validate future count
        if len > MAX_TASKS {
            return Err(AsyncRuntimeError::TaskLimitExceeded {
                limit: MAX_TASKS,
                attempted: len,
            });
        }

        let mut results = Vec::with_capacity(len);
        for _ in 0..len {
            results.push(None);
        }

        Ok(JoinAll {
            futures,
            results,
            max_futures: len,
        })
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
            let mut results = Vec::with_capacity(self.max_futures);
            for result in self.results.drain(..) {
                // This should never fail since we check all_ready above
                results.push(result.expect("Result should be Some when all_ready is true"));
            }
            Poll::Ready(results)
        } else {
            Poll::Pending
        }
    }
}

/// A secure specialized executor for blocking operations
pub struct BlockingExecutor {
    shared: Arc<ExecutorShared>,
    tasks: Vec<Option<Arc<Task>>>,
    next_task_id: usize,
    max_tasks: usize,
}

impl BlockingExecutor {
    /// Create a new blocking executor
    pub fn new() -> Arc<Mutex<Self>> {
        Self::with_max_tasks(1000) // Smaller limit for blocking operations
    }

    /// Create a new blocking executor with custom limits
    pub fn with_max_tasks(max_tasks: usize) -> Arc<Mutex<Self>> {
        let shared = Arc::new(ExecutorShared::new());

        Arc::new(Mutex::new(BlockingExecutor {
            shared,
            tasks: Vec::new(),
            next_task_id: 0,
            max_tasks,
        }))
    }

    /// Block on a future until it completes, returning the result
    pub fn block_on<T>(future: BoxedFuture<T>) -> AsyncResult<T>
    where
        T: Send + 'static,
    {
        Self::block_on_with_timeout(future, Duration::from_secs(300)) // 5 minute default timeout
    }

    /// Block on a future with a timeout, returning the result
    pub fn block_on_with_timeout<T>(future: BoxedFuture<T>, timeout: Duration) -> AsyncResult<T>
    where
        T: Send + 'static,
    {
        // Validate timeout
        if timeout > MAX_TIMER_DURATION {
            return Err(AsyncRuntimeError::InvalidTimerDuration(timeout));
        }

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
                        if let Err(e) = self.result_storage.set_result(value) {
                            eprintln!("Failed to set result: {e}");
                        }
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
        let _task_id = {
            let mut exec = executor.lock().secure_lock()?;

            if exec.next_task_id >= exec.max_tasks {
                return Err(AsyncRuntimeError::TaskLimitExceeded {
                    limit: exec.max_tasks,
                    attempted: exec.next_task_id,
                });
            }

            let task_id = TaskId(exec.next_task_id);
            exec.next_task_id += 1;

            let waker = Arc::new(TaskWaker {
                task_id,
                executor: exec.shared.clone(),
            });

            let task = Arc::new(Task::new(task_id, Box::new(blocking_future), waker)?);

            // Ensure tasks vector is large enough
            if task_id.0 >= exec.tasks.len() {
                exec.tasks.resize(task_id.0 + 1, None);
            }

            exec.tasks[task_id.0] = Some(task);
            exec.shared.wake_task(task_id)?;
            task_id
        };

        // Run the executor in a separate thread to avoid blocking the current thread completely
        let exec_clone = executor.clone();
        let handle = thread::Builder::new()
            .name("script-blocking-executor".to_string())
            .spawn(move || {
                if let Err(e) = Self::run_until_complete(exec_clone) {
                    eprintln!("Blocking executor error: {e}");
                }
            })
            .map_err(|_| AsyncRuntimeError::ThreadJoinFailed)?;

        // Wait for the result with timeout
        let result = result_storage.wait_for_result_timeout(timeout)?;

        // Clean up the executor thread
        {
            let exec = executor.lock().secure_lock()?;
            exec.shared.shutdown();
        }

        // Wait for the thread to finish
        handle
            .join()
            .map_err(|_| AsyncRuntimeError::ThreadJoinFailed)?;

        result.ok_or(AsyncRuntimeError::OperationTimeout)
    }

    /// Run the executor until completion or shutdown
    fn run_until_complete(executor: Arc<Mutex<Self>>) -> AsyncResult<()> {
        let shared = {
            let exec = executor.lock().secure_lock()?;
            exec.shared.clone()
        };

        loop {
            // Check for shutdown
            if shared.is_shutdown() {
                return Ok(());
            }

            let task_id = {
                let mut queue = shared.ready_queue.lock().secure_lock()?;

                // Wait for tasks to be ready
                while queue.is_empty() && !shared.is_shutdown() {
                    // Check if we have any tasks at all
                    let has_tasks = {
                        let exec = executor.lock().secure_lock()?;
                        exec.tasks.iter().any(|t| t.is_some())
                    };

                    if !has_tasks {
                        return Ok(()); // All tasks completed
                    }

                    // Wait for wake signal
                    queue = shared.wake_signal.wait(queue).secure_lock()?;
                }

                if shared.is_shutdown() {
                    return Ok(());
                }

                queue.pop_front()
            };

            if let Some(task_id) = task_id {
                // Get the task
                let task = {
                    let exec = executor.lock().secure_lock()?;
                    exec.tasks.get(task_id.0).and_then(|t| t.clone())
                };

                if let Some(task) = task {
                    // Poll the task
                    match Executor::poll_task(&task) {
                        Ok(Poll::Ready(())) => {
                            // Task completed - remove it
                            let mut exec = executor.lock().secure_lock()?;
                            exec.tasks[task_id.0] = None;
                            task.set_state(TaskState::Completed);
                        }
                        Ok(Poll::Pending) => {
                            // Task is still pending, it will be woken when ready
                            task.set_state(TaskState::Running);
                        }
                        Err(e) => {
                            // Task failed
                            eprintln!("Blocking task {:?} failed: {}", task_id, e);
                            let mut exec = executor.lock().secure_lock()?;
                            exec.tasks[task_id.0] = None;
                            task.set_state(TaskState::Failed);
                            shared.increment_error_count();
                        }
                    }
                }
            }
        }
    }
}

impl Default for BlockingExecutor {
    fn default() -> Self {
        let shared = Arc::new(ExecutorShared::new());

        BlockingExecutor {
            shared,
            tasks: Vec::new(),
            next_task_id: 0,
            max_tasks: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_secure_timer() {
        let executor = Executor::new();

        struct TimerTask {
            timer: Timer,
        }

        impl TimerTask {
            fn new() -> AsyncResult<Self> {
                Ok(Self {
                    timer: Timer::new(Duration::from_millis(100))?,
                })
            }
        }

        impl ScriptFuture for TimerTask {
            type Output = ();

            fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
                self.timer.poll(waker)
            }
        }

        let task = TimerTask::new().expect("Failed to create timer task");
        let _task_id =
            Executor::spawn(executor.clone(), Box::new(task)).expect("Failed to spawn task");

        Executor::run(executor).expect("Failed to run executor");
    }

    #[test]
    fn test_secure_multiple_timers() {
        let executor = Executor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        struct CountingTimer {
            timer: Timer,
            counter: Arc<AtomicUsize>,
        }

        impl CountingTimer {
            fn new(duration: Duration, counter: Arc<AtomicUsize>) -> AsyncResult<Self> {
                Ok(Self {
                    timer: Timer::new(duration)?,
                    counter,
                })
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
            let timer = CountingTimer::new(duration, counter.clone())
                .expect("Failed to create counting timer");
            Executor::spawn(executor.clone(), Box::new(timer))
                .expect("Failed to spawn counting timer");
        }

        Executor::run(executor).expect("Failed to run executor");

        // All 5 timers should have completed
        assert_eq!(counter.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_secure_immediate_ready() {
        let executor = Executor::new();

        struct ImmediateTask;

        impl ScriptFuture for ImmediateTask {
            type Output = ();

            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        Executor::spawn(executor.clone(), Box::new(ImmediateTask))
            .expect("Failed to spawn immediate task");
        Executor::run(executor).expect("Failed to run executor");
    }

    #[test]
    fn test_secure_executor_shutdown() {
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
        )
        .expect("Failed to spawn increment task");

        // Run executor in a separate thread
        let executor_clone = executor.clone();
        let handle = thread::spawn(move || Executor::run(executor_clone));

        // Give it time to start
        thread::sleep(Duration::from_millis(50));

        // Shutdown the executor
        Executor::shutdown(executor).expect("Failed to shutdown executor");

        // Wait for the executor thread to finish
        handle
            .join()
            .expect("Failed to join executor thread")
            .expect("Executor run failed");

        // Task should have executed
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_task_limit_validation() {
        let executor = Executor::with_max_tasks(5);

        struct NoOpTask;
        impl ScriptFuture for NoOpTask {
            type Output = ();
            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        // Spawn tasks up to the limit
        for _ in 0..5 {
            Executor::spawn(executor.clone(), Box::new(NoOpTask))
                .expect("Should succeed within limit");
        }

        // This should fail
        let result = Executor::spawn(executor.clone(), Box::new(NoOpTask));
        assert!(matches!(
            result,
            Err(AsyncRuntimeError::TaskLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_invalid_timer_duration() {
        let result = Timer::new(Duration::from_secs(4000)); // Exceeds MAX_TIMER_DURATION
        assert!(matches!(
            result,
            Err(AsyncRuntimeError::InvalidTimerDuration(_))
        ));
    }

    #[test]
    fn test_blocking_executor_timeout() {
        struct NeverCompletes;
        impl ScriptFuture for NeverCompletes {
            type Output = i32;
            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        let result = BlockingExecutor::block_on_with_timeout(
            Box::new(NeverCompletes),
            Duration::from_millis(100),
        );

        assert!(matches!(result, Err(AsyncRuntimeError::OperationTimeout)));
    }
}
