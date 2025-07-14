use crate::runtime::{RuntimeError, ScriptRc};
use crate::stdlib::{collections::ScriptVec, ScriptValue};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of worker threads to use
    pub num_threads: usize,
    /// Maximum work items per thread
    pub max_work_per_thread: usize,
    /// Whether to enable work stealing between threads
    pub enable_work_stealing: bool,
    /// Timeout for individual work items in milliseconds
    pub work_timeout_ms: Option<u64>,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        ParallelConfig {
            num_threads: num_cpus::get().max(1),
            max_work_per_thread: 1000,
            enable_work_stealing: true,
            work_timeout_ms: Some(30000), // 30 seconds
        }
    }
}

/// Work item for parallel execution
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub index: usize,
    pub value: ScriptValue,
}

/// Result from parallel work execution
#[derive(Debug, Clone)]
pub struct WorkResult {
    pub value: ScriptValue,
    pub index: usize,
}

/// Parallel executor for Script collections
pub struct ParallelExecutor {
    config: ParallelConfig,
}

impl ParallelExecutor {
    /// Create a new parallel executor with configuration
    pub fn new(config: ParallelConfig) -> Self {
        ParallelExecutor { config }
    }

    /// Execute parallel map operation on a vector
    pub fn parallel_map(
        &self,
        vec: &ScriptVec,
        mapper_fn: impl Fn(&ScriptValue) -> Result<ScriptValue, RuntimeError>
            + Send
            + Sync
            + Clone
            + 'static,
    ) -> Result<ScriptVec, RuntimeError> {
        let data = vec.data.read().map_err(|_| {
            RuntimeError::InvalidOperation("Failed to read vector data".to_string())
        })?;

        let work_items: Vec<WorkItem> = data
            .iter()
            .enumerate()
            .map(|(index, value)| WorkItem {
                index,
                value: value.clone(),
            })
            .collect();

        let results = self.execute_parallel_work(work_items, mapper_fn)?;

        // Sort results by original index to maintain order
        let mut sorted_results = results;
        sorted_results.sort_by_key(|r| r.index);

        let result_values: Vec<ScriptValue> = sorted_results.into_iter().map(|r| r.value).collect();

        Ok(ScriptVec::from_vec(result_values))
    }

    /// Execute parallel filter operation on a vector
    pub fn parallel_filter(
        &self,
        vec: &ScriptVec,
        predicate_fn: impl Fn(&ScriptValue) -> Result<bool, RuntimeError>
            + Send
            + Sync
            + Clone
            + 'static,
    ) -> Result<ScriptVec, RuntimeError> {
        let data = vec.data.read().map_err(|_| {
            RuntimeError::InvalidOperation("Failed to read vector data".to_string())
        })?;

        let work_items: Vec<WorkItem> = data
            .iter()
            .enumerate()
            .map(|(index, value)| WorkItem {
                index,
                value: value.clone(),
            })
            .collect();

        // Execute filter predicates in parallel
        let filter_results = self.execute_parallel_work(work_items, move |item| {
            let keep = predicate_fn(item)?;
            Ok(if keep {
                item.clone()
            } else {
                ScriptValue::Unit
            })
        })?;

        // Collect only the non-unit values, maintaining order
        let mut sorted_results = filter_results;
        sorted_results.sort_by_key(|r| r.index);

        let filtered_values: Vec<ScriptValue> = sorted_results
            .into_iter()
            .filter(|r| !matches!(r.value, ScriptValue::Unit))
            .map(|r| r.value)
            .collect();

        Ok(ScriptVec::from_vec(filtered_values))
    }

