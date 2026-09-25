//! vasm: an ARM64 assembler with VMS-style directives, writing vaxpunk object
//! modules. `docs/assembler.md` describes the language.

mod asm;
mod emit;
mod encode;
mod expr;
mod lex;

pub use asm::Diagnostic;
use vms_obj::obj::Record;

/// Assembles `source` into object records. `name` names the module when the
/// source has no `.TITLE`; `date` is its creation time, `dd-mmm-yyyy hh:mm`.
pub fn assemble(source: &str, name: &str, date: [u8; 17]) -> Result<Vec<Record>, Vec<Diagnostic>> {
    let module = asm::assemble(source)?;
    Ok(emit::records(
        &module,
        name,
        date,
        concat!("vasm ", env!("CARGO_PKG_VERSION")),
    ))
}
