//! vasm: assembles one source file into an object module.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use vms_obj::obj;

const USAGE: &str = "usage: vasm [/OBJECT=file | -o file] [/INCLUDE=dir | -I dir]... SOURCE";

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(msg) => {
            eprintln!("%VASM-F-{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool, String> {
    let usage = || format!("USAGE, {USAGE}");
    let (mut source, mut output, mut include) = (None, None, Vec::new());
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let qualifier = |name: &str| {
            arg.get(..name.len())
                .filter(|q| q.eq_ignore_ascii_case(name))
                .map(|_| PathBuf::from(&arg[name.len()..]))
        };
        if let Some(path) = qualifier("/OBJECT=") {
            output = Some(path);
        } else if let Some(dir) = qualifier("/INCLUDE=") {
            include.push(dir);
        } else if arg == "-o" {
            output = Some(args.next().ok_or_else(usage)?.into());
        } else if arg == "-I" {
            include.push(args.next().ok_or_else(usage)?.into());
        } else if source.is_none() && !arg.starts_with('-') {
            source = Some(PathBuf::from(arg));
        } else {
            return Err(usage());
        }
    }
    let source = source.ok_or_else(usage)?;
    let text =
        fs::read_to_string(&source).map_err(|e| format!("OPENIN, {}: {e}", source.display()))?;
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let output = output.unwrap_or_else(|| source.with_extension("obj"));
    let opts = vasm::Options {
        name: stem.chars().take(31).collect(),
        date: date(),
        path: Some(source),
        include,
    };

    match vasm::assemble(&text, &opts) {
        Ok(records) => {
            fs::write(&output, obj::write(&records))
                .map_err(|e| format!("WRITEERR, {}: {e}", output.display()))?;
            Ok(true)
        }
        Err(diags) => {
            for d in diags {
                eprintln!("{}:{}:{}: error: {}", d.file, d.line, d.col, d.msg);
                let pad: String = d
                    .text
                    .chars()
                    .take(d.col.saturating_sub(1))
                    .map(|c| if c == '\t' { '\t' } else { ' ' })
                    .collect();
                eprintln!("    {}\n    {pad}^", d.text);
                for context in d.context {
                    eprintln!("  {context}");
                }
            }
            Ok(false)
        }
    }
}

/// Now, or `SOURCE_DATE_EPOCH` for reproducible output, as `dd-mmm-yyyy hh:mm`
/// in UTC.
fn date() -> [u8; 17] {
    let secs = env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64)
        });
    let (days, secs) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + (month <= 2) as i64;
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let text = format!(
        "{day:02}-{}-{year:04} {:02}:{:02}",
        MONTHS[month as usize - 1],
        secs / 3600,
        secs % 3600 / 60
    );
    text.as_bytes().try_into().unwrap_or(*b"17-NOV-1858 00:00")
}
