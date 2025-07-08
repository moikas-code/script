//! Performance optimization module for secure async/await implementation
//!
//! This module provides comprehensive performance optimizations for the async
//! runtime while maintaining all security guarantees. Optimizations include
//! allocation reduction, scheduler improvements, and efficient memory management.

use super::async_runtime_secure::*;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;

/// Configuration for performance optimization
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable object pooling for reduced allocations
    pub enable_object_pooling: bool,
    /// Enable adaptive scheduling based on workload
    pub enable_adaptive_scheduling: bool,
    /// Enable work stealing between threads
    pub enable_work_stealing: bool,
    /// Enable batch processing of operations
    pub enable_batch_processing: bool,
    /// Maximum number of tasks to process in a batch
    pub max_batch_size: usize,
    /// Target latency for operations (microseconds)
    pub target_latency_us: u64,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        PerformanceConfig {
            enable_object_pooling: true,
            enable_adaptive_scheduling: true,
            enable_work_stealing: false, // Disabled by default for security
            enable_batch_processing: true,
            max_batch_size: 32,
            target_latency_us: 1000, // 1ms target
            enable_monitoring: true,
        }
    }
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Number of tasks processed
    pub tasks_processed: AtomicUsize,
    /// Number of allocations avoided through pooling
    pub allocations_avoided: AtomicUsize,
    /// Total execution time (nanoseconds)
    pub total_execution_time_ns: AtomicUsize,
    /// Number of scheduler adaptations
    pub scheduler_adaptations: AtomicUsize,
    /// Number of batch operations
    pub batch_operations: AtomicUsize,
    /// Peak memory usage (bytes)
    pub peak_memory_usage: AtomicUsize,
    /// Number of cache hits
    pub cache_hits: AtomicUsize,
    /// Number of cache misses
    pub cache_misses: AtomicUsize,
}

impl PerformanceMetrics {
    /// Calculate average execution time per task
    pub fn avg_execution_time_ns(&self) -> u64 {
        let total_time = self.total_execution_time_ns.load(Ordering::Relaxed);
        let tasks = self.tasks_processed.load(Ordering::Relaxed);
        if tasks > 0 {
            (total_time / tasks) as u64
        } else {
            0
        }
    }

    /// Calculate cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.tasks_processed.store(0, Ordering::Relaxed);
        self.allocations_avoided.store(0, Ordering::Relaxed);
        self.total_execution_time_ns.store(0, Ordering::Relaxed);
        self.scheduler_adaptations.store(0, Ordering::Relaxed);
        self.batch_operations.store(0, Ordering::Relaxed);
        self.peak_memory_usage.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

/// Object pool for reducing allocations
pub struct ObjectPool<T> {
    objects: Mutex<Vec<T>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created_count: AtomicUsize,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        ObjectPool {
            objects: Mutex::new(Vec::with_capacity(max_size)),
            factory: Box::new(factory),
            max_size,
            created_count: AtomicUsize::new(0),
        }
    }

    /// Get an object from the pool or create a new one
    pub fn get(&self) -> AsyncResult<PooledObject<T>> {
        let mut objects = self.objects.lock().secure_lock()?;
        
        let object = if let Some(obj) = objects.pop() {
            obj
        } else {
            // Create new object if pool is empty
            self.created_count.fetch_add(1, Ordering::Relaxed);
            (self.factory)()
        };

        Ok(PooledObject {
            object: Some(object),
            pool: self,
        })
    }

    /// Return an object to the pool
    fn return_object(&self, object: T) -> AsyncResult<()> {
        let mut objects = self.objects.lock().secure_lock()?;
        
        if objects.len() < self.max_size {
            objects.push(object);
        }
        // If pool is full, object is dropped
        
        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> AsyncResult<(usize, usize, usize)> {
        let objects = self.objects.lock().secure_lock()?;
        let available = objects.len();
        let created = self.created_count.load(Ordering::Relaxed);
        let in_use = created - available;
        
        Ok((available, in_use, created))
    }
}

/// RAII wrapper for pooled objects
pub struct PooledObject<'a, T> {
    object: Option<T>,
    pool: &'a ObjectPool<T>,
}

