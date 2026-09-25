//! vdump: decoded dump of vaxpunk object modules and images.

use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let files: Vec<String> = env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: vdump FILE...");
        return ExitCode::FAILURE;
    }
    let mut status = ExitCode::SUCCESS;
    for file in &files {
        let dump = fs::read(file)
            .map_err(|e| format!("OPENIN, {file}: {e}"))
            .and_then(|b| vdump::dump(&b).map_err(|e| format!("FORMAT, {file}: {e}")));
        match dump {
            Ok(text) if files.len() > 1 => print!("{file}:\n{text}\n"),
            Ok(text) => print!("{text}"),
            Err(msg) => {
                eprintln!("%VDUMP-E-{msg}");
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}
