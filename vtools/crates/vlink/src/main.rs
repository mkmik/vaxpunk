//! vlink: links object modules into an executable image.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

const USAGE: &str =
    "usage: vlink [/EXE=file] [/MAP[=file]] [/BASE=address] [/TRANSFER=symbol] FILE...
       vlink [-o file] [-m file] [--base address] [--transfer symbol] FILE...
A FILE is an object module, or an object library: LIB.OLB/LIBRARY or LIB.OLB.";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msgs) => {
            for m in msgs {
                eprintln!("{m}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Vec<String>> {
    let usage = || vec![format!("%VLINK-F-USAGE, {USAGE}")];
    let (mut inputs, mut exe, mut map, mut base, mut transfer) =
        (Vec::new(), None, None, None, None);
    let mut want_map = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        // DCL qualifiers, /NAME or /NAME=value, then Unix options.
        let (name, value) = match arg.split_once('=') {
            Some((n, v)) => (n.to_ascii_uppercase(), Some(v.to_string())),
            None => (arg.to_ascii_uppercase(), None),
        };
        match (name.as_str(), value) {
            ("/EXE" | "/EXECUTABLE", Some(v)) => exe = Some(PathBuf::from(v)),
            ("/MAP", v) => {
                want_map = true;
                map = v.map(PathBuf::from);
            }
            ("/BASE", Some(v)) => base = Some(v),
            ("/TRANSFER", Some(v)) => transfer = Some(v),
            ("-O", None) => exe = Some(args.next().ok_or_else(usage)?.into()),
            ("-M", None) => {
                want_map = true;
                map = Some(args.next().ok_or_else(usage)?.into());
            }
            ("--BASE", None) => base = Some(args.next().ok_or_else(usage)?),
            ("--TRANSFER", None) => transfer = Some(args.next().ok_or_else(usage)?),
            _ if arg.starts_with('-') => return Err(usage()),
            _ if name.ends_with("/LIBRARY") => {
                inputs.push((PathBuf::from(&arg[..arg.len() - 8]), true));
            }
            _ => inputs.push((PathBuf::from(arg), false)),
        }
    }
    let Some((first, _)) = inputs.first() else {
        return Err(usage());
    };
    let exe = exe.unwrap_or_else(|| first.with_extension("exe"));
    let map = want_map.then(|| map.unwrap_or_else(|| exe.with_extension("map")));
    let base = match base {
        None => vlink::DEFAULT_BASE,
        Some(b) => {
            number(&b).ok_or_else(|| vec![format!("%VLINK-F-BADBASE, bad base address {b}")])?
        }
    };

    let mut files = Vec::new();
    for (path, library) in &inputs {
        let bytes = fs::read(path)
            .map_err(|e| vec![format!("%VLINK-F-OPENIN, {}: {e}", path.display())])?;
        if *library && !vms_obj::olb::is_library(&bytes) {
            return Err(vec![format!(
                "%VLINK-F-NOTLIB, {} is not an object library",
                path.display()
            )]);
        }
        files.push((path.display().to_string(), bytes));
    }
    let name = exe
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let opts = vlink::Options {
        base,
        name,
        transfer: transfer.map(|t| t.to_ascii_uppercase()),
        link_time: link_time(),
    };
    let linked = vlink::link(&files, &opts)?;
    for w in &linked.warnings {
        eprintln!("{w}");
    }
    let write = |path: &PathBuf, bytes: &[u8]| {
        fs::write(path, bytes)
            .map_err(|e| vec![format!("%VLINK-F-WRITEERR, {}: {e}", path.display())])
    };
    write(&exe, &linked.image.write())?;
    if let Some(map) = map {
        write(&map, linked.map.as_bytes())?;
    }
    Ok(())
}

/// A number in decimal, `0x` hex or VMS `%X` hex.
fn number(s: &str) -> Option<u64> {
    let upper = s.to_ascii_uppercase();
    match upper
        .strip_prefix("0X")
        .or_else(|| upper.strip_prefix("%X"))
    {
        Some(hex) => u64::from_str_radix(&hex.replace('_', ""), 16).ok(),
        None => upper.parse().ok(),
    }
}

/// Now, or `SOURCE_DATE_EPOCH`, as a VMS time: 100 ns units since 17-Nov-1858.
fn link_time() -> u64 {
    const UNIX_EPOCH_VMS_SECONDS: u64 = 3_506_716_800;
    let secs = env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        });
    (secs + UNIX_EPOCH_VMS_SECONDS) * 10_000_000
}
