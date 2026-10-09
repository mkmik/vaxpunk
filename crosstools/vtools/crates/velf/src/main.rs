//! velf: turns one ELF relocatable object from a C compiler into an object
//! module.
//!
//!     velf [/OBJECT=file | -o file] FILE.o
//!
//! The object module goes to the file's name with `.obj` unless /OBJECT
//! says otherwise, and is named after the object file, in upper case: a
//! module name has at most 31 characters, so /OBJECT names a module whose
//! source file's name is longer.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use vms_obj::obj;

const USAGE: &str = "usage: velf [/OBJECT=file | -o file] FILE.o";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("%VELF-F-{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let (mut output, mut input) = (None, None);
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let upper = arg.to_ascii_uppercase();
        if arg == "-o" {
            output = Some(PathBuf::from(args.next().ok_or(format!("USAGE, {USAGE}"))?));
        } else if let Some(file) = upper.strip_prefix("/OBJECT=") {
            output = Some(PathBuf::from(&arg[arg.len() - file.len()..]));
        } else if arg.starts_with('-') || input.is_some() {
            return Err(format!("USAGE, {USAGE}"));
        } else {
            input = Some(PathBuf::from(arg));
        }
    }
    let input = input.ok_or(format!("USAGE, {USAGE}"))?;
    let elf = fs::read(&input).map_err(|e| format!("OPENIN, {}: {e}", input.display()))?;
    let output = output.unwrap_or_else(|| input.with_extension("obj"));
    let name = module_name(&output)?;
    let records = velf::convert(&elf, &name, vasm::date())
        .map_err(|e| format!("NOTCONV, {}: {e}", input.display()))?;
    fs::write(&output, obj::write(&records))
        .map_err(|e| format!("OPENOUT, {}: {e}", output.display()))
}

/// The module's name: the object file's, in upper case.
fn module_name(path: &Path) -> Result<String, String> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    if stem.is_empty() || stem.len() > 31 || !stem.is_ascii() {
        return Err(format!("BADNAME, {stem:?} can't name a module"));
    }
    Ok(stem.to_ascii_uppercase())
}
