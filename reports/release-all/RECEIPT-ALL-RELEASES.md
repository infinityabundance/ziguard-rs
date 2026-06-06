# RECEIPT — ZIGUARD-RS.1 all-IANA-releases oracle campaign — 2026-06-06

> **Claim (binding):** *ziguard-rs is byte-identical to the real upstream `ziguard.awk` it ports
> (pinned tzdb-2026b, sha256 `e4600a23…`) over the source of **every** IANA release that ships
> `ziguard.awk` — 38/38, all three forms. Where ziguard-rs differs from an **older** release's
> own `ziguard.awk`, the difference is fully attributable to that release's script being an
> earlier version (script evolution), not a port error.*

## Method

For every signed `tzdb-*.tar.lz` bundle in the IANA release archive
(`https://data.iana.org/time-zones/releases/`, 48 bundles, **2016g → 2026b**):

1. download the bundle **and** its `.asc`; **OpenPGP-verify** (must be `GOODSIG`); record `sha256`;
2. extract; if it ships `ziguard.awk`, hash it and resolve that release's `$(TDATA)` file set from
   its own `Makefile` (`make` expansion of `$(YDATA) $(NDATA) $(BACKWARD)`);
3. over that identical input set, run three transformers for each of `main`/`vanguard`/`rearguard`:
   - **`awkR`** — the release's **own** `ziguard.awk` (the "real ziguard" shipped that release);
   - **`awk26`** — the pinned **2026b** `ziguard.awk` (the exact script ziguard-rs ports);
   - **`rs`** — **ziguard-rs** (the Rust port);
4. compare two independent dimensions:
   - **A — port fidelity:** `rs == awk26` (must hold for **all** releases — any failure is a port bug);
   - **B — release reproduction:** `rs == awkR` (holds where the release's script equals 2026b's;
     classified *script-evolution* where it differs, corroborated by the `ziguard.awk` hash).

Harness: [`lab/releases/run.sh`](../../lab/releases/run.sh); raw results
[`lab/releases/releases.tsv`](../../lab/releases/releases.tsv); log `lab/releases/run.log`.
Oracle awk: GNU Awk 5.4.0. ziguard-rs: release build, this repo.

## Results

| metric | value |
|---|---|
| bundles examined | **48** (2016g → 2026b) |
| OpenPGP signature | **48 / 48 GOODSIG** (key `7E37 92A9 D8AC F7D6 33BC 1588 ED97 E90E 62AA 7E34`, Paul Eggert) |
| predate `ziguard.awk` | **10** (2016g–2017c, 2018a–2018c) — classified `no_ziguard_predates_tool` |
| ship `ziguard.awk` | **38** (2018d → 2026b) |
| **Dimension A — `rs == awk26` (port fidelity)** | **38 / 38 byte-identical** ✓ (all three forms) |
| **PORT_BUG** (A failed) | **0** |
| **UNEXPECTED** (same script, different output) | **0** |
| Dimension B — `reproduces_release_exactly` | **6** (2024b, 2025a, 2025b, 2025c, 2026a, 2026b) |
| Dimension B — `script_evolution_expected` | **32** (2018d → 2024a) |

### The decisive logical attribution

Because `rs == awk26` is **byte-identical for every release** (Dimension A, 38/38), it follows for the
32 script-evolution releases that

```
diff(rs, awkR)  ==  diff(awk26, awkR)
```

i.e. ziguard-rs's divergence from an older release equals **exactly** the difference between the 2026b
`ziguard.awk` and that release's older `ziguard.awk`. The divergence is the *script's* evolution, not the
*port's* infidelity. This is corroborated independently by the `ziguard.awk` hash (`script_eq = no` for
all 32, `yes` for the 6 that reproduce exactly).

### Script-evolution convergence (Dimension-B diff line counts, older → newer)

```
2018d : 48318   (pre-vanguard/rearguard format — major)
2018e–2022a : ~1568–1652   (the negative-SAVE / %z era; zone-specific rules still accreting)
2022b–2022e : 24
2022f–2024a : 4
2024b–2026b : 0   (ziguard.awk identical to the pinned 2026b script -> exact reproduction)
```

The monotone decay (1630 → 24 → 4 → 0) tracks `ziguard.awk` stabilising upstream; from **2024b** the
script is byte-identical to 2026b's and ziguard-rs reproduces the shipped `main.zi`/`vanguard.zi`/
`rearguard.zi` exactly.

## What this proves / does not prove

**Proves:**

- ziguard-rs faithfully reproduces the **2026b** `ziguard.awk` over the real source of **all 38**
  releases that have it (not just 2026b) — a strong cross-release fidelity result for the port.
- For the **6** releases shipping the same script, ziguard-rs reproduces the **distributed** profile
  bytes exactly.

**Does not prove (honest non-claims):**

- Does **not** claim to reproduce **older** releases' shipped profiles — those used an **earlier**
  `ziguard.awk` (this is the *script's* difference, recorded, not hidden). ziguard-rs ports the **2026b**
  script, not every historical version.
- Does **not** redefine tzdb source policy, claim civil-time truth, replace upstream authority, or claim
  all future `ziguard.awk` behaviour forever.
- Bounded to: these 48 bundles, this oracle awk (GNU Awk 5.4.0), this host. A release is admitted only
  with a `GOODSIG` and a recorded `sha256`.

## Reproduce

```sh
bash lab/releases/run.sh           # re-runs the whole campaign -> lab/releases/releases.tsv
```
