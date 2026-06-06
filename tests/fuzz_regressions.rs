//! Regression tests for inputs discovered by fuzzing (`fuzz/`).
//!
//! Each seed that once crashed is replayed here so a regression is caught by
//! `cargo test`, not only by re-running the fuzzer.
use ziguard_rs::{transform, DataForm, Options};

/// **F1** (`fuzz helpers`, input `2999999999999999999`): a `%z` line carrying a
/// huge offset overflowed `get_minutes` (`60 * hours`) under `overflow-checks`
/// in rearguard form. Fixed with saturating arithmetic — must not panic in any
/// form. (AWK uses f64 and never overflowed; saturating is byte-identical on all
/// real tz offsets, which are tiny.)
#[test]
fn f1_huge_offset_in_z_line_does_not_panic() {
    let input = "Zone\tX\t2999999999999999999:00\t-\t%z\n".to_string();
    for form in [DataForm::Main, DataForm::Vanguard, DataForm::Rearguard] {
        let _ = transform(&[("f1".to_string(), input.clone())], &Options::new(form));
    }
}

/// A grab-bag of hostile shapes (huge `#STDOFF`, multibyte Link target, lone
/// markers, long digit runs) must not panic in any form.
#[test]
fn hostile_shapes_do_not_panic() {
    let cases = [
        "\t\t#STDOFF\t99999999999999999:59:59.999\n\t\t99999999999999999:59:59.999\t-\tX\n",
        "Link\tÀÉ\tB\nZone\tB\t0\t-\tX\n",
        "Rule\tMorocco\t99999999999999999\tmax\t-\tMay\t1\t0:00\t-1:00\t-\n",
        "Zone GMT 0 - %z\n#",
        "\u{0}\t\t-9999999999999:00\t-\t%z",
    ];
    for c in cases {
        for form in [DataForm::Main, DataForm::Vanguard, DataForm::Rearguard] {
            let _ = transform(&[("h".to_string(), c.to_string())], &Options::new(form));
        }
    }
}
