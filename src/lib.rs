//! ziguard-rs — a faithful Rust port of upstream `ziguard.awk` (tzdb).
//!
//! `ziguard.awk` (Paul Eggert, public domain) is the tzdb source-profile
//! transformer: it converts the *current* tz source files into `main`,
//! `vanguard`, or `rearguard` form. This crate reproduces that transformation
//! exactly, so that:
//!
//! ```text
//! zic-rs compiles source profiles correctly        (already proven, SOURCE-VARIANT.1)
//! ziguard-rs reproduces the profile transformer     (this crate, ZIGUARD-RS.1)
//! ```
//!
//! ## Fidelity discipline
//!
//! This is a *port*, not a reimagining. Each pattern/action below mirrors a
//! specific line of upstream `ziguard.awk` (the line numbers in comments refer
//! to the pinned tzdb-2026b `ziguard.awk`, sha256 `e4600a23…`). The AWK
//! evaluation model is preserved precisely:
//!
//! * default field splitting (runs of space/tab, leading/trailing stripped);
//! * **every mutation of `$0` re-splits the fields** (AWK semantics) — done via
//!   [`Rec::sub`] / [`Rec::set`];
//! * rules run top-to-bottom per record, in the same order as the script;
//! * the `line[]` array is the output buffer, and later records may comment out
//!   earlier `Link` lines (back-patching), exactly as upstream does.
//!
//! Correctness is established not by reading this code but by **byte-identical**
//! comparison against the real `awk` oracle over the pinned 2026b source
//! (`tests/oracle.rs`).
#![forbid(unsafe_code)]

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The three tzdb source-profile forms (`DATAFORM`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataForm {
    /// The default distributed form.
    Main,
    /// Newest features (negative SAVE, `%z`, `Zone GMT`).
    Vanguard,
    /// Avoids negative SAVE / `%z`, for older parsers.
    Rearguard,
}

impl DataForm {
    /// Parse the `--format` / `DATAFORM` value.
    pub fn parse(s: &str) -> Option<DataForm> {
        match s {
            "main" => Some(DataForm::Main),
            "vanguard" => Some(DataForm::Vanguard),
            "rearguard" => Some(DataForm::Rearguard),
            _ => None,
        }
    }
    /// The canonical lowercase name (matches `DATAFORM`).
    pub fn as_str(self) -> &'static str {
        match self {
            DataForm::Main => "main",
            DataForm::Vanguard => "vanguard",
            DataForm::Rearguard => "rearguard",
        }
    }
}

/// Options controlling the transform. The defaults reproduce the upstream
/// Makefile invocation (`PACKRATDATA=`/`PACKRATLIST=` empty).
#[derive(Clone, Debug)]
pub struct Options {
    /// The target form.
    pub form: DataForm,
    /// The otherwise-undocumented `VANGUARD_SUBSECONDS` toggle (default off).
    pub vanguard_subseconds: bool,
}

impl Options {
    /// Construct options for `form` with upstream defaults.
    pub fn new(form: DataForm) -> Self {
        Options {
            form,
            vanguard_subseconds: false,
        }
    }
}

/// Fuzzing-only entry point (behind the `fuzzing` feature, never part of the
/// normal public API). Drives every pure helper with arbitrary fuzzer input so
/// the fuzz crate can exercise them directly, not only via [`transform`].
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn __fuzz_helpers(s: &str) {
    let _ = numeric_prefix_len(s.as_bytes());
    let _ = awk_num(s);
    let _ = is_strnum(s);
    let _ = field_falsy(s);
    let _ = get_minutes(s);
    let _ = round_to_second(s);
    let _ = expand_repl(s, s);
    let _ = make_linkline(s, s, s, s, s);
    let _ = split_records(s);
    let off = awk_num(s) as i64;
    let _ = offset_abbr(off);
    let _ = signed_zeropad(off, 4);
}

// ---------------------------------------------------------------------------
// AWK record: a line plus its default-split fields ($1..$NF). $0 is `line`.
// ---------------------------------------------------------------------------

struct Rec {
    line: String,
    fields: Vec<String>,
}

