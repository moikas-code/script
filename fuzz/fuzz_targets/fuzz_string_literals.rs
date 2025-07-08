#![no_main]
use libfuzzer_sys::fuzz_target;
use script::lexer::fuzz::fuzz_string_literals;

fuzz_target!(|data: &[u8]| {
    fuzz_string_literals(data);
});