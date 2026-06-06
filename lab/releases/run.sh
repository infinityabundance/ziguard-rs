#!/usr/bin/env bash
# ZIGUARD-RS.1 all-IANA-releases oracle campaign.
#
# For every signed tzdb-*.tar.lz release bundle:
#   - download + OpenPGP-verify + sha256-pin
#   - if it ships ziguard.awk, resolve that release's $(TDATA) file set and run:
#       awkR   = the release's OWN ziguard.awk        (the "real ziguard")
#       awk26  = the pinned 2026b ziguard.awk          (the script ziguard-rs ports)
#       rs     = ziguard-rs                            (the Rust port)
#     for all three forms, and compare:
#       Dimension A (port fidelity)      : rs == awk26   -> must hold for ALL releases
#       Dimension B (release reproduction): rs == awkR    -> holds where the script is unchanged
#   - classify each release.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"   # repo root (this script is at lab/releases/run.sh)
LAB=$ROOT/lab/releases
WORK=$LAB/work
ZIGUARD26=$ROOT/lab/admit/tzdb-2026b/ziguard.awk      # sha256 e4600a23…
ZIGUARD26_SHA=$(sha256sum "$ZIGUARD26" | cut -c1-12)
RSBIN=$ROOT/target/release/ziguard-rs
TSV=$LAB/releases.tsv
LOG=$LAB/run.log
mkdir -p "$WORK"
: > "$LOG"
printf 'release\tbundle_sha256\tsig\tziguard_awk_sha\tscript_eq_2026b\ttdata_files\tA_port_fidelity\tB_reproduces_release\tB_diff_lines\tclassification\n' > "$TSV"

log(){ echo "$@" | tee -a "$LOG"; }

BUNDLES=$(curl -s https://data.iana.org/time-zones/releases/ | grep -oE 'tzdb-[0-9]{4}[a-z]?\.tar\.lz' | sort -u)
total=$(echo "$BUNDLES" | wc -l)
log "campaign start: $total bundles; 2026b ziguard.awk sha=$ZIGUARD26_SHA"

i=0
for b in $BUNDLES; do
  i=$((i+1))
  rel=${b#tzdb-}; rel=${rel%.tar.lz}
  d="$WORK/tzdb-$rel"
  log "[$i/$total] $rel"
  # fetch bundle + signature
  curl -sf -o "$WORK/$b" "https://data.iana.org/time-zones/releases/$b" || { log "  fetch FAIL"; continue; }
  curl -sf -o "$WORK/$b.asc" "https://data.iana.org/time-zones/releases/$b.asc" 2>/dev/null
  bsha=$(sha256sum "$WORK/$b" | cut -c1-16)
  sig="unverified"
  if [ -f "$WORK/$b.asc" ]; then
    if gpg --verify "$WORK/$b.asc" "$WORK/$b" >/dev/null 2>&1; then sig="GOODSIG"; else sig="BADSIG"; fi
  fi
  # extract
  rm -rf "$d"; mkdir -p "$d"
  bsdtar -xf "$WORK/$b" -C "$WORK" 2>/dev/null || { log "  extract FAIL"; continue; }
  # bundle extracts to tzdb-<rel>/
  if [ ! -d "$d" ]; then
    # some bundles may extract to a differently-named dir; find it
    d=$(find "$WORK" -maxdepth 1 -type d -name "tzdb-$rel*" | head -1)
  fi
  if [ ! -f "$d/ziguard.awk" ]; then
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$rel" "$bsha" "$sig" "-" "-" "0" "-" "-" "-" "no_ziguard_predates_tool" >> "$TSV"
    log "  no ziguard.awk (predates tool)"
    rm -f "$WORK/$b" "$WORK/$b.asc"; rm -rf "$d"
    continue
  fi
  zsha=$(sha256sum "$d/ziguard.awk" | cut -c1-12)
  script_eq=$([ "$zsha" = "$ZIGUARD26_SHA" ] && echo yes || echo no)
  # resolve TDATA from the release Makefile (fallback: present standard region files)
  TDATA=$(cd "$d" && make -s --eval 'zgshow:; @printf "%s" "$(TDATA)"' zgshow 2>/dev/null)
  if [ -z "$TDATA" ]; then
    TDATA=""
    for f in africa antarctica asia australasia europe northamerica southamerica etcetera factory pacificnew backward systemv; do
      [ -f "$d/$f" ] && TDATA="$TDATA $f"
    done
  fi
  nfiles=$(echo $TDATA | wc -w)
  # run the three transformers over the same inputs, per form
  A=yes; B=yes; bdiff=0
  for form in main vanguard rearguard; do
    ( cd "$d" && awk -v DATAFORM=$form -v PACKRATDATA='' -v PACKRATLIST='' -f ziguard.awk $TDATA ) > "$WORK/awkR.$form" 2>/dev/null
    ( cd "$d" && awk -v DATAFORM=$form -v PACKRATDATA='' -v PACKRATLIST='' -f "$ZIGUARD26" $TDATA ) > "$WORK/awk26.$form" 2>/dev/null
    ( cd "$d" && $RSBIN --format $form $TDATA ) > "$WORK/rs.$form" 2>/dev/null
    cmp -s "$WORK/rs.$form" "$WORK/awk26.$form" || A=no
    if ! cmp -s "$WORK/rs.$form" "$WORK/awkR.$form"; then
      B=no
      bdiff=$((bdiff + $(diff "$WORK/awkR.$form" "$WORK/rs.$form" 2>/dev/null | grep -c '^[<>]') ))
    fi
  done
  # classify
  if [ "$A" = no ]; then
    cls="PORT_BUG_investigate"
  elif [ "$B" = yes ]; then
    cls="reproduces_release_exactly"
  elif [ "$script_eq" = no ]; then
    cls="script_evolution_expected"
  else
    cls="UNEXPECTED_same_script_diff_output"
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$rel" "$bsha" "$sig" "$zsha" "$script_eq" "$nfiles" "$A" "$B" "$bdiff" "$cls" >> "$TSV"
  log "  zsha=$zsha script_eq=$script_eq files=$nfiles A=$A B=$B bdiff=$bdiff -> $cls"
  rm -f "$WORK/$b" "$WORK/$b.asc" "$WORK"/awkR.* "$WORK"/awk26.* "$WORK"/rs.*; rm -rf "$d"
done

log "=== campaign done ==="
log "Dimension A (port fidelity, rs==2026b-awk):"
awk -F'\t' 'NR>1 && $7!="-"{a[$7]++} END{for(k in a)print "  A="k": "a[k]}' "$TSV" | tee -a "$LOG"
log "Dimension B (reproduces release exactly):"
awk -F'\t' 'NR>1{c[$10]++} END{for(k in c)print "  "k": "c[k]}' "$TSV" | tee -a "$LOG"
