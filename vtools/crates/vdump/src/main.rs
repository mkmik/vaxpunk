//! vdump: decoded dump of vaxpunk object modules, object libraries and images.

use std::process::ExitCode;
use std::{env, fs};

const USAGE: &str = "usage: vdump [--weights] [--map FILE] FILE...
  --weights   for objects: what each store needs for the image to move
  --map FILE  for images: the link map, which names psects";

fn main() -> ExitCode {
    let mut opts = vdump::Options::default();
    let mut files = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--weights" => opts.weights = true,
            "--map" => {
                let Some(map) = args.next() else {
                    eprintln!("{USAGE}");
                    return ExitCode::FAILURE;
                };
                match fs::read_to_string(&map) {
                    Ok(text) => opts.map = Some(text),
                    Err(e) => {
                        eprintln!("%VDUMP-E-OPENIN, {map}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut status = ExitCode::SUCCESS;
    for file in &files {
        let dump = fs::read(file)
            .map_err(|e| format!("OPENIN, {file}: {e}"))
            .and_then(|b| vdump::dump(&b, &opts).map_err(|e| format!("FORMAT, {file}: {e}")));
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
