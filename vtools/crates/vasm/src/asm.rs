//! Two passes over the source. Pass 1 parses every line and lays out the
//! psects; pass 2 evaluates operands, encodes instructions and collects each
//! psect's contents and relocations, which `emit` turns into records.

use std::collections::HashMap;

use vms_obj::obj::psc;

use crate::encode::{self, Cx, Encoded, Fix, Operand};
use crate::expr::{self, Expr, Scope, Value};
use crate::lex::{self, Cursor, Result, err};

/// An error at a line and column (both from 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

pub struct Psect {
    pub name: String,
    pub flags: u16,
    /// Alignment as a power of two.
    pub align: u8,
    pub size: u64,
    /// Contents in offset order. Gaps are zero.
    pub chunks: Vec<(u64, Chunk)>,
}

pub enum Chunk {
    Bytes(Vec<u8>),
    /// An instruction the linker patches.
    Insn {
        fix: Fix,
        word: u32,
        target: Value,
    },
    /// A value the linker stores, 1 to 8 bytes.
    Data {
        size: u8,
        value: Value,
    },
}

#[derive(Default)]
pub struct Symbol {
    pub value: Option<Value>,
    pub global: bool,
    pub weak: bool,
    pub external: bool,
}

/// An assembled module, ready for `emit`.
pub struct Module {
    pub name: Option<String>,
    pub version: Option<String>,
    pub psects: Vec<Psect>,
    /// Symbols in the order they first appeared.
    pub symbols: Vec<(String, Symbol)>,
    pub transfer: Option<(usize, u64)>,
}

enum Item {
    Insn {
        mn: String,
        col: usize,
        ops: Vec<Operand>,
    },
    Data {
        size: u8,
        exprs: Vec<(Expr, usize)>,
    },
    Bytes(Vec<u8>),
    /// A VMS string descriptor followed by the string.
    Ascid(Vec<u8>),
    /// Zero-filled space the object file doesn't store.
    Space,
}

struct Stmt {
    line: usize,
    psect: usize,
    offset: u64,
    item: Item,
}

/// The standard psects' attributes (Table B-7 of the Alpha object language).
const STANDARD: [(&str, u16); 7] = [
    ("$CODE$", psc::PIC | psc::REL | psc::SHR | psc::EXE),
    ("$DATA$", psc::REL | psc::RD | psc::WRT),
    ("$LINK$", psc::REL | psc::RD),
    ("$LITERAL$", psc::PIC | psc::REL | psc::SHR | psc::RD),
    ("$READONLY$", psc::PIC | psc::REL | psc::SHR | psc::RD),
    ("$BSS$", psc::REL | psc::RD | psc::WRT | psc::NOMOD),
    ("$ABS$", psc::SHR),
];
/// Any other psect: NOPIC, CON, REL, LCL, NOSHR, NOEXE, RD, WRT, QUAD.
const DEFAULT: u16 = psc::REL | psc::RD | psc::WRT;
const DEFAULT_ALIGN: u8 = 3;

/// Assembles `source`, or returns every error found.
pub fn assemble(source: &str) -> std::result::Result<Module, Vec<Diagnostic>> {
    let mut a = Asm::default();
    for (i, line) in source.lines().enumerate() {
        a.line = i + 1;
        if let Err(e) = a.statement(lex::strip_comment(line)) {
            a.error(e);
        }
        if a.ended {
            break;
        }
    }
    // A global or weak symbol this module doesn't define is a reference.
    for s in a.symbols.values_mut() {
        if s.value.is_none() && (s.global || s.weak) {
            s.external = true;
        }
    }
    for p in &a.psects {
        if p.size > u64::from(u32::MAX) {
            let msg = format!("psect {} is larger than 4 GB", p.name);
            a.diags.push(Diagnostic {
                line: 0,
                col: 1,
                msg,
            });
        }
    }
    a.pass2()
}

#[derive(Default)]
struct Asm {
    psects: Vec<Psect>,
    symbols: HashMap<String, Symbol>,
    order: Vec<String>,
    stmts: Vec<Stmt>,
    cur: Option<usize>,
    /// Local label block, bumped at every other label and psect change.
    block: u32,
    name: Option<String>,
    version: Option<String>,
    transfer: Option<(Expr, usize, usize)>,
    ended: bool,
    diags: Vec<Diagnostic>,
    line: usize,
}

/// Symbols and the location counter, as an expression sees them.
struct View<'a> {
    symbols: &'a HashMap<String, Symbol>,
    here: Value,
}

