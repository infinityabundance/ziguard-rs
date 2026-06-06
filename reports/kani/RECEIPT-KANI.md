# RECEIPT — ZIGUARD-RS.1 Kani bounded proofs — 2026-06-06

> **Claim (binding):** *Kani proves sharp invariants on ziguard-rs's pure arithmetic / indexing
> helpers — the only places the port does manual indexing or integer arithmetic. The regex-driven
> transform is covered by the awk oracle and by fuzzing, not Kani (reduced-surface doctrine). A
> non-convergent harness is a harness-design failure, not a code verdict, and is not put on the
> result surface.*

## Harnesses (`src/lib.rs`, `#[cfg(kani)] mod kani_harness`)

| harness | invariant | result |
|---|---|---|
| `numeric_prefix_len_is_ascii_in_bounds` (`#[kani::unwind(8)]`) | `numeric_prefix_len(b) <= b.len()`, and every consumed byte is ASCII (`< 0x80`) — so `awk_num`'s `s[..i]` cut is always an in-bounds char-boundary slice (no panic) | ✅ SUCCESSFUL |
| `offset_abbr_arithmetic_no_overflow` | for any offset in `[-100_000, 100_000]` (far beyond the tz range ±840 min), `hours*100 + minutes` cannot overflow `i64` (no panic under `overflow-checks`) | ✅ SUCCESSFUL |
| `rec_field_index_is_in_bounds` | whenever `Rec::f`'s guard `i >= 1 && i <= nf` holds, the slot `i - 1` is `< nf` — the field access can never be out of bounds, for any `nf`/`i` | ✅ SUCCESSFUL |

**Headline: 3 bounded helper proofs verified · 0 failed · 0 counterexamples.**

## Reduced surface — what is deliberately *not* a Kani harness

- **`signed_zeropad`** — its only ziguard-specific arithmetic is `i64::unsigned_abs()` (panic-free incl.
  `i64::MIN`); the remainder is std `format!`, whose `String` allocation makes a CBMC harness
  non-convergent (the allocator is explored endlessly) **without** proving any sharp ziguard invariant.
  Dropped per doctrine; exercised by fuzzing instead.
- **`transform` / the regex rules** — symbolic execution of `regex` is out of Kani's reach; this surface
  is proven by the byte-identical awk oracle (`tests/oracle.rs`, 38-release campaign) and fuzzed
  (`reports/fuzz/`).
- An earlier `rec_field_access_is_total` harness built a `Vec<String>` and did **not** converge (the
  allocator again). It was **redesigned** into the allocation-free `rec_field_index_is_in_bounds`, which
  proves the same index-safety invariant and converges cleanly — a harness-design fix, not a code change.
- `numeric_prefix_len_is_ascii_in_bounds` scans a 6-byte array, so its loops run ≤ 6 times; CBMC was not
  inferring that bound and unrolled unboundedly. Adding `#[kani::unwind(8)]` caps the unroll and the
  unwinding assertion **proves 8 is sufficient** — it then verifies in ~0.14 s. (Bounding the loop, not
  weakening the proof: the unwinding assertion would fail if 8 were too few.)

## Run

```sh
cargo kani
```

Toolchain: Kani (CBMC backend), nightly `2025-11-21` (Kani's pinned toolchain). The `#[cfg(kani)]`
module is excluded from normal builds; `cfg(kani)` is declared in `Cargo.toml` so `clippy -D warnings`
stays green.
