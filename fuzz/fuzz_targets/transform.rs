#![no_main]
//! Primary hostile-input surface: `transform` must never panic on arbitrary
//! "source" bytes, for any of the three forms.
use libfuzzer_sys::fuzz_target;
use ziguard_rs::{transform, DataForm, Options};

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data).into_owned();
    for form in [DataForm::Main, DataForm::Vanguard, DataForm::Rearguard] {
        let _ = transform(&[("fuzz".to_string(), s.clone())], &Options::new(form));
    }
});
