//! vcdu: compiles `.CLD` files into a command table's object module.

use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

const USAGE: &str = "usage: vcdu [/OBJECT=file | -o file] [/MACRO | --macro] FILE.CLD...";

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
    let usage = || vec![format!("%VCDU-F-USAGE, {USAGE}")];
    let (mut output, mut macro_, mut files) = (None, false, Vec::new());
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let upper = arg.to_ascii_uppercase();
        if let Some(path) = upper.strip_prefix("/OBJECT=") {
            output = Some(PathBuf::from(&arg[arg.len() - path.len()..]));
        } else if arg == "-o" {
            output = Some(args.next().ok_or_else(usage)?.into());
        } else if upper == "/MACRO" || upper == "--MACRO" {
            macro_ = true;
        } else if arg.starts_with('-') {
            return Err(usage());
        } else {
            files.push(PathBuf::from(arg));
        }
    }
    let first = files.first().ok_or_else(usage)?.clone();
    let sources = files
        .iter()
        .map(|f| {
            fs::read_to_string(f)
                .map(|text| (f.display().to_string(), text))
                .map_err(|e| vec![format!("%VCDU-F-OPENIN, {}: {e}", f.display())])
        })
        .collect::<Result<Vec<_>, _>>()?;
    let name = first
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let source = vcdu::compile(&name, &sources)?;
    let ext = if macro_ { "mar" } else { "obj" };
    let output = output.unwrap_or_else(|| first.with_extension(ext));
    let bytes = if macro_ {
        source.into_bytes()
    } else {
        let opts = vasm::Options {
            name,
            ..Default::default()
        };
        let object = vmacro::compile(&source, &opts).map_err(|diags| {
            diags
                .iter()
                .map(|d| format!("%VCDU-F-BUG, {}:{}: {}", d.file, d.line, d.msg))
                .collect::<Vec<_>>()
        })?;
        vms_obj::obj::write(&object.records)
    };
    fs::write(&output, bytes)
        .map_err(|e| vec![format!("%VCDU-F-WRITEERR, {}: {e}", output.display())])
}