impl Scope for View<'_> {
    fn lookup(&self, name: &str) -> Option<Value> {
        let s = self.symbols.get(name)?;
        s.value.clone().or_else(|| {
            let ext = s.external || s.global || s.weak;
            ext.then(|| Value::Ext {
                name: name.to_string(),
                offset: 0,
            })
        })
    }
    fn here(&self) -> Value {
        self.here.clone()
    }
}

impl Asm {
    fn error(&mut self, e: lex::Error) {
        self.diags.push(Diagnostic {
            line: self.line,
            col: e.col,
            msg: e.msg,
        });
    }

    fn symbol(&mut self, name: &str) -> &mut Symbol {
        if !self.symbols.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.symbols.entry(name.to_string()).or_default()
    }

    fn eval(&self, e: &Expr, col: usize, here: Value) -> Result<Value> {
        let view = View {
            symbols: &self.symbols,
            here,
        };
        expr::eval(e, &view).or_else(|msg| err(col, msg))
    }

    /// A constant, which must be known in pass 1.
    fn constant(&mut self, c: &mut Cursor, what: &str) -> Result<i64> {
        c.skip_ws();
        let col = c.col();
        let e = expr::parse(c, self.block)?;
        let here = self.here();
        match self.eval(&e, col, here)? {
            Value::Abs(n) => Ok(n),
            _ => err(col, format!("{what} must be a constant")),
        }
    }

    /// The current psect, `$CODE$` if none was chosen.
    fn current(&mut self) -> usize {
        if self.cur.is_none() {
            self.open("$CODE$".into(), None, 1)
                .expect("$CODE$ is a valid psect");
        }
        self.cur.unwrap()
    }

    fn here(&mut self) -> Value {
        let psect = self.current();
        Value::Psect {
            psect,
            offset: self.psects[psect].size as i64,
        }
    }

    fn define(&mut self, name: &str, value: Value, col: usize) -> Result<()> {
        let sym = self.symbol(name);
        if sym.value.is_some() {
            return err(col, format!("{} is already defined", expr::display(name)));
        }
        if sym.external {
            return err(col, format!("{} is declared external", expr::display(name)));
        }
        sym.value = Some(value);
        Ok(())
    }

    fn item(&mut self, item: Item, size: u64) {
        let psect = self.current();
        let p = &mut self.psects[psect];
        self.stmts.push(Stmt {
            line: self.line,
            psect,
            offset: p.size,
            item,
        });
        p.size += size;
    }

    fn statement(&mut self, text: &str) -> Result<()> {
        let mut c = Cursor::new(text);
        // Labels: NAME: local to the module, NAME:: global, 10$: a local label.
        loop {
            let save = c.at;
            c.skip_ws();
            let col = c.col();
            let label = match local_label(&mut c) {
                Some(n) => Some(format!("{n}$@{}", self.block)),
                None => c.name(),
            };
            match label {
                Some(name) if c.rest().starts_with(':') => {
                    c.at += 1;
                    let global = c.rest().starts_with(':');
                    c.at += global as usize;
                    self.label(&name, global, col)?;
                }
                _ => {
                    c.at = save;
                    break;
                }
            }
        }
        if c.at_end() {
            return Ok(());
        }
        let col = c.col();
        let Some(word) = c.name() else {
            return err(col, "expected a label, directive or instruction");
        };
        if c.eat('=') {
            // NAME = value, or NAME == value to make it global.
            let global = c.eat('=');
            c.skip_ws();
            let vcol = c.col();
            let e = expr::parse(&mut c, self.block)?;
            end(&mut c)?;
            let here = self.here();
            let value = self.eval(&e, vcol, here)?;
            self.define(&word, value, col)?;
            self.symbol(&word).global |= global;
            return Ok(());
        }
        if word.starts_with('.') {
            return self.directive(&word, &mut c, col);
        }
        let ops = encode::operands(&mut c, self.block)?;
        let psect = self.current();
        if !self.psects[psect].size.is_multiple_of(4) {
            return err(
                col,
                "instruction at an address that isn't 4-byte aligned; use .ALIGN LONG",
            );
        }
        self.item(Item::Insn { mn: word, col, ops }, 4);
        Ok(())
    }

    fn label(&mut self, name: &str, global: bool, col: usize) -> Result<()> {
        let local = name.contains('@');
        if global && local {
            return err(col, "a local label can't be global");
        }
        let here = self.here();
        self.define(name, here, col)?;
        self.symbol(name).global |= global;
        if !local {
            self.block += 1;
        }
        Ok(())
    }