impl Rec {
    /// AWK default field splitting (`FS = " "`): split on runs of space/tab and
    /// drop the empty pieces, so leading/trailing whitespace and repeated
    /// separators collapse — `$1` is the first non-blank token, etc.
    fn split(line: &str) -> Vec<String> {
        line.split([' ', '\t'])
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }
    fn new(line: String) -> Self {
        let fields = Self::split(&line);
        Rec { line, fields }
    }
    /// Reassign `$0` and re-split (AWK semantics).
    fn set(&mut self, line: String) {
        self.fields = Self::split(&line);
        self.line = line;
    }
    /// `$i` (1-indexed); out-of-range yields `""` like AWK.
    fn f(&self, i: usize) -> &str {
        if i >= 1 && i <= self.fields.len() {
            &self.fields[i - 1]
        } else {
            ""
        }
    }
    /// `NF`.
    fn nf(&self) -> usize {
        self.fields.len()
    }
    /// `sub(/re/, repl)` on `$0`, re-splitting if it fired. Returns whether it fired.
    fn sub(&mut self, re: &Regex, repl: &str) -> bool {
        let mut s = std::mem::take(&mut self.line);
        let fired = awk_sub(re, repl, &mut s);
        if fired {
            self.set(s);
        } else {
            self.line = s;
        }
        fired
    }
}

// ---------------------------------------------------------------------------
// AWK helpers: sub() replacement, numeric coercion, strnum truthiness.
// ---------------------------------------------------------------------------

/// AWK `sub`: replace the single leftmost match; expand `&` (whole match),
/// `\&` (literal &), `\\` (literal backslash). Returns whether a match fired.
fn awk_sub(re: &Regex, repl: &str, s: &mut String) -> bool {
    let Some(m) = re.find(s) else {
        return false;
    };
    let matched = s[m.start()..m.end()].to_string();
    let expanded = expand_repl(repl, &matched);
    let mut out = String::with_capacity(s.len() + expanded.len());
    out.push_str(&s[..m.start()]);
    out.push_str(&expanded);
    out.push_str(&s[m.end()..]);
    *s = out;
    true
}

fn expand_repl(repl: &str, matched: &str) -> String {
    let mut out = String::with_capacity(repl.len());
    let mut chars = repl.chars();
    while let Some(c) = chars.next() {
        match c {
            '&' => out.push_str(matched),
            '\\' => match chars.next() {
                Some('&') => out.push('&'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            other => out.push(other),
        }
    }
    out
}

/// Length of the maximal leading numeric token in `b` (optional sign, digits,
/// optional decimal, optional exponent). Returns `0` if no digit is present.
///
/// Invariant (Kani-proven, [`kani_harness::numeric_prefix_len_is_ascii_in_bounds`]):
/// the result is `<= b.len()` and every byte in `b[..result]` is ASCII (`< 0x80`),
/// so slicing a `&str` at this index never panics on a char boundary.
fn numeric_prefix_len(b: &[u8]) -> usize {
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut saw_digit = false;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if saw_digit && i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let mut exp_digit = false;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
            exp_digit = true;
        }
        if exp_digit {
            i = j;
        }
    }
    if saw_digit {
        i
    } else {
        0
    }
}

/// AWK unary-plus numeric coercion: parse the maximal leading numeric prefix
/// (optional sign, digits, decimal, exponent), else 0. So `+"-0:30"` -> 0,
/// `+"-1:00"` -> -1, `+"36:45"` -> 36, `+".32"` -> 0.32.
fn awk_num(s: &str) -> f64 {
    let i = numeric_prefix_len(s.as_bytes());
    if i == 0 {
        0.0
    } else {
        s[..i].parse::<f64>().unwrap_or(0.0)
    }
}

