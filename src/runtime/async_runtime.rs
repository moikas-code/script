use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

/// Configuration for the async runtime with security limits
#[derive(Debug, Clone)]
pub struct AsyncRuntimeConfig {
    /// Maximum number of concurrent tasks (default: 1000)
    pub max_concurrent_tasks: usize,
    /// Maximum task queue size (default: 10000)
    pub max_queue_size: usize,
    /// Global timeout for all operations (default: 30s)
    pub global_timeout: Duration,
    /// Maximum memory usage per executor in bytes (default: 100MB)
    pub max_memory_usage: usize,
    /// Enable resource monitoring (default: true)
    pub enable_monitoring: bool,
    /// Task eviction policy when limits are reached
    pub eviction_policy: EvictionPolicy,
}

/// Policy for evicting tasks when resource limits are reached
#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    /// First In, First Out - evict oldest tasks
    Fifo,
    /// Reject new tasks when limit is reached
    Reject,
    /// Evict tasks based on priority (not implemented yet)
    Priority,
}

/// Security-specific error types for async runtime
#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Resource exhaustion - too many tasks
    ResourceExhaustion(String),
    /// Timeout exceeded
    TimeoutExceeded(Duration),
    /// Memory limit exceeded
    MemoryLimitExceeded(usize),
    /// Task queue full
    QueueFull(usize),
    /// Invalid configuration
    InvalidConfig(String),
}

/// Resource monitor for tracking runtime usage
#[derive(Debug)]
pub struct ResourceMonitor {
    /// Current number of active tasks
    active_tasks: AtomicUsize,
    /// Current queue size
    queue_size: AtomicUsize,
    /// Estimated memory usage in bytes
    memory_usage: AtomicUsize,
    /// Start time for tracking global timeout
    start_time: Instant,
    /// Configuration limits
    config: AsyncRuntimeConfig,
}

impl Default for AsyncRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 1000,
            max_queue_size: 10000,
            global_timeout: Duration::from_secs(30),
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            enable_monitoring: true,
            eviction_policy: EvictionPolicy::Fifo,
        }
    }
}

impl ResourceMonitor {
    fn new(config: AsyncRuntimeConfig) -> Self {
        Self {
            active_tasks: AtomicUsize::new(0),
            queue_size: AtomicUsize::new(0),
            memory_usage: AtomicUsize::new(0),
            start_time: Instant::now(),
            config,
        }
    }

    /// Check if we can add a new task without exceeding limits
    fn can_add_task(&self) -> Result<()> {
        let current_tasks = self.active_tasks.load(Ordering::Relaxed);
        if current_tasks >= self.config.max_concurrent_tasks {
            return Err(Error::security_error(format!(
                "Maximum concurrent tasks exceeded: {}/{}",
                current_tasks, self.config.max_concurrent_tasks
            )));
        }

        let current_queue = self.queue_size.load(Ordering::Relaxed);
        if current_queue >= self.config.max_queue_size {
            return Err(Error::security_error(format!(
                "Task queue full: {}/{}",
                current_queue, self.config.max_queue_size
            )));
        }

        let current_memory = self.memory_usage.load(Ordering::Relaxed);
        if current_memory >= self.config.max_memory_usage {
            return Err(Error::security_error(format!(
                "Memory limit exceeded: {}/{} bytes",
                current_memory, self.config.max_memory_usage
            )));
        }

        // Check global timeout
        if self.start_time.elapsed() >= self.config.global_timeout {
            return Err(Error::security_error(format!(
                "Global timeout exceeded: {:?}",
                self.config.global_timeout
            )));
        }

        Ok(())
    }

    /// Atomically reserve a task slot with rollback on failure
    fn reserve_task_slot(&self) -> Result<()> {
        // SECURITY: Use fetch_add to atomically increment and check
        let previous_tasks = self.active_tasks.fetch_add(1, Ordering::SeqCst);

        // Check if we exceeded the limit after increment
        if previous_tasks >= self.config.max_concurrent_tasks {
            // Rollback the increment
            self.active_tasks.fetch_sub(1, Ordering::SeqCst);
            return Err(Error::security_error(format!(
                "Maximum concurrent tasks exceeded: {}/{}",
                previous_tasks, self.config.max_concurrent_tasks
            )));
        }

        // Check other limits
        let current_queue = self.queue_size.load(Ordering::Relaxed);
        if current_queue >= self.config.max_queue_size {
            // Rollback the increment
            self.active_tasks.fetch_sub(1, Ordering::SeqCst);
            return Err(Error::security_error(format!(
                "Task queue full: {}/{}",
                current_queue, self.config.max_queue_size
            )));
        }

        let current_memory = self.memory_usage.load(Ordering::Relaxed);
        if current_memory >= self.config.max_memory_usage {
            // Rollback the increment
            self.active_tasks.fetch_sub(1, Ordering::SeqCst);
            return Err(Error::security_error(format!(
                "Memory limit exceeded: {}/{} bytes",
                current_memory, self.config.max_memory_usage
            )));
        }

        // Check global timeout
        if self.start_time.elapsed() >= self.config.global_timeout {
            // Rollback the increment
            self.active_tasks.fetch_sub(1, Ordering::SeqCst);
            return Err(Error::security_error(format!(
                "Global timeout exceeded: {:?}",
                self.config.global_timeout
            )));
        }

        Ok(())
    }