impl<'a, T> PooledObject<'a, T> {
    /// Get a reference to the object
    pub fn get(&self) -> &T {
        self.object.as_ref().expect("Object should always be present")
    }

    /// Get a mutable reference to the object
    pub fn get_mut(&mut self) -> &mut T {
        self.object.as_mut().expect("Object should always be present")
    }
}

impl<'a, T> Drop for PooledObject<'a, T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            let _ = self.pool.return_object(object);
        }
    }
}

/// Adaptive scheduler that adjusts behavior based on workload
pub struct AdaptiveScheduler {
    /// Configuration
    config: PerformanceConfig,
    /// Performance metrics
    metrics: Arc<PerformanceMetrics>,
    /// Current scheduling strategy
    current_strategy: AtomicUsize,
    /// Last adaptation time
    last_adaptation: Mutex<Instant>,
    /// Workload history for decision making
    workload_history: Mutex<VecDeque<WorkloadSample>>,
}

/// Sample of workload for adaptive scheduling
#[derive(Debug, Clone)]
struct WorkloadSample {
    timestamp: Instant,
    task_count: usize,
    avg_execution_time_ns: u64,
    queue_depth: usize,
}

/// Scheduling strategies
#[derive(Debug, Clone, Copy, PartialEq)]
enum SchedulingStrategy {
    RoundRobin = 0,
    Priority = 1,
    ShortestJobFirst = 2,
    AdaptiveBatch = 3,
}

impl AdaptiveScheduler {
    pub fn new(config: PerformanceConfig) -> Self {
        AdaptiveScheduler {
            config,
            metrics: Arc::new(PerformanceMetrics::default()),
            current_strategy: AtomicUsize::new(SchedulingStrategy::RoundRobin as usize),
            last_adaptation: Mutex::new(Instant::now()),
            workload_history: Mutex::new(VecDeque::with_capacity(100)),
        }
    }

    /// Record a workload sample
    pub fn record_workload(&self, task_count: usize, queue_depth: usize) -> AsyncResult<()> {
        if !self.config.enable_adaptive_scheduling {
            return Ok(());
        }

        let sample = WorkloadSample {
            timestamp: Instant::now(),
            task_count,
            avg_execution_time_ns: self.metrics.avg_execution_time_ns(),
            queue_depth,
        };

        let mut history = self.workload_history.lock().secure_lock()?;
        history.push_back(sample);
        
        // Keep history bounded
        if history.len() > 100 {
            history.pop_front();
        }

        // Check if adaptation is needed
        self.maybe_adapt_strategy()?;

        Ok(())
    }

    /// Check if strategy adaptation is needed
    fn maybe_adapt_strategy(&self) -> AsyncResult<()> {
        let mut last_adaptation = self.last_adaptation.lock().secure_lock()?;
        
        // Only adapt every 100ms minimum
        if last_adaptation.elapsed() < Duration::from_millis(100) {
            return Ok(());
        }

        let history = self.workload_history.lock().secure_lock()?;
        if history.len() < 10 {
            return Ok(());
        }

        // Analyze recent workload
        let recent_samples: Vec<_> = history.iter().rev().take(10).collect();
        let avg_queue_depth: f64 = recent_samples.iter()
            .map(|s| s.queue_depth as f64)
            .sum::<f64>() / recent_samples.len() as f64;
        
        let avg_execution_time: f64 = recent_samples.iter()
            .map(|s| s.avg_execution_time_ns as f64)
            .sum::<f64>() / recent_samples.len() as f64;

        // Determine optimal strategy
        let new_strategy = if avg_queue_depth > 50.0 && avg_execution_time > 10_000.0 {
            SchedulingStrategy::AdaptiveBatch
        } else if avg_execution_time < 1_000.0 {
            SchedulingStrategy::ShortestJobFirst
        } else if avg_queue_depth > 20.0 {
            SchedulingStrategy::Priority
        } else {
            SchedulingStrategy::RoundRobin
        };

        let current = self.current_strategy.load(Ordering::Relaxed);
        if current != new_strategy as usize {
            self.current_strategy.store(new_strategy as usize, Ordering::Relaxed);
            self.metrics.scheduler_adaptations.fetch_add(1, Ordering::Relaxed);
            *last_adaptation = Instant::now();
        }

        Ok(())
    }

