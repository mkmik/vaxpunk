//! The command line of vasm, and of the compilers built on it: one source
//! file into an object module.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use vms_obj::obj;

use crate::Dialect;

/// Runs the command `name` (`vasm`, `vmacro`), which translates with
/// `dialect` if given.
pub fn main(name: &str, dialect: Option<&mut dyn Dialect>) -> ExitCode {
    match run(name, dialect) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(msg) => {
            eprintln!("%{}-F-{msg}", name.to_ascii_uppercase());
            ExitCode::FAILURE
        }
    }
}

fn run(name: &str, mut dialect: Option<&mut dyn Dialect>) -> Result<bool, String> {
    let usage = || {
        format!(
            "USAGE, usage: {name} [/OBJECT=file | -o file] [/INCLUDE=dir | -I dir]... \
             [/NOWARNINGS=NOTPIC | --nowarnings NOTPIC] [/ENABLE=what | --enable what] SOURCE"
        )
    };
    // What the dialect may enable, as vmacro's QUADWORD.
    let mut enable = |what: &str| {
        dialect
            .as_deref_mut()
            .is_some_and(|d| d.enable(what))
            .then_some(())
            .ok_or_else(usage)
    };
    let (mut source, mut output, mut include) = (None, None, Vec::new());
    // The only warning so far is NOTPIC.
    let mut warnings = true;
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
        } else if let Some(name) = qualifier("/NOWARNINGS=") {
            if !name.as_os_str().eq_ignore_ascii_case("NOTPIC") {
                return Err(usage());
            }
            warnings = false;
        } else if let Some(what) = qualifier("/ENABLE=") {
            enable(&what.to_string_lossy())?;
        } else if arg == "--enable" {
            enable(&args.next().ok_or_else(usage)?)?;
        } else if arg == "-o" {
            output = Some(args.next().ok_or_else(usage)?.into());
        } else if arg == "-I" {
            include.push(args.next().ok_or_else(usage)?.into());
        } else if arg == "--nowarnings" {
            if !args
                .next()
                .is_some_and(|n| n.eq_ignore_ascii_case("NOTPIC"))
            {
                return Err(usage());
            }
            warnings = false;
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
    let opts = crate::Options {
        name: stem.chars().take(31).collect(),
        date: date(),
        path: Some(source),
        include,
    };

    let tool = format!("{name} {}", env!("CARGO_PKG_VERSION"));
    let (records, diags) = match crate::assemble_with(&text, &opts, &tool, dialect) {
        Ok(object) => (Some(object.records), object.warnings),
        Err(diags) => (None, diags),
    };
    for d in diags.iter().filter(|d| warnings || !d.warning) {
        let level = if d.warning { "warning" } else { "error" };
        eprintln!("{}:{}:{}: {level}: {}", d.file, d.line, d.col, d.msg);
        let pad: String = d
            .text
            .chars()
            .take(d.col.saturating_sub(1))
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .collect();
        eprintln!("    {}\n    {pad}^", d.text);
        for context in &d.context {
            eprintln!("  {context}");
        }
    }
    let Some(records) = records else {
        return Ok(false);
    };
    fs::write(&output, obj::write(&records))
        .map_err(|e| format!("WRITEERR, {}: {e}", output.display()))?;
    Ok(true)
}

/// Now, or `SOURCE_DATE_EPOCH` for reproducible output, as `dd-mmm-yyyy hh:mm`
/// in UTC.
pub fn date() -> [u8; 17] {
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
