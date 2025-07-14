#![no_main]
use libfuzzer_sys::fuzz_target;
use script::lexer::fuzz::fuzz_unicode_edge_cases;

fuzz_target!(|data: &[u8]| {
    fuzz_unicode_edge_cases(data);
});