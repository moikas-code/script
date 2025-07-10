//! Comprehensive benchmarks for cycle detection performance
//!
//! This benchmark suite evaluates the performance of the Bacon-Rajan
//! cycle detection algorithm under various conditions and workloads.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use script::runtime::{gc, type_registry, ScriptRc, Value};
use std::collections::HashMap;
use std::time::Duration;

/// Test node for creating reference cycles
#[derive(Debug)]
struct TestNode {
    id: usize,
    value: i32,
    children: Vec<ScriptRc<TestNode>>,
    parent: Option<ScriptRc<TestNode>>,
    data: HashMap<String, ScriptRc<Value>>,
}

impl TestNode {
    fn new(id: usize, value: i32) -> Self {
        TestNode {
            id,
            value,
            children: Vec::new(),
            parent: None,
            data: HashMap::new(),
        }
    }

    fn add_child(&mut self, child: ScriptRc<TestNode>) {
        self.children.push(child);
    }

    fn set_parent(&mut self, parent: ScriptRc<TestNode>) {
        self.parent = Some(parent);
    }
}

impl script::runtime::traceable::Traceable for TestNode {
    fn trace(&self, visitor: &mut dyn FnMut(&dyn std::any::Any)) {
        // Trace children
        for child in &self.children {
            visitor(child as &dyn std::any::Any);
            child.trace(visitor);
        }

        // Trace parent
        if let Some(parent) = &self.parent {
            visitor(parent as &dyn std::any::Any);
            parent.trace(visitor);
        }

        // Trace data values
        for value in self.data.values() {
            visitor(value as &dyn std::any::Any);
            value.trace(visitor);
        }
    }

    fn trace_size(&self) -> usize {
        std::mem::size_of::<TestNode>()
            + self.children.capacity() * std::mem::size_of::<ScriptRc<TestNode>>()
            + self.data.capacity()
                * (std::mem::size_of::<String>() + std::mem::size_of::<ScriptRc<Value>>())
    }
}

impl script::runtime::type_registry::RegisterableType for TestNode {
    fn type_name() -> &'static str {
        "TestNode"
    }

    fn trace_refs(ptr: *const u8, visitor: &mut dyn FnMut(&dyn std::any::Any)) {
        unsafe {
            let node = &*(ptr as *const TestNode);
            node.trace(visitor);
        }
    }

    fn drop_value(ptr: *mut u8) {
        unsafe {
            std::ptr::drop_in_place(ptr as *mut TestNode);
        }
    }
}

/// Create a simple cycle: A -> B -> A
fn create_simple_cycle() -> (ScriptRc<TestNode>, ScriptRc<TestNode>) {
    let node_a = ScriptRc::new(TestNode::new(1, 100));
    let node_b = ScriptRc::new(TestNode::new(2, 200));

    // Create cycle: A -> B -> A
    unsafe {
        let mut a_ref = node_a.get_mut();
        a_ref.add_child(node_b.clone());

        let mut b_ref = node_b.get_mut();
        b_ref.set_parent(node_a.clone());
    }

    (node_a, node_b)
}

