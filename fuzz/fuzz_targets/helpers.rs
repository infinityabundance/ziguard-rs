#![no_main]
//! The pure helpers (numeric coercion, abbreviation formatting, sub-replacement,
//! link-line rewriting, record splitting) must never panic on arbitrary input.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    ziguard_rs::__fuzz_helpers(&s);
});
