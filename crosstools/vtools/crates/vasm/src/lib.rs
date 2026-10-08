//! vasm: an ARM64 assembler with VMS-style directives and MACRO-style
//! macros, writing vaxpunk object modules. `docs/assembler.md` describes the
//! language.

mod asm;
mod cli;
mod emit;
mod encode;
mod expr;
mod lex;
mod macros;

use std::path::PathBuf;

pub use asm::Diagnostic;
pub use cli::{date, main};
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

/// An assembled object module.
pub struct Object {
    pub records: Vec<Record>,
    pub warnings: Vec<Diagnostic>,
}

/// A language layered on vasm, which translates its statements to ARM64:
/// MACRO-32 (`vmacro`). vasm still does labels, macros, conditionals,
/// repeat blocks and the directives the dialect leaves to it. With a
/// dialect, expressions are in MACRO-32 syntax (`expr.rs`), and an
/// instruction after data is aligned, with the labels just before it.
pub trait Dialect {
    /// Translates a statement: `word` is its first word after the labels,
    /// in upper case, and `rest` the rest without the comment. Returns the
    /// lines that replace it, assembled as they are, or `None` to leave it
    /// to vasm. `constant` gives the value of an expression if it is a
    /// constant already known.
    fn statement(
        &mut self,
        word: &str,
        rest: &str,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Option<Result<Vec<String>, String>>;

    /// A label defined at the current location, `global` for `NAME::`. A
    /// local label's name has an `@`.
    fn label(&mut self, _name: &str, _global: bool) {}

    /// Called after each pass over the source: whether to assemble it
    /// again, once the dialect learned from this pass what it needs to.
    fn again(&mut self) -> bool {
        false
    }

    /// The command line's `/ENABLE=what`: whether the dialect has it.
    fn enable(&mut self, _what: &str) -> bool {
        false
    }
}

/// Assembles `source` into object records, or returns every error found,
/// with the warnings.
pub fn assemble(source: &str, opts: &Options) -> Result<Object, Vec<Diagnostic>> {
    let tool = concat!("vasm ", env!("CARGO_PKG_VERSION"));
    assemble_with(source, opts, tool, None)
}

/// Assembles `source` in `dialect`, if given; `tool` goes into the module
/// header.
pub fn assemble_with(
    source: &str,
    opts: &Options,
    tool: &str,
    dialect: Option<&mut dyn Dialect>,
) -> Result<Object, Vec<Diagnostic>> {
    let mut dialect = dialect;
    let module = loop {
        let module = asm::assemble(
            source,
            opts.path.as_deref(),
            &opts.include,
            dialect.as_mut().map(|d| &mut **d as &mut dyn Dialect),
        );
        if !dialect.as_mut().is_some_and(|d| d.again()) {
            break module?;
        }
    };
    Ok(Object {
        records: emit::records(&module, &opts.name, opts.date, tool),
        warnings: module.warnings,
    })
}
