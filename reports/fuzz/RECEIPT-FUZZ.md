# RECEIPT — ZIGUARD-RS.1 fuzzing — 2026-06-06

> **Claim (binding):** *ziguard-rs's `transform` and pure helpers were fuzzed; one hostile-input
> panic was found (`get_minutes` integer overflow), fixed (saturating arithmetic, byte-identical on
> real data), and re-verified clean. A fuzz target existing is not a fuzz result; this records a
> bounded smoke run, **not** saturation.*

## Harness

Detached `fuzz/` crate (empty `[workspace]` → never compiled by the parent gate; the zic-rs `fuzz/`
discipline). Two libFuzzer targets:

| target | surface | input |
|---|---|---|
| `transform` | `transform(...)` for all three forms — the primary hostile-input surface | arbitrary bytes → `from_utf8_lossy` → source text |
| `helpers` | every pure helper (`numeric_prefix_len`, `awk_num`, `is_strnum`, `field_falsy`, `get_minutes`, `round_to_second`, `offset_abbr`, `signed_zeropad`, `expand_repl`, `make_linkline`, `split_records`) via `__fuzz_helpers` (behind the `fuzzing` feature) | arbitrary bytes |

Corpus seeded from the real per-rule fixtures + targeted `%z` / Morocco / Link / `#STDOFF` / numeric seeds.

**Toolchain:** `cargo-fuzz 0.13.1`, `rustc 1.98.0-nightly (31a9463c6 2026-05-25)`, libFuzzer + AddressSanitizer,
x86_64-linux. Build carries `debug-assertions` + `overflow-checks` (so integer overflow traps).

## Finding F1 — `get_minutes` integer overflow (found, fixed, re-verified)

- **Target:** `helpers`. **Input:** `2999999999999999999` (a 19-digit run).
- **Cause:** `get_minutes` computed `60 * hours` (and `sign * minutes`) in `i64`. A `%z` line carrying a
  huge offset (reachable via `transform` in **rearguard** form) overflowed under `overflow-checks` → panic.
  Upstream `ziguard.awk` uses f64 and never overflows.
- **Fix:** saturating arithmetic in `get_minutes`, `offset_abbr`, and `round_to_second`. On every real tz
  offset (|min| < ~1000) this is **bit-for-bit identical** to the previous code — the full-source oracle
  test (`main`/`vanguard`/`rearguard` sha256) and the 38-release campaign still pass unchanged. Also
  proactively replaced a byte-index slice in `make_linkline` with `starts_with` (multibyte-safe).
- **Regression:** `tests/fuzz_regressions.rs::f1_huge_offset_in_z_line_does_not_panic` (+ a
  `hostile_shapes_do_not_panic` grab-bag) replay the crash via `transform`; the crash artifact is folded
  into `fuzz/corpus/helpers/` so the fuzzer replays it forever.

## Smoke run (after the fix)

| target | runs | wall | crashes | exit | coverage |
|---|---|---|---|---|---|
| `helpers`   | **941,539** | 21 s | **0** | 0 | cov 3327 |
| `transform` | **1,752**   | 26 s | **0** | 0 | cov 6466 |

(The `transform` target is slower per-exec: it runs all three forms through the full regex transform per input.)

## Non-claims (honest)

- This is a **bounded smoke run** (tens of seconds per target), **not** a saturation / 24h-class campaign.
  No coverage-saturation or exhaustiveness is claimed.
- "No crash in this run" ≠ "no possible crash". A longer campaign is a future operator step.
- Results are for this host / this libFuzzer+ASan build / this corpus.

## Reproduce

```sh
cd fuzz
cargo +nightly fuzz run helpers   -- -max_total_time=25
cargo +nightly fuzz run transform -- -max_total_time=25
```
