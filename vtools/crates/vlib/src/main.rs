//! vlib: puts object modules into object libraries.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use vms_obj::olb::Library;

const USAGE: &str = "usage: vlib [/CREATE] LIBRARY.OLB FILE.OBJ...
       vlib /LIST [/NAMES] LIBRARY.OLB
       (or --create, --list, --names)";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let usage = || format!("%VLIB-F-USAGE, {USAGE}");
    let (mut create, mut list, mut names, mut files) = (false, false, false, Vec::new());
    for arg in env::args().skip(1) {
        match arg.to_ascii_uppercase().as_str() {
            "/CREATE" | "--CREATE" => create = true,
            "/LIST" | "--LIST" => list = true,
            "/NAMES" | "--NAMES" => names = true,
            _ if arg.starts_with('-') => return Err(usage()),
            _ => files.push(PathBuf::from(arg)),
        }
    }
    let Some((path, objects)) = files.split_first() else {
        return Err(usage());
    };
    let read =
        |p: &PathBuf| fs::read(p).map_err(|e| format!("%VLIB-F-OPENIN, {}: {e}", p.display()));
    let parse = |p: &PathBuf| {
        Library::parse(&read(p)?).map_err(|e| {
            format!(
                "%VLIB-F-NOTLIB, {} is not an object library: {e}",
                p.display()
            )
        })
    };

    if list {
        if create || !objects.is_empty() {
            return Err(usage());
        }
        for m in parse(path)?.modules {
            println!("{}", m.name);
            if names {
                for s in m.symbols {
                    println!("    {s}");
                }
            }
        }
        return Ok(());
    }
    if names || (objects.is_empty() && !create) {
        return Err(usage());
    }
    let time = now();
    let mut lib = if create {
        vlib::new(time)
    } else {
        parse(path)?
    };
    lib.updated = time;
    for object in objects {
        let file = object.display().to_string();
        for w in vlib::replace(&mut lib, &file, &read(object)?, time)? {
            eprintln!("{w}");
        }
    }
    fs::write(path, lib.write()).map_err(|e| format!("%VLIB-F-WRITEERR, {}: {e}", path.display()))
}

/// Now, or `SOURCE_DATE_EPOCH`, as a VMS time: 100 ns units since 17-Nov-1858.
fn now() -> u64 {
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
