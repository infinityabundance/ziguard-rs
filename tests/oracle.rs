//! Full-source oracle test (the primary correctness proof).
//!
//! Runs the transform over the complete pinned tzdb-2026b source set (the exact
//! `$(TDATA)` file list, in Makefile order) and asserts the SHA-256 of each
//! produced profile equals the upstream-distributed value — i.e. the bytes
//! `awk -f ziguard.awk` produces, which are also the bytes IANA ships as
//! `main.zi` / `vanguard.zi` / `rearguard.zi`.
//!
//! The source lives under `lab/admit/tzdb-2026b/` (admitted: sha256
//! `ffad46a0…`, OpenPGP GOODSIG by Paul Eggert `7E37…7E34`). That tree is
//! excluded from the published crate, so when it is absent (e.g. running tests
//! from the packaged crate) the test **skips** rather than fails — the same
//! oracle-availability discipline used elsewhere. Set `ZIGUARD_TZDB_SRC` to a
//! directory containing the region files to point it elsewhere.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use ziguard_rs::{transform, DataForm, Options};

/// `$(TDATA)` in Makefile order.
const TDATA: [&str; 10] = [
    "africa",
    "antarctica",
    "asia",
    "australasia",
    "europe",
    "northamerica",
    "southamerica",
    "etcetera",
    "factory",
    "backward",
];

/// Upstream-distributed hashes of the 2026b profiles (also produced by the awk
/// oracle here, and pinned independently by zic-rs's T12.5d).
const SHA_MAIN: &str = "e0225823ae0c3a99a016a4afd7e3c48cfd948132b65fbaa596a47c53ae45e4e1";
const SHA_VANGUARD: &str = "49e16da4a6252a2e432fc1f68bf6daac9a6f73507dde3e3bdbcbbf78e86727ce";
const SHA_REARGUARD: &str = "91c4f362a6bb297efd3cd35bce6b62367a4c00a9721a773bae0cbb0d1bf9fe23";

fn src_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("ZIGUARD_TZDB_SRC") {
        let p = PathBuf::from(d);
        if p.join("africa").is_file() {
            return Some(p);
        }
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lab/admit/tzdb-2026b");
    p.join("africa").is_file().then_some(p)
}

fn load_inputs(dir: &Path) -> Vec<(String, String)> {
    TDATA
        .iter()
        .map(|name| {
            let content = std::fs::read_to_string(dir.join(name))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            (name.to_string(), content)
        })
        .collect()
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn full_source_oracle_byte_identical() {
    let Some(dir) = src_dir() else {
        eprintln!(
            "SKIP full_source_oracle_byte_identical: pinned tzdb-2026b source not found \
             (set ZIGUARD_TZDB_SRC or populate lab/admit/tzdb-2026b/)"
        );
        return;
    };
    let inputs = load_inputs(&dir);
    for (form_name, expected) in [
        ("main", SHA_MAIN),
        ("vanguard", SHA_VANGUARD),
        ("rearguard", SHA_REARGUARD),
    ] {
        let form = DataForm::parse(form_name).unwrap();
        let out = transform(&inputs, &Options::new(form));
        let got = sha256_hex(&out);
        assert_eq!(
            got, expected,
            "{form_name}.zi sha256 diverged from the upstream/awk-oracle value"
        );
    }
}

/// Upstream's own `ziguard.ck` consistency: the transform must round-trip
/// between forms. We verify it purely in-process (no awk needed): converting
/// the produced `vanguard` output to `rearguard` reproduces the `rearguard`
/// output, and vice versa.
#[test]
fn ziguard_ck_round_trip() {
    let Some(dir) = src_dir() else {
        eprintln!("SKIP ziguard_ck_round_trip: pinned source not found");
        return;
    };
    let inputs = load_inputs(&dir);
    let vanguard = transform(&inputs, &Options::new(DataForm::Vanguard));
    let rearguard = transform(&inputs, &Options::new(DataForm::Rearguard));

    // rearguard(vanguard.zi) == rearguard.zi
    let r_from_v = transform(
        &[("vanguard.zi".to_string(), vanguard.clone())],
        &Options::new(DataForm::Rearguard),
    );
    assert_eq!(
        r_from_v, rearguard,
        "rearguard(vanguard.zi) != rearguard.zi"
    );

    // vanguard(rearguard.zi) == vanguard.zi
    let v_from_r = transform(
        &[("rearguard.zi".to_string(), rearguard)],
        &Options::new(DataForm::Vanguard),
    );
    assert_eq!(v_from_r, vanguard, "vanguard(rearguard.zi) != vanguard.zi");
}
