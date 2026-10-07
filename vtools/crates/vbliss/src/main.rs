//! vbliss: compiles one BLISS-64 source file into an object module.
//!
//!     vbliss [/A64 | /A32] [/ASSUME=(LONG_DEFAULT, REF_LONG, SIGNED_LONG)]
//!            [/OBJECT=file | -o file] [/LIST[=file]] [/VARIANT=n]
//!            [/INCLUDE=(dir,...) | -I dir] [--ir] [--asm] SOURCE
//!
//! A .B32 source is BLISS-32 as with /A32.
//!
//! --ir and --asm print the IR or the assembly instead of writing an object.
//! Diagnostics go to stderr as BLISS writes them to the terminal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("%VBLISS-F-{msg}");
            ExitCode::FAILURE
        }
    }
}

/// `/QUALIFIER=value` or `/QUALIFIER`: its value, if `arg` is it.
fn qualifier<'a>(arg: &'a str, name: &str) -> Option<Option<&'a str>> {
    let (q, v) = match arg.split_once('=') {
        Some((q, v)) => (q, Some(v)),
        None => (arg, None),
    };
    q.eq_ignore_ascii_case(name).then_some(v)
}

fn run() -> Result<(), String> {
    let usage = || {
        "USAGE, usage: vbliss [/OBJECT=file | -o file] [/LIST[=file]] [/VARIANT=n] [--ir] [--asm] SOURCE"
            .to_string()
    };
    let (mut source, mut output, mut list, mut ir, mut asm) = (None, None, None, false, false);
    let mut opts = vbliss::Options::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(v) = qualifier(&arg, "/OBJECT") {
            output = Some(PathBuf::from(v.ok_or_else(usage)?));
        } else if let Some(v) = qualifier(&arg, "/LIST") {
            list = Some(v.map(PathBuf::from));
        } else if arg.starts_with('/')
            && opts.qualifier(&arg).map_err(|e| format!("BADVALUE, {e}"))?
        {
        } else if let Some(v) = qualifier(&arg, "/INCLUDE") {
            opts.include.extend(
                v.ok_or_else(usage)?
                    .trim_matches(['(', ')'])
                    .split(',')
                    .map(PathBuf::from),
            );
        } else if arg == "-I" {
            opts.include.push(args.next().ok_or_else(usage)?.into());
        } else if arg == "-o" {
            output = Some(args.next().ok_or_else(usage)?.into());
        } else if arg == "--ir" {
            ir = true;
        } else if arg == "--asm" {
            asm = true;
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
    let vopts = vasm::Options {
        name: stem.chars().take(31).collect(),
        date: vasm::date(),
        path: Some(source.clone()),
        include: opts.include.clone(),
    };
    let (object, out) = vbliss::compile_with(&text, &vopts, &opts);
    for m in &out.messages {
        eprintln!("{m}");
    }
    for d in &out.diags {
        report(d);
    }
    if let Some(list) = list {
        let path = list.unwrap_or_else(|| source.with_extension("lis"));
        fs::write(&path, &out.listing).map_err(|e| format!("WRITEERR, {}: {e}", path.display()))?;
    }
    if out.error().is_some() {
        return Err("ENDDIAGS, compilation failed".into());
    }
    if ir || asm {
        print!("{}", if ir { out.ir } else { out.asm }.unwrap_or_default());
        return Ok(());
    }
    match object {
        Ok(object) => {
            let output = output.unwrap_or_else(|| source.with_extension("obj"));
            fs::write(&output, vms_obj::obj::write(&object.records))
                .map_err(|e| format!("WRITEERR, {}: {e}", output.display()))
        }
        Err(diags) => {
            for d in diags {
                eprintln!("{}:{}: error: {}", d.file, d.line, d.msg);
            }
            Err("ENDDIAGS, compilation failed".into())
        }
    }
}

/// A diagnostic as BLISS writes it on the terminal: the line, a marker
/// under the place, the message and where.
fn report(d: &vbliss::Diag) {
    let line = fs::read_to_string(Path::new(&d.file)).ok().and_then(|t| {
        t.lines()
            .nth((d.line as usize).wrapping_sub(1))
            .map(String::from)
    });
    eprintln!();
    if let Some(line) = line {
        eprintln!("{line}");
        eprintln!("{}^", ".".repeat(d.col as usize));
    }
    eprintln!("%BLS64-{}-TEXT, {}", d.sev, d.msg);
    eprintln!("at line number {} in file {}", d.line, d.file);
}
