#![no_main]

use libfuzzer_sys::fuzz_target;
use script::runtime::{Runtime, RuntimeConfig};
use std::alloc::Layout;

fuzz_target!(|data: &[u8]| {
    // Limit input size to prevent resource exhaustion
    if data.len() > 1_000 {
        return;
    }
    
    // Initialize runtime for testing
    if let Ok(runtime) = Runtime::new(RuntimeConfig::default()) {
        // Test memory allocation patterns based on fuzz input
        for chunk in data.chunks(8) {
            if chunk.len() >= 2 {
                let size = ((chunk[0] as usize) % 1024) + 1; // 1-1024 bytes
                let align = 1 << ((chunk[1] as usize) % 4); // 1, 2, 4, 8 byte alignment
                
                if let Ok(layout) = Layout::from_size_align(size, align) {
                    // Test allocation and deallocation
                    if let Ok(ptr) = runtime.memory().allocate(layout) {
                        // Runtime should handle allocation/deallocation safely
                        unsafe {
                            runtime.memory().deallocate(ptr, layout);
                        }
                    }
                }
            }
        }
        
        // Test garbage collection with fuzz data
        runtime.collect_garbage();
    }
});