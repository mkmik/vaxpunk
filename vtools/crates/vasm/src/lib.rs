//! vasm: an ARM64 assembler with VMS-style directives and MACRO-style
//! macros, writing vaxpunk object modules. `docs/assembler.md` describes the
//! language.

mod asm;
mod emit;
mod encode;
mod expr;
mod lex;
mod macros;

use std::path::PathBuf;

pub use asm::Diagnostic;
use vms_obj::obj::Record;

#[derive(Default)]
pub struct Options {
    /// The module name when the source has no `.TITLE`.
    pub name: String,
    /// Creation time, `dd-mmm-yyyy hh:mm`.
    pub date: [u8; 17],
    /// Where the source was read from, for messages and `.LIBRARY`.
    pub path: Option<PathBuf>,
    /// Where else `.LIBRARY` looks for macro libraries.
    pub include: Vec<PathBuf>,
}

/// Assembles `source` into object records, or returns every error found.
pub fn assemble(source: &str, opts: &Options) -> Result<Vec<Record>, Vec<Diagnostic>> {
    let module = asm::assemble(source, opts.path.as_deref(), &opts.include)?;
    let tool = concat!("vasm ", env!("CARGO_PKG_VERSION"));
    Ok(emit::records(&module, &opts.name, opts.date, tool))
}
