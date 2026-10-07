//! vbliss: a BLISS-64 compiler for ARM64, writing vaxpunk object modules
//! (PRD-0004 stage 0). `docs/bliss64.md` describes the dialect.
//!
//! The passes: `lex` reads lexemes, `lexical` expands macros and lexical
//! functions, `parse` reads declarations and expressions with their names
//! resolved, `irgen` turns routines into the IR (`ir`), and `arm64` the IR
//! into vasm source, which vasm assembles. `listing` writes the listing.

mod arm64;
mod ir;
mod irgen;
mod lex;
mod lexical;
mod listing;
mod parse;

use std::fs;
use std::path::Path;

use vasm::{Diagnostic, Object};

pub use parse::Options;

const TOOL: &str = concat!("vbliss ", env!("CARGO_PKG_VERSION"));

/// A diagnostic: severity `I`, `W`, `E`, the file, line and column (from
/// 0), and the message.
#[derive(Clone, Debug)]
pub struct Diag {
    pub sev: char,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

/// What compiling a source gives: the IR and the assembly if it compiled,
/// and in any case its diagnostics, its listing and what %MESSAGE wrote.
pub struct Output {
    pub ir: Option<String>,
    pub asm: Option<String>,
    pub diags: Vec<Diag>,
    pub listing: String,
    pub messages: Vec<String>,
}

impl Output {
    /// The first error, if there is one.
    pub fn error(&self) -> Option<&Diag> {
        self.diags.iter().find(|d| d.sev == 'E')
    }
}

/// Compiles BLISS-64 `source`, read from `path`, to its IR and vasm
/// assembly. REQUIRE files are looked for in `path`'s directory.
pub fn translate(path: &Path, source: &str, opts: &Options) -> Output {
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.display().to_string();
    let front = parse::parse(&name, source, &|n| require(dir, n), opts);
    let mut diags: Vec<Diag> = front
        .diags
        .into_iter()
        .map(|(file, d)| Diag {
            sev: d.sev,
            file,
            line: d.line,
            col: d.col,
            msg: d.msg,
        })
        .collect();
    let (mut ir, mut asm) = (None, None);
    if let Ok(module) = front.module {
        match irgen::generate(&module) {
            Ok(m) => {
                asm = Some(arm64::module(&m));
                ir = Some(m.to_string());
            }
            Err(msg) => diags.push(Diag {
                sev: 'E',
                file: name,
                line: 0,
                col: 0,
                msg,
            }),
        }
    }
    Output {
        ir,
        asm,
        diags,
        listing: front.listing,
        messages: front.messages,
    }
}

/// Compiles BLISS-64 `source` into object records, as `vmacro::compile`
/// does MACRO-32; errors come back as the assembler's diagnostics.
pub fn compile(source: &str, opts: &vasm::Options) -> Result<Object, Vec<Diagnostic>> {
    let (obj, out) = compile_with(source, opts, &Options::default());
    match out.error() {
        Some(d) => Err(vec![Diagnostic {
            file: d.file.clone(),
            line: d.line as usize,
            col: d.col as usize + 1,
            text: String::new(),
            msg: d.msg.clone(),
            context: Vec::new(),
            warning: false,
        }]),
        None => obj,
    }
}

/// Compiles BLISS-64 `source` into object records, with the translation's
/// output.
pub fn compile_with(
    source: &str,
    opts: &vasm::Options,
    bliss: &Options,
) -> (Result<Object, Vec<Diagnostic>>, Output) {
    let path = opts.path.clone().unwrap_or_else(|| "<source>".into());
    let out = translate(&path, source, bliss);
    let obj = match (&out.asm, out.error()) {
        (Some(asm), None) => vasm::assemble_with(asm, opts, TOOL, None),
        _ => Err(Vec::new()),
    };
    (obj, out)
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
