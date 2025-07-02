use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

mod common;
use common::BenchmarkAdapter;

/// Custom allocator to track memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

/// Reset memory tracking counters
fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    DEALLOCATED.store(0, Ordering::SeqCst);
}

/// Get current memory usage
fn get_memory_usage() -> (usize, usize) {
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    let deallocated = DEALLOCATED.load(Ordering::SeqCst);
    (allocated, allocated.saturating_sub(deallocated))
}

/// Benchmark memory usage during compilation
fn benchmark_compilation_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_compilation");

    let sources = vec![
        (
            "small",
            r#"
            let x = 42
            let y = x + 10
        "#,
        ),
        (
            "medium",
            r#"
            fn factorial(n) {
                if n <= 1 {
                    return 1
                }
                return n * factorial(n - 1)
            }
            
            let result1 = factorial(5)
            let result2 = factorial(6)
            let result3 = factorial(7)
        "#,
        ),
        (
            "large",
            r#"
            fn fibonacci(n) {
                if n <= 1 {
                    return n
                }
                return fibonacci(n - 1) + fibonacci(n - 2)
            }
            
            fn factorial(n) {
                if n <= 1 {
                    return 1
                }
                return n * factorial(n - 1)
            }
            
            fn is_prime(n) {
                if n <= 1 {
                    return 0
                }
                if n <= 3 {
                    return 1
                }
                if n % 2 == 0 || n % 3 == 0 {
                    return 0
                }
                
                let i = 5
                while i * i <= n {
                    if n % i == 0 || n % (i + 2) == 0 {
                        return 0
                    }
                    i = i + 6
                }
                return 1
            }
            
            let fib_result = fibonacci(10)
            let fact_result = factorial(8)
            let prime_result = is_prime(97)
        "#,
        ),
    ];

    for (name, source) in sources {
        group.bench_with_input(
            BenchmarkId::new("peak_memory", name),
            &source,
            |b, source| {
                b.iter_custom(|iters| {
                    let mut total_memory = 0u64;

                    for _ in 0..iters {
                        reset_memory_tracking();

                        // Compile the program
                        let _ = BenchmarkAdapter::prepare_program(black_box(source));

                        let (_, peak) = get_memory_usage();
                        total_memory += peak as u64;
                    }

                    std::time::Duration::from_nanos(total_memory / iters)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage during program preparation
fn benchmark_program_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_program");

    // Memory allocation stress test
    let allocation_stress = r#"
        fn create_arrays() {
            let arrays = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            let sum = 0
            
            let i = 0
            while i < 3 {
                let j = 0
                while j < 3 {
                    sum = sum + arrays[i][j]
                    j = j + 1
                }
                i = i + 1
            }
            
            return sum
        }
        
        create_arrays()
    "#;

    group.bench_function("allocation_stress", |b| {
        b.iter_custom(|iters| {
            let mut total_memory = 0u64;

            for _ in 0..iters {
                reset_memory_tracking();
                let _ = BenchmarkAdapter::prepare_program(black_box(allocation_stress));
                let (_, peak) = get_memory_usage();
                total_memory += peak as u64;
            }

            std::time::Duration::from_nanos(total_memory / iters)
        })
    });

    // String concatenation memory test
    let string_memory = r#"
        fn string_stress() {
            let result = ""
            let i = 0
            
            while i < 20 {
                result = result + "Line " + i + ": Hello World!\n"
                i = i + 1
            }
            
            return result
        }
        
        string_stress()
    "#;

    group.bench_function("string_concatenation", |b| {
        b.iter_custom(|iters| {
            let mut total_memory = 0u64;

            for _ in 0..iters {
                reset_memory_tracking();
                let _ = BenchmarkAdapter::prepare_program(black_box(string_memory));
                let (_, peak) = get_memory_usage();
                total_memory += peak as u64;
            }

            std::time::Duration::from_nanos(total_memory / iters)
        })
    });

    // Array operations memory test
    let array_memory = r#"
        fn array_operations() {
            let arr1 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            let arr2 = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
            let result = []
            
            let i = 0
            while i < 10 {
                result[i] = arr1[i] + arr2[i]
                i = i + 1
            }
            
            return result
        }
        
        array_operations()
    "#;

    group.bench_function("array_operations", |b| {
        b.iter_custom(|iters| {
            let mut total_memory = 0u64;

            for _ in 0..iters {
                reset_memory_tracking();
                let _ = BenchmarkAdapter::prepare_program(black_box(array_memory));
                let (_, peak) = get_memory_usage();
                total_memory += peak as u64;
            }

            std::time::Duration::from_nanos(total_memory / iters)
        })
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn benchmark_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Function call memory pattern
    let function_calls = r#"
        fn recursive_sum(n) {
            if n <= 0 {
                return 0
            }
            return n + recursive_sum(n - 1)
        }
        
        fn iterative_sum(n) {
            let sum = 0
            let i = 1
            while i <= n {
                sum = sum + i
                i = i + 1
            }
            return sum
        }
        
        let recursive_result = recursive_sum(50)
        let iterative_result = iterative_sum(50)
    "#;

    group.bench_function("function_calls", |b| {
        b.iter_custom(|iters| {
            let mut total_memory = 0u64;

            for _ in 0..iters {
                reset_memory_tracking();
                let _ = BenchmarkAdapter::prepare_program(black_box(function_calls));
                let (_, peak) = get_memory_usage();
                total_memory += peak as u64;
            }

            std::time::Duration::from_nanos(total_memory / iters)
        })
    });

    group.finish();
}

/// Benchmark memory fragmentation patterns
fn benchmark_memory_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation");

    let fragmentation_test = r#"
        fn fragment_memory() {
            let data1 = [1, 2, 3, 4, 5]
            let data2 = [6, 7, 8, 9, 10]
            let data3 = [11, 12, 13, 14, 15]
            
            let combined = []
            let i = 0
            while i < 5 {
                combined[i * 3] = data1[i]
                combined[i * 3 + 1] = data2[i]
                combined[i * 3 + 2] = data3[i]
                i = i + 1
            }
            
            let sum = 0
            let j = 0
            while j < 15 {
                sum = sum + combined[j]
                j = j + 1
            }
            
            return sum
        }
        
        fragment_memory()
    "#;

    group.bench_function("allocation_patterns", |b| {
        b.iter_custom(|iters| {
            let mut total_memory = 0u64;

            for _ in 0..iters {
                reset_memory_tracking();
                let _ = BenchmarkAdapter::prepare_program(black_box(fragmentation_test));
                let (allocated, _) = get_memory_usage();
                total_memory += allocated as u64;
            }

            std::time::Duration::from_nanos(total_memory / iters)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_compilation_memory,
    benchmark_program_memory,
    benchmark_memory_patterns,
    benchmark_memory_fragmentation
);
criterion_main!(benches);
