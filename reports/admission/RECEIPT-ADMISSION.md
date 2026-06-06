# RECEIPT — ZIGUARD-RS.1 reference admission (tzdb-2026b) — 2026-06-06

> **Claim (binding):** *the oracle and source ziguard-rs is validated against are admitted by
> signature + hash, not assumed. The transformer script (`ziguard.awk`) and the source set are pinned;
> the three produced profiles match the upstream-distributed hashes.*

## Bundle

| item | value |
|---|---|
| archive | `tzdb-2026b.tar.lz` (`https://data.iana.org/time-zones/releases/`) |
| sha256 | `ffad46a04c8d1624197056630af475a35f3556d0887f028ac1bd33b7d47dc653` |
| OpenPGP | **GOODSIG** — key `7E37 92A9 D8AC F7D6 33BC 1588 ED97 E90E 62AA 7E34`, "Paul Eggert" (fingerprint-anchored) |
| extractor | `bsdtar` (libarchive, lzip) |

## The oracle script

| file | sha256 (prefix) |
|---|---|
| **`ziguard.awk`** | `e4600a2360b69224…` — the exact script ziguard-rs ports (line numbers in `src/lib.rs` cite it) |

Oracle interpreter: **GNU Awk 5.4.0**. Invocation per the upstream `Makefile` (rule at `Makefile:744`):
`awk -v DATAFORM=<form> -v PACKRATDATA='' -v PACKRATLIST='' -f ziguard.awk $(TDATA)`.

## Source set — `$(TDATA)`, in Makefile order

| # | file | sha256 (prefix) |
|---|---|---|
| 1 | `africa` | `c19940072a9e79d5…` |
| 2 | `antarctica` | `e410ad71c9450828…` |
| 3 | `asia` | `cd12fe2bd64a02d8…` |
| 4 | `australasia` | `e60bee81387d105d…` |
| 5 | `europe` | `b9c98254bed0773d…` |
| 6 | `northamerica` | `30bdcadf734a87b7…` |
| 7 | `southamerica` | `c6e17ee367c6d7c1…` |
| 8 | `etcetera` | `7281f095b42c13c4…` |
| 9 | `factory` | `ae2ec1d36dabf79a…` |
| 10 | `backward` | `d2f4c8953f204982…` |

(`PACKRATDATA` / `PACKRATLIST` empty — the upstream Makefile default; the packrat paths are inert.)

## Produced profiles (oracle == ziguard-rs == upstream-distributed)

| profile | sha256 | lines |
|---|---|---|
| `main.zi` | `e0225823ae0c3a99a016a4afd7e3c48cfd948132b65fbaa596a47c53ae45e4e1` | 18798 |
| `vanguard.zi` | `49e16da4a6252a2e432fc1f68bf6daac9a6f73507dde3e3bdbcbbf78e86727ce` | 18798 |
| `rearguard.zi` | `91c4f362a6bb297efd3cd35bce6b62367a4c00a9721a773bae0cbb0d1bf9fe23` | 18798 |

These three are independently pinned by `zic-rs` (T12.5d `REF_2026B_{MAIN,VANGUARD,REARGUARD}_ZI_SHA256`).

## Reproduce

```sh
# admit
curl -sO https://data.iana.org/time-zones/releases/tzdb-2026b.tar.lz{,.asc}
sha256sum tzdb-2026b.tar.lz          # ffad46a0…
gpg --verify tzdb-2026b.tar.lz.asc tzdb-2026b.tar.lz   # GOODSIG 7E37…7E34
bsdtar -xf tzdb-2026b.tar.lz && cd tzdb-2026b
# oracle
for f in main vanguard rearguard; do
  awk -v DATAFORM=$f -f ziguard.awk africa antarctica asia australasia europe \
      northamerica southamerica etcetera factory backward | sha256sum
done
```
