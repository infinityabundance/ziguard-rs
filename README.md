# ziguard-rs

A faithful Rust port of upstream **`ziguard.awk`** — the IANA tzdb source-profile transformer that
converts the tz source files into **`main`**, **`vanguard`**, and **`rearguard`** form.

```text
zic-rs       compiles the source profiles correctly   (already proven, SOURCE-VARIANT.1)
ziguard-rs   reproduces the profile transformer itself (this crate, ZIGUARD-RS.1)
```

`ziguard-rs` is a companion to the Rust tzdb tooling ecosystem — especially
[`zic-rs`](https://github.com/infinityabundance/zic-rs), which *consumes* these profiles and compiles
them to TZif, and it sits alongside the witness tooling `zdump` /
[`zdump-rs`](https://github.com/infinityabundance/zdump-rs). ziguard-rs is the **source-profile
transformation layer** — not part of the compiler core, but the producer-side step that makes the
profiles in the first place, so that layer is Rust-native too. It does **not** redefine tzdb source
policy; it reproduces the upstream transform.

The three roles form the source-profile loop:

```text
ziguard-rs   source-profile transformation        (this crate — produce main/vanguard/rearguard)
zic-rs       source profile -> TZif compilation    (consume + compile)
zdump / zdump-rs   behaviour witness               (verify the result)
```

## Install / use

```sh
cargo install ziguard-rs
```

Usage mirrors the upstream AWK-style stream model:

```sh
# with file arguments (as `awk -f ziguard.awk f1 f2 …` does), in $(TDATA) order:
ziguard-rs --format main      africa antarctica asia australasia europe \
                              northamerica southamerica etcetera factory backward > main.zi
ziguard-rs --format vanguard  africa … backward > vanguard.zi
ziguard-rs --format rearguard africa … backward > rearguard.zi

# or as a stdin filter:
cat africa … backward | ziguard-rs --format rearguard > rearguard.zi
```

`--format` is required (`main` | `vanguard` | `rearguard`). With no file arguments it reads stdin.

## Correctness — how it's proven

Correctness is **not** asserted from the code; it is the **byte-identical** match against the real
`awk` oracle and the upstream-distributed profiles.

- **Full-source oracle** (`tests/oracle.rs`): over the complete pinned **tzdb-2026b** source set, the
  three outputs match the upstream-distributed SHA-256 exactly:
  `main e0225823…`, `vanguard 49e16da4…`, `rearguard 91c4f362…` — the same hashes IANA ships and that
  `zic-rs` independently pins. The source is admitted with `sha256 ffad46a0…` + OpenPGP `GOODSIG`
  (Paul Eggert, `7E37…7E34`).
- **Upstream's own `ziguard.ck`**: round-tripping the forms (rearguard∘vanguard = rearguard,
  vanguard∘rearguard = vanguard) reproduces upstream's consistency check.
- **All IANA releases** (`reports/release-all/`): across **48** signed release bundles (2016g→2026b),
  ziguard-rs is **byte-identical to the 2026b `ziguard.awk` it ports over all 38 releases that ship the
  script** — zero port bugs; and reproduces the **6** releases (2024b→2026b) whose shipped script is
  unchanged **exactly**. Older releases differ only by the script's own evolution (recorded, not hidden).
- **Per-rule fixtures** (`tests/rules.rs`): each transform class (Czechoslovakia / Ireland / Namibia /
  Portugal / Morocco / Japan / `Etc/GMT`↔`GMT` / `%z`↔explicit-abbr / link dedup / link-chain
  shortening / `#STDOFF`) localised to an awk-derived golden.
- **Kani** (`#[cfg(kani)]`): 3 bounded proofs on the pure arithmetic helpers (numeric-prefix slice
  bounds, abbreviation-arithmetic non-overflow, field-index bounds) — see `reports/kani/`.
- **Fuzzing** (`fuzz/`): `transform` + the helpers; one hostile-input overflow found, fixed (saturating
  arithmetic — byte-identical on real data), and re-verified clean — see `reports/fuzz/`.

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test   # the gate
```

## What this does and does not claim

**It claims:** to reproduce the **2026b** `ziguard.awk` transformation byte-for-byte for admitted tzdb
releases, and to reproduce the distributed `main`/`vanguard`/`rearguard` profiles where the upstream
script is unchanged.

**It does not claim:**

- to redefine or own tzdb source policy, or to be a new profile standard;
- civil-time truth, or to replace upstream IANA authority;
- to reproduce **older** releases' shipped profiles — those used an **earlier** `ziguard.awk` (the
  difference is the script's, recorded in the campaign receipt, not a port error);
- all future `ziguard.awk` behaviour forever, or correctness beyond the pinned admitted releases.

Single dependency: `regex` (an AWK port is fundamentally an ERE transformer). The crate's own code is
`#![forbid(unsafe_code)]`.

## License

Apache-2.0. Upstream `ziguard.awk` is in the public domain (Paul Eggert); this is an independent Rust
reimplementation.