    /// Switches to psect `name`, creating it with `attrs` or its defaults.
    fn open(&mut self, name: String, attrs: Option<(u16, u8)>, col: usize) -> Result<()> {
        if name.len() > 31 {
            return err(col, "psect names are at most 31 characters");
        }
        self.block += 1;
        if let Some(i) = self.psects.iter().position(|p| p.name == name) {
            let p = &self.psects[i];
            if attrs.is_some_and(|a| a != (p.flags, p.align)) {
                return err(
                    col,
                    format!("psect {name} was declared with other attributes"),
                );
            }
            self.cur = Some(i);
            return Ok(());
        }
        let (flags, align) = attrs.unwrap_or_else(|| {
            let standard = STANDARD.iter().find(|s| s.0 == name);
            (standard.map_or(DEFAULT, |s| s.1), DEFAULT_ALIGN)
        });
        if flags & psc::EXE != 0 && flags & psc::WRT != 0 {
            return err(col, "a psect can't be both EXE and WRT");
        }
        self.psects.push(Psect {
            name,
            flags,
            align,
            size: 0,
            chunks: Vec::new(),
        });
        self.cur = Some(self.psects.len() - 1);
        Ok(())
    }

    fn directive(&mut self, name: &str, c: &mut Cursor, col: usize) -> Result<()> {
        match name {
            ".TITLE" => {
                let Some(title) = c.name() else {
                    return err(c.col(), "expected a module name");
                };
                if title.len() > 31 {
                    return err(col, "module names are at most 31 characters");
                }
                self.name = Some(title);
                c.at = c.line.len(); // the rest is the title text
            }
            ".IDENT" => {
                let ident = self.strings(c)?;
                if ident.len() > 31 {
                    return err(col, "idents are at most 31 characters");
                }
                self.version = Some(String::from_utf8_lossy(&ident).into_owned());
            }
            ".PSECT" => {
                let Some(psect) = c.name() else {
                    return err(c.col(), "expected a psect name");
                };
                let attrs = if c.eat(',') {
                    Some(self.attributes(c)?)
                } else {
                    None
                };
                self.open(psect, attrs, col)?;
            }
            ".GLOBAL" | ".GLOBL" | ".EXTERNAL" | ".EXTERN" | ".WEAK" => loop {
                c.skip_ws();
                let ncol = c.col();
                let Some(sym) = c.name() else {
                    return err(ncol, "expected a symbol name");
                };
                let s = self.symbol(&sym);
                match name {
                    ".WEAK" => s.weak = true,
                    ".GLOBAL" | ".GLOBL" => s.global = true,
                    _ if s.value.is_some() => {
                        return err(
                            ncol,
                            format!("{sym} is defined here, so it can't be external"),
                        );
                    }
                    _ => s.external = true,
                }
                if !c.eat(',') {
                    break;
                }
            },
            ".BYTE" => self.data(c, 1)?,
            ".WORD" => self.data(c, 2)?,
            ".LONG" => self.data(c, 4)?,
            ".QUAD" | ".ADDRESS" => self.data(c, 8)?,
            ".ASCII" | ".ASCIZ" | ".ASCIC" | ".ASCID" => {
                let mut s = self.strings(c)?;
                match name {
                    ".ASCIZ" => s.push(0),
                    ".ASCIC" if s.len() > 255 => {
                        return err(col, ".ASCIC strings are at most 255 bytes");
                    }
                    ".ASCIC" => s.insert(0, s.len() as u8),
                    ".ASCID" if s.len() > 65535 => {
                        return err(col, ".ASCID strings are at most 65535 bytes");
                    }
                    _ => {}
                }
                let len = s.len() as u64;
                if name == ".ASCID" {
                    self.item(Item::Ascid(s), 8 + len);
                } else {
                    self.item(Item::Bytes(s), len);
                }
            }
            ".BLKB" | ".BLKW" | ".BLKL" | ".BLKQ" => {
                let unit = match name {
                    ".BLKB" => 1,
                    ".BLKW" => 2,
                    ".BLKL" => 4,
                    _ => 8,
                };
                let n = if c.at_end() {
                    1
                } else {
                    self.constant(c, "the count")?
                };
                if n < 0 {
                    return err(col, "the count is negative");
                }
                self.item(Item::Space, n as u64 * unit);
            }
            ".ALIGN" => {
                c.skip_ws();
                let acol = c.col();
                let save = c.at;
                let align = match c.name().and_then(|w| alignment(&w)) {
                    Some(a) => a,
                    None => {
                        c.at = save;
                        let n = self.constant(c, "the alignment")?;
                        if !(0..=16).contains(&n) {
                            return err(acol, "the alignment must be 0 to 16, a power of two");
                        }
                        n as u8
                    }
                };
                let psect = self.current();
                let p = &mut self.psects[psect];
                p.align = p.align.max(align);
                let pad = p.size.next_multiple_of(1 << align) - p.size;
                self.item(Item::Space, pad);
            }
            ".END" => {
                if !c.at_end() {
                    c.skip_ws();
                    let ecol = c.col();
                    self.transfer = Some((expr::parse(c, self.block)?, self.line, ecol));
                }
                self.ended = true;
            }
            _ => return err(col, format!("unknown directive {name}")),
        }
        end(c)
    }

