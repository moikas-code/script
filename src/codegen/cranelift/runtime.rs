use cranelift_jit::JITBuilder;
use std::collections::HashMap;
use std::sync::Mutex;

/// Maximum allowed string length to prevent DoS attacks
const MAX_STRING_LENGTH: usize = 10 * 1024 * 1024; // 10MB

/// Maximum allowed allocation size
const MAX_ALLOCATION_SIZE: usize = 100 * 1024 * 1024; // 100MB

/// Global allocation tracker for memory safety
static ALLOCATION_TRACKER: Mutex<Option<AllocationTracker>> = Mutex::new(None);

/// Tracks memory allocations for safety
struct AllocationTracker {
    allocations: HashMap<usize, AllocationInfo>,
}

#[derive(Debug)]
struct AllocationInfo {
    size: usize,
    #[cfg(debug_assertions)]
    backtrace: std::backtrace::Backtrace,
}

impl AllocationTracker {
    fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    fn track_allocation(&mut self, ptr: usize, size: usize) {
        self.allocations.insert(
            ptr,
            AllocationInfo {
                size,
                #[cfg(debug_assertions)]
                backtrace: std::backtrace::Backtrace::capture(),
            },
        );
    }

    fn untrack_allocation(&mut self, ptr: usize) -> Option<AllocationInfo> {
        self.allocations.remove(&ptr)
    }
}

/// Initialize the allocation tracker
fn ensure_tracker_initialized() {
    match ALLOCATION_TRACKER.lock() {
        Ok(mut tracker) => {
            if tracker.is_none() {
                *tracker = Some(AllocationTracker::new());
            }
        }
        Err(poisoned) => {
            // If the mutex is poisoned, we can still recover
            let mut tracker = poisoned.into_inner();
            if tracker.is_none() {
                *tracker = Some(AllocationTracker::new());
            }
        }
    }
}

/// Runtime support functions for Script
pub struct RuntimeSupport;

// Function will be moved to end of file after closure definitions

/// Print a string to stdout with safety checks
#[no_mangle]
pub unsafe extern "C" fn script_print(ptr: *const u8, len: usize) {
    // Validate inputs
    if ptr.is_null() || len == 0 {
        return;
    }

    // Prevent DoS with excessive string lengths
    if len > MAX_STRING_LENGTH {
        eprintln!(
            "Script error: String too long for print (max {} bytes)",
            MAX_STRING_LENGTH
        );
        return;
    }

    // Create slice with validated bounds
    let slice = unsafe {
        // SAFETY: We've validated ptr is non-null and len is reasonable
        std::slice::from_raw_parts(ptr, len)
    };

    // Handle UTF-8 validation gracefully
    match std::str::from_utf8(slice) {
        Ok(s) => print!("{}", s),
        Err(e) => {
            // Try to print the valid portion
            if let Ok(s) = std::str::from_utf8(&slice[..e.valid_up_to()]) {
                print!("{}", s);
            }
            eprintln!(
                "\nScript warning: Invalid UTF-8 in print at byte {}",
                e.valid_up_to()
            );
        }
    }

    // Ensure output is flushed
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// Allocate memory with safety tracking
#[no_mangle]
pub extern "C" fn script_alloc(size: usize) -> *mut u8 {
    ensure_tracker_initialized();

    // Handle zero-size allocations
    if size == 0 {
        return std::ptr::null_mut();
    }

    // Prevent excessive allocations
    if size > MAX_ALLOCATION_SIZE {
        eprintln!(
            "Script error: Allocation too large ({} bytes, max {} bytes)",
            size, MAX_ALLOCATION_SIZE
        );
        return std::ptr::null_mut();
    }

    // Create layout with proper alignment
    let layout = match std::alloc::Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => {
            eprintln!(
                "Script error: Invalid allocation alignment for size {}",
                size
            );
            return std::ptr::null_mut();
        }
    };

    // Allocate memory
    let ptr = unsafe {
        // SAFETY: Layout has been validated above
        std::alloc::alloc(layout)
    };

    if ptr.is_null() {
        eprintln!(
            "Script error: Out of memory (failed to allocate {} bytes)",
            size
        );
        return std::ptr::null_mut();
    }

    // Track the allocation
    if let Ok(mut tracker) = ALLOCATION_TRACKER.lock() {
        if let Some(tracker) = tracker.as_mut() {
            tracker.track_allocation(ptr as usize, size);
        }
    }

    #[cfg(debug_assertions)]
    {
        // Fill with pattern in debug mode to detect use of uninitialized memory
        unsafe {
            std::ptr::write_bytes(ptr, 0xCD, size);
        }
    }

    ptr
}

