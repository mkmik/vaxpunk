//! vbliss: compiles one BLISS-64 source file into an object module.
//!
//!     vbliss [/OBJECT=file | -o file] [--ir] [--asm] SOURCE.B64
//!
//! --ir and --asm print the IR or the assembly instead of writing an object.

use std::path::PathBuf;
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

fn run() -> Result<(), String> {
    let usage =
        || "USAGE, usage: vbliss [/OBJECT=file | -o file] [--ir] [--asm] SOURCE".to_string();
    let (mut source, mut output, mut ir, mut asm) = (None, None, false, false);
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg.len() > 8 && arg[..8].eq_ignore_ascii_case("/OBJECT=") {
            output = Some(PathBuf::from(&arg[8..]));
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
    if ir || asm {
        let dir = source.parent().unwrap_or(std::path::Path::new("."));
        let out = vbliss::translate(&text, dir)
            .map_err(|(line, msg)| format!("ERROR, {}:{line}: {msg}", source.display()))?;
        print!("{}", if ir { out.ir } else { out.asm });
        return Ok(());
    }
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let opts = vasm::Options {
        name: stem.chars().take(31).collect(),
        date: vasm::date(),
        path: Some(source.clone()),
        include: Vec::new(),
    };
    match vbliss::compile(&text, &opts) {
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
