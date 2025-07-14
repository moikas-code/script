use crossbeam::deque::{Injector, Steal, Worker};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use super::async_runtime::{BoxedFuture, ScriptFuture, TaskId};
use std::task::{Poll, Waker};

/// A work-stealing task scheduler for async execution
pub struct Scheduler {
    /// Global queue for tasks
    injector: Arc<Injector<Task>>,
    /// Worker threads
    workers: Vec<WorkerHandle>,
    /// Number of worker threads
    num_workers: usize,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Wake signal for workers
    wake_signal: Arc<(Mutex<bool>, Condvar)>,
}

/// Handle to a worker thread
struct WorkerHandle {
    thread: Option<thread::JoinHandle<()>>,
    stealer: crossbeam::deque::Stealer<Task>,
}

/// A scheduled task
pub struct Task {
    pub id: TaskId,
    pub future: Arc<Mutex<BoxedFuture<()>>>,
}

/// Core scheduler data shared between threads
pub struct SchedulerCore {
    injector: Arc<Injector<Task>>,
    stealers: Vec<crossbeam::deque::Stealer<Task>>,
    shutdown: Arc<AtomicBool>,
    active_tasks: AtomicUsize,
    wake_signal: Arc<(Mutex<bool>, Condvar)>,
    /// Stores tasks that need to be re-queued when woken
    task_storage: Arc<Mutex<HashMap<TaskId, Arc<Mutex<BoxedFuture<()>>>>>>,
}

