//! Two passes over the source. Pass 1 reads lines, from the file and from
//! macro expansions, repeat blocks and macro libraries, parses them and lays
//! out the psects. Pass 2 evaluates operands, encodes instructions and
//! collects each psect's contents and relocations, which `emit` turns into
//! records.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use vms_obj::obj::psc;

use crate::encode::{self, Cx, Encoded, Fix, Operand};
use crate::expr::{self, Expr, Scope, Value};
use crate::lex::{self, Cursor, Result, err};
use crate::macros::{self, Line, Loc, Macro};

/// An error: where (file, line and column from 1), the line's text, and
/// the macro calls and repeat blocks it is inside of, innermost first.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub msg: String,
    pub text: String,
    pub context: Vec<String>,
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
    /// Defined by a label, so it can't be redefined.
    label: bool,
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
    /// An assignment, done again in pass 2 so later lines see its value.
    Assign {
        name: String,
        expr: Expr,
        col: usize,
        here: Value,
    },
}

struct Stmt {
    src: Line,
    psect: usize,
    offset: u64,
    item: Item,
}

/// Where lines come from: the source, a macro expansion, a repeat block or
/// a macro library.
struct Frame {
    lines: Vec<Line>,
    pos: usize,
    /// Conditionals open when the frame started; the frame must close its own.
    conds: usize,
    /// For a macro expansion, the number of positional arguments.
    narg: Option<usize>,
    /// A macro library, which may only define macros.
    library: bool,
}

/// An open `.IF`: its condition, whether the enclosing code is being
/// assembled, and which part (after `.IF_TRUE`, `.IF_FALSE`...) we are in.
struct Cond {
    value: bool,
    outer: bool,
    part: Part,
}

