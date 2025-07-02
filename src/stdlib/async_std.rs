use crate::runtime::{BoxedFuture, ScriptFuture, Timer};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::Duration;

/// Sleep for the specified duration
pub struct Sleep {
    timer: Timer,
}

impl Sleep {
    /// Create a new sleep future
    pub fn new(duration: Duration) -> Self {
        Sleep {
            timer: Timer::new(duration),
        }
    }
}

impl ScriptFuture for Sleep {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        self.timer.poll(waker)
    }
}

/// Timeout wrapper for futures
pub struct Timeout<F: ScriptFuture> {
    future: F,
    timer: Timer,
    completed: bool,
}

impl<F: ScriptFuture> Timeout<F> {
    /// Create a new timeout future
    pub fn new(future: F, duration: Duration) -> Self {
        Timeout {
            future,
            timer: Timer::new(duration),
            completed: false,
        }
    }
}

impl<F: ScriptFuture> ScriptFuture for Timeout<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if self.completed {
            return Poll::Ready(Err(TimeoutError));
        }

        // First check if the future is ready
        match self.future.poll(waker) {
            Poll::Ready(value) => {
                self.completed = true;
                Poll::Ready(Ok(value))
            }
            Poll::Pending => {
                // Check if we've timed out
                match self.timer.poll(waker) {
                    Poll::Ready(()) => {
                        self.completed = true;
                        Poll::Ready(Err(TimeoutError))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation timed out")
    }
}

impl std::error::Error for TimeoutError {}

/// Yield control back to the executor
pub struct Yield {
    yielded: bool,
}

impl Yield {
    pub fn new() -> Self {
        Yield { yielded: false }
    }
}

impl ScriptFuture for Yield {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if !self.yielded {
            self.yielded = true;
            waker.wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// Race two futures, returning the result of the first to complete
pub struct Race<F1: ScriptFuture, F2: ScriptFuture> {
    future1: Option<F1>,
    future2: Option<F2>,
}

impl<F1: ScriptFuture, F2: ScriptFuture> Race<F1, F2> {
    pub fn new(future1: F1, future2: F2) -> Self {
        Race {
            future1: Some(future1),
            future2: Some(future2),
        }
    }
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<F1: ScriptFuture, F2: ScriptFuture> ScriptFuture for Race<F1, F2> {
    type Output = Either<F1::Output, F2::Output>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if let Some(mut f1) = self.future1.take() {
            match f1.poll(waker) {
                Poll::Ready(value) => return Poll::Ready(Either::Left(value)),
                Poll::Pending => self.future1 = Some(f1),
            }
        }

        if let Some(mut f2) = self.future2.take() {
            match f2.poll(waker) {
                Poll::Ready(value) => return Poll::Ready(Either::Right(value)),
                Poll::Pending => self.future2 = Some(f2),
            }
        }

        Poll::Pending
    }
}

/// Join multiple futures in parallel
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
        let mut all_done = true;

        for (i, future) in self.futures.iter_mut().enumerate() {
            if self.results[i].is_none() {
                match future.poll(waker) {
                    Poll::Ready(value) => {
                        self.results[i] = Some(value);
                    }
                    Poll::Pending => {
                        all_done = false;
                    }
                }
            }
        }

        if all_done {
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

/// Interval timer that yields periodically
pub struct Interval {
    timer: Timer,
    period: Duration,
    first: bool,
}

impl Interval {
    pub fn new(period: Duration) -> Self {
        Interval {
            timer: Timer::new(period),
            period,
            first: true,
        }
    }
}

impl ScriptFuture for Interval {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if self.first {
            self.first = false;
            return Poll::Ready(());
        }

        match self.timer.poll(waker) {
            Poll::Ready(()) => {
                // Reset the timer for the next interval
                self.timer = Timer::new(self.period);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Channel for async communication
pub struct Channel<T> {
    queue: Arc<Mutex<Vec<T>>>,
    wakers: Arc<Mutex<Vec<Waker>>>,
}

impl<T> Channel<T> {
    pub fn new() -> (Sender<T>, Receiver<T>) {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let wakers = Arc::new(Mutex::new(Vec::new()));

        let sender = Sender {
            queue: queue.clone(),
            wakers: wakers.clone(),
        };

        let receiver = Receiver { queue, wakers };

        (sender, receiver)
    }
}

pub struct Sender<T> {
    queue: Arc<Mutex<Vec<T>>>,
    wakers: Arc<Mutex<Vec<Waker>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(value);

        // Wake all waiting receivers
        let mut wakers = self.wakers.lock().unwrap();
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }
}

pub struct Receiver<T> {
    queue: Arc<Mutex<Vec<T>>>,
    wakers: Arc<Mutex<Vec<Waker>>>,
}

pub struct RecvFuture<T> {
    receiver: Arc<Receiver<T>>,
}

impl<T> ScriptFuture for RecvFuture<T> {
    type Output = Option<T>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        let mut queue = self.receiver.queue.lock().unwrap();
        if let Some(value) = queue.pop() {
            Poll::Ready(Some(value))
        } else {
            // Register waker
            let mut wakers = self.receiver.wakers.lock().unwrap();
            wakers.push(waker.clone());
            Poll::Pending
        }
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> RecvFuture<T> {
        RecvFuture {
            receiver: Arc::new(unsafe { std::ptr::read(self) }),
        }
    }
}

/// Async mutex for synchronization
pub struct AsyncMutex<T> {
    inner: Arc<Mutex<AsyncMutexInner<T>>>,
}

struct AsyncMutexInner<T> {
    value: Option<T>,
    waiters: Vec<Waker>,
}

impl<T> AsyncMutex<T> {
    pub fn new(value: T) -> Self {
        AsyncMutex {
            inner: Arc::new(Mutex::new(AsyncMutexInner {
                value: Some(value),
                waiters: Vec::new(),
            })),
        }
    }

    pub fn lock(&self) -> AsyncMutexLockFuture<T> {
        AsyncMutexLockFuture {
            mutex: self.inner.clone(),
        }
    }
}

pub struct AsyncMutexLockFuture<T> {
    mutex: Arc<Mutex<AsyncMutexInner<T>>>,
}

impl<T> ScriptFuture for AsyncMutexLockFuture<T> {
    type Output = AsyncMutexGuard<T>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        let mut inner = self.mutex.lock().unwrap();

        if let Some(value) = inner.value.take() {
            Poll::Ready(AsyncMutexGuard {
                mutex: self.mutex.clone(),
                value: Some(value),
            })
        } else {
            inner.waiters.push(waker.clone());
            Poll::Pending
        }
    }
}

pub struct AsyncMutexGuard<T> {
    mutex: Arc<Mutex<AsyncMutexInner<T>>>,
    value: Option<T>,
}

impl<T> Drop for AsyncMutexGuard<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            let mut inner = self.mutex.lock().unwrap();
            inner.value = Some(value);

            // Wake one waiter
            if let Some(waker) = inner.waiters.pop() {
                waker.wake();
            }
        }
    }
}

// Helper functions to create async stdlib futures

/// Sleep for the specified number of milliseconds
pub fn sleep(millis: u64) -> Sleep {
    Sleep::new(Duration::from_millis(millis))
}

/// Create a timeout future
pub fn timeout<F: ScriptFuture>(future: F, millis: u64) -> Timeout<F> {
    Timeout::new(future, Duration::from_millis(millis))
}

/// Yield control to the executor
pub fn yield_now() -> Yield {
    Yield::new()
}

/// Race two futures
pub fn race<F1: ScriptFuture, F2: ScriptFuture>(f1: F1, f2: F2) -> Race<F1, F2> {
    Race::new(f1, f2)
}

/// Join all futures
pub fn join_all<T>(futures: Vec<BoxedFuture<T>>) -> JoinAll<T> {
    JoinAll::new(futures)
}

/// Create an interval timer
pub fn interval(millis: u64) -> Interval {
    Interval::new(Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_future() {
        let mut sleep = sleep(100);
        let waker = unsafe {
            std::task::Waker::from_raw(std::task::RawWaker::new(
                std::ptr::null(),
                &std::task::RawWakerVTable::new(
                    |_| {
                        std::task::RawWaker::new(
                            std::ptr::null(),
                            &std::task::RawWakerVTable::new(|_| panic!(), |_| {}, |_| {}, |_| {}),
                        )
                    },
                    |_| {},
                    |_| {},
                    |_| {},
                ),
            ))
        };

        // First poll should return pending
        assert!(matches!(sleep.poll(&waker), Poll::Pending));
    }

    #[test]
    fn test_yield_future() {
        let mut yield_future = yield_now();
        let waker = unsafe {
            std::task::Waker::from_raw(std::task::RawWaker::new(
                std::ptr::null(),
                &std::task::RawWakerVTable::new(
                    |_| {
                        std::task::RawWaker::new(
                            std::ptr::null(),
                            &std::task::RawWakerVTable::new(|_| panic!(), |_| {}, |_| {}, |_| {}),
                        )
                    },
                    |_| {},
                    |_| {},
                    |_| {},
                ),
            ))
        };

        // First poll should return pending
        assert!(matches!(yield_future.poll(&waker), Poll::Pending));

        // Second poll should return ready
        assert!(matches!(yield_future.poll(&waker), Poll::Ready(())));
    }
}