    /// Release a reserved task slot
    fn release_task_slot(&self) {
        self.active_tasks.fetch_sub(1, Ordering::SeqCst);
    }

    /// Increment active task count
    fn add_task(&self) {
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
        // Estimate memory usage per task (rough estimate)
        self.memory_usage.fetch_add(1024, Ordering::Relaxed);
    }

    /// Decrement active task count
    fn remove_task(&self) {
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.memory_usage.fetch_sub(1024, Ordering::Relaxed);
    }

    /// Increment queue size
    fn add_to_queue(&self) {
        self.queue_size.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement queue size
    fn remove_from_queue(&self) {
        self.queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current resource usage stats
    fn get_stats(&self) -> ResourceStats {
        ResourceStats {
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
            queue_size: self.queue_size.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            uptime: self.start_time.elapsed(),
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub active_tasks: usize,
    pub queue_size: usize,
    pub memory_usage: usize,
    pub uptime: Duration,
}

/// Bounded task queue with configurable limits
#[derive(Debug)]
pub struct BoundedTaskQueue {
    queue: VecDeque<TaskId>,
    max_size: usize,
    eviction_policy: EvictionPolicy,
}

impl BoundedTaskQueue {
    fn new(max_size: usize, policy: EvictionPolicy) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size.min(1000)), // Cap initial capacity
            max_size,
            eviction_policy: policy,
        }
    }

    /// Add a task to the queue, handling overflow according to policy
    fn push(&mut self, task_id: TaskId) -> Result<Option<TaskId>> {
        if self.queue.len() >= self.max_size {
            match self.eviction_policy {
                EvictionPolicy::Fifo => {
                    // Evict oldest task
                    let evicted = self.queue.pop_front();
                    self.queue.push_back(task_id);
                    Ok(evicted)
                }
                EvictionPolicy::Reject => {
                    // Reject new task
                    Err(Error::security_error(format!(
                        "Task queue full, rejecting new task: {}",
                        self.max_size
                    )))
                }
                EvictionPolicy::Priority => {
                    // Not implemented yet, fall back to FIFO
                    let evicted = self.queue.pop_front();
                    self.queue.push_back(task_id);
                    Ok(evicted)
                }
            }
        } else {
            self.queue.push_back(task_id);
            Ok(None)
        }
    }

    /// Remove a task from the front of the queue
    fn pop(&mut self) -> Option<TaskId> {
        self.queue.pop_front()
    }

    /// Check if queue is empty
    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get current queue size
    fn len(&self) -> usize {
        self.queue.len()
    }

    /// Clear the queue
    fn clear(&mut self) {
        self.queue.clear();
    }
}

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
        let mut result = self
            .result
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on shared result"))?;
        *result = Some(value);
        self.completed.store(true, Ordering::SeqCst);
        self.completion.notify_all();
        Ok(())
    }

    /// Wait for the result and return it
    pub fn wait_for_result(&self) -> Result<T> {
        let mut result = self
            .result
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on shared result"))?;
        while result.is_none() {
            result = self
                .completion
                .wait(result)
                .map_err(|_| Error::lock_poisoned("Condition variable wait failed"))?;
        }
        result
            .take()
            .ok_or_else(|| Error::internal("Shared result was empty after wait completed"))
    }

    /// Wait for the result with a timeout
    pub fn wait_for_result_timeout(&self, timeout: Duration) -> Result<Option<T>> {
        let mut result = self
            .result
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on shared result"))?;
        let start = Instant::now();

        while result.is_none() && start.elapsed() < timeout {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            let (guard, timeout_result) = self
                .completion
                .wait_timeout(result, remaining)
                .map_err(|_| Error::lock_poisoned("Condition variable wait timeout failed"))?;
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
    /// Atomic flag to prevent double-execution
    is_running: AtomicBool,
    /// Atomic flag to mark as completed
    is_completed: AtomicBool,
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
    /// Ready queue protected by mutex with bounds checking
    ready_queue: Mutex<BoundedTaskQueue>,
    /// Condition variable for waking the executor
    wake_signal: Condvar,
    /// Flag to check if executor should shut down
    shutdown: AtomicBool,
    /// Resource monitor for tracking usage
    monitor: ResourceMonitor,
    /// Configuration for runtime limits
    config: AsyncRuntimeConfig,
}

/// The async executor that runs tasks
pub struct Executor {
    tasks: Vec<Option<Arc<Task>>>,
    next_id: usize,
    shared: Arc<ExecutorShared>,
    /// Maximum number of tasks to prevent unbounded growth
    max_tasks: usize,
}

impl ExecutorShared {
    fn new() -> Self {
        Self::with_config(AsyncRuntimeConfig::default())
    }

    fn with_config(config: AsyncRuntimeConfig) -> Self {
        let monitor = ResourceMonitor::new(config.clone());
        ExecutorShared {
            ready_queue: Mutex::new(BoundedTaskQueue::new(
                config.max_queue_size,
                config.eviction_policy,
            )),
            wake_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
            monitor,
            config,
        }
    }

    fn wake_task(&self, task_id: TaskId) -> Result<()> {
        // Check resource limits before adding to queue
        self.monitor.can_add_task()?;

        let mut queue = self
            .ready_queue
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on ready queue"))?;

        // Add to bounded queue, handling eviction if necessary
        match queue.push(task_id)? {
            Some(evicted_task) => {
                // Log evicted task (in production, this should be properly logged)
                eprintln!(
                    "Warning: Task {:?} was evicted due to queue overflow",
                    evicted_task
                );
            }
            None => {
                // Task added successfully
                self.monitor.add_to_queue();
            }
        }

        self.wake_signal.notify_one();
        Ok(())
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.wake_signal.notify_all();

        // Clear the queue on shutdown
        if let Ok(mut queue) = self.ready_queue.lock() {
            queue.clear();
        }
    }

    /// Get current resource usage statistics
    fn get_resource_stats(&self) -> ResourceStats {
        self.monitor.get_stats()
    }

    /// Check if the executor is healthy (within resource limits)
    fn is_healthy(&self) -> bool {
        self.monitor.can_add_task().is_ok()
    }
}

impl Executor {
    /// Create a new executor with default configuration
    pub fn new() -> Arc<Mutex<Self>> {
        Self::with_config(AsyncRuntimeConfig::default())
    }

    /// Create a new executor with custom configuration
    pub fn with_config(config: AsyncRuntimeConfig) -> Arc<Mutex<Self>> {
        let max_tasks = config.max_concurrent_tasks;
        Arc::new(Mutex::new(Executor {
            tasks: Vec::with_capacity(max_tasks.min(1000)), // Cap initial capacity
            next_id: 0,
            shared: Arc::new(ExecutorShared::with_config(config)),
            max_tasks,
        }))
    }

    /// Get resource usage statistics
    pub fn get_stats(executor: Arc<Mutex<Self>>) -> Result<ResourceStats> {
        let exec = executor
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?;
        Ok(exec.shared.get_resource_stats())
    }

    /// Check if the executor is healthy
    pub fn is_healthy(executor: Arc<Mutex<Self>>) -> Result<bool> {
        let exec = executor
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?;
        Ok(exec.shared.is_healthy())
    }

    /// Spawn a new task with security checks and atomic operations
    pub fn spawn(executor: Arc<Mutex<Self>>, future: BoxedFuture<()>) -> Result<TaskId> {
        let (task_id, shared) = {
            let mut exec = executor
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?;

            // SECURITY: Atomic resource reservation to prevent TOCTOU race
            // Reserve resources BEFORE creating the task
            exec.shared.monitor.reserve_task_slot()?;

            // Check if we're at maximum task limit
            if exec.next_id >= exec.max_tasks {
                // SECURITY: Release reserved slot on failure
                exec.shared.monitor.release_task_slot();
                return Err(Error::security_error(format!(
                    "Maximum tasks reached: {}",
                    exec.max_tasks
                )));
            }

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
                is_running: AtomicBool::new(false),
                is_completed: AtomicBool::new(false),
            });

            // Ensure tasks vector is large enough but bounded
            if task_id.0 >= exec.tasks.len() {
                let new_size = (task_id.0 + 1).min(exec.max_tasks);
                exec.tasks.resize(new_size, None);
            }

            exec.tasks[task_id.0] = Some(task);
            // SECURITY: Don't call add_task here - we already reserved the slot
            // Just update memory usage
            exec.shared
                .monitor
                .memory_usage
                .fetch_add(1024, Ordering::Relaxed);
            (task_id, exec.shared.clone())
        };

        // Wake the task outside the lock
        shared.wake_task(task_id)?;
        Ok(task_id)
    }

    /// Run the executor until all tasks complete or timeout
    pub fn run(executor: Arc<Mutex<Self>>) -> Result<()> {
        let shared = executor
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?
            .shared
            .clone();

        loop {
            // Check for shutdown
            if shared.shutdown.load(Ordering::Relaxed) {
                return Ok(());
            }

            // Check for global timeout
            if shared.monitor.start_time.elapsed() >= shared.config.global_timeout {
                return Err(Error::security_error(format!(
                    "Global timeout exceeded: {:?}",
                    shared.config.global_timeout
                )));
            }

            let task_info = {
                let mut queue = shared
                    .ready_queue
                    .lock()
                    .map_err(|_| Error::lock_poisoned("Failed to acquire lock on ready queue"))?;

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

                    // Check for global timeout before waiting
                    if shared.monitor.start_time.elapsed() >= shared.config.global_timeout {
                        return Err(Error::security_error(format!(
                            "Global timeout exceeded while waiting for tasks: {:?}",
                            shared.config.global_timeout
                        )));
                    }

                    // Wait for wake signal with timeout
                    let wait_timeout = Duration::from_millis(100); // Short timeout to check global timeout regularly
                    let (new_queue, _timeout_result) = shared
                        .wake_signal
                        .wait_timeout(queue, wait_timeout)
                        .map_err(|_| Error::lock_poisoned("Condition variable wait failed"))?;

                    queue = new_queue;

                    // Continue loop to check timeout even if we timed out on wait
                    // This ensures we don't block indefinitely
                }

                if shared.shutdown.load(Ordering::Relaxed) {
                    return Ok(());
                }

                // Get next ready task from bounded queue
                if let Some(task_id) = queue.pop() {
                    shared.monitor.remove_from_queue(); // Update queue size tracking
                    let exec = executor
                        .lock()
                        .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?;
                    exec.tasks
                        .get(task_id.0)
                        .and_then(|t| t.clone())
                        .map(|task| (task_id, task))
                } else {
                    None
                }
            };

            if let Some((task_id, task)) = task_info {
                // Check timeout before executing task
                if shared.monitor.start_time.elapsed() >= shared.config.global_timeout {
                    return Err(Error::security_error(format!(
                        "Global timeout exceeded before executing task: {:?}",
                        shared.config.global_timeout
                    )));
                }

                // Check if task is already completed or running (race condition protection)
                if task.is_completed.load(Ordering::Relaxed) {
                    continue; // Skip already completed tasks
                }

                // Try to mark as running atomically
                if task
                    .is_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
                    .is_err()
                {
                    continue; // Task is already running, skip
                }

                let mut future = task
                    .future
                    .lock()
                    .map_err(|_| Error::lock_poisoned("Failed to acquire lock on task future"))?;
                let waker = create_waker(task.waker.clone());

                match future.poll(&waker) {
                    Poll::Ready(()) => {
                        // Task completed, mark as completed and remove it
                        task.is_completed.store(true, Ordering::SeqCst);
                        drop(future);
                        let mut exec = executor.lock().map_err(|_| {
                            Error::lock_poisoned("Failed to acquire lock on executor")
                        })?;
                        exec.tasks[task_id.0] = None;
                        exec.shared.monitor.remove_task(); // Update resource monitor
                    }
                    Poll::Pending => {
                        // Task not ready, will be re-queued when woken
                        task.is_running.store(false, Ordering::SeqCst); // Mark as not running
                    }
                }
            }
        }
    }

    /// Shutdown the executor
    pub fn shutdown(executor: Arc<Mutex<Self>>) -> Result<()> {
        let shared = executor
            .lock()
            .map_err(|_| Error::lock_poisoned("Failed to acquire lock on executor"))?
            .shared
            .clone();
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
    // SECURITY: Validate pointer before use
    if data.is_null() {
        // Return a no-op waker if pointer is invalid
        return std::task::RawWaker::new(std::ptr::null(), &NOOP_WAKER_VTABLE);
    }

    // SAFETY: We increment the refcount without consuming the Arc
    // This prevents use-after-free by maintaining proper reference counting
    let waker_ptr = data as *const TaskWaker;
    Arc::increment_strong_count(waker_ptr);
    let waker = Arc::from_raw(waker_ptr);
    let cloned = waker.clone();
    let _ = Arc::into_raw(waker); // Restore original refcount

    std::task::RawWaker::new(Arc::into_raw(cloned) as *const (), &WAKER_VTABLE)
}

