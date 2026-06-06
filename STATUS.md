# STATUS — ziguard-rs

**ZIGUARD-RS.1 — sealed.** A faithful Rust port of upstream `ziguard.awk`, the IANA tzdb
source-profile transformer (produces `main` / `vanguard` / `rearguard` from the tz source files).

## Live status

- **2026b full-source profiles byte-identical to the upstream-distributed hashes** —
  `main e0225823…`, `vanguard 49e16da4…`, `rearguard 91c4f362…`; admitted source `sha256 ffad46a0…`
  + OpenPGP **GOODSIG** (Paul Eggert `7E37…7E34`).
- **upstream `ziguard.ck` consistency reproduced** (rearguard∘vanguard = rearguard, vanguard∘rearguard = vanguard).
- **48 signed release bundles** (2016g → 2026b) swept.
- **38 releases shipping `ziguard.awk` byte-identical under the ported 2026b script — 0 port bugs.**
- older differences **attributed to upstream script evolution** (port bug vs version-difference never collapsed).
- **13 per-rule fixtures** cover the transformation classes (Czechoslovakia · Ireland · Namibia ·
  Portugal · Morocco · Japan · `Etc/GMT`↔`GMT` · `%z`↔abbr · link dedup · link-chain · `#STDOFF`).
- **Kani** proves 3 bounded helper invariants (numeric-prefix slice bounds · abbreviation-arithmetic
  non-overflow · field-index bounds) — 0 failed.
- **Fuzzing** found one hostile-input overflow (F1, `get_minutes`), fixed (saturating arithmetic,
  byte-identical on real data) and re-verified clean.
- own crate **`#![forbid(unsafe_code)]`**; gate: `fmt` · `clippy -D warnings` · **18 tests**.

## Published

- **crates.io:** `ziguard-rs` v0.1.0 (Apache-2.0) — `cargo install ziguard-rs`
- **GitHub:** <https://github.com/infinityabundance/ziguard-rs>

## Place in the ecosystem

```text
ziguard-rs   source-profile transformation   (produce main/vanguard/rearguard)
zic-rs       source profile -> TZif           (consume + compile)
zdump-rs     behaviour witness                (verify)
```

ziguard-rs closes the source-profile loop: the Rust tzdb stack now *generates and verifies* the
profiles, not merely *consumes* them.

## Receipts

- [`reports/RECEIPT-ZIGUARD-RS-1.md`](reports/RECEIPT-ZIGUARD-RS-1.md) — the seal (acceptance gate, evidence map)
- [`reports/admission/`](reports/admission/) — source + oracle admission (hash + signature)
- [`reports/release-all/`](reports/release-all/) — the 48-bundle all-releases campaign
- [`reports/kani/`](reports/kani/) — bounded proofs · [`reports/fuzz/`](reports/fuzz/) — fuzzing (F1)

## Non-claims

Does **not** redefine tzdb source policy, claim civil-time truth, replace upstream IANA authority, or
reproduce **older** releases' shipped profiles (those used an earlier `ziguard.awk` — a script
difference, recorded, not a port error). Verified for the admitted releases, this oracle awk, this host.