    /// PSECT attributes after the name: keywords and an alignment.
    fn attributes(&mut self, c: &mut Cursor) -> Result<(u16, u8)> {
        let (mut flags, mut align) = (DEFAULT, DEFAULT_ALIGN);
        loop {
            c.skip_ws();
            let col = c.col();
            let save = c.at;
            match c.name() {
                Some(word) => match alignment(&word) {
                    Some(a) => align = a,
                    None => {
                        let (on, off) = match word.as_str() {
                            "PIC" => (psc::PIC, 0),
                            "NOPIC" => (0, psc::PIC),
                            "OVR" => (psc::OVR, 0),
                            "CON" => (0, psc::OVR),
                            "REL" => (psc::REL, 0),
                            "ABS" => (0, psc::REL),
                            "GBL" => (psc::GBL, 0),
                            "LCL" => (0, psc::GBL),
                            "SHR" => (psc::SHR, 0),
                            "NOSHR" => (0, psc::SHR),
                            "EXE" => (psc::EXE, 0),
                            "NOEXE" => (0, psc::EXE),
                            "RD" => (psc::RD, 0),
                            "NORD" => (0, psc::RD),
                            "WRT" => (psc::WRT, 0),
                            "NOWRT" => (0, psc::WRT),
                            "VEC" => (psc::VEC, 0),
                            "NOVEC" => (0, psc::VEC),
                            "NOMOD" => (psc::NOMOD, 0),
                            "MIX" | "NOMIX" => (0, 0),
                            _ => return err(col, format!("unknown psect attribute {word}")),
                        };
                        flags = (flags | on) & !off;
                    }
                },
                None => {
                    c.at = save;
                    let n = self.constant(c, "the alignment")?;
                    if !(0..=16).contains(&n) {
                        return err(col, "the alignment must be 0 to 16, a power of two");
                    }
                    align = n as u8;
                }
            }
            if !c.eat(',') {
                return Ok((flags, align));
            }
        }
    }

    fn data(&mut self, c: &mut Cursor, size: u8) -> Result<()> {
        let mut exprs = Vec::new();
        loop {
            c.skip_ws();
            let col = c.col();
            exprs.push((expr::parse(c, self.block)?, col));
            if !c.eat(',') {
                break;
            }
        }
        let len = exprs.len() as u64 * u64::from(size);
        self.item(Item::Data { size, exprs }, len);
        Ok(())
    }

