//! `ziguard-rs` — CLI front end for the [`ziguard_rs`] transform.
//!
//! Usage mirrors the upstream AWK-style stream model:
//!
//! ```text
//! ziguard-rs --format main      africa antarctica … backward > main.zi
//! ziguard-rs --format vanguard  < concatenated-source            > vanguard.zi
//! ziguard-rs --format rearguard africa … backward                > rearguard.zi
//! ```
//!
//! With file arguments it reads them in order (like `awk -f ziguard.awk f1 f2…`);
//! with none, it reads stdin.
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};
use std::process::ExitCode;

use ziguard_rs::{transform, DataForm, Options};

const USAGE: &str = "\
ziguard-rs — Rust port of upstream ziguard.awk (tzdb source-profile transformer)

USAGE:
    ziguard-rs --format <main|vanguard|rearguard> [FILES...]

    With FILES, reads them in order (as awk does); with none, reads stdin.
    Writes the transformed profile to stdout.

OPTIONS:
    --format <FORM>        Target form: main | vanguard | rearguard (required)
    --vanguard-subseconds  Honour VANGUARD_SUBSECONDS (#STDOFF subsecond) in vanguard form
    -h, --help             Print this help
    -V, --version          Print version
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ziguard-rs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut form: Option<DataForm> = None;
    let mut vanguard_subseconds = false;
    let mut files: Vec<String> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("ziguard-rs {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--format" => {
                let v = args
                    .next()
                    .ok_or_else(|| "--format requires a value".to_string())?;
                form =
                    Some(DataForm::parse(&v).ok_or_else(|| {
                        format!("invalid --format '{v}' (main|vanguard|rearguard)")
                    })?);
            }
            s if s.starts_with("--format=") => {
                let v = &s["--format=".len()..];
                form =
                    Some(DataForm::parse(v).ok_or_else(|| {
                        format!("invalid --format '{v}' (main|vanguard|rearguard)")
                    })?);
            }
            "--vanguard-subseconds" => vanguard_subseconds = true,
            "--" => {
                files.extend(args.by_ref());
                break;
            }
            s if s.starts_with('-') && s != "-" => {
                return Err(format!("unknown option '{s}' (try --help)"));
            }
            _ => files.push(arg),
        }
    }

    let form = form.ok_or_else(|| "--format is required (try --help)".to_string())?;
    let opt = Options {
        form,
        vanguard_subseconds,
    };

    // Read inputs: each file in order (preserving filename), else stdin.
    let mut inputs: Vec<(String, String)> = Vec::new();
    if files.is_empty() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        inputs.push(("-".to_string(), buf));
    } else {
        for f in &files {
            let buf = std::fs::read_to_string(f).map_err(|e| format!("reading {f}: {e}"))?;
            inputs.push((f.clone(), buf));
        }
    }

    let out = transform(&inputs, &opt);
    io::stdout()
        .write_all(out.as_bytes())
        .map_err(|e| format!("writing stdout: {e}"))?;
    Ok(())
}
