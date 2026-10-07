//! vdefs: writes the BLISS require files for vaxpunk's macro libraries.
//!
//!     vdefs [DIR]
//!
//! For lib.mlb and starlet.mlb in DIR (default vtools/lib), writes
//! lib.r64 and starlet.r64 for BLISS-64 and lib.req and starlet.req for
//! BLISS-32 next to them. vdefs's test fails when they are stale.

use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

use vdefs::Dialect;

fn main() -> ExitCode {
    let dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "vtools/lib".into()));
    for lib in ["lib", "starlet"] {
        let mlb = dir.join(format!("{lib}.mlb"));
        let text = match fs::read_to_string(&mlb) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("%VDEFS-F-OPENIN, {}: {e}", mlb.display());
                return ExitCode::FAILURE;
            }
        };
        for (ext, dialect) in [("r64", Dialect::Bliss64), ("req", Dialect::Bliss32)] {
            let out = match vdefs::generate(&format!("{lib}.mlb"), &text, dialect) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("%VDEFS-F-NAMING, {e}");
                    return ExitCode::FAILURE;
                }
            };
            let path = dir.join(format!("{lib}.{ext}"));
            if let Err(e) = fs::write(&path, out) {
                eprintln!("%VDEFS-F-WRITEERR, {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
