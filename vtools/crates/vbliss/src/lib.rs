//! vbliss: a BLISS-64 compiler for ARM64, writing vaxpunk object modules
//! (PRD-0004 stage 0). `docs/bliss64.md` describes the dialect.
//!
//! The passes: `lex` reads lexemes, `parse` declarations and expressions
//! with their names resolved, `irgen` turns routines into the IR (`ir`),
//! and `arm64` the IR into vasm source, which vasm assembles.

mod arm64;
mod ir;
mod irgen;
mod lex;
mod parse;

use std::fs;
use std::path::Path;

use vasm::{Diagnostic, Object, Options};

const TOOL: &str = concat!("vbliss ", env!("CARGO_PKG_VERSION"));

/// What a compilation produces besides the object.
pub struct Output {
    pub ir: String,
    pub asm: String,
}

/// Compiles BLISS-64 `source` to its IR and vasm assembly, or returns the
/// error and its line (0 if it has none). REQUIRE files are looked for in
/// `dir`.
pub fn translate(source: &str, dir: &Path) -> Result<Output, (u32, String)> {
    let toks = lex::lex(source)?;
    let module = parse::parse(toks, &|name| require(dir, name))?;
    let ir = irgen::generate(&module).map_err(|msg| (0, msg))?;
    Ok(Output {
        ir: ir.to_string(),
        asm: arm64::module(&ir),
    })
}

/// Compiles BLISS-64 `source` into object records, as `vmacro::compile`
/// does MACRO-32.
pub fn compile(source: &str, opts: &Options) -> Result<Object, Vec<Diagnostic>> {
    let dir = opts
        .path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(Path::new("."));
    let out = translate(source, dir).map_err(|(line, msg)| {
        vec![Diagnostic {
            file: opts
                .path
                .as_ref()
                .map_or("<source>".into(), |p| p.display().to_string()),
            line: line as usize,
            col: 1,
            text: source
                .lines()
                .nth((line as usize).wrapping_sub(1))
                .unwrap_or("")
                .into(),
            msg,
            context: Vec::new(),
            warning: false,
        }]
    })?;
    vasm::assemble_with(&out.asm, opts, TOOL, None)
}

/// A require file's text: `name` in `dir`, as given, or with `.R64` or
/// `.REQ` after it if it has no type, in upper or lower case.
fn require(dir: &Path, name: &str) -> Result<String, String> {
    let mut names = vec![name.to_string()];
    if !name.contains('.') {
        names.extend([".R64", ".REQ"].map(|t| format!("{name}{t}")));
    }
    names
        .iter()
        .flat_map(|n| [n.clone(), n.to_ascii_lowercase(), n.to_ascii_uppercase()])
        .find_map(|n| fs::read_to_string(dir.join(n)).ok())
        .ok_or_else(|| format!("can't find require file {name} in {}", dir.display()))
}