unsafe fn wake_waker(data: *const ()) {
    // SECURITY: Validate pointer before use
    if data.is_null() {
        return;
    }

    // SAFETY: This consumes the Arc, properly decrementing refcount
    let waker = Arc::from_raw(data as *const TaskWaker);
    waker.wake();
}

unsafe fn wake_by_ref_waker(data: *const ()) {
    // SECURITY: Validate pointer before use
    if data.is_null() {
        return;
    }

    // SAFETY: Temporarily borrow the Arc without consuming it
    let waker_ptr = data as *const TaskWaker;
    Arc::increment_strong_count(waker_ptr);
    let waker = Arc::from_raw(waker_ptr);
    waker.wake();
    Arc::decrement_strong_count(waker_ptr); // Restore original refcount
}

unsafe fn drop_waker(data: *const ()) {
    // SECURITY: Validate pointer before dropping
    if !data.is_null() {
        // SAFETY: This properly decrements the refcount and drops if zero
        drop(Arc::from_raw(data as *const TaskWaker));
    }
}

// No-op waker vtable for invalid pointers
static NOOP_WAKER_VTABLE: std::task::RawWakerVTable =
    std::task::RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

unsafe fn noop_clone(_: *const ()) -> std::task::RawWaker {
    std::task::RawWaker::new(std::ptr::null(), &NOOP_WAKER_VTABLE)
}

