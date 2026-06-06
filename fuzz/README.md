# ziguard-rs fuzzing

A **detached** `cargo-fuzz` crate: the empty `[workspace]` in `Cargo.toml` makes this its own
workspace root, so the parent crate's `cargo build` / `test` / `clippy` / `fmt` never compile it. The
fuzz crate is inert to the parent gate.

## Targets

- **`transform`** — `transform(...)` over arbitrary "source" bytes, for `main`/`vanguard`/`rearguard`.
  The primary hostile-input surface; must never panic.
- **`helpers`** — the pure helpers (numeric coercion, abbreviation formatting, `sub`-replacement,
  link-line rewriting, record splitting) via the `fuzzing`-feature-gated `__fuzz_helpers`.

## Run

```sh
cargo +nightly fuzz run transform -- -max_total_time=60
cargo +nightly fuzz run helpers   -- -max_total_time=60
```

## Discipline (the standing non-claim)

> A fuzz target existing is **not** a fuzz result. A run is admitted only by a receipt
> (`../reports/fuzz/`). A bounded smoke run is **not** saturation — no exhaustiveness is claimed.

Known fixed finding: **F1** (`get_minutes` integer overflow) — see `../reports/fuzz/RECEIPT-FUZZ.md`
and the replay in `../tests/fuzz_regressions.rs`.