impl Scheduler {
    /// Create a new scheduler with the specified number of worker threads
    pub fn new(num_workers: usize) -> Self {
        let num_workers = if num_workers == 0 {
            num_cpus::get()
        } else {
            num_workers
        };

        let injector = Arc::new(Injector::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let wake_signal = Arc::new((Mutex::new(false), Condvar::new()));

        let mut workers = Vec::with_capacity(num_workers);
        let mut stealers = Vec::with_capacity(num_workers);

        // Create worker threads
        for worker_id in 0..num_workers {
            let worker = Worker::new_fifo();
            let stealer = worker.stealer();
            stealers.push(stealer.clone());

            let injector_clone = injector.clone();
            let shutdown_clone = shutdown.clone();
            let wake_signal_clone = wake_signal.clone();
            let stealers_clone = stealers.clone();
            let core = Arc::new(SchedulerCore {
                injector: injector_clone.clone(),
                stealers: stealers_clone.clone(),
                shutdown: shutdown_clone.clone(),
                active_tasks: AtomicUsize::new(0),
                wake_signal: wake_signal_clone.clone(),
                task_storage: Arc::new(Mutex::new(HashMap::new())),
            });

            let thread = thread::Builder::new()
                .name(format!("script-worker-{worker_id}"))
                .spawn(move || {
                    worker_thread(
                        worker_id,
                        worker,
                        injector_clone,
                        &stealers_clone,
                        shutdown_clone,
                        wake_signal_clone,
                        core,
                    );
                })
                .expect("Failed to spawn worker thread");

            workers.push(WorkerHandle {
                thread: Some(thread),
                stealer,
            });
        }

        Scheduler {
            injector,
            workers,
            num_workers,
            shutdown,
            wake_signal,
        }
    }

    /// Get the shared scheduler core
    fn get_core(&self) -> Arc<SchedulerCore> {
        Arc::new(SchedulerCore {
            injector: self.injector.clone(),
            stealers: self.workers.iter().map(|w| w.stealer.clone()).collect(),
            shutdown: self.shutdown.clone(),
            active_tasks: AtomicUsize::new(0),
            wake_signal: self.wake_signal.clone(),
            task_storage: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Spawn a new async task
    pub fn spawn(&self, future: BoxedFuture<()>) -> TaskId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let task = Task {
            id,
            future: Arc::new(Mutex::new(future)),
        };

        self.injector.push(task);

        // Wake up a worker
        let (lock, cvar) = &*self.wake_signal;
        let mut woken = lock.lock().unwrap();
        *woken = true;
        cvar.notify_one();

        id
    }

    /// Shutdown the scheduler and wait for all workers to finish
    pub fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Wake all workers
        for _worker in &self.workers {
            // Signal shutdown by pushing empty tasks
            self.injector.push(Task {
                id: TaskId(usize::MAX),
                future: Arc::new(Mutex::new(Box::new(ShutdownFuture))),
            });
        }

        // Join all worker threads
        for mut worker in self.workers.drain(..) {
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("Worker thread panicked");
            }
        }
    }
}

/// Worker thread function
fn worker_thread(
    id: usize,
    worker: Worker<Task>,
    injector: Arc<Injector<Task>>,
    stealers: &[crossbeam::deque::Stealer<Task>],
    shutdown: Arc<AtomicBool>,
    wake_signal: Arc<(Mutex<bool>, Condvar)>,
    core: Arc<SchedulerCore>,
) {
    let mut local_queue = VecDeque::new();

    while !shutdown.load(Ordering::Relaxed) {
        // Try to get work from local queue first
        if let Some(task) = local_queue.pop_front() {
            execute_task(task, &core);
            continue;
        }

        // Try to pop from own worker queue
        if let Some(task) = worker.pop() {
            execute_task(task, &core);
            continue;
        }

        // Try to steal from global injector
        loop {
            match injector.steal() {
                Steal::Success(task) => {
                    execute_task(task, &core);
                    break;
                }
                Steal::Empty => break,
                Steal::Retry => continue,
            }
        }

        // Try to steal from other workers
        let mut found = false;
        for (i, stealer) in stealers.iter().enumerate() {
            if i == id {
                continue; // Don't steal from ourselves
            }

            loop {
                match stealer.steal() {
                    Steal::Success(task) => {
                        execute_task(task, &core);
                        found = true;
                        break;
                    }
                    Steal::Empty => break,
                    Steal::Retry => continue,
                }
            }

            if found {
                break;
            }
        }

        if !found {
            // No work available, wait for signal
            let (lock, cvar) = &*wake_signal;
            let mut woken = lock.lock().unwrap();
            while !*woken && !shutdown.load(Ordering::Relaxed) {
                woken = cvar
                    .wait_timeout(woken, Duration::from_millis(100))
                    .unwrap()
                    .0;
            }
            *woken = false;
        }
    }
}

/// Execute a single task
fn execute_task(task: Task, scheduler: &Arc<SchedulerCore>) {
    // Check for shutdown task
    if task.id.0 == usize::MAX {
        return;
    }

    let future_arc = task.future.clone();
    let mut future = future_arc.lock().unwrap();

    // Store the future so it can be re-queued when woken
    {
        let mut storage = scheduler.task_storage.lock().unwrap();
        storage.insert(task.id, future_arc.clone());
    }

    // Create waker for this task
    let waker = create_waker(task.id, scheduler.clone());

    match future.poll(&waker) {
        Poll::Ready(()) => {
            // Task completed, remove from storage
            let mut storage = scheduler.task_storage.lock().unwrap();
            storage.remove(&task.id);
            scheduler.active_tasks.fetch_sub(1, Ordering::Relaxed);
        }
        Poll::Pending => {
            // Task not ready, will be re-scheduled when woken
        }
    }
}

/// Waker implementation for scheduler
struct SchedulerWaker {
    task_id: TaskId,
    scheduler: Arc<SchedulerCore>,
}

impl SchedulerWaker {
    fn wake(&self) {
        // Re-queue the task by getting it from storage
        let future = {
            let storage = self.scheduler.task_storage.lock().unwrap();
            storage.get(&self.task_id).cloned()
        };

        if let Some(future) = future {
            let task = Task {
                id: self.task_id,
                future,
            };

            self.scheduler.injector.push(task);

            // Wake a worker
            let (lock, cvar) = &*self.scheduler.wake_signal;
            let mut woken = lock.lock().unwrap();
            *woken = true;
            cvar.notify_one();
        }
    }
}

/// Create a waker for a task
fn create_waker(task_id: TaskId, scheduler: Arc<SchedulerCore>) -> Waker {
    unsafe {
        std::task::Waker::from_raw(std::task::RawWaker::new(
            Arc::into_raw(Arc::new(SchedulerWaker { task_id, scheduler })) as *const (),
            &SCHEDULER_WAKER_VTABLE,
        ))
    }
}

// Waker vtable for scheduler tasks
static SCHEDULER_WAKER_VTABLE: std::task::RawWakerVTable = std::task::RawWakerVTable::new(
    clone_scheduler_waker,
    wake_scheduler_waker,
    wake_by_ref_scheduler_waker,
    drop_scheduler_waker,
);

unsafe fn clone_scheduler_waker(data: *const ()) -> std::task::RawWaker {
    let waker = Arc::from_raw(data as *const SchedulerWaker);
    let cloned = waker.clone();
    let _ = Arc::into_raw(waker); // Convert back to raw pointer without dropping
    std::task::RawWaker::new(Arc::into_raw(cloned) as *const (), &SCHEDULER_WAKER_VTABLE)
}

unsafe fn wake_scheduler_waker(data: *const ()) {
    let waker = Arc::from_raw(data as *const SchedulerWaker);
    waker.wake();
}

unsafe fn wake_by_ref_scheduler_waker(data: *const ()) {
    let waker = Arc::from_raw(data as *const SchedulerWaker);
    waker.wake();
    let _ = Arc::into_raw(waker); // Convert back to raw pointer without dropping
}

unsafe fn drop_scheduler_waker(data: *const ()) {
    drop(Arc::from_raw(data as *const SchedulerWaker));
}

/// Shutdown future that always returns ready
struct ShutdownFuture;

impl ScriptFuture for ShutdownFuture {
    type Output = ();

    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::async_runtime::Timer;

    #[test]
    fn test_scheduler_basic() {
        let scheduler = Scheduler::new(2);

        struct TestTask;
        impl ScriptFuture for TestTask {
            type Output = ();

            fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
                println!("Task executed on thread: {:?}", thread::current().name());
                Poll::Ready(())
            }
        }

        // Spawn multiple tasks
        for _i in 0..10 {
            scheduler.spawn(Box::new(TestTask));
        }

        // Give tasks time to execute
        thread::sleep(Duration::from_millis(100));

        scheduler.shutdown();
    }

    #[test]
    fn test_scheduler_with_timer() {
        let scheduler = Scheduler::new(4);

        // Spawn timer tasks
        for i in 0..5 {
            let duration = Duration::from_millis(50 * (i + 1) as u64);
            scheduler.spawn(Box::new(Timer::new(duration)));
        }

        // Wait for timers to complete
        thread::sleep(Duration::from_millis(500));

        scheduler.shutdown();
    }
}