/// Create a complex cyclic graph with multiple interconnected nodes
fn create_complex_cycle(size: usize) -> Vec<ScriptRc<TestNode>> {
    let mut nodes = Vec::new();

    // Create nodes
    for i in 0..size {
        nodes.push(ScriptRc::new(TestNode::new(i, i as i32)));
    }

    // Create interconnections
    for i in 0..size {
        unsafe {
            let mut node_ref = nodes[i].get_mut();

            // Add some children (creating forward references)
            for j in 1..=3 {
                let child_idx = (i + j) % size;
                node_ref.add_child(nodes[child_idx].clone());
            }

            // Add parent (creating back reference)
            if i > 0 {
                node_ref.set_parent(nodes[i - 1].clone());
            } else {
                // Create cycle by connecting last to first
                node_ref.set_parent(nodes[size - 1].clone());
            }

            // Add some data values
            for k in 0..5 {
                let key = format!("key_{}", k);
                let value = match k % 4 {
                    0 => Value::I32(k as i32),
                    1 => Value::String(format!("value_{}", k)),
                    2 => Value::Array(vec![
                        ScriptRc::new(Value::I32(k as i32)),
                        ScriptRc::new(Value::String(format!("item_{}", k),
                    ]),
                    _ => Value::Object({
                        let mut obj = HashMap::new();
                        obj.insert("nested".to_string(), ScriptRc::new(Value::I32(k as i32)));
                        obj
                    }),
                };
                node_ref.data.insert(key, ScriptRc::new(value));
            }
        }
    }

    nodes
}

/// Create a deep chain that cycles back to the beginning
fn create_chain_cycle(depth: usize) -> Vec<ScriptRc<TestNode>> {
    let mut nodes = Vec::new();

    // Create chain
    for i in 0..depth {
        nodes.push(ScriptRc::new(TestNode::new(i, i as i32)));
    }

    // Link chain
    for i in 0..depth {
        unsafe {
            let mut node_ref = nodes[i].get_mut();
            if i < depth - 1 {
                node_ref.add_child(nodes[i + 1].clone());
            } else {
                // Close the cycle
                node_ref.add_child(nodes[0].clone());
            }
        }
    }

    nodes
}

fn bench_simple_cycle_detection(c: &mut Criterion) {
    // Initialize GC system
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("simple_cycle_detection");

    group.bench_function("create_and_collect_simple_cycle", |b| {
        b.iter(|| {
            let (_a, _b) = black_box(create_simple_cycle());
            // Force collection
            gc::collect_cycles();
        });
    });

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_complex_cycle_detection(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("complex_cycle_detection");

    let sizes = [10, 50, 100, 500, 1000];

    for size in sizes.iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("create_complex_cycle", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let _nodes = black_box(create_complex_cycle(size));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("collect_complex_cycle", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || create_complex_cycle(size),
                    |_nodes| {
                        gc::collect_cycles();
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_incremental_collection(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("incremental_collection");

    let work_limits = [10, 50, 100, 500];

    for work_limit in work_limits.iter() {
        group.bench_with_input(
            BenchmarkId::new("incremental_collect", work_limit),
            work_limit,
            |b, &work_limit| {
                b.iter_batched(
                    || create_complex_cycle(200),
                    |_nodes| {
                        // Perform incremental collection
                        let mut complete = false;
                        let mut iterations = 0;
                        while !complete && iterations < 100 {
                            complete = gc::collect_cycles_incremental(work_limit);
                            iterations += 1;
                        }
                        black_box(iterations);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_chain_cycle_detection(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("chain_cycle_detection");

    let depths = [10, 100, 1000, 5000];

    for depth in depths.iter() {
        group.throughput(Throughput::Elements(*depth as u64));

        group.bench_with_input(
            BenchmarkId::new("collect_chain_cycle", depth),
            depth,
            |b, &depth| {
                b.iter_batched(
                    || create_chain_cycle(depth),
                    |_nodes| {
                        gc::collect_cycles();
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_collection_threshold_impact(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("collection_threshold");

    let thresholds = [100, 500, 1000, 5000];

    for threshold in thresholds.iter() {
        group.bench_with_input(
            BenchmarkId::new("allocation_with_threshold", threshold),
            threshold,
            |b, &threshold| {
                b.iter(|| {
                    // Create many objects to trigger collections
                    let mut nodes = Vec::new();
                    for i in 0..threshold * 2 {
                        nodes.push(ScriptRc::new(TestNode::new(i, i as i32)));
                    }

                    black_box(nodes);
                });
            },
        );
    }

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_concurrent_collection(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("concurrent_collection");

    group.bench_function("background_collection", |b| {
        b.iter(|| {
            // Create cycles that will be detected by background collector
            let _nodes = black_box(create_complex_cycle(100));

            // Wait a bit for background collection
            std::thread::sleep(Duration::from_millis(10));
        });
    });

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_memory_pressure(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("memory_pressure");

    let object_counts = [1000, 5000, 10000];

    for count in object_counts.iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("high_memory_pressure", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut all_nodes = Vec::new();

                    // Create many interconnected cycles
                    for batch in 0..(count / 100) {
                        let nodes = create_complex_cycle(100);
                        all_nodes.extend(nodes);

                        // Trigger collection every few batches
                        if batch % 10 == 0 {
                            gc::collect_cycles();
                        }
                    }

                    black_box(all_nodes);
                });
            },
        );
    }

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

fn bench_type_recovery_performance(c: &mut Criterion) {
    type_registry::initialize();
    gc::initialize().expect("Failed to initialize GC");
    type_registry::register_with_trait::<TestNode>();

    let mut group = c.benchmark_group("type_recovery");

    group.bench_function("type_registry_lookup", |b| {
        let _node = ScriptRc::new(TestNode::new(1, 42));
        let type_id = type_registry::register_with_trait::<TestNode>();

        b.iter(|| {
            let info = black_box(type_registry::get_type_info(type_id));
            black_box(info);
        });
    });

    group.bench_function("trace_function_call", |b| {
        let node = ScriptRc::new(TestNode::new(1, 42));

        b.iter(|| {
            let mut count = 0;
            node.trace(&mut |_| count += 1);
            black_box(count);
        });
    });

    group.finish();

    gc::shutdown().expect("Failed to shutdown GC");
    type_registry::shutdown();
}

criterion_group!(
    benches,
    bench_simple_cycle_detection,
    bench_complex_cycle_detection,
    bench_incremental_collection,
    bench_chain_cycle_detection,
    bench_collection_threshold_impact,
    bench_concurrent_collection,
    bench_memory_pressure,
    bench_type_recovery_performance
);

criterion_main!(benches);