    /// Execute work items in parallel across multiple threads
    fn execute_parallel_work<F>(
        &self,
        work_items: Vec<WorkItem>,
        work_fn: F,
    ) -> Result<Vec<WorkResult>, RuntimeError>
    where
        F: Fn(&ScriptValue) -> Result<ScriptValue, RuntimeError> + Send + Sync + Clone + 'static,
    {
        let num_threads = self.config.num_threads.min(work_items.len()).max(1);
        let chunk_size = (work_items.len() + num_threads - 1) / num_threads;

        let work_chunks: Vec<Vec<WorkItem>> = work_items
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let results = Arc::new(Mutex::new(Vec::new()));
        let error = Arc::new(Mutex::new(None::<RuntimeError>));
        let mut handles = Vec::new();

        for chunk in work_chunks {
            let results_clone = Arc::clone(&results);
            let error_clone = Arc::clone(&error);
            let work_fn_clone = work_fn.clone();

            let handle = thread::spawn(move || {
                let mut thread_results = Vec::new();

                for work_item in chunk {
                    // Check if another thread encountered an error
                    if error_clone.lock().unwrap().is_some() {
                        break;
                    }

                    match work_fn_clone(&work_item.value) {
                        Ok(result_value) => {
                            thread_results.push(WorkResult {
                                value: result_value,
                                index: work_item.index,
                            });
                        }
                        Err(e) => {
                            *error_clone.lock().unwrap() = Some(e);
                            break;
                        }
                    }
                }

                // Merge thread results into global results
                let mut global_results = results_clone.lock().unwrap();
                global_results.extend(thread_results);
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle
                .join()
                .map_err(|_| RuntimeError::InvalidOperation("Thread panicked".to_string()))?;
        }

        // Check if any thread encountered an error
        if let Some(error) = error.lock().unwrap().take() {
            return Err(error);
        }

        let results = results.lock().unwrap().clone();
        Ok(results)
    }
}

/// Parallel reduce operation with associative combiner
pub fn parallel_reduce<T, F, G>(
    items: Vec<T>,
    initial: T,
    work_fn: F,
    combine_fn: G,
    config: &ParallelConfig,
) -> Result<T, RuntimeError>
where
    T: Clone + Send + 'static,
    F: Fn(T, &T) -> Result<T, RuntimeError> + Send + Sync + Clone + 'static,
    G: Fn(T, T) -> Result<T, RuntimeError> + Send + Sync + Clone + 'static,
{
    if items.is_empty() {
        return Ok(initial);
    }

    let num_threads = config.num_threads.min(items.len()).max(1);
    let chunk_size = (items.len() + num_threads - 1) / num_threads;

    let item_chunks: Vec<Vec<T>> = items
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::new();

    for chunk in item_chunks {
        let sender_clone = sender.clone();
        let work_fn_clone = work_fn.clone();
        let initial_clone = initial.clone();

        let handle = thread::spawn(move || {
            let mut accumulator = initial_clone;

            for item in &chunk {
                match work_fn_clone(accumulator, item) {
                    Ok(result) => accumulator = result,
                    Err(e) => {
                        let _ = sender_clone.send(Err(e));
                        return;
                    }
                }
            }

            let _ = sender_clone.send(Ok(accumulator));
        });

        handles.push(handle);
    }

    // Close the original sender
    drop(sender);

    // Collect results from all threads
    let mut partial_results = Vec::new();
    while let Ok(result) = receiver.recv() {
        match result {
            Ok(value) => partial_results.push(value),
            Err(e) => return Err(e),
        }
    }

    // Wait for all threads to finish
    for handle in handles {
        handle
            .join()
            .map_err(|_| RuntimeError::InvalidOperation("Thread panicked".to_string()))?;
    }

    // Combine all partial results
    let mut final_result = initial;
    for partial in partial_results {
        final_result = combine_fn(final_result, partial)?;
    }

    Ok(final_result)
}

/// Standard library function implementations for parallel operations

/// Implementation of vec_parallel_map for stdlib registry
pub(crate) fn vec_parallel_map_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_parallel_map expects 2-3 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_parallel_map must be an array".to_string(),
            ))
        }
    };

    // For now, return the original array
    // In full implementation: would execute mapper function in parallel
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_parallel_filter for stdlib registry
pub(crate) fn vec_parallel_filter_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_parallel_filter expects 2-3 arguments, got {}",
            args.len()
        )));
    }

    let vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_parallel_filter must be an array".to_string(),
            ))
        }
    };

    // For now, return the original array
    Ok(ScriptValue::Array(vec.clone()))
}

/// Implementation of vec_parallel_reduce for stdlib registry
pub(crate) fn vec_parallel_reduce_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() < 3 || args.len() > 4 {
        return Err(RuntimeError::InvalidOperation(format!(
            "vec_parallel_reduce expects 3-4 arguments, got {}",
            args.len()
        )));
    }

    let _vec = match &args[0] {
        ScriptValue::Array(v) => v,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument to vec_parallel_reduce must be an array".to_string(),
            ))
        }
    };

    let initial_value = &args[2];

    // For now, return the initial value
    Ok(initial_value.clone())
}

/// Implementation of parallel_config_create for stdlib registry
pub(crate) fn parallel_config_create_impl(
    args: &[ScriptValue],
) -> std::result::Result<ScriptValue, RuntimeError> {
    if args.len() > 4 {
        return Err(RuntimeError::InvalidOperation(format!(
            "parallel_config_create expects 0-4 arguments, got {}",
            args.len()
        )));
    }

    let mut config = ParallelConfig::default();

    if args.len() > 0 {
        if let ScriptValue::I32(num_threads) = &args[0] {
            if *num_threads > 0 {
                config.num_threads = *num_threads as usize;
            }
        }
    }

    if args.len() > 1 {
        if let ScriptValue::I32(max_work) = &args[1] {
            if *max_work > 0 {
                config.max_work_per_thread = *max_work as usize;
            }
        }
    }

    if args.len() > 2 {
        if let ScriptValue::Bool(enable_stealing) = &args[2] {
            config.enable_work_stealing = *enable_stealing;
        }
    }

    if args.len() > 3 {
        if let ScriptValue::I32(timeout) = &args[3] {
            config.work_timeout_ms = if *timeout > 0 {
                Some(*timeout as u64)
            } else {
                None
            };
        }
    }

    // For now, return a simple object representation
    let mut config_map = HashMap::new();
    config_map.insert(
        "num_threads".to_string(),
        ScriptValue::I32(config.num_threads as i32),
    );
    config_map.insert(
        "max_work_per_thread".to_string(),
        ScriptValue::I32(config.max_work_per_thread as i32),
    );
    config_map.insert(
        "enable_work_stealing".to_string(),
        ScriptValue::Bool(config.enable_work_stealing),
    );

    if let Some(timeout) = config.work_timeout_ms {
        config_map.insert(
            "work_timeout_ms".to_string(),
            ScriptValue::I32(timeout as i32),
        );
    }

    Ok(ScriptValue::Object(ScriptRc::new(config_map)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.num_threads > 0);
        assert_eq!(config.max_work_per_thread, 1000);
        assert!(config.enable_work_stealing);
        assert_eq!(config.work_timeout_ms, Some(30000));
    }

    #[test]
    fn test_work_item_creation() {
        let item = WorkItem {
            index: 0,
            value: ScriptValue::I32(42),
        };
        assert_eq!(item.index, 0);
        if let ScriptValue::I32(val) = item.value {
            assert_eq!(val, 42);
        } else {
            panic!("Expected I32 value");
        }
    }

    #[test]
    fn test_parallel_executor_creation() {
        let config = ParallelConfig::default();
        let executor = ParallelExecutor::new(config);
        assert_eq!(executor.config.num_threads, num_cpus::get().max(1));
    }
}
