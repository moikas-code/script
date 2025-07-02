use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

mod common;
use common::{simple_patterns, BenchmarkAdapter};

/// Benchmark Fibonacci calculation (classic recursion test)
fn benchmark_fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("iterative", |b| {
        b.iter(|| {
            BenchmarkAdapter::prepare_program(black_box(simple_patterns::FIBONACCI_ITERATIVE))
        })
    });

    group.bench_function("recursive", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(simple_patterns::FIBONACCI_SIMPLE)))
    });

    group.finish();
}

/// Benchmark sorting algorithms (simplified versions)
fn benchmark_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorting");

    // Simplified bubble sort
    let bubble_sort = r#"
        fn bubble_sort(arr, size) {
            let i = 0
            while i < size {
                let j = 0
                while j < size - 1 - i {
                    if arr[j] > arr[j + 1] {
                        let temp = arr[j]
                        arr[j] = arr[j + 1]
                        arr[j + 1] = temp
                    }
                    j = j + 1
                }
                i = i + 1
            }
            return arr
        }
        
        let data = [64, 34, 25, 12, 22, 11, 90, 88, 76, 50]
        let sorted = bubble_sort(data, 10)
    "#;

    group.bench_function("bubble_sort_10", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(bubble_sort)))
    });

    // Selection sort
    let selection_sort = r#"
        fn selection_sort(arr, size) {
            let i = 0
            while i < size - 1 {
                let min_idx = i
                let j = i + 1
                
                while j < size {
                    if arr[j] < arr[min_idx] {
                        min_idx = j
                    }
                    j = j + 1
                }
                
                if min_idx != i {
                    let temp = arr[i]
                    arr[i] = arr[min_idx]
                    arr[min_idx] = temp
                }
                
                i = i + 1
            }
            return arr
        }
        
        let data = [64, 34, 25, 12, 22, 11, 90, 88, 76, 50, 33, 77, 44, 66, 99, 23]
        let sorted = selection_sort(data, 16)
    "#;

    group.bench_function("selection_sort_16", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(selection_sort)))
    });

    group.finish();
}

/// Benchmark simple data processing
fn benchmark_data_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_processing");

    let data_processing = r#"
        fn process_data(data, size) {
            let sum = 0
            let max_val = data[0]
            let min_val = data[0]
            let even_count = 0
            let odd_count = 0
            
            let i = 0
            while i < size {
                let val = data[i]
                sum = sum + val
                
                if val > max_val {
                    max_val = val
                }
                
                if val < min_val {
                    min_val = val
                }
                
                if val % 2 == 0 {
                    even_count = even_count + 1
                } else {
                    odd_count = odd_count + 1
                }
                
                i = i + 1
            }
            
            let avg = sum / size
            return avg + max_val + min_val
        }
        
        let numbers = [15, 23, 87, 45, 12, 67, 34, 89, 56, 78, 23, 45, 67, 89, 12, 34, 56, 78, 90, 21]
        let result = process_data(numbers, 20)
    "#;

    group.bench_function("array_processing", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(data_processing)))
    });

    group.finish();
}

/// Benchmark search algorithms
fn benchmark_search_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_algorithms");

    // Linear search
    let linear_search = r#"
        fn linear_search(arr, size, target) {
            let i = 0
            while i < size {
                if arr[i] == target {
                    return i
                }
                i = i + 1
            }
            return -1
        }
        
        let data = [12, 45, 23, 67, 89, 34, 56, 78, 90, 21, 43, 65, 87, 32, 54, 76, 98, 10]
        let result1 = linear_search(data, 18, 67)
        let result2 = linear_search(data, 18, 99)
        let result3 = linear_search(data, 18, 12)
    "#;

    group.bench_function("linear_search", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(linear_search)))
    });

    group.finish();
}

/// Benchmark mathematical calculations
fn benchmark_mathematical_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mathematical_calculations");

    // Prime number calculation
    let prime_calculation = r#"
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
        
        fn count_primes(limit) {
            let count = 0
            let i = 2
            while i <= limit {
                if is_prime(i) {
                    count = count + 1
                }
                i = i + 1
            }
            return count
        }
        
        let prime_count = count_primes(100)
    "#;

    group.bench_function("prime_calculation", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(prime_calculation)))
    });

    // GCD calculation
    let gcd_calculation = r#"
        fn gcd(a, b) {
            while b != 0 {
                let temp = b
                b = a % b
                a = temp
            }
            return a
        }
        
        fn lcm(a, b) {
            return (a * b) / gcd(a, b)
        }
        
        let result1 = gcd(48, 18)
        let result2 = lcm(12, 15)
        let result3 = gcd(100, 75)
        let result4 = lcm(20, 30)
    "#;

    group.bench_function("gcd_lcm", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(gcd_calculation)))
    });

    group.finish();
}

/// Benchmark string processing
fn benchmark_string_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_processing");

    // Simple string building
    let string_building = r#"
        let result = ""
        let i = 0
        while i < 50 {
            result = result + "Item " + i + " "
            i = i + 1
        }
        
        let final_result = "Result: " + result
    "#;

    group.bench_function("string_building", |b| {
        b.iter(|| BenchmarkAdapter::prepare_program(black_box(string_building)))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_fibonacci,
    benchmark_sorting,
    benchmark_data_processing,
    benchmark_search_algorithms,
    benchmark_mathematical_calculations,
    benchmark_string_processing
);
criterion_main!(benches);
