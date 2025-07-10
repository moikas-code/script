use crate::runtime::closure::Closure;
use crate::runtime::value::Value;
use crate::runtime::RuntimeError;
use crate::runtime::ScriptRc;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Async generator state
#[derive(Debug, Clone)]
pub enum AsyncGeneratorState {
    /// Generator is ready to run
    Start,
    /// Generator is yielding a value
    Yielding(Value),
    /// Generator is complete
    Complete,
    /// Generator encountered an error
    Error(RuntimeError),
}

/// Async generator instance
#[derive(Debug, Clone)]
pub struct AsyncGenerator {
    /// The closure that implements the generator logic
    closure: ScriptRc<Closure>,
    /// Current state of the generator
    state: Arc<Mutex<AsyncGeneratorState>>,
    /// Values yielded but not yet consumed
    buffer: Arc<Mutex<VecDeque<Value>>>,
    /// Generator ID for debugging
    id: String,
}

impl AsyncGenerator {
    /// Create a new async generator from a closure
    pub fn new(closure: ScriptRc<Closure>, id: String) -> Self {
        Self {
            closure,
            state: Arc::new(Mutex::new(AsyncGeneratorState::Start)),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            id,
        }
    }

    /// Yield a value from the generator
    pub async fn yield_value(&self, value: Value) -> Result<(), RuntimeError> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push_back(value);
        Ok(())
    }

    /// Get the next value from the generator
    pub async fn next(&self) -> Option<Value> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.pop_front()
    }

    /// Check if the generator is complete
    pub fn is_complete(&self) -> bool {
        matches!(*self.state.lock().unwrap(), AsyncGeneratorState::Complete)
    }

    /// Mark the generator as complete
    pub fn complete(&self) {
        *self.state.lock().unwrap() = AsyncGeneratorState::Complete;
    }

    /// Get the generator ID
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Future implementation for async generator iteration
pub struct AsyncGeneratorNext {
    generator: Arc<AsyncGenerator>,
}

impl AsyncGeneratorNext {
    pub fn new(generator: Arc<AsyncGenerator>) -> Self {
        Self { generator }
    }
}

impl Future for AsyncGeneratorNext {
    type Output = Option<Value>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let generator = &self.generator;

        // Check if we have buffered values
        if let Some(value) = generator.buffer.lock().unwrap().pop_front() {
            return Poll::Ready(Some(value));
        }

        // Check if generator is complete
        if generator.is_complete() {
            return Poll::Ready(None);
        }

        // Otherwise, we need to wait for more values
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Runtime support for async generators
pub struct AsyncGeneratorRuntime {
    /// Active generators
    generators: Arc<Mutex<Vec<Arc<AsyncGenerator>>>>,
}

impl AsyncGeneratorRuntime {
    pub fn new() -> Self {
        Self {
            generators: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new async generator
    pub fn create_generator(&self, closure: ScriptRc<Closure>) -> Arc<AsyncGenerator> {
        let id = format!("async_gen_{}", uuid::Uuid::new_v4());
        let generator = Arc::new(AsyncGenerator::new(closure, id));
        self.generators.lock().unwrap().push(Arc::clone(&generator));
        generator
    }

    /// Get all active generators
    pub fn active_generators(&self) -> Vec<Arc<AsyncGenerator>> {
        self.generators.lock().unwrap().clone()
    }

    /// Clean up completed generators
    pub fn cleanup(&self) {
        let mut generators = self.generators.lock().unwrap();
        generators.retain(|g| !g.is_complete());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::closure::Closure;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_async_generator_basic() {
        let closure = ScriptRc::new(Closure::new("test_gen".to_string(), vec![], HashMap::new()));
        let generator = AsyncGenerator::new(closure, "test".to_string());

        // Yield some values
        generator.yield_value(Value::I32(1)).await.unwrap();
        generator.yield_value(Value::I32(2)).await.unwrap();
        generator.yield_value(Value::I32(3)).await.unwrap();

        // Consume values
        assert_eq!(generator.next().await, Some(Value::I32(1)));
        assert_eq!(generator.next().await, Some(Value::I32(2)));
        assert_eq!(generator.next().await, Some(Value::I32(3)));
        assert_eq!(generator.next().await, None);
    }

    #[tokio::test]
    async fn test_async_generator_runtime() {
        let runtime = AsyncGeneratorRuntime::new();
        let closure = ScriptRc::new(Closure::new("test_gen".to_string(), vec![], HashMap::new()));

        let generator = runtime.create_generator(closure);
        assert_eq!(runtime.active_generators().len(), 1);

        generator.complete();
        runtime.cleanup();
        assert_eq!(runtime.active_generators().len(), 0);
    }
}