    /// MACRO-style strings: text between a delimiter of choice (`/text/`,
    /// `"text"`) and `<expr>` for one byte, in any sequence.
    fn strings(&mut self, c: &mut Cursor) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        while let Some(d) = c.peek() {
            let col = c.col();
            c.at += d.len_utf8();
            if d == '<' {
                let e = expr::parse(c, self.block)?;
                c.expect('>')?;
                let here = self.here();
                match self.eval(&e, col, here)? {
                    Value::Abs(n) if (-128..=255).contains(&n) => out.push(n as u8),
                    _ => return err(col, "expected a byte value"),
                }
            } else {
                let rest = c.rest();
                let Some(len) = rest.find(d) else {
                    return err(col, format!("missing closing {d}"));
                };
                out.extend_from_slice(&rest.as_bytes()[..len]);
                c.at += len + d.len_utf8();
            }
        }
        Ok(out)
    }

    // ----- pass 2 -----

    fn pass2(mut self) -> std::result::Result<Module, Vec<Diagnostic>> {
        for s in std::mem::take(&mut self.stmts) {
            self.line = s.line;
            if let Err(e) = self.encode(&s) {
                self.error(e);
            }
        }
        let mut transfer = None;
        if let Some((e, line, col)) = self.transfer.take() {
            self.line = line;
            match self.eval(&e, col, Value::Abs(0)) {
                Ok(Value::Psect { psect, offset }) => transfer = Some((psect, offset as u64)),
                Ok(_) => self.error(lex::Error {
                    col,
                    msg: "the transfer address must be a label in this module".into(),
                }),
                Err(e) => self.error(e),
            }
        }
        if !self.diags.is_empty() {
            self.diags.sort_by_key(|d| (d.line, d.col));
            return Err(self.diags);
        }
        let mut symbols = std::mem::take(&mut self.symbols);
        let symbols = self
            .order
            .iter()
            .map(|n| (n.clone(), symbols.remove(n).unwrap()))
            .collect();
        Ok(Module {
            name: self.name,
            version: self.version,
            psects: self.psects,
            symbols,
            transfer,
        })
    }

    fn encode(&mut self, s: &Stmt) -> Result<()> {
        let here = Value::Psect {
            psect: s.psect,
            offset: s.offset as i64,
        };
        match &s.item {
            Item::Insn { mn, col, ops } => {
                let view = View {
                    symbols: &self.symbols,
                    here: here.clone(),
                };
                let cx = Cx {
                    scope: &view,
                    here,
                    col: *col,
                };
                let Encoded { word, fixup } = encode::encode(mn, ops, &cx)?;
                match fixup {
                    None => self.bytes(s.psect, s.offset, &word.to_le_bytes()),
                    Some((fix, target)) => {
                        self.chunk(s.psect, s.offset, Chunk::Insn { fix, word, target })
                    }
                }
            }
            Item::Data { size, exprs } => {
                for (i, (e, col)) in exprs.iter().enumerate() {
                    let offset = s.offset + i as u64 * u64::from(*size);
                    let at = Value::Psect {
                        psect: s.psect,
                        offset: offset as i64,
                    };
                    match self.eval(e, *col, at)? {
                        Value::Abs(n) => {
                            let bits = 8 * u32::from(*size);
                            if bits < 64 && (n < -(1 << (bits - 1)) || n >= 1 << bits) {
                                return err(*col, format!("{n} doesn't fit in {size} bytes"));
                            }
                            self.bytes(s.psect, offset, &n.to_le_bytes()[..usize::from(*size)]);
                        }
                        value => self.chunk(s.psect, offset, Chunk::Data { size: *size, value }),
                    }
                }
            }
            Item::Bytes(b) => self.bytes(s.psect, s.offset, b),
            Item::Ascid(text) => {
                // DSC$W_LENGTH, DSC$B_DTYPE (text), DSC$B_CLASS (static),
                // then DSC$A_POINTER, the 32-bit address of the text.
                let mut dsc = (text.len() as u16).to_le_bytes().to_vec();
                dsc.extend([14, 1]);
                self.bytes(s.psect, s.offset, &dsc);
                let text_at = Value::Psect {
                    psect: s.psect,
                    offset: s.offset as i64 + 8,
                };
                self.chunk(
                    s.psect,
                    s.offset + 4,
                    Chunk::Data {
                        size: 4,
                        value: text_at,
                    },
                );
                self.bytes(s.psect, s.offset + 8, text);
            }
            Item::Space => {}
        }
        Ok(())
    }

    fn bytes(&mut self, psect: usize, offset: u64, b: &[u8]) {
        let chunks = &mut self.psects[psect].chunks;
        if let Some((at, Chunk::Bytes(prev))) = chunks.last_mut()
            && *at + prev.len() as u64 == offset
        {
            prev.extend_from_slice(b);
            return;
        }
        chunks.push((offset, Chunk::Bytes(b.to_vec())));
    }

    fn chunk(&mut self, psect: usize, offset: u64, chunk: Chunk) {
        self.psects[psect].chunks.push((offset, chunk));
    }
}

/// A local label's number: digits followed by `$`.
fn local_label(c: &mut Cursor) -> Option<String> {
    let rest = c.rest();
    let n = rest
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(rest.len());
    if n > 0 && rest[n..].starts_with('$') {
        c.at += n + 1;
        Some(rest[..n].to_string())
    } else {
        None
    }
}

fn alignment(word: &str) -> Option<u8> {
    Some(match word {
        "BYTE" => 0,
        "WORD" => 1,
        "LONG" => 2,
        "QUAD" => 3,
        "OCTA" => 4,
        "PAGE" => 16,
        _ => return None,
    })
}

fn end(c: &mut Cursor) -> Result<()> {
    if c.at_end() {
        Ok(())
    } else {
        err(c.col(), "unexpected text")
    }
}