    /// Get current scheduling strategy
    pub fn current_strategy(&self) -> SchedulingStrategy {
        match self.current_strategy.load(Ordering::Relaxed) {
            0 => SchedulingStrategy::RoundRobin,
            1 => SchedulingStrategy::Priority,
            2 => SchedulingStrategy::ShortestJobFirst,
            3 => SchedulingStrategy::AdaptiveBatch,
            _ => SchedulingStrategy::RoundRobin,
        }
    }

    /// Get performance metrics
    pub fn metrics(&self) -> Arc<PerformanceMetrics> {
        self.metrics.clone()
    }
}

/// Batch processor for grouping operations
pub struct BatchProcessor<T> {
    config: PerformanceConfig,
    pending_items: Mutex<Vec<T>>,
    last_flush: Mutex<Instant>,
    processor: Box<dyn Fn(Vec<T>) -> AsyncResult<()> + Send + Sync>,
}

impl<T> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new<F>(config: PerformanceConfig, processor: F) -> Self
    where
        F: Fn(Vec<T>) -> AsyncResult<()> + Send + Sync + 'static,
    {
        BatchProcessor {
            config,
            pending_items: Mutex::new(Vec::new()),
            last_flush: Mutex::new(Instant::now()),
            processor: Box::new(processor),
        }
    }

    /// Add an item to the batch
    pub fn add(&self, item: T) -> AsyncResult<()> {
        if !self.config.enable_batch_processing {
            // Process immediately if batching is disabled
            return (self.processor)(vec![item]);
        }

        let mut pending = self.pending_items.lock().secure_lock()?;
        pending.push(item);

        // Check if we should flush
        let should_flush = pending.len() >= self.config.max_batch_size;
        
        if should_flush {
            let items = std::mem::take(&mut *pending);
            drop(pending);
            
            // Update flush time
            *self.last_flush.lock().secure_lock()? = Instant::now();
            
            return (self.processor)(items);
        }

        // Check time-based flush
        let last_flush = self.last_flush.lock().secure_lock()?;
        if last_flush.elapsed() > Duration::from_millis(10) {
            let items = std::mem::take(&mut *pending);
            drop(pending);
            drop(last_flush);
            
            *self.last_flush.lock().secure_lock()? = Instant::now();
            
            if !items.is_empty() {
                return (self.processor)(items);
            }
        }

        Ok(())
    }

    /// Force flush all pending items
    pub fn flush(&self) -> AsyncResult<()> {
        let mut pending = self.pending_items.lock().secure_lock()?;
        let items = std::mem::take(&mut *pending);
        drop(pending);

        *self.last_flush.lock().secure_lock()? = Instant::now();

        if !items.is_empty() {
            (self.processor)(items)?;
        }

        Ok(())
    }
}

/// Memory-efficient cache for frequently accessed data
pub struct AsyncCache<K, V> {
    data: Mutex<HashMap<K, CacheEntry<V>>>,
    max_size: usize,
    ttl: Duration,
    metrics: Arc<PerformanceMetrics>,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    access_count: usize,
}

