//! vsflog — decode a `verichrome.log.vsf` structured log into human-readable lines.
//!
//! The log is a stream of complete VSF records, each `{creation_time (Eagle), section "log"
//! {lvl, msg, val*}}` (see `vsf::logging`). This walks the records in order and prints
//! `<eagle-time>  [LEVEL]  <msg>`, rendering the stored template with its typed `val` fields substituted at read time (numbers live binary in the record).
//!
//! Pull the file off a phone with:
//!   `adb pull /storage/emulated/0/Android/data/<pkg>/files/verichrome.log.vsf`
//! (the external files dir — adb-readable on a non-debuggable release dev APK).
//!
//! Usage: `vsflog [PATH] [flags]` (PATH defaults to ./verichrome.log.vsf)
//!   -l, --level LEVEL   only records at this severity or higher (TRACE|DEBUG|INFO|WARN|ERROR, 0..4)
//!   -g, --grep SUBSTR   only records whose message contains SUBSTR (case-insensitive)
//!   -f, --follow        keep reading as new records are appended (tail -f); survives rotation

use std::io::Read;

fn level_name(lvl: u64) -> &'static str {
    match lvl {
        0 => "TRACE",
        1 => "DEBUG",
        2 => "INFO ",
        3 => "WARN ",
        4 => "ERROR",
        _ => "?????",
    }
}

fn level_from_arg(s: &str) -> Option<u64> {
    match s.to_ascii_uppercase().as_str() {
        "TRACE" => Some(0),
        "DEBUG" => Some(1),
        "INFO" => Some(2),
        "WARN" | "WARNING" => Some(3),
        "ERROR" => Some(4),
        _ => s.parse::<u64>().ok().filter(|n| *n <= 4),
    }
}

/// Eagle oscillations → a readable UTC string (display-only conversion).
fn eagle_display(osc: i64) -> String {
    vsf::types::EagleTime::from_oscillations(osc)
        .to_datetime()
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string()
}

struct Filter {
    min_level: u64,
    grep: Option<String>,
}

/// Decode and print whole records from `buf` (via the shared `vsf::parse_log_records`), applying the filter. Returns the offset of the last COMPLETE record boundary, so a half-written trailing record (mid-append) is left for the next pass instead of being mis-decoded.
fn print_records(buf: &[u8], filter: &Filter) -> usize {
    let (records, consumed) = vsf::parse_log_records(buf);
    for r in &records {
        let pass_level = r.level >= filter.min_level;
        let pass_grep = match &filter.grep {
            Some(g) => r.msg.to_lowercase().contains(g),
            None => true,
        };
        if pass_level && pass_grep {
            let ts = if r.osc != 0 { eagle_display(r.osc) } else { "(no time)".to_string() };
            println!("{ts}  [{}]  {}", level_name(r.level), r.msg);
        }
    }
    consumed
}

fn read_all(path: &str) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn main() {
    let mut path: Option<String> = None;
    let mut min_level = 0u64;
    let mut grep: Option<String> = None;
    let mut follow = false;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-f" | "--follow" => follow = true,
            "-l" | "--level" => match args.next().as_deref().and_then(level_from_arg) {
                Some(l) => min_level = l,
                None => {
                    eprintln!("vsflog: --level needs TRACE|DEBUG|INFO|WARN|ERROR or 0..4");
                    std::process::exit(2);
                }
            },
            "-g" | "--grep" => match args.next() {
                Some(g) => grep = Some(g.to_lowercase()),
                None => {
                    eprintln!("vsflog: --grep needs a substring");
                    std::process::exit(2);
                }
            },
            "-h" | "--help" => {
                eprintln!(
                    "vsflog [PATH] [-l LEVEL] [-g SUBSTR] [-f]\n  \
                     PATH defaults to ./verichrome.log.vsf"
                );
                return;
            }
            other if !other.starts_with('-') && path.is_none() => path = Some(other.to_string()),
            other => {
                eprintln!("vsflog: unrecognized argument '{other}'");
                std::process::exit(2);
            }
        }
    }

    let path = path.unwrap_or_else(|| "verichrome.log.vsf".to_string());
    let filter = Filter { min_level, grep };

    let bytes = match read_all(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("vsflog: cannot read {path}: {e}");
            std::process::exit(1);
        }
    };
    let mut consumed = print_records(&bytes, &filter);

    if follow {
        use std::io::{Seek, SeekFrom};
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let Ok(mut f) = std::fs::File::open(&path) else { continue };
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            // Rotation/clear shrank the file below where we were: re-sync from the start.
            if len < consumed as u64 {
                consumed = 0;
            }
            if len <= consumed as u64 {
                continue;
            }
            if f.seek(SeekFrom::Start(consumed as u64)).is_err() {
                continue;
            }
            let mut tail = Vec::new();
            if f.read_to_end(&mut tail).is_err() {
                continue;
            }
            consumed += print_records(&tail, &filter);
        }
    }
}
