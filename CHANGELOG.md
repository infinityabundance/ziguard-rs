# Changelog

## 0.1.1

- Docs only — no code, no behaviour change (the transform is byte-for-byte unchanged).
- Clarified the README's ecosystem positioning: links to
  [`zic-rs`](https://github.com/infinityabundance/zic-rs) (consumes these profiles → TZif) and the
  witness tooling [`zdump-rs`](https://github.com/infinityabundance/zdump-rs), plus the source-profile
  loop diagram.

## 0.1.0

- Initial release. A faithful Rust port of upstream `ziguard.awk` (tzdb source-profile transformer):
  `main` / `vanguard` / `rearguard` output **byte-identical** to the upstream-distributed 2026b profile
  hashes, with the admitted source pinned by sha256 + OpenPGP signature.
- Verified across 48 signed tzdb release bundles (byte-identical to the 2026b script over all 38
  releases that ship it; zero port bugs), upstream `ziguard.ck` round-trip, 13 per-rule fixtures,
  3 Kani bounded helper proofs, and fuzzing (one hostile-input overflow found, fixed, re-verified).
- `#![forbid(unsafe_code)]`; single dependency `regex`.
