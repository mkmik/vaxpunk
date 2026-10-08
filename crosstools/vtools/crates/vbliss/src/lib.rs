//! vbliss: a BLISS-64 compiler for ARM64, writing vaxpunk object modules
//! (PRD-0004 stage 0). `docs/bliss64.md` describes the dialect.
//!
//! The passes: `lex` reads lexemes, `lexical` expands macros and lexical
//! functions, `parse` reads declarations and expressions with their names
//! resolved, `irgen` turns routines into the IR (`ir`), and `arm64` the IR
//! into vasm source, which vasm assembles. `listing` writes the listing.

mod arm64;
mod builtin;
mod data;
mod ir;
mod irgen;
mod lex;
mod lexical;
mod linkage;
mod lint;
mod listing;
mod parse;

use std::fs;
use std::path::Path;

use vasm::{Diagnostic, Object};

pub use parse::{Dialect, Options};

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
    /// The dot lint's warnings (PRD-0004 *The dot lint*), which aren't
    /// BLISS's diagnostics and stay out of the listing.
    pub lints: Vec<Diag>,
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
    let mut dirs = vec![path.parent().unwrap_or(Path::new(".")).to_path_buf()];
    dirs.extend(opts.include.iter().cloned());
    let name = path.display().to_string();
    // A .B32 source is BLISS-32, as /A32 says.
    let mut opts = opts.clone();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("b32"))
    {
        opts.dialect.a32 = true;
    }
    let types: &[&str] = if opts.dialect.a32 {
        &[".R32", ".REQ"]
    } else {
        &[".R64", ".REQ"]
    };
    let front = parse::parse(&name, source, &|n| require(&dirs, n, types), &opts);
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
    let lints = front
        .lints
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
        lints,
        listing: front.listing,
        messages: front.messages,
    }
}

/// Compiles BLISS-64 `source` into object records, as `vmacro::compile`
/// does MACRO-32; errors come back as the assembler's diagnostics.
pub fn compile(source: &str, opts: &vasm::Options) -> Result<Object, Vec<Diagnostic>> {
    let bliss = Options {
        include: opts.include.clone(),
        ..Options::default()
    };
    let (obj, out) = compile_with(source, opts, &bliss);
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

/// A require or library file's text: `name` in one of `dirs`, as given,
/// or with one of `types` after it if it has no type (`.R64` or `.R32`,
/// then `.REQ`), in upper or lower case. A device or directory before the
/// name (`SYS$LIBRARY:STARLET`) stands for the directories searched; a
/// library's `.L64` or `.L32` is its source.
fn require(dirs: &[std::path::PathBuf], name: &str, types: &[&str]) -> Result<String, String> {
    // ponytail: [-] is the parent directory; any other directory is dropped.
    let (up, name) = match name.strip_prefix("[-]") {
        Some(rest) => ("../", rest),
        None => ("", name),
    };
    let base = name.rsplit([':', ']', '>']).next().unwrap_or(name);
    let base = &format!("{up}{base}");
    let base = [".L64", ".l64", ".L32", ".l32"]
        .iter()
        .find_map(|t| base.strip_suffix(t))
        .unwrap_or(base);
    let mut names = vec![base.to_string()];
    if !base.rsplit('/').next().unwrap_or(base).contains('.') {
        names.extend(types.iter().map(|t| format!("{base}{t}")));
    }
    dirs.iter()
        .flat_map(|d| {
            names
                .iter()
                .flat_map(|n| [n.clone(), n.to_ascii_lowercase(), n.to_ascii_uppercase()])
                .map(move |n| d.join(n))
        })
        .find_map(|p| fs::read_to_string(p).ok())
        .ok_or_else(|| format!("can't find require file {name}"))
}