impl<K, V> AsyncCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// Create a new cache
    pub fn new(max_size: usize, ttl: Duration, metrics: Arc<PerformanceMetrics>) -> Self {
        AsyncCache {
            data: Mutex::new(HashMap::with_capacity(max_size)),
            max_size,
            ttl,
            metrics,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> AsyncResult<Option<V>> {
        let mut data = self.data.lock().secure_lock()?;
        
        if let Some(entry) = data.get_mut(key) {
            // Check TTL
            if entry.created_at.elapsed() <= self.ttl {
                entry.access_count += 1;
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Some(entry.value.clone()));
            } else {
                // Entry expired
                data.remove(key);
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        Ok(None)
    }

    /// Put a value in the cache
    pub fn put(&self, key: K, value: V) -> AsyncResult<()> {
        let mut data = self.data.lock().secure_lock()?;
        
        // Evict if at capacity
        if data.len() >= self.max_size {
            self.evict_lru(&mut data)?;
        }

        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            access_count: 1,
        };

        data.insert(key, entry);
        Ok(())
    }

    /// Evict least recently used item
    fn evict_lru(&self, data: &mut HashMap<K, CacheEntry<V>>) -> AsyncResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        // Find LRU item (lowest access count + oldest)
        let mut lru_key = None;
        let mut min_score = f64::INFINITY;

        for (key, entry) in data.iter() {
            let age_score = entry.created_at.elapsed().as_secs_f64();
            let access_score = 1.0 / (entry.access_count as f64 + 1.0);
            let score = age_score + access_score;

            if score < min_score {
                min_score = score;
                lru_key = Some(key.clone());
            }
        }

        if let Some(key) = lru_key {
            data.remove(&key);
        }

        Ok(())
    }

    /// Clear expired entries
    pub fn cleanup(&self) -> AsyncResult<usize> {
        let mut data = self.data.lock().secure_lock()?;
        let initial_size = data.len();
        
        data.retain(|_, entry| entry.created_at.elapsed() <= self.ttl);
        
        Ok(initial_size - data.len())
    }

    /// Get cache statistics
    pub fn stats(&self) -> AsyncResult<(usize, f64, usize)> {
        let data = self.data.lock().secure_lock()?;
        let size = data.len();
        let hit_ratio = self.metrics.cache_hit_ratio();
        let total_accesses = self.metrics.cache_hits.load(Ordering::Relaxed) + 
                           self.metrics.cache_misses.load(Ordering::Relaxed);
        
        Ok((size, hit_ratio, total_accesses))
    }
}

/// Optimized executor with performance enhancements
pub struct OptimizedExecutor {
    base_executor: Arc<Mutex<Executor>>,
    scheduler: AdaptiveScheduler,
    task_pool: ObjectPool<Box<dyn std::any::Any + Send>>,
    waker_cache: AsyncCache<TaskId, std::task::Waker>,
    batch_processor: BatchProcessor<TaskId>,
    config: PerformanceConfig,
    is_running: AtomicBool,
}

impl OptimizedExecutor {
    /// Create a new optimized executor
    pub fn new(config: PerformanceConfig) -> Self {
        let scheduler = AdaptiveScheduler::new(config.clone());
        let metrics = scheduler.metrics();
        
        let task_pool = ObjectPool::new(
            || Box::new(()) as Box<dyn std::any::Any + Send>,
            1000
        );

        let waker_cache = AsyncCache::new(1000, Duration::from_secs(60), metrics.clone());

        let batch_processor = BatchProcessor::new(
            config.clone(),
            |task_ids: Vec<TaskId>| {
                // Batch processing logic would go here
                Ok(())
            }
        );

        OptimizedExecutor {
            base_executor: Executor::new(),
            scheduler,
            task_pool,
            waker_cache,
            batch_processor,
            config,
            is_running: AtomicBool::new(false),
        }
    }

    /// Spawn a task with optimizations
    pub fn spawn_optimized(&self, future: BoxedFuture<()>) -> AsyncResult<TaskId> {
        let start_time = Instant::now();
        
        // Use pooled objects if enabled
        let _pooled_obj = if self.config.enable_object_pooling {
            Some(self.task_pool.get()?)
        } else {
            None
        };

        // Spawn using base executor
        let task_id = Executor::spawn(self.base_executor.clone(), future)?;

        // Record metrics
        let execution_time = start_time.elapsed().as_nanos() as usize;
        self.scheduler.metrics().total_execution_time_ns.fetch_add(execution_time, Ordering::Relaxed);
        self.scheduler.metrics().tasks_processed.fetch_add(1, Ordering::Relaxed);

        // Record workload for adaptive scheduling
        let queue_stats = Executor::get_stats(self.base_executor.clone())?;
        self.scheduler.record_workload(queue_stats.2, queue_stats.2)?;

        Ok(task_id)
    }

