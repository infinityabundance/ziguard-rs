# RECEIPT — ZIGUARD-RS.1 (Rust port of upstream `ziguard.awk`) — 2026-06-06

> **Claim (binding):** *ziguard-rs ports the upstream `ziguard.awk` (pinned tzdb-2026b, sha256
> `e4600a23…`) into Rust, reproduces the pinned tzdb source-profile transformations, and is verified
> **byte-identical** to the real `awk` oracle for `main`/`vanguard`/`rearguard` before and after the
> port. It does not redefine tzdb source policy, claim civil-time truth, or replace upstream authority.*

## What shipped

| component | file |
|---|---|
| transform library (faithful port, AWK line-cited) | `src/lib.rs` (`#![forbid(unsafe_code)]`) |
| CLI (`ziguard-rs --format …`, AWK-style stream) | `src/main.rs` |
| full-source oracle test (SHA-pinned) + `ziguard.ck` round-trip | `tests/oracle.rs` |
| 13 per-rule regression fixtures (awk-derived goldens) | `tests/rules.rs`, `tests/fixtures/rules/` |
| fuzz regression replays | `tests/fuzz_regressions.rs` |
| Kani bounded proofs | `src/lib.rs` `#[cfg(kani)] mod kani_harness` |
| detached fuzz crate | `fuzz/` |

Single dependency: `regex` (an AWK port is fundamentally an ERE transformer). `sha2` is a test-only dev-dep.

## Acceptance — all met

| # | gate | result |
|---|---|---|
| 1 | Admit upstream `ziguard.awk` + source (hash + signature) | ✓ `tzdb-2026b.tar.lz` sha256 `ffad46a0…`, OpenPGP **GOODSIG** (Eggert `7E37…7E34`); `ziguard.awk` `e4600a23…`; 10 `$(TDATA)` files pinned — `reports/admission/` |
| 2 | Implement as a **detached** tool, no compiler-core change | ✓ standalone crate; not part of `zic-rs`; no behaviour change to any compiler |
| 3 | Reproduce all three profiles | ✓ `main` / `vanguard` / `rearguard` produced |
| 4 | **Byte-identical** to the oracle | ✓ `main e0225823…`, `vanguard 49e16da4…`, `rearguard 91c4f362…` — equal to the awk oracle **and** the upstream-distributed bytes (no silent normalisation) |
| 5 | Compile-clean check vs upstream consistency | ✓ upstream `ziguard.ck` round-trip reproduced (rearguard∘vanguard, vanguard∘rearguard) |
| 6 | Per-transformation-class behaviour | ✓ 13 awk-derived fixtures (Czechoslovakia, Ireland/Eire, Namibia, Portugal/`%z`, Morocco, Japan, `Etc/GMT`↔`GMT`, `%z`→abbr, link dedup, link-chain, `#STDOFF`) |
| 7 | Regression fixtures per class | ✓ (item 6) + fuzz replays |
| 8 | Source-profile evidence (consumed vs generated) | ✓ recorded in `README.md` (`zic-rs` consumes; `ziguard-rs` reproduces the transformer) |
| 9 | Gate | ✓ `fmt` · `clippy -D warnings` · **18 tests** · oracle byte-identity |

## Evidence dimensions

- **Full-source oracle** — `tests/oracle.rs`: 3/3 forms SHA-pinned to the upstream-distributed values
  over the complete pinned 2026b source. (Independently pinned by `zic-rs` T12.5d.)
- **All IANA releases** — `reports/release-all/RECEIPT-ALL-RELEASES.md`: 48 signed bundles (2016g→2026b),
  **38/38 byte-identical to the 2026b `ziguard.awk` it ports** (Dimension A, 0 port bugs); 6 releases
  reproduced **exactly**; 32 classified script-evolution (corroborated by the `ziguard.awk` hash); 10
  predate the tool.
- **Kani** — `reports/kani/RECEIPT-KANI.md`: bounded sharp proofs on the pure arithmetic helpers
  (numeric-prefix slice bounds · abbreviation-arithmetic non-overflow · field-index bounds), reduced
  surface per doctrine.
- **Fuzzing** — `reports/fuzz/RECEIPT-FUZZ.md`: `transform` + helpers; **F1** (`get_minutes` overflow)
  found → fixed (saturating, byte-identical on real data) → re-verified clean + regression-tested.

## The port's fidelity discipline

`src/lib.rs` mirrors `ziguard.awk` statement-by-statement (every rule cites its AWK line number),
preserving the AWK evaluation model exactly: default field splitting, **field re-split on every `$0`
mutation**, top-to-bottom per-record rule order, and the `line[]` output buffer with `Link`-line
back-patching. Correctness is the oracle match, not a reading of the code.

## Non-claims

- Does **not** redefine tzdb source policy / define a new profile standard.
- Does **not** claim civil-time truth or replace upstream IANA authority.
- Does **not** reproduce **older** releases' shipped profiles (those used an earlier `ziguard.awk` — a
  *script* difference, recorded in the release campaign, not a port error).
- Does **not** claim all future `ziguard.awk` behaviour forever, or correctness beyond the pinned
  admitted releases.
- `PACKRATDATA`/`PACKRATLIST` are not supported in v1 (the upstream Makefile default is empty, which is
  what this reproduces); the packrat paths are inert documented stubs.

## Reproduce

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test   # gate (18 tests)
bash lab/releases/run.sh                                                        # all-releases campaign
cargo kani                                                                      # bounded proofs
( cd fuzz && cargo +nightly fuzz run transform -- -max_total_time=60 )          # fuzz
```
