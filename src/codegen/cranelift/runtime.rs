use cranelift_jit::JITBuilder;
use std::ffi::CString;

/// Runtime support functions for Script
pub struct RuntimeSupport;

/// Register runtime functions with the JIT builder
pub fn register_runtime_functions(builder: &mut JITBuilder) {
    // Register print function
    builder.symbol("script_print", script_print as *const u8);
    
    // Register memory management functions
    builder.symbol("script_alloc", script_alloc as *const u8);
    builder.symbol("script_free", script_free as *const u8);
    
    // Register panic handler
    builder.symbol("script_panic", script_panic as *const u8);
}

/// Print a string to stdout
#[no_mangle]
pub extern "C" fn script_print(ptr: *const u8, len: usize) {
    unsafe {
        if !ptr.is_null() && len > 0 {
            let slice = std::slice::from_raw_parts(ptr, len);
            if let Ok(s) = std::str::from_utf8(slice) {
                print!("{}", s);
            }
        }
    }
}

/// Allocate memory
#[no_mangle]
pub extern "C" fn script_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    
    let layout = std::alloc::Layout::from_size_align(size, 8)
        .unwrap_or_else(|_| script_panic(b"Invalid allocation size\0".as_ptr()));
    
    unsafe {
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            script_panic(b"Out of memory\0".as_ptr());
        }
        ptr
    }
}

/// Free memory
#[no_mangle]
pub extern "C" fn script_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        let layout = std::alloc::Layout::from_size_align(size, 8)
            .unwrap_or_else(|_| script_panic(b"Invalid deallocation size\0".as_ptr()));
        
        unsafe {
            std::alloc::dealloc(ptr, layout);
        }
    }
}

/// Panic handler
#[no_mangle]
pub extern "C" fn script_panic(msg: *const u8) -> ! {
    unsafe {
        let c_str = CString::from_raw(msg as *mut i8);
        eprintln!("Script panic: {}", c_str.to_string_lossy());
        std::mem::forget(c_str); // Don't free the string we don't own
    }
    std::process::exit(1);
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
        
        script_free(ptr, size);
    }
    
    #[test]
    fn test_script_alloc_zero() {
        let ptr = script_alloc(0);
        assert!(ptr.is_null());
    }
}