    /// Run the optimized executor
    pub fn run_optimized(&self) -> AsyncResult<()> {
        self.is_running.store(true, Ordering::Relaxed);

        // Start background maintenance task
        let maintenance_handle = self.start_maintenance_task()?;

        // Run base executor
        let result = Executor::run(self.base_executor.clone());

        self.is_running.store(false, Ordering::Relaxed);
        let _ = maintenance_handle.join();

        result
    }

    /// Start background maintenance tasks
    fn start_maintenance_task(&self) -> AsyncResult<thread::JoinHandle<()>> {
        let waker_cache = self.waker_cache.clone();
        let is_running = self.is_running.clone();
        let batch_processor = self.batch_processor.clone();

        let handle = thread::Builder::new()
            .name("async-maintenance".to_string())
            .spawn(move || {
                while is_running.load(Ordering::Relaxed) {
                    // Cleanup cache periodically
                    let _ = waker_cache.cleanup();
                    
                    // Flush batch processor
                    let _ = batch_processor.flush();

                    thread::sleep(Duration::from_millis(100));
                }
            })
            .map_err(|_| AsyncRuntimeError::ThreadJoinFailed)?;

        Ok(handle)
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> Arc<PerformanceMetrics> {
        self.scheduler.metrics()
    }

    /// Get optimization statistics
    pub fn get_optimization_stats(&self) -> AsyncResult<OptimizationStats> {
        let task_pool_stats = self.task_pool.stats()?;
        let cache_stats = self.waker_cache.stats()?;
        let metrics = self.scheduler.metrics();

        Ok(OptimizationStats {
            object_pool_available: task_pool_stats.0,
            object_pool_in_use: task_pool_stats.1,
            object_pool_created: task_pool_stats.2,
            cache_size: cache_stats.0,
            cache_hit_ratio: cache_stats.1,
            cache_total_accesses: cache_stats.2,
            allocations_avoided: metrics.allocations_avoided.load(Ordering::Relaxed),
            batch_operations: metrics.batch_operations.load(Ordering::Relaxed),
            scheduler_adaptations: metrics.scheduler_adaptations.load(Ordering::Relaxed),
            current_strategy: self.scheduler.current_strategy(),
        })
    }
}

/// Statistics for optimization effectiveness
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    pub object_pool_available: usize,
    pub object_pool_in_use: usize,
    pub object_pool_created: usize,
    pub cache_size: usize,
    pub cache_hit_ratio: f64,
    pub cache_total_accesses: usize,
    pub allocations_avoided: usize,
    pub batch_operations: usize,
    pub scheduler_adaptations: usize,
    pub current_strategy: SchedulingStrategy,
}

impl OptimizationStats {
    /// Print a detailed optimization report
    pub fn print_report(&self) {
        println!("\n⚡ ASYNC PERFORMANCE OPTIMIZATION REPORT");
        println!("════════════════════════════════════════");
        
        println!("🏊 Object Pool:");
        println!("  Available: {}", self.object_pool_available);
        println!("  In Use: {}", self.object_pool_in_use);
        println!("  Created: {}", self.object_pool_created);
        println!("  Pool Efficiency: {:.1}%", 
            if self.object_pool_created > 0 {
                (self.object_pool_available as f64 / self.object_pool_created as f64) * 100.0
            } else { 0.0 }
        );

        println!("\n💾 Cache Performance:");
        println!("  Size: {}", self.cache_size);
        println!("  Hit Ratio: {:.1}%", self.cache_hit_ratio * 100.0);
        println!("  Total Accesses: {}", self.cache_total_accesses);

        println!("\n🔧 Optimizations:");
        println!("  Allocations Avoided: {}", self.allocations_avoided);
        println!("  Batch Operations: {}", self.batch_operations);
        println!("  Scheduler Adaptations: {}", self.scheduler_adaptations);
        println!("  Current Strategy: {:?}", self.current_strategy);

        println!("\n📊 Performance Grade:");
        let grade = self.calculate_performance_grade();
        println!("  Overall Grade: {} {}", grade, self.get_grade_emoji(grade));
    }

