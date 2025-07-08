#![no_main]
use libfuzzer_sys::fuzz_target;
use script::lexer::fuzz::fuzz_lexer;

fuzz_target!(|data: &[u8]| {
    fuzz_lexer(data);
});