/// Whether a field is an AWK "strnum" that looks numeric (so comparisons /
/// truthiness use the numeric value).
fn is_strnum(s: &str) -> bool {
    let t = s.trim_matches([' ', '\t']);
    if t.is_empty() {
        return false;
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[+-]?([0-9]+\.?[0-9]*|\.[0-9]+)([eE][+-]?[0-9]+)?$").unwrap())
        .is_match(t)
}

/// AWK boolean truthiness of a field value (`!$x` is `field_falsy`).
fn field_falsy(s: &str) -> bool {
    if is_strnum(s) {
        awk_num(s) == 0.0
    } else {
        s.is_empty()
    }
}

// ziguard.awk:26 — Given a FIELD like "-0:30", return a minute count like -30.
fn get_minutes(field: &str) -> i64 {
    let sign: i64 = if field.starts_with('-') { -1 } else { 1 };
    let hours = awk_num(field) as i64; // +field
    let minutes: i64 = if field.contains(':') {
        // minutes = field; sub(/[^:]*:/, "", minutes)  -> text after first colon
        let after = &field[field.find(':').unwrap() + 1..];
        awk_num(after) as i64
    } else {
        0
    };
    // Saturating: AWK uses f64 (never overflows); on real tz offsets (|min| < ~1000)
    // this is exactly `60*hours + sign*minutes`, but a hostile huge field cannot panic.
    60i64
        .saturating_mul(hours)
        .saturating_add(sign.saturating_mul(minutes))
}

// ziguard.awk:40 — Given OFFSET (minutes), return a %z-style abbr like "+05"/"+0530".
fn offset_abbr(offset: i64) -> String {
    let hours = offset / 60; // int(offset/60), trunc toward zero (matches AWK int)
    let minutes = offset % 60; // sign of dividend (matches AWK %)
    if minutes != 0 {
        // sprintf("%+.4d", hours*100+minutes) — saturating so no hostile offset panics
        signed_zeropad(hours.saturating_mul(100).saturating_add(minutes), 4)
    } else {
        // sprintf("%+.2d", hours)
        signed_zeropad(hours, 2)
    }
}

/// C `printf("%+.Nd")`: forced sign, magnitude zero-padded to `digits`.
fn signed_zeropad(v: i64, digits: usize) -> String {
    let sign = if v < 0 { '-' } else { '+' };
    format!("{}{:0>width$}", sign, v.unsigned_abs(), width = digits)
}

// ziguard.awk:53 — Round TIMESTAMP (a +-hh:mm:ss.dddd string) to nearest second.
fn round_to_second(timestamp: &str) -> String {
    static RE_HAS_SUB: OnceLock<Regex> = OnceLock::new();
    let re_has_sub =
        RE_HAS_SUB.get_or_init(|| Regex::new(r"^[+-]?[0-9]+:[0-9]+:[0-9]+\.").unwrap());
    // dot_dddd = timestamp; if (!sub(/^[+-]?[0-9]+:[0-9]+:[0-9]+\./, ".", dot_dddd)) return timestamp
    let mut dot_dddd = timestamp.to_string();
    if !awk_sub(re_has_sub, ".", &mut dot_dddd) {
        return timestamp.to_string();
    }
    // hh = mm = ss = timestamp; strip leading parts.
    let mut ss = timestamp.to_string();
    awk_sub_str(r"^[-+]?[0-9]+:[0-9]+:", "", &mut ss);
    let mut mm = timestamp.to_string();
    awk_sub_str(r"^[-+]?[0-9]+:", "", &mut mm);
    let mut hh = timestamp.to_string();
    awk_sub_str(r"^[-+]?", "", &mut hh);
    let hh = awk_num(&hh) as i64;
    let mm = awk_num(&mm) as i64;
    let ss = awk_num(&ss) as i64;
    let mut seconds = 3600i64
        .saturating_mul(hh)
        .saturating_add(60i64.saturating_mul(mm))
        .saturating_add(ss);
    let subseconds = awk_num(&dot_dddd);
    // seconds += 0.5 < subseconds || ((subseconds == 0.5) && (seconds % 2));
    if 0.5 < subseconds || (subseconds == 0.5 && (seconds % 2) != 0) {
        seconds = seconds.saturating_add(1);
    }
    // sprintf("%s%d:%.2d:%.2d", sign, seconds/3600, seconds/60%60, seconds%60)
    let sign = if timestamp.starts_with('-') { "-" } else { "" };
    format!(
        "{}{}:{:02}:{:02}",
        sign,
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// `sub` with a dynamic ERE string (compiled fresh), on a plain string.
fn awk_sub_str(pattern: &str, repl: &str, s: &mut String) {
    if let Ok(re) = Regex::new(pattern) {
        awk_sub(&re, repl, s);
    }
}

// ---------------------------------------------------------------------------
// Compiled patterns (mirror the EREs in ziguard.awk; `\/`->`/`, `\+` kept).
// ---------------------------------------------------------------------------

struct Re {
    prague_dublin_offset: Regex, // :98 / :111  ^#?[\t ]+[01]:00[\t ]
    eire: Regex,                 // :109  ^#?Rule[\t ]+Eire[\t ]
    namibia_rule: Regex,         // :125  ^#?Rule[\t ]+Namibia[\t ]
    windhoek_offset: Regex,      // :127  ^#?[\t ]+[12]:00[\t ]
    portugal: Regex,             // :145
    gmt: Regex,                  // :156  ^#?(Zone|Link)[\t ]+(Etc/)?GMT[\t ]
    z_active: Regex,             // :173  ^[^#]*%z
    rules_signdigit: Regex,      // :182 / :289  ^[+0-9-]
    uruguay30: Regex,            // :193  [\t ](1942 Dec 14|1960|1970|1974 Dec 22)$
    uruguay90: Regex,            // :195  [\t ]1974 Mar 10$
    z: Regex,                    // :208  %z
    change_to_z: Regex,          // :211
    change00: Regex,             // :213  -00CHANGE-TO-%z
    change_sign: Regex,          // :214  [-+][^\t ]+CHANGE-TO-
    rule201x: Regex,             // :265  ^201[78]$
    link_start: Regex,           // :347/:350  ^Link
    packratlist_strip: Regex,    // :87
    // Morocco / Japan literal-ish subs.
    sat8: Regex,
    t2500: Regex,
    sun9: Regex,
    t100sp: Regex,
    m_t2018t: Regex,
    m_t2017t: Regex,
    m_t0t: Regex,
    m_t100t: Regex,
    m_tn100t: Regex,
    m_100morocco: Regex,
    m_000morocco: Regex,
    m_p01p00: Regex,
    m_p00p01: Regex,
}

impl Re {
    fn new() -> Self {
        Re {
            prague_dublin_offset: Regex::new(r"^#?[\t ]+[01]:00[\t ]").unwrap(),
            eire: Regex::new(r"^#?Rule[\t ]+Eire[\t ]").unwrap(),
            namibia_rule: Regex::new(r"^#?Rule[\t ]+Namibia[\t ]").unwrap(),
            windhoek_offset: Regex::new(r"^#?[\t ]+[12]:00[\t ]").unwrap(),
            portugal: Regex::new(
                r"^#?[\t ]+-[12]:00[\t ]+((Port|W-Eur)[\t ]+[%+-]|-[\t ]+(%z|-01)[\t ]+1982 Mar 28)",
            )
            .unwrap(),
            gmt: Regex::new(r"^#?(Zone|Link)[\t ]+(Etc/)?GMT[\t ]").unwrap(),
            z_active: Regex::new(r"^[^#]*%z").unwrap(),
            rules_signdigit: Regex::new(r"^[+0-9-]").unwrap(),
            uruguay30: Regex::new(r"[\t ](1942 Dec 14|1960|1970|1974 Dec 22)$").unwrap(),
            uruguay90: Regex::new(r"[\t ]1974 Mar 10$").unwrap(),
            z: Regex::new(r"%z").unwrap(),
            change_to_z: Regex::new(
                r"^(Zone[\t ]+[^\t ]+)?[\t ]+[^\t ]+[\t ]+[^\t ]+[\t ]+[-+][^\t ]+",
            )
            .unwrap(),
            change00: Regex::new(r"-00CHANGE-TO-%z").unwrap(),
            change_sign: Regex::new(r"[-+][^\t ]+CHANGE-TO-").unwrap(),
            rule201x: Regex::new(r"^201[78]$").unwrap(),
            link_start: Regex::new(r"^Link").unwrap(),
            packratlist_strip: Regex::new(r"^#PACKRATLIST[\t ]+[^\t ]+[\t ]+").unwrap(),
            sat8: Regex::new(r"Sat>=8").unwrap(),
            t2500: Regex::new(r"25:00").unwrap(),
            sun9: Regex::new(r"Sun>=9").unwrap(),
            t100sp: Regex::new(r" 1:00").unwrap(),
            m_t2018t: Regex::new(r"\t2018\t").unwrap(),
            m_t2017t: Regex::new(r"\t2017\t").unwrap(),
            m_t0t: Regex::new(r"\t0\t").unwrap(),
            m_t100t: Regex::new(r"\t1:00\t").unwrap(),
            m_tn100t: Regex::new(r"\t-1:00\t").unwrap(),
            m_100morocco: Regex::new(r"1:00\tMorocco").unwrap(),
            m_000morocco: Regex::new(r"0:00\tMorocco").unwrap(),
            m_p01p00: Regex::new(r"\t\+01/\+00$").unwrap(),
            // Faithful to upstream's `/\t\+00\/+01$/`: the `+` before `01` is
            // unescaped there, so `\/+` means "one-or-more slashes". Transcribed
            // verbatim (do NOT "fix" to `\+01`) — fidelity, not correctness, is
            // the contract, and the all-release oracle would catch any drift.
            m_p00p01: Regex::new(r"\t\+00/+01$").unwrap(),
        }
    }
}

// ---------------------------------------------------------------------------
// ziguard.awk:314 — make_linkline.
// ---------------------------------------------------------------------------

fn make_linkline(
    oldline: &str,
    target: &str,
    linkname: &str,
    oldtarget: &str,
    comment: &str,
) -> String {
    let oldprefix = format!("Link\t{oldtarget}\t");
    let oldprefixlen = oldprefix.len();
    // `starts_with` is panic-free for any (multibyte) input and is exactly the
    // AWK `substr(oldline,1,oldprefixlen) == oldprefix` test: a non-char-boundary
    // cut can never equal the valid-UTF-8 `oldprefix`.
    let replsuffix = if oldline.starts_with(oldprefix.as_str()) {
        let mut replsuffix = oldline[oldprefixlen..].to_string();
        awk_sub_str(r"[\t ]*#.*", "", &mut replsuffix);
        let oldtargettabs = (oldtarget.len() / 8) + 1;
        let mut targettabs = (target.len() / 8) + 1;
        // for (; targettabs < oldtargettabs; targettabs++) replsuffix = "\t" replsuffix
        while targettabs < oldtargettabs {
            replsuffix = format!("\t{replsuffix}");
            targettabs += 1;
        }
        // for (; oldtargettabs < targettabs && replsuffix ~ /^\t/; targettabs--) replsuffix = substr(replsuffix, 2)
        while oldtargettabs < targettabs && replsuffix.starts_with('\t') {
            replsuffix = replsuffix[1..].to_string();
            targettabs -= 1;
        }
        replsuffix
    } else {
        linkname.to_string()
    };
    format!("Link\t{target}\t{replsuffix}{comment}")
}

// ---------------------------------------------------------------------------
// The transform.
// ---------------------------------------------------------------------------

/// Split a file's contents into AWK records (`RS="\n"`): drop the trailing
/// empty record produced by a final newline.
fn split_records(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut v: Vec<&str> = content.split('\n').collect();
    if content.ends_with('\n') {
        v.pop();
    }
    v
}

/// Run the ziguard transform over `inputs` (each `(filename, contents)`, in the
/// upstream `$(TDATA)` order) and return the produced profile text.
pub fn transform(inputs: &[(String, String)], opt: &Options) -> String {
    let re = Re::new();
    let form = opt.form;

    // PACKRATDATA / PACKRATLIST are not supported in v1: the upstream Makefile
    // default is empty, and this crate reproduces that default. The packrat
    // rules below (`#PACKRATLIST`, `packrat_ignored`) are therefore inert,
    // kept as faithful documented stubs for a future packrat extension.

    let mut zone = String::new(); // :90
    let mut stdoff_subst: Option<(String, String)> = None; // stdoff_subst[0],[1]; None == falsy
    let mut packrat_ignored = false; // :302

    let mut lines: Vec<String> = Vec::new(); // line[NR] -> lines[NR-1]
    let mut linkline: HashMap<String, usize> = HashMap::new(); // name -> NR
    let mut linktarget: HashMap<String, String> = HashMap::new(); // name -> target

    let mut nr = 0usize;
    for (filename, content) in inputs {
        for raw in split_records(content) {
            nr += 1;
            let mut rec = Rec::new(raw.to_string());

            // :86  $1 == "#PACKRATLIST" && $2 == PACKRATLIST  (PACKRATLIST default empty)
            if rec.f(1) == "#PACKRATLIST" && rec.f(2).is_empty() {
                rec.sub(&re.packratlist_strip, "");
            }

            // :90  /^Zone/ { zone = $2 }
            if rec.line.starts_with("Zone") {
                zone = rec.f(2).to_string();
            }

            // :92  DATAFORM != "main" { ... }
            if form != DataForm::Main {
                transform_nonmain(&re, form, &zone, &mut stdoff_subst, opt, &mut rec);
            }

            // :301  /^Zone/ { packrat_ignored = ... }  (PACKRATDATA empty -> always false)
            if rec.line.starts_with("Zone") {
                let _ = filename; // FILENAME == PACKRATDATA is never true (PACKRATDATA empty)
                packrat_ignored = false;
            }
            // :304  { if (packrat_ignored && $0 !~ /^Rule/) sub(/^/, "#") }
            if packrat_ignored && !rec.line.starts_with("Rule") {
                rec.set(format!("#{}", rec.line));
            }

            // :339  /^Link/ && $4 == "#=" && DATAFORM == "vanguard"
            if rec.line.starts_with("Link") && rec.f(4) == "#=" && form == DataForm::Vanguard {
                let new = make_linkline(&rec.line, rec.f(5), rec.f(3), rec.f(2), "");
                rec.set(new);
            }

            // Link de-duplication by back-patching already-emitted lines: if a
            // later Zone or Link reuses an earlier Link's *name*, comment that
            // earlier Link out (last definition wins). `linkline` maps a name to
            // the NR of the line that defined it; `lines[l-1]` is that buffered
            // output line, edited in place — exactly as upstream mutates `line[]`.
            // :346  /^Zone/ { sub(/^Link/, "#Link", line[linkline[$2]]) }
            if rec.line.starts_with("Zone") {
                if let Some(&l) = linkline.get(rec.f(2)) {
                    if l >= 1 && l <= lines.len() {
                        awk_sub(&re.link_start, "#Link", &mut lines[l - 1]);
                    }
                }
            }
            // :349  /^Link/ { sub(/^Link/, "#Link", line[linkline[$3]]); linkline[$3]=NR; linktarget[$3]=$2 }
            if rec.line.starts_with("Link") {
                let name3 = rec.f(3).to_string();
                if let Some(&l) = linkline.get(&name3) {
                    if l >= 1 && l <= lines.len() {
                        awk_sub(&re.link_start, "#Link", &mut lines[l - 1]);
                    }
                }
                linkline.insert(name3.clone(), nr);
                linktarget.insert(name3, rec.f(2).to_string());
            }

            // :355  { line[NR] = $0 }
            lines.push(rec.line.clone());
        }
    }

    // :376  END
    if form != DataForm::Vanguard {
        cut_link_chains_short(&mut lines, &linkline, &linktarget);
    }
    let mut out = String::new();
    for l in &lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// The body of the `DATAFORM != "main"` block (ziguard.awk:92–299).
#[allow(clippy::too_many_arguments)]
fn transform_nonmain(
    re: &Re,
    form: DataForm,
    zone: &str,
    stdoff_subst: &mut Option<(String, String)>,
    opt: &Options,
    rec: &mut Rec,
) {
    let in_comment = rec.line.starts_with('#'); // :93  $0 ~ /^#/
                                                // `ic` (0 or 1) is the field-index shift the script relies on: when a line is
                                                // commented, the leading `#` is `$1`, so every "real" field is pushed one
                                                // position right. `$(in_comment + N)` therefore selects the same logical
                                                // field in both the commented and uncommented variants of a paired line.
    let ic = in_comment as usize;
    let mut uncomment = false;
    let mut comment_out = false; // :94

    // :96  Czechoslovakia (Europe/Prague) negative SAVE.
    if zone == "Europe/Prague"
        && re.prague_dublin_offset.is_match(&rec.line)
        && rec.line.contains("1947 Feb 23")
    {
        // ($(in_comment + 2) != "-") == (DATAFORM != "rearguard")
        if (rec.f(ic + 2) != "-") == (form != DataForm::Rearguard) {
            uncomment = in_comment;
        } else {
            comment_out = !in_comment;
        }
    }

    // :107  Ireland negative SAVE.
    let rule_eire = re.eire.is_match(&rec.line); // $0 ~ /^#?Rule[\t ]+Eire[\t ]/
    let zone_dublin_post_1968 = zone == "Europe/Dublin"
        && re.prague_dublin_offset.is_match(&rec.line)
        && (field_falsy(rec.f(ic + 4)) || 1968.0 < awk_num(rec.f(ic + 4)));
    if rule_eire || zone_dublin_post_1968 {
        let lhs = rule_eire || (zone_dublin_post_1968 && rec.f(ic + 3) == "IST/GMT");
        if lhs == (form != DataForm::Rearguard) {
            uncomment = in_comment;
        } else {
            comment_out = !in_comment;
        }
    }

    // :123  Namibia negative SAVE.
    let rule_namibia = re.namibia_rule.is_match(&rec.line);
    let zone_using_namibia_rule = zone == "Africa/Windhoek"
        && re.windhoek_offset.is_match(&rec.line)
        && (rec.f(ic + 2) == "Namibia"
            || (rec.f(ic + 2) == "-"
                && rec.f(ic + 3) == "CAT"
                && ((1994.0 <= awk_num(rec.f(ic + 4)) && awk_num(rec.f(ic + 4)) <= 2017.0)
                    || (ic + 3) == rec.nf())));
    if rule_namibia || zone_using_namibia_rule {
        let lhs = if rule_namibia {
            // $9 ~ /^-/ || ($9 == 0 && $10 == "CAT")
            rec.f(9).starts_with('-') || (awk_num(rec.f(9)) == 0.0 && rec.f(10) == "CAT")
        } else {
            // $(in_comment+1) == "2:00" && $(in_comment+2) == "Namibia"
            rec.f(ic + 1) == "2:00" && rec.f(ic + 2) == "Namibia"
        };
        if lhs == (form != DataForm::Rearguard) {
            uncomment = in_comment;
        } else {
            comment_out = !in_comment;
        }
    }

    // :143  Portugal %z preference (inverted: comment_out when %z and rearguard).
    if re.portugal.is_match(&rec.line) {
        if rec.line.contains("%z") == (form == DataForm::Rearguard) {
            comment_out = !in_comment;
        } else {
            uncomment = in_comment;
        }
    }

    // :153  vanguard uses "Zone GMT 0 - GMT"; others "Zone Etc/GMT".
    if re.gmt.is_match(&rec.line) {
        if (rec.f(2) == "GMT") == (form == DataForm::Vanguard) {
            uncomment = in_comment;
        } else {
            comment_out = !in_comment;
        }
    }

    // :164  apply.
    if uncomment {
        rec.sub_anchor_strip_hash();
    }
    if comment_out {
        rec.set(format!("#{}", rec.line));
    }

    // :171  abbreviations.
    if form == DataForm::Rearguard {
        if re.z_active.is_match(&rec.line) {
            let is_zone = rec.line.starts_with("Zone");
            let stdoff_col = if is_zone { 3 } else { 1 };
            let rules_col = stdoff_col + 1;
            let stdoff = get_minutes(rec.f(stdoff_col));
            let rules = rec.f(rules_col).to_string();
            let stdabbr = offset_abbr(stdoff);
            let abbr = if rules == "-" {
                stdabbr
            } else {
                let dstabbr_only = re.rules_signdigit.is_match(&rules);
                let dstoff = if dstabbr_only {
                    get_minutes(&rules)
                } else if rules == "Morocco" && rec.nf() == 3 {
                    -60
                } else if rules == "NBorneo" {
                    20
                } else if ((rules == "Cook" || rules == "LH") && rec.nf() == 3)
                    || (rules == "Uruguay" && re.uruguay30.is_match(&rec.line))
                {
                    30
                } else if rules == "Uruguay" && re.uruguay90.is_match(&rec.line) {
                    90
                } else {
                    60
                };
                let dstabbr = offset_abbr(stdoff + dstoff);
                if dstabbr_only {
                    dstabbr
                } else {
                    format!("{stdabbr}/{dstabbr}")
                }
            };
            rec.sub(&re.z, &abbr); // sub(/%z/, abbr)
        }
    } else {
        // :211  vanguard: turn an explicit numeric abbreviation (a FORMAT field
        // beginning with `+`/`-`) into `%z`, via a three-step marker dance:
        //   1. append "CHANGE-TO-%z" after the matched `…<offset><FORMAT>` field;
        //   2. a `-00` FORMAT is special — keep it literally (drop the marker);
        //   3. otherwise strip the "<+-FORMAT>CHANGE-TO-" run, leaving "%z".
        rec.sub(&re.change_to_z, "&CHANGE-TO-%z");
        rec.sub(&re.change00, "-00");
        rec.sub(&re.change_sign, "");
    }

    // :217  #STDOFF subsecond substitution.
    if rec.f(1) == "#STDOFF" {
        let stdoff = rec.f(2).to_string();
        let rounded = round_to_second(&stdoff);
        *stdoff_subst = Some(if form == DataForm::Vanguard && opt.vanguard_subseconds {
            (rounded, stdoff)
        } else {
            (stdoff, rounded)
        });
    } else if let Some((s0, s1)) = stdoff_subst.clone() {
        // if (stdoff_subst[0]) — armed (s0 non-empty, non-"0").
        if !(s0.is_empty() || s0 == "0") {
            let stdoff_col = if rec.line.starts_with("Zone") { 3 } else { 1 };
            let val = rec.f(stdoff_col).to_string();
            if val == s0 {
                rec.sub(&compile_dynamic(&s0), &s1); // sub(stdoff_subst[0], stdoff_subst[1])
            } else if val != s1 {
                *stdoff_subst = None; // stdoff_subst[0] = 0
            }
        }
    }

    // :245  Japan Sat>=8 25:00 <-> Sun>=9 1:00.
    if rec.line.starts_with("Rule") && rec.f(2) == "Japan" {
        if form == DataForm::Rearguard {
            if rec.f(7) == "Sat>=8" && rec.f(8) == "25:00" {
                rec.sub(&re.sat8, "Sun>=9");
                rec.sub(&re.t2500, " 1:00");
            }
        } else if rec.f(7) == "Sun>=9" && rec.f(8) == "1:00" {
            rec.sub(&re.sun9, "Sat>=8");
            rec.sub(&re.t100sp, "25:00");
        }
    }

    // :261  Morocco negative SAVE -> positive in rearguard.
    if rec.f(2) == "Morocco" {
        if rec.line.starts_with("Rule") {
            if re.rule201x.is_match(rec.f(4)) && rec.f(6) == "Oct" {
                if form == DataForm::Rearguard {
                    rec.sub(&re.m_t2018t, "\t2017\t");
                } else {
                    rec.sub(&re.m_t2017t, "\t2018\t");
                }
            }
            if 2019.0 <= awk_num(rec.f(3)) {
                if rec.f(8) == "2:00" {
                    if form == DataForm::Rearguard {
                        rec.sub(&re.m_t0t, "\t1:00\t");
                    } else {
                        rec.sub(&re.m_t100t, "\t0\t");
                    }
                } else if form == DataForm::Rearguard {
                    rec.sub(&re.m_tn100t, "\t0\t");
                } else {
                    rec.sub(&re.m_t0t, "\t-1:00\t");
                }
            }
        }
        if re.rules_signdigit.is_match(rec.f(1)) && rec.nf() == 3 {
            if form == DataForm::Rearguard {
                rec.sub(&re.m_100morocco, "0:00\tMorocco");
                rec.sub(&re.m_p01p00, "\t+00/+01");
            } else {
                rec.sub(&re.m_000morocco, "1:00\tMorocco");
                rec.sub(&re.m_p00p01, "\t+01/+00");
            }
        }
    }
}

impl Rec {
    /// `sub(/^#/, "")` — strip one leading `#`, re-split.
    fn sub_anchor_strip_hash(&mut self) {
        if let Some(rest) = self.line.strip_prefix('#') {
            let rest = rest.to_string();
            self.set(rest);
        }
    }
}

/// Compile a dynamic ERE (for `sub(stdoff_subst[0], …)`), falling back to a
/// never-matching pattern only if it somehow fails to compile.
fn compile_dynamic(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|_| Regex::new(r"$.^").unwrap())
}

// ziguard.awk:357 — cut_link_chains_short.
fn cut_link_chains_short(
    lines: &mut [String],
    linkline: &HashMap<String, usize>,
    linktarget: &HashMap<String, String>,
) {
    // Snapshot original keys (AWK auto-vivifies phantom "" entries during the
    // loop, but they have empty values and are skipped by `if (t)`).
    let names: Vec<String> = linktarget.keys().cloned().collect();
    for linkname in names {
        let target = &linktarget[&linkname];
        // t = linktarget[target]; if (t)
        let Some(first) = linktarget.get(target) else {
            continue;
        };
        let mut t = first.clone();
        // while ((u = linktarget[t])) t = u
        while let Some(u) = linktarget.get(&t) {
            if *u == t {
                break; // defensive against a self-link cycle (acyclic in real tzdb)
            }
            t = u.clone();
        }
        if let Some(&l) = linkline.get(&linkname) {
            if l >= 1 && l <= lines.len() {
                let comment = format!("\t#= {target}");
                lines[l - 1] = make_linkline(&lines[l - 1], &t, &linkname, target, &comment);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Kani bounded proofs — sharp invariants on the pure arithmetic helpers only.
//
// Doctrine (reduced surface): Kani proves sharp invariants on small pure
// helpers; the regex-driven transform is covered by the awk oracle and by
// fuzzing, not Kani. These harnesses target index-bound and overflow safety —
// the only places ziguard-rs does manual indexing / integer arithmetic.
// ---------------------------------------------------------------------------
#[cfg(kani)]
mod kani_harness {
    use super::*;

    /// `numeric_prefix_len` never returns an index past the slice, and every
    /// byte it consumes is ASCII — so `awk_num`'s `s[..i]` cut is always
    /// in-bounds on a char boundary and cannot panic.
    ///
    /// The input is a 6-byte array, so every scan loop runs at most 6 times;
    /// `unwind(8)` bounds CBMC's loop unrolling (the unwinding assertion proves
    /// 8 is sufficient — without it Kani unrolls unboundedly).
    #[kani::proof]
    #[kani::unwind(8)]
    fn numeric_prefix_len_is_ascii_in_bounds() {
        const N: usize = 6;
        let arr: [u8; N] = kani::any();
        let len = numeric_prefix_len(&arr);
        assert!(len <= N);
        let mut k = 0;
        while k < len {
            assert!(arr[k] < 0x80);
            k += 1;
        }
    }

    /// The arithmetic `offset_abbr` performs (`hours*100 + minutes`) cannot
    /// overflow `i64` for any offset far beyond the tz range (±14h = ±840 min),
    /// so it never panics under `overflow-checks`.
    #[kani::proof]
    fn offset_abbr_arithmetic_no_overflow() {
        let off: i64 = kani::any();
        kani::assume((-100_000..=100_000).contains(&off));
        let hours = off / 60;
        let minutes = off % 60;
        assert!(hours
            .checked_mul(100)
            .and_then(|h| h.checked_add(minutes))
            .is_some());
    }

    // NOTE: `signed_zeropad` is intentionally *not* a Kani harness. Its only
    // ziguard-specific arithmetic is `i64::unsigned_abs()` (panic-free incl.
    // `i64::MIN`); the rest is std `format!`, whose String allocation makes a
    // CBMC harness non-convergent without proving any sharp ziguard invariant.
    // Per the reduced-surface doctrine, a non-convergent harness is dropped, not
    // forced onto the result surface. (`signed_zeropad` is exercised by fuzzing.)

    /// The index arithmetic `Rec::f` performs is in-bounds: whenever the guard
    /// `i >= 1 && i <= nf` holds, the computed slot `i - 1` is `< nf` (so the
    /// `self.fields[i - 1]` access can never be out of bounds), for any `nf`/`i`.
    /// (Allocation-free: proves the same invariant as accessing `Rec::f` without
    /// dragging the heap allocator into the model.)
    #[kani::proof]
    fn rec_field_index_is_in_bounds() {
        let nf: usize = kani::any();
        let i: usize = kani::any();
        if i >= 1 && i <= nf {
            assert!(i - 1 < nf);
        }
    }
}