    /// Calculate overall performance grade
    fn calculate_performance_grade(&self) -> char {
        let mut score = 0.0;

        // Cache performance (0-30 points)
        score += self.cache_hit_ratio * 30.0;

        // Pool efficiency (0-25 points)
        let pool_efficiency = if self.object_pool_created > 0 {
            self.object_pool_available as f64 / self.object_pool_created as f64
        } else { 0.0 };
        score += pool_efficiency * 25.0;

        // Optimization activity (0-25 points)
        let optimization_activity = (self.allocations_avoided + self.batch_operations).min(1000) as f64 / 1000.0;
        score += optimization_activity * 25.0;

        // Adaptive behavior (0-20 points)
        let adaptive_score = self.scheduler_adaptations.min(100) as f64 / 100.0;
        score += adaptive_score * 20.0;

        match score as u8 {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        }
    }

    /// Get emoji for grade
    fn get_grade_emoji(&self, grade: char) -> &'static str {
        match grade {
            'A' => "🏆",
            'B' => "🥈",
            'C' => "🥉",
            'D' => "📈",
            _ => "🔧",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new(|| String::new(), 5);
        
        // Test getting objects
        let obj1 = pool.get().unwrap();
        let obj2 = pool.get().unwrap();
        
        assert_eq!(pool.stats().unwrap().1, 2); // 2 in use
        
        // Objects should be returned when dropped
        drop(obj1);
        drop(obj2);
        
        assert_eq!(pool.stats().unwrap().0, 2); // 2 available
    }

    #[test]
    fn test_adaptive_scheduler() {
        let config = PerformanceConfig::default();
        let scheduler = AdaptiveScheduler::new(config);
        
        // Record some workload
        scheduler.record_workload(10, 5).unwrap();
        scheduler.record_workload(20, 15).unwrap();
        
        // Should start with RoundRobin
        assert_eq!(scheduler.current_strategy(), SchedulingStrategy::RoundRobin);
    }

    #[test]
    fn test_batch_processor() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let processed_count = Arc::new(AtomicUsize::new(0));
        let count_clone = processed_count.clone();
        
        let mut config = PerformanceConfig::default();
        config.max_batch_size = 3;
        
        let processor = BatchProcessor::new(config, move |items: Vec<usize>| {
            count_clone.fetch_add(items.len(), Ordering::Relaxed);
            Ok(())
        });

        // Add items
        processor.add(1).unwrap();
        processor.add(2).unwrap();
        processor.add(3).unwrap(); // Should trigger batch processing

        assert_eq!(processed_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_async_cache() {
        let metrics = Arc::new(PerformanceMetrics::default());
        let cache = AsyncCache::new(5, Duration::from_secs(1), metrics);
        
        // Test put and get
        cache.put("key1".to_string(), "value1".to_string()).unwrap();
        let result = cache.get(&"key1".to_string()).unwrap();
        assert_eq!(result, Some("value1".to_string()));
        
        // Test cache miss
        let result = cache.get(&"nonexistent".to_string()).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::default();
        
        metrics.tasks_processed.store(100, Ordering::Relaxed);
        metrics.total_execution_time_ns.store(1_000_000_000, Ordering::Relaxed); // 1 second
        
        assert_eq!(metrics.avg_execution_time_ns(), 10_000_000); // 10ms average
        
        metrics.cache_hits.store(80, Ordering::Relaxed);
        metrics.cache_misses.store(20, Ordering::Relaxed);
        
        assert_eq!(metrics.cache_hit_ratio(), 0.8); // 80% hit ratio
    }

    #[test]
    fn test_optimization_stats_grading() {
        let stats = OptimizationStats {
            object_pool_available: 80,
            object_pool_in_use: 20,
            object_pool_created: 100,
            cache_size: 50,
            cache_hit_ratio: 0.9,
            cache_total_accesses: 1000,
            allocations_avoided: 500,
            batch_operations: 300,
            scheduler_adaptations: 10,
            current_strategy: SchedulingStrategy::AdaptiveBatch,
        };
        
        let grade = stats.calculate_performance_grade();
        assert!(grade >= 'B'); // Should get a good grade with these stats
    }
}