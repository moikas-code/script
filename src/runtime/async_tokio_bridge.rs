//! Bridge between Script's async runtime and Tokio
//!
//! This module provides integration between Script's custom async runtime
//! and the Tokio async runtime, allowing Script programs to leverage
//! Tokio's mature ecosystem while maintaining security controls.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use tokio::task::JoinHandle;

use crate::error::{Error, ErrorKind, Result};
use crate::runtime::async_runtime::AsyncRuntimeConfig;
use crate::runtime::Value;

/// Tokio runtime configuration for Script
#[derive(Debug, Clone)]
pub struct TokioConfig {
    /// Number of worker threads (default: number of CPU cores)
    pub worker_threads: usize,
    /// Stack size for worker threads (default: 2MB)
    pub thread_stack_size: usize,
    /// Enable I/O driver (default: true)
    pub enable_io: bool,
    /// Enable time driver (default: true)
    pub enable_time: bool,
    /// Thread name prefix (default: "script-worker")
    pub thread_name_prefix: String,
    /// Maximum blocking threads (default: 512)
    pub max_blocking_threads: usize,
}

impl Default for TokioConfig {
    fn default() -> Self {
        TokioConfig {
            worker_threads: num_cpus::get(),
            thread_stack_size: 2 * 1024 * 1024, // 2MB
            enable_io: true,
            enable_time: true,
            thread_name_prefix: "script-worker".to_string(),
            max_blocking_threads: 512,
        }
    }
}

/// Tokio runtime bridge for Script
pub struct TokioBridge {
    /// The Tokio runtime instance
    runtime: Arc<Runtime>,
    /// Configuration for security limits
    config: AsyncRuntimeConfig,
    /// Active task counter for resource monitoring
    active_tasks: Arc<std::sync::atomic::AtomicUsize>,
}

impl TokioBridge {
    /// Create a new Tokio bridge with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(TokioConfig::default(), AsyncRuntimeConfig::default())
    }

    /// Create a new Tokio bridge with custom configuration
    pub fn with_config(
        tokio_config: TokioConfig,
        async_config: AsyncRuntimeConfig,
    ) -> Result<Self> {
        let mut builder = RuntimeBuilder::new_multi_thread();

        builder
            .worker_threads(tokio_config.worker_threads)
            .thread_stack_size(tokio_config.thread_stack_size)
            .thread_name(tokio_config.thread_name_prefix)
            .max_blocking_threads(tokio_config.max_blocking_threads);

        if tokio_config.enable_io {
            builder.enable_io();
        }

        if tokio_config.enable_time {
            builder.enable_time();
        }

        let runtime = builder.build().map_err(|e| {
            Error::new(
                ErrorKind::RuntimeError,
                format!("Failed to create Tokio runtime: {e}"),
            )
        })?;

        Ok(TokioBridge {
            runtime: Arc::new(runtime),
            config: async_config,
            active_tasks: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Spawn a future on the Tokio runtime with security checks
    pub fn spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // Check resource limits
        let current_tasks = self.active_tasks.load(std::sync::atomic::Ordering::Relaxed);
        if current_tasks >= self.config.max_concurrent_tasks {
            return Err(Error::security_error(format!(
                "Maximum concurrent tasks exceeded: {}/{}",
                current_tasks, self.config.max_concurrent_tasks
            )));
        }

        // Increment task counter
        self.active_tasks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Clone Arc for the spawned task
        let active_tasks = self.active_tasks.clone();

        // Wrap the future to decrement counter on completion
        let wrapped_future = async move {
            let result = future.await;
            active_tasks.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            result
        };

        // Spawn on Tokio runtime
        Ok(self.runtime.spawn(wrapped_future))
    }

    /// Block on a future using the Tokio runtime
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        self.runtime.block_on(future)
    }

    /// Spawn a blocking task on the Tokio runtime
    pub fn spawn_blocking<F, R>(&self, f: F) -> Result<JoinHandle<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Check resource limits
        let current_tasks = self.active_tasks.load(std::sync::atomic::Ordering::Relaxed);
        if current_tasks >= self.config.max_concurrent_tasks {
            return Err(Error::security_error(format!(
                "Maximum concurrent tasks exceeded: {}/{}",
                current_tasks, self.config.max_concurrent_tasks
            )));
        }

        // Increment task counter
        self.active_tasks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Clone Arc for the spawned task
        let active_tasks = self.active_tasks.clone();

        // Wrap the function to decrement counter on completion
        let wrapped_fn = move || {
            let result = f();
            active_tasks.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            result
        };

        Ok(self.runtime.spawn_blocking(wrapped_fn))
    }

    /// Execute an async closure from Script code
    pub async fn execute_async_closure(
        &self,
        closure: &crate::runtime::closure::Closure,
        args: &[Value],
    ) -> Result<Value> {
        // Create a oneshot channel for the result
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Clone what we need for the async block
        let _closure = closure.clone();
        let _args = args.to_vec();

        // Spawn the closure execution
        self.spawn(async move {
            // This is where we'd integrate with the closure runtime
            // For now, return a placeholder
            let _ = tx.send(Ok(Value::Null));
        })?;

        // Wait for the result with timeout
        match tokio::time::timeout(self.config.global_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::new(
                ErrorKind::RuntimeError,
                "Async closure execution cancelled",
            )),
            Err(_) => Err(Error::security_error(format!(
                "Async closure execution timed out after {:?}",
                self.config.global_timeout
            ))),
        }
    }

    /// Get the number of active tasks
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Shutdown the Tokio runtime gracefully
    pub fn shutdown(self) {
        // Note: dropping the runtime will shut it down
        drop(self.runtime);
    }
}

