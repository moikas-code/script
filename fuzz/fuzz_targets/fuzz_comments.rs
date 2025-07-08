#![no_main]
use libfuzzer_sys::fuzz_target;
use script::lexer::fuzz::fuzz_comments;

fuzz_target!(|data: &[u8]| {
    fuzz_comments(data);
});