unsafe fn noop_wake(_: *const ()) {}

unsafe fn noop_drop(_: *const ()) {}

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
        let config = AsyncRuntimeConfig::default();
        let shared = Arc::new(ExecutorShared::with_config(config));

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
        Self::block_on_timeout(future, Duration::from_secs(30))
            .map(|opt| opt.ok_or_else(|| Error::security_error("Operation timed out")))
            .and_then(|x| x)
    }

    /// Block on a future until it completes, returning the result (internal implementation)
    fn block_on_internal<T>(future: BoxedFuture<T>) -> Result<T>
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
                        let _ = self.result_storage.set_result(value);
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
            let mut exec = executor
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on blocking executor"))?;
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
                is_running: AtomicBool::new(false),
                is_completed: AtomicBool::new(false),
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
            let exec = executor
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on blocking executor"))?;
            exec.shared.shutdown.store(true, Ordering::SeqCst);
            exec.shared.wake_signal.notify_all();
        }

        // Wait for the thread to finish
        handle
            .join()
            .map_err(|_| Error::async_error("Failed to join executor thread"))?;

        Ok(result)
    }

    /// Block on a future with a timeout
    pub fn block_on_timeout<T>(future: BoxedFuture<T>, timeout: Duration) -> Result<Option<T>>
    where
        T: Send + 'static,
    {
        // Validate timeout
        if timeout.is_zero() {
            return Err(Error::security_error("Timeout cannot be zero"));
        }

        // Cap timeout to reasonable maximum (5 minutes)
        let max_timeout = Duration::from_secs(300);
        let actual_timeout = timeout.min(max_timeout);

        if timeout > max_timeout {
            eprintln!(
                "Warning: Timeout capped from {:?} to {:?}",
                timeout, max_timeout
            );
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
                        let _ = self.result_storage.set_result(value);
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
            let mut exec = executor
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on blocking executor"))?;
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
                is_running: AtomicBool::new(false),
                is_completed: AtomicBool::new(false),
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
        let result = result_storage.wait_for_result_timeout(actual_timeout)?;

        // Clean up the executor thread
        {
            let exec = executor
                .lock()
                .map_err(|_| Error::lock_poisoned("Failed to acquire lock on blocking executor"))?;
            exec.shared.shutdown.store(true, Ordering::SeqCst);
            exec.shared.wake_signal.notify_all();
        }

        // Wait for the thread to finish
        handle
            .join()
            .map_err(|_| Error::async_error("Failed to join executor thread"))?;

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

                queue.pop()
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
        let config = AsyncRuntimeConfig::default();
        let shared = Arc::new(ExecutorShared::with_config(config));

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
