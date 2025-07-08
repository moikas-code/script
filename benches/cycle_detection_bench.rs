//! Benchmarks for memory cycle detection
//!
//! These benchmarks measure the performance of the cycle collector
//! under various workloads and cycle patterns.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use script::runtime::{self, ScriptRc, Traceable};
use std::cell::RefCell;

/// A simple node that can form cycles
struct Node {
    value: i32,
    next: RefCell<Option<ScriptRc<Node>>>,
}

impl Node {
    fn new(value: i32) -> Self {
        Node {
            value,
            next: RefCell::new(None),
        }
    }
    
    fn set_next(&self, next: Option<ScriptRc<Node>>) {
        *self.next.borrow_mut() = next;
    }
}

impl Traceable for Node {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn std::any::Any)) {
        if let Some(ref next) = *self.next.borrow() {
            visitor(next as &dyn std::any::Any);
            next.trace(visitor);
        }
    }
}

/// Benchmark simple cycle creation and collection
fn bench_simple_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_cycles");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                runtime::initialize().ok();
                
                // Create simple cycles
                for _ in 0..size {
                    let node_a = ScriptRc::new(Node::new(1));
                    let node_b = ScriptRc::new(Node::new(2));
                    
                    node_a.set_next(Some(node_b.clone()));
                    node_b.set_next(Some(node_a.clone()));
                    
                    // Drop references to create cycle
                    drop(node_a);
                    drop(node_b);
                }
                
                // Force collection
                runtime::gc::collect_cycles();
                
                runtime::shutdown().ok();
            });
        });
    }
    
    group.finish();
}

/// Benchmark complex cycle patterns
fn bench_complex_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_cycles");
    
    group.bench_function("chain_cycle", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            // Create a chain that forms a cycle
            let nodes: Vec<_> = (0..100)
                .map(|i| ScriptRc::new(Node::new(i)))
                .collect();
            
            // Link them in a chain
            for i in 0..99 {
                nodes[i].set_next(Some(nodes[i + 1].clone()));
            }
            
            // Create the cycle
            nodes[99].set_next(Some(nodes[0].clone()));
            
            // Drop all references
            drop(nodes);
            
            // Force collection
            runtime::gc::collect_cycles();
            
            runtime::shutdown().ok();
        });
    });
    
    group.bench_function("tree_with_cycles", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            // Create a binary tree where leaves point back to root
            fn create_tree(depth: usize, root: &ScriptRc<Node>) -> ScriptRc<Node> {
                let node = ScriptRc::new(Node::new(depth as i32));
                
                if depth > 0 {
                    let left = create_tree(depth - 1, root);
                    let right = create_tree(depth - 1, root);
                    node.set_next(Some(left));
                    // In a real tree we'd have separate left/right, but for this test
                    // we just create more cycles
                } else {
                    // Leaf points back to root
                    node.set_next(Some(root.clone()));
                }
                
                node
            }
            
            let root = ScriptRc::new(Node::new(0));
            let tree = create_tree(5, &root);
            
            drop(root);
            drop(tree);
            
            runtime::gc::collect_cycles();
            
            runtime::shutdown().ok();
        });
    });
    
    group.finish();
}

/// Benchmark incremental collection
fn bench_incremental_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_collection");
    
    for work_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(work_size), 
            work_size, 
            |b, &work_size| {
                b.iter(|| {
                    runtime::initialize().ok();
                    
                    // Create many cycles
                    for _ in 0..1000 {
                        let node_a = ScriptRc::new(Node::new(1));
                        let node_b = ScriptRc::new(Node::new(2));
                        
                        node_a.set_next(Some(node_b.clone()));
                        node_b.set_next(Some(node_a.clone()));
                        
                        drop(node_a);
                        drop(node_b);
                    }
                    
                    // Collect incrementally
                    while !runtime::gc::collect_cycles_incremental(work_size) {
                        // Continue collecting
                    }
                    
                    runtime::shutdown().ok();
                });
            }
        );
    }
    
    group.finish();
}

/// Benchmark collection overhead with no cycles
fn bench_no_cycles_overhead(c: &mut Criterion) {
    c.bench_function("no_cycles_overhead", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            // Create many objects but no cycles
            let mut nodes = Vec::new();
            for i in 0..1000 {
                nodes.push(ScriptRc::new(Node::new(i)));
            }
            
            // Link them in a chain (no cycles)
            for i in 0..999 {
                nodes[i].set_next(Some(nodes[i + 1].clone()));
            }
            
            // Drop in order - should not trigger much GC work
            drop(nodes);
            
            runtime::gc::collect_cycles();
            
            runtime::shutdown().ok();
        });
    });
}

/// Benchmark concurrent cycle creation and collection
fn bench_concurrent_cycles(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;
    
    c.bench_function("concurrent_cycles", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let done_clone = done.clone();
            
            // Spawn thread that creates cycles
            let creator = thread::spawn(move || {
                while !done_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    let node_a = ScriptRc::new(Node::new(1));
                    let node_b = ScriptRc::new(Node::new(2));
                    
                    node_a.set_next(Some(node_b.clone()));
                    node_b.set_next(Some(node_a.clone()));
                    
                    drop(node_a);
                    drop(node_b);
                }
            });
            
            // Main thread performs collections
            for _ in 0..10 {
                runtime::gc::collect_cycles();
                thread::sleep(std::time::Duration::from_micros(100));
            }
            
            done.store(true, std::sync::atomic::Ordering::Relaxed);
            creator.join().unwrap();
            
            runtime::shutdown().ok();
        });
    });
}

/// Benchmark memory usage with cycles vs without
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("with_cycles", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            let initial_stats = runtime::profiler::get_stats();
            
            // Create cycles that would leak without GC
            for _ in 0..1000 {
                let node_a = ScriptRc::new(Node::new(1));
                let node_b = ScriptRc::new(Node::new(2));
                
                node_a.set_next(Some(node_b.clone()));
                node_b.set_next(Some(node_a.clone()));
            }
            
            runtime::gc::collect_cycles();
            
            let final_stats = runtime::profiler::get_stats();
            black_box(final_stats.current_bytes - initial_stats.current_bytes);
            
            runtime::shutdown().ok();
        });
    });
    
    group.bench_function("without_cycles", |b| {
        b.iter(|| {
            runtime::initialize().ok();
            
            let initial_stats = runtime::profiler::get_stats();
            
            // Create same objects but no cycles
            for _ in 0..1000 {
                let _node_a = ScriptRc::new(Node::new(1));
                let _node_b = ScriptRc::new(Node::new(2));
            }
            
            let final_stats = runtime::profiler::get_stats();
            black_box(final_stats.current_bytes - initial_stats.current_bytes);
            
            runtime::shutdown().ok();
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_simple_cycles,
    bench_complex_cycles,
    bench_incremental_collection,
    bench_no_cycles_overhead,
    bench_concurrent_cycles,
    bench_memory_usage
);
criterion_main!(benches);