/// Script Future wrapper for Tokio compatibility
pub struct ScriptFuture<T> {
    inner: Pin<Box<dyn Future<Output = Result<T>> + Send>>,
}

impl<T> ScriptFuture<T> {
    /// Create a new Script future
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = Result<T>> + Send + 'static,
    {
        ScriptFuture {
            inner: Box::pin(future),
        }
    }
}

impl<T> Future for ScriptFuture<T> {
    type Output = Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

/// Global Tokio runtime instance for Script
static GLOBAL_RUNTIME: std::sync::OnceLock<TokioBridge> = std::sync::OnceLock::new();

/// Initialize the global Tokio runtime
pub fn init_global_runtime() -> Result<()> {
    GLOBAL_RUNTIME.get_or_init(|| TokioBridge::new().expect("Failed to initialize Tokio runtime"));
    Ok(())
}

/// Get the global Tokio runtime
pub fn global_runtime() -> Result<&'static TokioBridge> {
    GLOBAL_RUNTIME
        .get()
        .ok_or_else(|| Error::new(ErrorKind::RuntimeError, "Tokio runtime not initialized"))
}

/// Spawn a future on the global runtime
pub fn spawn<F>(future: F) -> Result<JoinHandle<F::Output>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    global_runtime()?.spawn(future)
}

/// Block on a future using the global runtime
pub fn block_on<F>(future: F) -> Result<F::Output>
where
    F: Future,
{
    Ok(global_runtime()?.block_on(future))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokio_bridge_creation() {
        let bridge = TokioBridge::new().unwrap();
        assert_eq!(bridge.active_task_count(), 0);
    }

    #[test]
    fn test_spawn_and_block_on() {
        let bridge = TokioBridge::new().unwrap();

        let result = bridge.block_on(async {
            let handle = bridge.spawn(async { 42 }).unwrap();

            handle.await.unwrap()
        });

        assert_eq!(result, 42);
    }

    #[test]
    fn test_resource_limits() {
        let mut config = AsyncRuntimeConfig::default();
        config.max_concurrent_tasks = 2;

        let bridge = TokioBridge::with_config(TokioConfig::default(), config).unwrap();

        // Spawn two tasks (at limit)
        let _h1 = bridge
            .spawn(async { tokio::time::sleep(tokio::time::Duration::from_millis(100)).await })
            .unwrap();
        let _h2 = bridge
            .spawn(async { tokio::time::sleep(tokio::time::Duration::from_millis(100)).await })
            .unwrap();

        // Third task should fail
        let result = bridge.spawn(async { 42 });
        assert!(result.is_err());
    }
}