#[derive(Clone, Copy)]
enum Part {
    True,
    False,
    Both,
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
/// Limits that stop runaway macros.
const MAX_DEPTH: usize = 100;
const MAX_REPEAT: i64 = 65536;

/// Assembles `source`, read from `path`; `.LIBRARY` files are looked up
/// next to it, then in `include`. Returns every error found.
pub fn assemble(
    source: &str,
    path: Option<&Path>,
    include: &[PathBuf],
) -> std::result::Result<Module, Vec<Diagnostic>> {
    let file: Rc<str> = path.map_or_else(|| "<source>".into(), |p| p.display().to_string().into());
    let mut a = Asm {
        next_label: 30000,
        include: include.to_vec(),
        ..Asm::default()
    };
    a.frames.push(Frame {
        lines: macros::lines(&file, source),
        pos: 0,
        conds: 0,
        narg: None,
        library: false,
    });
    while let Some(line) = a.next_line() {
        a.src = line;
        let text = a.src.text.clone();
        if let Err(e) = a.process(&text) {
            a.error(e);
        }
        if a.ended {
            break;
        }
    }
    if !a.conds.is_empty() {
        a.error(lex::Error {
            col: 1,
            msg: "missing .ENDC".into(),
        });
    }
    // A global or weak symbol this module doesn't define is a reference.
    for s in a.symbols.values_mut() {
        if s.value.is_none() && (s.global || s.weak) {
            s.external = true;
        }
    }
    let big: Vec<String> = a
        .psects
        .iter()
        .filter(|p| p.size > u64::from(u32::MAX))
        .map(|p| p.name.clone())
        .collect();
    for name in big {
        a.error(lex::Error {
            col: 1,
            msg: format!("psect {name} is larger than 4 GB"),
        });
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
    /// The local label block, and how many blocks there have been.
    block: u32,
    blocks: u32,
    name: Option<String>,
    version: Option<String>,
    transfer: Option<(Expr, Line, usize)>,
    ended: bool,
    /// Errors, with the order of the line they belong to.
    diags: Vec<(usize, Diagnostic)>,
    /// The line being assembled, and how many lines were read.
    src: Line,
    seq: usize,
    frames: Vec<Frame>,
    conds: Vec<Cond>,
    macros: HashMap<String, Rc<Macro>>,
    /// The next created local label, from 30000$ as in MACRO.
    next_label: u32,
    /// `.SAVE_PSECT`: psect, and local label block if saved.
    saved: Vec<(Option<usize>, Option<u32>)>,
    include: Vec<PathBuf>,
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
        let loc = &self.src.loc;
        let d = Diagnostic {
            file: loc.file.to_string(),
            line: loc.line,
            col: e.col,
            msg: e.msg,
            text: self.src.text.clone(),
            context: loc.context(),
        };
        self.diags.push((self.seq, d));
    }

    fn next_line(&mut self) -> Option<Line> {
        loop {
            let frame = self.frames.last_mut()?;
            if let Some(line) = frame.lines.get(frame.pos).cloned() {
                frame.pos += 1;
                self.seq += 1;
                return Some(line);
            }
            let frame = self.frames.pop()?;
            if self.conds.len() > frame.conds {
                self.conds.truncate(frame.conds);
                self.error(lex::Error {
                    col: 1,
                    msg: "missing .ENDC before the end of the block".into(),
                });
            }
        }
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

    /// The location counter, without opening a psect.
    fn loc(&self) -> Value {
        match self.cur {
            Some(psect) => Value::Psect {
                psect,
                offset: self.psects[psect].size as i64,
            },
            None => Value::Abs(0),
        }
    }

    /// A constant, which must be known in pass 1.
    fn constant(&mut self, c: &mut Cursor, what: &str) -> Result<i64> {
        c.skip_ws();
        let col = c.col();
        let e = expr::parse(c, self.block)?;
        match self.eval(&e, col, self.loc())? {
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

    fn new_block(&mut self) {
        self.blocks += 1;
        self.block = self.blocks;
    }

    fn define(&mut self, name: &str, value: Value, col: usize, label: bool) -> Result<()> {
        let sym = self.symbol(name);
        if sym.value.is_some() && (sym.label || label) {
            return err(col, format!("{} is already defined", expr::display(name)));
        }
        if sym.external {
            return err(col, format!("{} is declared external", expr::display(name)));
        }
        sym.value = Some(value);
        sym.label = label;
        Ok(())
    }

    fn item(&mut self, item: Item, size: u64) {
        let psect = self.current();
        let p = &mut self.psects[psect];
        self.stmts.push(Stmt {
            src: self.src.clone(),
            psect,
            offset: p.size,
            item,
        });
        p.size += size;
    }

    /// One line: conditionals first, as they apply even while skipping.
    fn process(&mut self, raw: &str) -> Result<()> {
        let text = lex::strip_comment(raw);
        let mut c = Cursor::new(text);
        if let Some(word) = c.name()
            && self.conditional(&word, &mut c)?
        {
            return Ok(());
        }
        if self.active() {
            self.statement(text)
        } else {
            Ok(())
        }
    }

    fn active(&self) -> bool {
        self.conds.last().is_none_or(|c| {
            c.outer
                && match c.part {
                    Part::True => c.value,
                    Part::False => !c.value,
                    Part::Both => true,
                }
        })
    }

    /// `.IF` and friends. Returns whether `word` was one of them.
    fn conditional(&mut self, word: &str, c: &mut Cursor) -> Result<bool> {
        let col = c.col();
        let part = match word {
            ".IF" => {
                let outer = self.active();
                let value = outer && self.condition(c)?;
                if outer {
                    end(c)?;
                }
                self.conds.push(Cond {
                    value,
                    outer,
                    part: Part::True,
                });
                return Ok(true);
            }
            ".ENDC" => {
                let floor = self.frames.last().map_or(0, |f| f.conds);
                if self.conds.len() <= floor {
                    return err(col, ".ENDC without .IF");
                }
                self.conds.pop();
                return Ok(true);
            }
            ".IF_FALSE" | ".IFF" | ".ELSE" => Part::False,
            ".IF_TRUE" | ".IFT" => Part::True,
            ".IF_TRUE_FALSE" | ".IFTF" => Part::Both,
            _ => return Ok(false),
        };
        match self.conds.last_mut() {
            Some(cond) => cond.part = part,
            None => return err(col, format!("{word} outside .IF")),
        }
        Ok(true)
    }

    /// A condition and its arguments, as in `.IF` and `.IIF`.
    fn condition(&mut self, c: &mut Cursor) -> Result<bool> {
        c.skip_ws();
        let col = c.col();
        let Some(kind) = c.name() else {
            return err(col, "expected a condition such as EQ, DF or B");
        };
        c.eat(',');
        let k = kind.as_str();
        Ok(match k {
            "EQ" | "EQUAL" => self.constant(c, "the value")? == 0,
            "NE" | "NOT_EQUAL" => self.constant(c, "the value")? != 0,
            "GT" | "GREATER" => self.constant(c, "the value")? > 0,
            "LT" | "LESS_THAN" => self.constant(c, "the value")? < 0,
            "GE" | "GREATER_EQUAL" => self.constant(c, "the value")? >= 0,
            "LE" | "LESS_EQUAL" => self.constant(c, "the value")? <= 0,
            "DF" | "DEFINED" | "NDF" | "NOT_DEFINED" => {
                c.skip_ws();
                let ncol = c.col();
                let Some(name) = c.name() else {
                    return err(ncol, "expected a symbol name");
                };
                let defined = self.symbols.get(&name).is_some_and(|s| s.value.is_some());
                defined == matches!(k, "DF" | "DEFINED")
            }
            "B" | "BLANK" | "NB" | "NOT_BLANK" => {
                macros::arg(c).trim().is_empty() == matches!(k, "B" | "BLANK")
            }
            "IDN" | "IDENTICAL" | "DIF" | "DIFFERENT" => {
                let a = macros::arg(c);
                c.expect(',')?;
                let b = macros::arg(c);
                a.trim().eq_ignore_ascii_case(b.trim()) == matches!(k, "IDN" | "IDENTICAL")
            }
            _ => return err(col, format!("unknown condition {kind}")),
        })
    }

    fn statement(&mut self, text: &str) -> Result<()> {
        let mut c = Cursor::new(text);
        // Labels: NAME: local to the module, NAME:: global, 10$: a local label.
        loop {
            let save = c.at;
            c.skip_ws();
            let col = c.col();
            let label = match local_label(&mut c) {
                Some(n) => Some(expr::local_name(&n, self.block)),
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
        if self.frames.last().is_some_and(|f| f.library) && word != ".MACRO" {
            return err(col, "a macro library can only define macros");
        }
        if c.eat('=') {
            // NAME = value, or NAME == value to make it global.
            let global = c.eat('=');
            c.skip_ws();
            let vcol = c.col();
            let e = expr::parse(&mut c, self.block)?;
            end(&mut c)?;
            return self.assign(&word, e, global, vcol);
        }
        match word.as_str() {
            ".MACRO" => return self.define_macro(&mut c, col),
            ".IRP" | ".IRPC" | ".REPEAT" | ".REPT" => return self.repeat(&word, &mut c, col),
            ".ENDM" | ".ENDR" => return err(col, format!("{word} without a block to end")),
            ".MEXIT" => {
                end(&mut c)?;
                let Some(i) = self.frames.iter().rposition(|f| f.narg.is_some()) else {
                    return err(col, ".MEXIT outside a macro");
                };
                self.conds.truncate(self.frames[i].conds);
                self.frames.truncate(i);
                return Ok(());
            }
            ".NARG" => {
                let Some(n) = self.frames.iter().rev().find_map(|f| f.narg) else {
                    return err(col, ".NARG outside a macro");
                };
                c.skip_ws();
                let ncol = c.col();
                let Some(name) = c.name() else {
                    return err(ncol, "expected a symbol name");
                };
                end(&mut c)?;
                return self.assign(&name, Expr::Num(n as i64), false, ncol);
            }
            ".IIF" => {
                let yes = self.condition(&mut c)?;
                c.expect(',')?;
                return if yes {
                    self.statement(c.rest())
                } else {
                    Ok(())
                };
            }
            ".LIBRARY" => return self.library(&mut c, col),
            _ => {}
        }
        if let Some(m) = self.macros.get(&word).cloned() {
            return self.invoke(&m, c.rest(), col);
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
        self.define(name, here, col, true)?;
        self.symbol(name).global |= global;
        if !local {
            self.new_block();
        }
        Ok(())
    }

    /// NAME = value: assigned now, and again in pass 2, so an assignment
    /// can be redefined and each use sees the value current at that point.
    fn assign(&mut self, name: &str, expr: Expr, global: bool, col: usize) -> Result<()> {
        let here = self.loc();
        let value = self.eval(&expr, col, here.clone())?;
        self.define(name, value, col, false)?;
        self.symbol(name).global |= global;
        let item = Item::Assign {
            name: name.to_string(),
            expr,
            col,
            here,
        };
        self.stmts.push(Stmt {
            src: self.src.clone(),
            psect: 0,
            offset: 0,
            item,
        });
        Ok(())
    }

    // ----- macros -----

    fn define_macro(&mut self, c: &mut Cursor, col: usize) -> Result<()> {
        c.skip_ws();
        let ncol = c.col();
        let Some(name) = c.name() else {
            return err(ncol, "expected a macro name");
        };
        c.eat(',');
        let params = macros::params(c.rest()).or_else(|msg| err(ncol, msg))?;
        let (body, rest) = self.collect(&[".MACRO"], ".ENDM", &format!("macro {name}"), col)?;
        let end_name = rest.trim();
        if !end_name.is_empty() && !end_name.eq_ignore_ascii_case(&name) {
            return err(col, format!("macro {name} ends with .ENDM {end_name}"));
        }
        // The source's own definition wins over a library's.
        if self.frames.last().is_some_and(|f| f.library) && self.macros.contains_key(&name) {
            return Ok(());
        }
        self.macros
            .insert(name.clone(), Rc::new(Macro { name, params, body }));
        Ok(())
    }

    /// The lines up to the `close` that matches, from the current frame.
    /// Also returns what follows `close` on its line.
    fn collect(
        &mut self,
        open: &[&str],
        close: &str,
        what: &str,
        col: usize,
    ) -> Result<(Vec<Line>, String)> {
        let frame = self.frames.last_mut().expect("lines come from a frame");
        let mut depth = 0;
        let mut body = Vec::new();
        while let Some(line) = frame.lines.get(frame.pos).cloned() {
            frame.pos += 1;
            self.seq += 1;
            let (word, rest) = first_word(&line.text);
            if open.contains(&word.as_str()) {
                depth += 1;
            } else if word == close {
                if depth == 0 {
                    return Ok((body, rest));
                }
                depth -= 1;
            }
            body.push(line);
        }
        err(col, format!("missing {close} for {what}"))
    }

    fn invoke(&mut self, m: &Macro, args: &str, col: usize) -> Result<()> {
        let args = if args.trim().is_empty() {
            Vec::new()
        } else {
            macros::split(args)
        };
        let (values, narg) =
            macros::bind(m, &args, &mut self.next_label).or_else(|msg| err(col, msg))?;
        let via = Rc::new((format!("macro {}", m.name), self.src.loc.clone()));
        let lines = expand(&m.body, &values, &via);
        self.push(lines, Some(narg), col)
    }

    /// `.REPEAT n`, `.IRP sym, <a, b>` and `.IRPC sym, <chars>`.
    fn repeat(&mut self, word: &str, c: &mut Cursor, col: usize) -> Result<()> {
        let copies: Vec<HashMap<String, String>> = match word {
            ".REPEAT" | ".REPT" => {
                let n = self.constant(c, "the count")?;
                if !(0..=MAX_REPEAT).contains(&n) {
                    return err(col, format!("the count must be 0 to {MAX_REPEAT}"));
                }
                end(c)?;
                vec![HashMap::new(); n as usize]
            }
            _ => {
                c.skip_ws();
                let scol = c.col();
                let Some(sym) = c.name() else {
                    return err(scol, "expected a symbol name");
                };
                c.expect(',')?;
                let list = macros::arg(c);
                end(c)?;
                let values: Vec<String> = if word == ".IRP" {
                    macros::split(&list)
                } else {
                    list.chars().map(String::from).collect()
                };
                values
                    .into_iter()
                    .map(|v| HashMap::from([(sym.clone(), v)]))
                    .collect()
            }
        };
        let (body, _) = self.collect(&[".IRP", ".IRPC", ".REPEAT", ".REPT"], ".ENDR", word, col)?;
        let via = Rc::new((word.to_string(), self.src.loc.clone()));
        let lines = copies
            .iter()
            .flat_map(|values| expand(&body, values, &via))
            .collect();
        self.push(lines, None, col)
    }

    fn push(&mut self, lines: Vec<Line>, narg: Option<usize>, col: usize) -> Result<()> {
        if self.frames.len() >= MAX_DEPTH {
            return err(
                col,
                format!("macros and repeat blocks nested more than {MAX_DEPTH} deep"),
            );
        }
        self.frames.push(Frame {
            lines,
            pos: 0,
            conds: self.conds.len(),
            narg,
            library: false,
        });
        Ok(())
    }

    /// `.LIBRARY "file"`: macro definitions from a file, found next to the
    /// current one or in an include directory.
    fn library(&mut self, c: &mut Cursor, col: usize) -> Result<()> {
        let spec = String::from_utf8_lossy(&self.strings(c)?).into_owned();
        end(c)?;
        let here = Path::new(&*self.src.loc.file)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let found = std::iter::once(here)
            .chain(self.include.iter().cloned())
            .map(|d| d.join(&spec))
            .find(|p| p.is_file());
        let Some(path) = found else {
            return err(col, format!("macro library {spec} not found"));
        };
        let text = std::fs::read_to_string(&path)
            .or_else(|e| err(col, format!("{}: {e}", path.display())))?;
        let file: Rc<str> = path.display().to_string().into();
        self.push(macros::lines(&file, &text), None, col)?;
        self.frames.last_mut().unwrap().library = true;
        Ok(())
    }

    // ----- directives -----

    /// Switches to psect `name`, creating it with `attrs` or its defaults.
    fn open(&mut self, name: String, attrs: Option<(u16, u8)>, col: usize) -> Result<()> {
        if name.len() > 31 {
            return err(col, "psect names are at most 31 characters");
        }
        self.new_block();
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
            ".SAVE_PSECT" => {
                let local = c.name().is_some_and(|w| w == "LOCAL_BLOCK");
                self.saved.push((self.cur, local.then_some(self.block)));
            }
            ".RESTORE_PSECT" => {
                let Some((psect, block)) = self.saved.pop() else {
                    return err(col, ".RESTORE_PSECT without .SAVE_PSECT");
                };
                self.cur = psect;
                match block {
                    Some(b) => self.block = b,
                    None => self.new_block(),
                }
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
                    self.transfer = Some((expr::parse(c, self.block)?, self.src.clone(), ecol));
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
                match self.eval(&e, col, self.loc())? {
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
            self.src = s.src.clone();
            if let Err(e) = self.encode(&s) {
                self.error(e);
            }
        }
        let mut transfer = None;
        if let Some((e, src, col)) = self.transfer.take() {
            self.src = src;
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
            self.diags.sort_by_key(|(seq, d)| (*seq, d.col));
            return Err(self.diags.into_iter().map(|(_, d)| d).collect());
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
            Item::Assign {
                name,
                expr,
                col,
                here,
            } => {
                let value = self.eval(expr, *col, here.clone())?;
                self.symbols
                    .get_mut(name)
                    .expect("assigned in pass 1")
                    .value = Some(value);
            }
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

/// A macro or repeat body with its arguments substituted.
fn expand(body: &[Line], values: &HashMap<String, String>, via: &Rc<(String, Loc)>) -> Vec<Line> {
    body.iter()
        .map(|l| Line {
            text: macros::substitute(&l.text, values),
            loc: Loc {
                via: Some(via.clone()),
                ..l.loc.clone()
            },
        })
        .collect()
}

/// A line's first word after its labels, in upper case, and the rest.
fn first_word(text: &str) -> (String, String) {
    let mut c = Cursor::new(lex::strip_comment(text));
    loop {
        let save = c.at;
        c.skip_ws();
        let label = local_label(&mut c).is_some() || c.name().is_some();
        if label && c.rest().starts_with(':') {
            c.at += 1 + c.rest()[1..].starts_with(':') as usize;
        } else {
            c.at = save;
            break;
        }
    }
    let word = c.name().unwrap_or_default();
    (word, c.rest().to_string())
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
