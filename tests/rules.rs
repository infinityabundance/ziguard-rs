//! Per-rule regression fixtures.
//!
//! Each `tests/fixtures/rules/<unit>.in` is a real zone/rule block extracted
//! from the pinned tzdb-2026b source; each `<unit>.<form>.out` is the
//! corresponding **`awk -f ziguard.awk` oracle output** (so the expectations
//! cannot be a hand-port error). Every unit isolates a transformation class:
//!
//! * `prague`      — Czechoslovakia negative-SAVE swap
//! * `dublin`/`eire` — Ireland negative-SAVE + `IST/GMT`
//! * `windhoek`/`namibia` — Namibia negative-SAVE
//! * `lisbon`      — Portugal `%z` + `#STDOFF` subsecond context
//! * `casablanca`/`morocco` — Morocco negative-SAVE + inline `%z` abbreviation
//! * `etc_gmt`     — `Zone Etc/GMT` ↔ `Zone GMT` (vanguard) + Link comment-out
//! * `simferopol`/`tirane` — `%z` → explicit abbreviation (rearguard)
//! * `japan`       — `Sat>=8 25:00` ↔ `Sun>=9 1:00` (rearguard)
//! * `backward_head` — Link dedup / link-chain shortening
//!
//! The full proof is the byte-identical full-source oracle in `oracle.rs`; these
//! fixtures localise any regression to a single rule.

use std::fs;
use std::path::PathBuf;

use ziguard_rs::{transform, DataForm, Options};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}

fn check_unit(stem: &str) {
    let dir = fixtures_dir();
    let input = fs::read_to_string(dir.join(format!("{stem}.in")))
        .unwrap_or_else(|e| panic!("read {stem}.in: {e}"));
    for form_name in ["main", "vanguard", "rearguard"] {
        let form = DataForm::parse(form_name).unwrap();
        let expected = fs::read_to_string(dir.join(format!("{stem}.{form_name}.out")))
            .unwrap_or_else(|e| panic!("read {stem}.{form_name}.out: {e}"));
        let got = transform(
            &[(format!("{stem}.in"), input.clone())],
            &Options::new(form),
        );
        assert_eq!(
            got, expected,
            "ziguard-rs output for unit '{stem}' form '{form_name}' diverged from the awk oracle"
        );
    }
}

macro_rules! unit_test {
    ($name:ident, $stem:literal) => {
        #[test]
        fn $name() {
            check_unit($stem);
        }
    };
}

unit_test!(prague_czechoslovakia_negative_save, "prague");
unit_test!(dublin_ireland_negative_save, "dublin");
unit_test!(eire_rule_negative_save, "eire");
unit_test!(windhoek_namibia_zone, "windhoek");
unit_test!(namibia_rule, "namibia");
unit_test!(lisbon_portugal_z_and_stdoff, "lisbon");
unit_test!(casablanca_morocco_zone, "casablanca");
unit_test!(morocco_rules, "morocco");
unit_test!(etc_gmt_vanguard_swap, "etc_gmt");
unit_test!(simferopol_z_expansion, "simferopol");
unit_test!(tirane_z_expansion, "tirane");
unit_test!(japan_sat8_sun9, "japan");
unit_test!(backward_link_dedup_and_chains, "backward_head");

/// The CLI's stdin path (one concatenated input) must equal the file-args path,
/// since the only file-boundary-sensitive logic (packrat) is inert by default.
#[test]
fn stdin_equivalent_to_file_args() {
    let dir = fixtures_dir();
    let mut concatenated = String::new();
    let mut per_file: Vec<(String, String)> = Vec::new();
    for stem in ["prague", "japan", "etc_gmt"] {
        let c = fs::read_to_string(dir.join(format!("{stem}.in"))).unwrap();
        concatenated.push_str(&c);
        per_file.push((format!("{stem}.in"), c));
    }
    for form_name in ["main", "vanguard", "rearguard"] {
        let form = DataForm::parse(form_name).unwrap();
        let as_stdin = transform(
            &[("-".to_string(), concatenated.clone())],
            &Options::new(form),
        );
        let as_files = transform(&per_file, &Options::new(form));
        assert_eq!(
            as_stdin, as_files,
            "stdin vs file-args mismatch ({form_name})"
        );
    }
}