/// Free memory with safety validation
#[no_mangle]
pub unsafe extern "C" fn script_free(ptr: *mut u8, size: usize) {
    // Handle null pointer
    if ptr.is_null() {
        return;
    }

    // Validate size
    if size == 0 || size > MAX_ALLOCATION_SIZE {
        eprintln!("Script error: Invalid deallocation size {}", size);
        return;
    }

    // Verify this allocation was tracked
    let tracked_info = if let Ok(mut tracker) = ALLOCATION_TRACKER.lock() {
        if let Some(tracker) = tracker.as_mut() {
            tracker.untrack_allocation(ptr as usize)
        } else {
            None
        }
    } else {
        None
    };

    match tracked_info {
        Some(info) => {
            if info.size != size {
                eprintln!(
                    "Script error: Size mismatch in free: allocated {} bytes, freeing {} bytes",
                    info.size, size
                );
                // Continue with deallocation using the tracked size
                let layout = match std::alloc::Layout::from_size_align(info.size, 8) {
                    Ok(layout) => layout,
                    Err(_) => {
                        eprintln!("Script error: Invalid layout for deallocation");
                        return;
                    }
                };

                unsafe {
                    // SAFETY: We tracked this allocation and have the correct size
                    std::alloc::dealloc(ptr, layout);
                }
            } else {
                // Size matches, proceed with deallocation
                let layout = match std::alloc::Layout::from_size_align(size, 8) {
                    Ok(layout) => layout,
                    Err(_) => {
                        eprintln!("Script error: Invalid layout for deallocation");
                        return;
                    }
                };

                unsafe {
                    // SAFETY: Size has been validated against tracked allocation
                    std::alloc::dealloc(ptr, layout);
                }
            }
        }
        None => {
            eprintln!("Script warning: Freeing untracked pointer {:p}", ptr);
            // In production, we might want to skip deallocation of untracked pointers
            // For now, proceed with caution
            let layout = match std::alloc::Layout::from_size_align(size, 8) {
                Ok(layout) => layout,
                Err(_) => {
                    eprintln!("Script error: Invalid layout for deallocation");
                    return;
                }
            };

            unsafe {
                // SAFETY: Caller claims this is a valid allocation
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

/// Safe panic handler that doesn't assume null-terminated strings
#[no_mangle]
pub unsafe extern "C" fn script_panic(msg: *const u8, len: usize) -> ! {
    let message = if msg.is_null() || len == 0 {
        "Script panic: (no message provided)"
    } else if len > MAX_STRING_LENGTH {
        "Script panic: (error message too long)"
    } else {
        // Create slice from provided pointer and length
        let slice = unsafe {
            // SAFETY: We've validated that ptr is non-null and len is reasonable
            std::slice::from_raw_parts(msg, len)
        };

        // Convert to string, handling invalid UTF-8 gracefully
        match std::str::from_utf8(slice) {
            Ok(s) => {
                // Use a static buffer to avoid allocation during panic
                static mut PANIC_BUFFER: [u8; 1024] = [0; 1024];
                let prefix = "Script panic: ";
                let prefix_len = prefix.len();

                unsafe {
                    // SAFETY: We're in a panic handler, single-threaded context
                    if prefix_len + s.len() <= PANIC_BUFFER.len() {
                        PANIC_BUFFER[..prefix_len].copy_from_slice(prefix.as_bytes());
                        PANIC_BUFFER[prefix_len..prefix_len + s.len()]
                            .copy_from_slice(s.as_bytes());
                        if let Ok(full_msg) =
                            std::str::from_utf8(&PANIC_BUFFER[..prefix_len + s.len()])
                        {
                            full_msg
                        } else {
                            "Script panic: (internal error formatting message)"
                        }
                    } else {
                        "Script panic: (message too long for buffer)"
                    }
                }
            }
            Err(_) => "Script panic: (invalid UTF-8 in error message)",
        }
    };

    // Print to stderr
    eprintln!("{}", message);

    // Flush stderr to ensure message is visible
    use std::io::Write;
    let _ = std::io::stderr().flush();

    // In debug mode, print a backtrace
    #[cfg(debug_assertions)]
    {
        eprintln!("\nBacktrace:");
        let backtrace = std::backtrace::Backtrace::capture();
        eprintln!("{}", backtrace);
    }

    // Exit with error code
    std::process::exit(101);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_alloc_free() {
        let size = 1024;
        let ptr = script_alloc(size);
        assert!(!ptr.is_null());

        // Write some data
        unsafe {
            for i in 0..size {
                *ptr.add(i) = (i % 256) as u8;
            }
        }

        unsafe {
            script_free(ptr, size);
        }
    }

    #[test]
    fn test_script_alloc_zero() {
        let ptr = script_alloc(0);
        assert!(ptr.is_null());
    }
}

/// Global closure registry for mapping function IDs to executable code
static CLOSURE_REGISTRY: Mutex<Option<ClosureRegistry>> = Mutex::new(None);

struct ClosureRegistry {
    functions: HashMap<String, ClosureFunction>,
}

type ClosureFunction = Box<dyn Fn(&[*const u8]) -> *const u8 + Send + Sync>;

impl ClosureRegistry {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    fn register(&mut self, id: String, func: ClosureFunction) {
        self.functions.insert(id, func);
    }

    fn get(&self, id: &str) -> Option<&ClosureFunction> {
        self.functions.get(id)
    }
}

/// Create a closure with the given parameters
#[no_mangle]
pub unsafe extern "C" fn script_create_closure(
    function_id_ptr: *const u8,
    function_id_len: usize,
    _param_names: *const u8,
    _param_lengths: *const usize,
    _param_count: usize,
    _capture_names: *const u8,
    _capture_name_lengths: *const usize,
    _capture_values: *const *const u8,
    capture_count: usize,
    _captures_by_ref: u8,
) -> *mut u8 {
    // Validate inputs
    if function_id_ptr.is_null() || function_id_len == 0 {
        return std::ptr::null_mut();
    }

    // In a real implementation, this would:
    // 1. Create a closure structure with the function ID and parameters
    // 2. Store captured values
    // 3. Return a pointer to the heap-allocated closure

    // For now, allocate a simple structure
    let closure_size = 32 + (capture_count * 16); // Simplified size calculation
    let closure_ptr = script_alloc(closure_size);

    if closure_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Store function ID at the beginning
    unsafe {
        let _id_slice = std::slice::from_raw_parts(function_id_ptr, function_id_len);
        if function_id_len <= 24 {
            // Copy function ID (up to 24 bytes)
            std::ptr::copy_nonoverlapping(function_id_ptr, closure_ptr, function_id_len);
            // Store length
            *(closure_ptr.add(24) as *mut usize) = function_id_len;
        }
    }

    closure_ptr
}

/// Invoke a closure with the given arguments
#[no_mangle]
pub extern "C" fn script_invoke_closure(
    closure_ptr: *const u8,
    _args_ptr: *const *const u8,
    _arg_count: i32,
) -> *mut u8 {
    // Validate inputs
    if closure_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // In a real implementation, this would:
    // 1. Extract function ID from closure
    // 2. Look up the function in the registry
    // 3. Set up captured environment
    // 4. Call the function with arguments
    // 5. Return the result

    // For now, return a dummy result
    let result_ptr = script_alloc(8);
    if !result_ptr.is_null() {
        unsafe {
            // Store a dummy i64 value
            *(result_ptr as *mut i64) = 42;
        }
    }

    result_ptr
}

/// Fast-path closure invocation for ≤4 arguments
#[no_mangle]
pub extern "C" fn script_invoke_closure_fast(
    closure_ptr: *const u8,
    arg0: *const u8,
    arg1: *const u8,
    arg2: *const u8,
    arg3: *const u8,
    actual_arg_count: i32,
) -> *mut u8 {
    // Validate inputs
    if closure_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Pack arguments into array based on actual count
    let args: Vec<*const u8> = match actual_arg_count {
        0 => vec![],
        1 => vec![arg0],
        2 => vec![arg0, arg1],
        3 => vec![arg0, arg1, arg2],
        4 => vec![arg0, arg1, arg2, arg3],
        _ => return std::ptr::null_mut(), // Invalid count
    };

    // Delegate to regular invoke
    script_invoke_closure(closure_ptr, args.as_ptr(), actual_arg_count)
}

/// Register runtime functions with the JIT builder
pub fn register_runtime_functions(builder: &mut JITBuilder) {
    // Register print function
    builder.symbol("script_print", script_print as *const u8);

    // Register memory management functions
    builder.symbol("script_alloc", script_alloc as *const u8);
    builder.symbol("script_free", script_free as *const u8);

    // Register panic handler
    builder.symbol("script_panic", script_panic as *const u8);

    // Register closure functions
    builder.symbol("script_create_closure", script_create_closure as *const u8);
    builder.symbol("script_invoke_closure", script_invoke_closure as *const u8);
    builder.symbol(
        "script_invoke_closure_fast",
        script_invoke_closure_fast as *const u8,
    );
}
