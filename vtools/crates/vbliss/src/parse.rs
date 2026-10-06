//! The parser: lexemes into routines and data, with names resolved as they
//! are declared, since BLISS declares everything before it is used.

use std::collections::HashMap;

use crate::lex::{Lexeme, Tok};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Storage {
    Own,
    Global,
    External,
    /// In the routine's frame slot n.
    Local(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// A data segment: where it is, how many bytes it takes, and the field
    /// a fetch or a store of its name uses (size in bytes, signed).
    Data {
        storage: Storage,
        bytes: u32,
        size: u8,
        signed: bool,
        /// `VECTOR[n, unit, ext]`: the unit in bytes, and signed.
        vector: Option<(u8, bool)>,
    },
    /// A routine; `defined` once its body is seen, `global` if exported.
    Routine {
        global: bool,
        external: bool,
        novalue: bool,
    },
    Literal(i64),
    Label,
}

#[derive(Clone, Debug)]
pub struct Sym {
    pub name: String,
    /// The name in the assembly: the BLISS name, made unique for OWN.
    pub asm: String,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shift,
    And,
    Or,
    Xor,
    Eqv,
    /// A comparison: its relation, and `unsigned` for the U and A forms.
    Rel(Rel, bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rel {
    Eql,
    Neq,
    Lss,
    Leq,
    Gtr,
    Geq,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseLabel {
    Range(i64, i64),
    Inrange,
    Outrange,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectLabel {
    Range(Expr, Option<Expr>),
    Otherwise,
    Always,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(i64),
    /// A data segment's or a routine's name: its address.
    Name(usize),
    /// `%ASCID 'text'`: the address of a descriptor of the text.
    Ascid(Vec<u8>),
    Fetch(Box<Expr>),
    /// `base<pos, size, ext>`; ext is a constant.
    Field(Box<Expr>, Box<Expr>, Box<Expr>, bool),
    /// `vector[index]`, on a VECTOR's name.
    Index(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BOp, Box<Expr>, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    /// A block's expressions; `value` unless it ends with a semicolon.
    Block(Vec<Expr>, bool),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// WHILE/UNTIL c DO b (`post` false), DO b WHILE/UNTIL c (`post`).
    Loop {
        until: bool,
        post: bool,
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    /// INCR and DECR, `unsigned` for the U and A forms.
    Incr {
        var: usize,
        down: bool,
        unsigned: bool,
        from: Option<Box<Expr>>,
        to: Option<Box<Expr>>,
        by: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Case {
        sel: Box<Expr>,
        lo: i64,
        hi: i64,
        arms: Vec<(Vec<CaseLabel>, Expr)>,
    },
    Select {
        sel: Box<Expr>,
        one: bool,
        unsigned: bool,
        arms: Vec<(Vec<SelectLabel>, Expr)>,
    },
    Labeled(usize, Box<Expr>),
    Leave(usize, Option<Box<Expr>>),
    Exitloop(Option<Box<Expr>>),
    Return(Option<Box<Expr>>),
}

pub struct Routine {
    pub sym: usize,
    pub formals: Vec<usize>,
    pub body: Expr,
    /// The sizes of its frame slots.
    pub slots: Vec<u32>,
}

/// Static data: an OWN or GLOBAL, and its INITIAL values, each with its
/// size in bytes.
pub struct Static {
    pub sym: usize,
    pub init: Vec<(Expr, u8)>,
}

#[derive(Default)]
pub struct Module {
    pub name: String,
    pub ident: Option<String>,
    pub main: Option<String>,
    pub syms: Vec<Sym>,
    pub routines: Vec<Routine>,
    pub statics: Vec<Static>,
}

pub type Error = (u32, String);
type R<T> = Result<T, Error>;

/// Reads a require file by the name REQUIRE gives.
pub type Loader<'a> = &'a dyn Fn(&str) -> Result<String, String>;

struct Parser<'a> {
    load: Loader<'a>,
    toks: Vec<Lexeme>,
    pos: usize,
    scopes: Vec<HashMap<String, usize>>,
    m: Module,
    /// Frame slots of the routine being parsed.
    slots: Vec<u32>,
    /// Names the assembly already uses.
    asm_names: HashMap<String, u32>,
}

/// Parses a module.
pub fn parse(toks: Vec<Lexeme>, load: Loader) -> R<Module> {
    let mut p = Parser {
        load,
        toks,
        pos: 0,
        scopes: vec![HashMap::new()],
        m: Module::default(),
        slots: Vec::new(),
        asm_names: HashMap::new(),
    };
    p.module()?;
    Ok(p.m)
}

fn is_name(t: &Tok, n: &str) -> bool {
    matches!(t, Tok::Name(s) if s == n)
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
    }

    fn line(&self) -> u32 {
        self.toks[self.pos].line
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if t != Tok::Eof {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: impl Into<String>) -> R<T> {
        Err((self.line(), msg.into()))
    }

    fn at(&self, n: &str) -> bool {
        is_name(self.peek(), n)
    }

    fn at_punct(&self, c: char) -> bool {
        *self.peek() == Tok::Punct(c)
    }

    fn eat(&mut self, n: &str) -> bool {
        let yes = self.at(n);
        if yes {
            self.pos += 1;
        }
        yes
    }

    fn eat_punct(&mut self, c: char) -> bool {
        let yes = self.at_punct(c);
        if yes {
            self.pos += 1;
        }
        yes
    }

    fn expect(&mut self, n: &str) -> R<()> {
        if self.eat(n) {
            Ok(())
        } else {
            self.err(format!("expected {n}, found {}", self.describe()))
        }
    }

    fn expect_punct(&mut self, c: char) -> R<()> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            self.err(format!("expected {c}, found {}", self.describe()))
        }
    }

    fn describe(&self) -> String {
        match self.peek() {
            Tok::Name(n) => n.clone(),
            Tok::Num(n) => n.to_string(),
            Tok::Str(s) => format!("'{}'", String::from_utf8_lossy(s)),
            Tok::Punct(c) => c.to_string(),
            Tok::Eof => "the end of the file".into(),
        }
    }

    fn name(&mut self) -> R<String> {
        match self.next() {
            Tok::Name(n) if !RESERVED.contains(&n.as_str()) => Ok(n),
            _ => {
                self.pos -= 1;
                self.err(format!("expected a name, found {}", self.describe()))
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    /// Declares `name` in the innermost scope; a forward routine
    /// declaration is completed rather than redeclared.
    fn declare(&mut self, name: String, kind: Kind) -> R<usize> {
        if let Some(&id) = self.scopes.last().unwrap().get(&name) {
            let forward = matches!(self.m.syms[id].kind, Kind::Routine { external: true, .. })
                && matches!(
                    kind,
                    Kind::Routine {
                        external: false,
                        ..
                    }
                );
            if !forward {
                return self.err(format!("{name} is declared twice"));
            }
            self.m.syms[id].kind = kind;
            return Ok(id);
        }
        // Statics and routines are named in the assembly; locals aren't.
        let asm = match kind {
            Kind::Data {
                storage: Storage::Own,
                ..
            } => {
                let n = self.asm_names.entry(name.clone()).or_insert(0);
                *n += 1;
                if *n == 1 {
                    name.clone()
                } else {
                    format!("{name}.{}", *n - 1)
                }
            }
            _ => {
                self.asm_names.insert(name.clone(), 1);
                name.clone()
            }
        };
        let id = self.m.syms.len();
        self.m.syms.push(Sym {
            name: name.clone(),
            asm,
            kind,
        });
        self.scopes.last_mut().unwrap().insert(name, id);
        Ok(id)
    }

    fn slot(&mut self, bytes: u32) -> u32 {
        self.slots.push(bytes);
        self.slots.len() as u32 - 1
    }

    // Modules and declarations.

    fn module(&mut self) -> R<()> {
        self.expect("MODULE")?;
        self.m.name = self.name()?;
        if self.eat_punct('(') {
            loop {
                self.switch()?;
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
        }
        self.expect_punct('=')?;
        let paren = self.eat_punct('(');
        if !paren {
            self.expect("BEGIN")?;
        }
        self.declarations()?;
        if paren {
            self.expect_punct(')')?;
        } else {
            self.expect("END")?;
        }
        self.expect("ELUDOM")?;
        if *self.peek() != Tok::Eof {
            return self.err("text after ELUDOM");
        }
        if let Some(main) = &self.m.main {
            let main = main.clone();
            match self.lookup(&main).map(|id| &self.m.syms[id].kind) {
                Some(Kind::Routine {
                    external: false, ..
                }) => {}
                _ => return self.err(format!("MAIN routine {main} is not defined")),
            }
        }
        Ok(())
    }

    /// A module switch: MAIN and IDENT matter; the rest is accepted.
    fn switch(&mut self) -> R<()> {
        let name = self.name().or_else(|_| match self.next() {
            Tok::Name(n) => Ok(n),
            _ => self.err("expected a module switch"),
        })?;
        if self.eat_punct('=') {
            match (name.as_str(), self.next()) {
                ("MAIN", Tok::Name(n)) => self.m.main = Some(n),
                ("IDENT", Tok::Str(s)) => {
                    self.m.ident = Some(String::from_utf8_lossy(&s).into_owned())
                }
                (_, Tok::Name(_) | Tok::Str(_) | Tok::Num(_)) => {}
                _ => return self.err(format!("bad value for {name}")),
            }
        } else if self.at_punct('(') {
            let mut depth = 0;
            loop {
                match self.next() {
                    Tok::Punct('(') => depth += 1,
                    Tok::Punct(')') => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Tok::Eof => return self.err("unbalanced parentheses"),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Declarations, each ending with a semicolon, as long as there are.
    fn declarations(&mut self) -> R<()> {
        loop {
            if self.eat("REQUIRE") {
                self.require()?;
            } else if self.declaration()? {
                self.expect_punct(';')?;
            } else {
                return Ok(());
            }
        }
    }

    /// `REQUIRE 'file';`: the file's lexemes take the declaration's place.
    // ponytail: messages about a require file's text give the line in it
    // without its name, until the listing (step 5) tracks files.
    fn require(&mut self) -> R<()> {
        let Tok::Str(name) = self.next() else {
            return self.err("expected a file name after REQUIRE");
        };
        self.expect_punct(';')?;
        let name = String::from_utf8_lossy(&name).into_owned();
        let text = (self.load)(&name).map_err(|e| (self.line(), e))?;
        let mut toks = crate::lex::lex(&text)?;
        toks.pop(); // its Eof
        self.toks.splice(self.pos..self.pos, toks);
        Ok(())
    }

    /// One declaration, if one is next.
    fn declaration(&mut self) -> R<bool> {
        let Tok::Name(word) = self.peek().clone() else {
            return Ok(false);
        };
        let global = word == "GLOBAL";
        let external = word == "EXTERNAL";
        let forward = word == "FORWARD";
        if global || external || forward {
            match self.peek2().clone() {
                Tok::Name(n) if n == "ROUTINE" => {
                    self.pos += 2;
                    return if global {
                        self.routine(true).map(|_| true)
                    } else {
                        self.routine_names().map(|_| true)
                    };
                }
                Tok::Name(n) if n == "LITERAL" => {
                    self.pos += 2;
                    if external {
                        return self.err("EXTERNAL LITERAL is not supported yet");
                    }
                    return self.literals(global).map(|_| true);
                }
                _ if forward => return self.err("expected ROUTINE after FORWARD"),
                _ => {}
            }
        }
        match word.as_str() {
            "OWN" | "GLOBAL" | "EXTERNAL" | "LOCAL" | "STACKLOCAL" => {
                self.pos += 1;
                let storage = match word.as_str() {
                    "OWN" => Storage::Own,
                    "GLOBAL" => Storage::Global,
                    "EXTERNAL" => Storage::External,
                    _ => Storage::Local(0),
                };
                self.data(storage)?;
            }
            "ROUTINE" => {
                self.pos += 1;
                self.routine(false)?;
            }
            "LITERAL" => {
                self.pos += 1;
                self.literals(false)?;
            }
            "LABEL" => {
                self.pos += 1;
                loop {
                    let name = self.name()?;
                    self.declare(name, Kind::Label)?;
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            "REGISTER" | "BIND" | "MACRO" | "KEYWORDMACRO" | "LINKAGE" | "STRUCTURE" | "FIELD"
            | "PSECT" | "SWITCHES" | "LIBRARY" | "BUILTIN" | "UNDECLARE" | "ENABLE" => {
                return self.err(format!("{word} declarations are not supported yet"));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn data(&mut self, storage: Storage) -> R<()> {
        loop {
            let name = self.name()?;
            let (mut size, mut signed, mut init) = (8u8, false, Vec::new());
            let mut vector = None;
            if self.eat_punct(':') {
                while let Tok::Name(attr) = self.peek().clone() {
                    match attr.as_str() {
                        "BYTE" | "WORD" | "LONG" | "QUAD" => size = unit(&attr),
                        "SIGNED" => signed = true,
                        "UNSIGNED" => signed = false,
                        "VOLATILE" | "ALIAS" => {}
                        "VECTOR" => {
                            self.pos += 1;
                            vector = Some(self.vector()?);
                            continue;
                        }
                        "ALIGN" => {
                            self.pos += 1;
                            self.expect_punct('(')?;
                            self.ctce()?;
                            self.expect_punct(')')?;
                            continue;
                        }
                        "INITIAL" => {
                            if matches!(storage, Storage::Local(_) | Storage::External) {
                                return self
                                    .err("INITIAL on a LOCAL or EXTERNAL is not supported yet");
                            }
                            self.pos += 1;
                            init = self.initial(size)?;
                            continue;
                        }
                        _ => break,
                    }
                    self.pos += 1;
                }
                if !self.at_punct(',') && !self.at_punct(';') {
                    return self.err(format!(
                        "attribute {} is not supported yet",
                        self.describe()
                    ));
                }
            }
            let bytes = match vector {
                Some((n, unit, _)) => n * u32::from(unit),
                None => size.into(),
            };
            let storage = match storage {
                Storage::Local(_) => Storage::Local(self.slot(bytes)),
                s => s,
            };
            let id = self.declare(
                name,
                Kind::Data {
                    storage,
                    bytes,
                    size,
                    signed,
                    vector: vector.map(|(_, unit, ext)| (unit, ext)),
                },
            )?;
            if matches!(storage, Storage::Own | Storage::Global) {
                self.m.statics.push(Static { sym: id, init });
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `VECTOR[n, unit, ext]`, after VECTOR: n, and the unit in bytes,
    /// default QUAD, and SIGNED or UNSIGNED, default UNSIGNED.
    // ponytail: VECTOR alone, until step 6's STRUCTURE declarations
    // predeclare it like the others.
    fn vector(&mut self) -> R<(u32, u8, bool)> {
        self.expect_punct('[')?;
        let n = self.ctce()?;
        let (mut unit, mut signed) = (8, false);
        while self.eat_punct(',') {
            match self.next() {
                Tok::Name(u) if matches!(u.as_str(), "BYTE" | "WORD" | "LONG" | "QUAD") => {
                    unit = self::unit(&u)
                }
                Tok::Name(u) if u == "SIGNED" => signed = true,
                Tok::Name(u) if u == "UNSIGNED" => signed = false,
                _ => return self.err("expected a unit or an extension in VECTOR[]"),
            }
        }
        self.expect_punct(']')?;
        if !(0..=1 << 24).contains(&n) {
            return self.err(format!("VECTOR[{n}] too large"));
        }
        Ok((n as u32, unit, signed))
    }

    /// INITIAL(values): each of the unit of the data, or of its own.
    fn initial(&mut self, size: u8) -> R<Vec<(Expr, u8)>> {
        self.expect_punct('(')?;
        let mut items = Vec::new();
        loop {
            let size = match self.peek() {
                Tok::Name(n) if matches!(n.as_str(), "BYTE" | "WORD" | "LONG" | "QUAD") => {
                    let s = unit(&n.clone());
                    self.pos += 1;
                    self.expect_punct('(')?;
                    let e = self.expr()?;
                    self.expect_punct(')')?;
                    items.push((e, s));
                    None
                }
                _ => Some(size),
            };
            if let Some(size) = size {
                items.push((self.expr()?, size));
            }
            if !self.eat_punct(',') {
                break;
            }
        }
        self.expect_punct(')')?;
        Ok(items)
    }

    fn literals(&mut self, global: bool) -> R<()> {
        if global {
            return self.err("GLOBAL LITERAL is not supported yet");
        }
        loop {
            let name = self.name()?;
            self.expect_punct('=')?;
            let v = self.ctce()?;
            if self.eat_punct(':') {
                while matches!(self.peek(), Tok::Name(n) if matches!(n.as_str(),
                    "BYTE" | "WORD" | "LONG" | "QUAD" | "SIGNED" | "UNSIGNED"))
                {
                    self.pos += 1;
                }
            }
            self.declare(name, Kind::Literal(v))?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// EXTERNAL ROUTINE and FORWARD ROUTINE: names with attributes.
    fn routine_names(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            let novalue = self.routine_attributes()?;
            self.declare(
                name,
                Kind::Routine {
                    global: false,
                    external: true,
                    novalue,
                },
            )?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `: NOVALUE` and the like; returns whether NOVALUE.
    fn routine_attributes(&mut self) -> R<bool> {
        let mut novalue = false;
        if self.eat_punct(':') {
            loop {
                if self.eat("NOVALUE") {
                    novalue = true;
                } else if !(self.eat("WEAK") || self.eat("VARIABLE")) {
                    break;
                }
            }
        }
        Ok(novalue)
    }

    fn routine(&mut self, global: bool) -> R<()> {
        let name = self.name()?;
        let mut names = Vec::new();
        if self.eat_punct('(') {
            loop {
                names.push(self.name()?);
                if self.eat_punct(':') {
                    return self.err("formal attributes are not supported yet");
                }
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
        }
        let novalue = self.routine_attributes()?;
        let sym = self.declare(
            name,
            Kind::Routine {
                global,
                external: false,
                novalue,
            },
        )?;
        if let Some(Kind::Routine { global: g, .. }) = self.m.syms.get_mut(sym).map(|s| &mut s.kind)
        {
            *g = global;
        }
        self.expect_punct('=')?;
        let outer = std::mem::take(&mut self.slots);
        self.scopes.push(HashMap::new());
        let mut formals = Vec::new();
        for n in names {
            let slot = self.slot(8);
            formals.push(self.declare(
                n,
                Kind::Data {
                    storage: Storage::Local(slot),
                    bytes: 8,
                    size: 8,
                    signed: false,
                    vector: None,
                },
            )?);
        }
        let body = self.expr()?;
        self.scopes.pop();
        let slots = std::mem::replace(&mut self.slots, outer);
        self.m.routines.push(Routine {
            sym,
            formals,
            body,
            slots,
        });
        Ok(())
    }

    // Expressions, from the loosest operator to the tightest.

    pub fn expr(&mut self) -> R<Expr> {
        let left = self.binary(0)?;
        if self.eat_punct('=') {
            let right = self.expr()?;
            return Ok(Expr::Assign(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    /// Binary operators of `level` and tighter: EQV and XOR, OR, AND, NOT,
    /// the relations, + and -, * / MOD, ^.
    fn binary(&mut self, level: u8) -> R<Expr> {
        if level == 3 {
            if self.eat("NOT") {
                return Ok(Expr::Not(Box::new(self.binary(3)?)));
            }
            return self.binary(4);
        }
        if level == 8 {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        while let Some(op) = self.operator(level) {
            self.pos += 1;
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn operator(&self, level: u8) -> Option<BOp> {
        let t = self.peek();
        let op = match t {
            Tok::Punct('+') => BOp::Add,
            Tok::Punct('-') => BOp::Sub,
            Tok::Punct('*') => BOp::Mul,
            Tok::Punct('/') => BOp::Div,
            Tok::Punct('^') => BOp::Shift,
            Tok::Name(n) => match n.as_str() {
                "MOD" => BOp::Mod,
                "AND" => BOp::And,
                "OR" => BOp::Or,
                "XOR" => BOp::Xor,
                "EQV" => BOp::Eqv,
                n => {
                    let (rel, rest) = n.split_at(n.len().min(3));
                    let rel = match rel {
                        "EQL" => Rel::Eql,
                        "NEQ" => Rel::Neq,
                        "LSS" => Rel::Lss,
                        "LEQ" => Rel::Leq,
                        "GTR" => Rel::Gtr,
                        "GEQ" => Rel::Geq,
                        _ => return None,
                    };
                    match rest {
                        "" => BOp::Rel(rel, false),
                        "U" | "A" => BOp::Rel(rel, true),
                        _ => return None,
                    }
                }
            },
            _ => return None,
        };
        let op_level = match op {
            BOp::Eqv | BOp::Xor => 0,
            BOp::Or => 1,
            BOp::And => 2,
            BOp::Rel(..) => 4,
            BOp::Add | BOp::Sub => 5,
            BOp::Mul | BOp::Div | BOp::Mod => 6,
            BOp::Shift => 7,
        };
        (op_level == level).then_some(op)
    }

    /// Fetch and the signs, then a primary with its calls and fields.
    fn unary(&mut self) -> R<Expr> {
        if self.eat_punct('.') {
            return Ok(Expr::Fetch(Box::new(self.unary()?)));
        }
        if self.eat_punct('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_punct('+') {
            return self.unary();
        }
        let mut e = self.primary()?;
        loop {
            if self.eat_punct('(') {
                let mut args = Vec::new();
                if !self.eat_punct(')') {
                    loop {
                        args.push(self.expr()?);
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                    self.expect_punct(')')?;
                }
                e = Expr::Call(Box::new(e), args);
            } else if self.eat_punct('<') {
                let pos = self.expr()?;
                self.expect_punct(',')?;
                let size = self.expr()?;
                let ext = if self.eat_punct(',') {
                    self.ctce()? != 0
                } else {
                    false
                };
                self.expect_punct('>')?;
                e = Expr::Field(Box::new(e), Box::new(pos), Box::new(size), ext);
            } else if self.at_punct('[') {
                let vector = match e {
                    Expr::Name(id) => matches!(
                        self.m.syms[id].kind,
                        Kind::Data {
                            vector: Some(_),
                            ..
                        }
                    ),
                    _ => false,
                };
                if !vector {
                    return self.err("structure references are only supported on VECTOR names yet");
                }
                self.pos += 1;
                let index = self.expr()?;
                self.expect_punct(']')?;
                e = Expr::Index(Box::new(e), Box::new(index));
            } else {
                return Ok(e);
            }
        }
    }

    fn primary(&mut self) -> R<Expr> {
        let line = self.line();
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => string_value(&s)
                .map(Expr::Num)
                .ok_or((line, "string too long for a value".into())),
            Tok::Punct('(') => self.block(')'),
            Tok::Name(n) => match n.as_str() {
                "BEGIN" => self.block('E'),
                "IF" => {
                    let c = self.expr()?;
                    self.expect("THEN")?;
                    let t = self.expr()?;
                    let e = if self.eat("ELSE") {
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    Ok(Expr::If(Box::new(c), Box::new(t), e))
                }
                "WHILE" | "UNTIL" => {
                    let cond = self.expr()?;
                    self.expect("DO")?;
                    let body = self.expr()?;
                    Ok(Expr::Loop {
                        until: n == "UNTIL",
                        post: false,
                        cond: Box::new(cond),
                        body: Box::new(body),
                    })
                }
                "DO" => {
                    let body = self.expr()?;
                    let until = if self.eat("UNTIL") {
                        true
                    } else {
                        self.expect("WHILE")?;
                        false
                    };
                    let cond = self.expr()?;
                    Ok(Expr::Loop {
                        until,
                        post: true,
                        cond: Box::new(cond),
                        body: Box::new(body),
                    })
                }
                "INCR" | "INCRU" | "INCRA" | "DECR" | "DECRU" | "DECRA" => self.incr(&n),
                "CASE" => self.case(),
                "SELECT" | "SELECTU" | "SELECTA" | "SELECTONE" | "SELECTONEU" | "SELECTONEA" => {
                    self.select(&n)
                }
                "LEAVE" => {
                    let name = self.name()?;
                    let label = match self.lookup(&name) {
                        Some(id) if self.m.syms[id].kind == Kind::Label => id,
                        _ => return self.err(format!("{name} is not a label")),
                    };
                    let v = if self.eat("WITH") {
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    Ok(Expr::Leave(label, v))
                }
                "EXITLOOP" => Ok(Expr::Exitloop(self.optional_value()?)),
                "RETURN" => Ok(Expr::Return(self.optional_value()?)),
                "%ASCID" => match self.next() {
                    Tok::Str(s) => Ok(Expr::Ascid(s)),
                    _ => self.err("expected a string after %ASCID"),
                },
                "%ASCII" => match self.next() {
                    Tok::Str(s) => string_value(&s)
                        .map(Expr::Num)
                        .ok_or((line, "string too long for a value".into())),
                    _ => self.err("expected a string after %ASCII"),
                },
                _ if RESERVED.contains(&n.as_str()) => {
                    Err((line, format!("{n} can't start an expression")))
                }
                _ if n.starts_with('%') => Err((line, format!("{n} is not supported yet"))),
                _ => {
                    let id = self
                        .lookup(&n)
                        .ok_or((line, format!("{n} is not declared")))?;
                    match self.m.syms[id].kind.clone() {
                        Kind::Literal(v) => Ok(Expr::Num(v)),
                        Kind::Label if self.eat_punct(':') => {
                            Ok(Expr::Labeled(id, Box::new(self.expr()?)))
                        }
                        Kind::Label => Err((line, format!("label {n} used as a value"))),
                        _ => Ok(Expr::Name(id)),
                    }
                }
            },
            _ => Err((
                line,
                format!("expected an expression, found {}", {
                    self.pos -= 1;
                    self.describe()
                }),
            )),
        }
    }

    /// An expression, unless the next lexeme ends one.
    fn optional_value(&mut self) -> R<Option<Box<Expr>>> {
        let ends = matches!(self.peek(), Tok::Punct(';' | ')' | ',') | Tok::Eof)
            || ["END", "ELSE", "TES", "THEN", "DO", "WHILE", "UNTIL"]
                .iter()
                .any(|w| self.at(w));
        Ok(if ends {
            None
        } else {
            Some(Box::new(self.expr()?))
        })
    }

    /// A block, after its BEGIN or `(`: declarations, then expressions
    /// separated by semicolons, up to END (`close` 'E') or `)`.
    fn block(&mut self, close: char) -> R<Expr> {
        self.scopes.push(HashMap::new());
        self.declarations()?;
        let mut exprs = Vec::new();
        let mut value = false;
        let at_close = |p: &Self| {
            if close == 'E' {
                p.at("END")
            } else {
                p.at_punct(')')
            }
        };
        while !at_close(self) {
            exprs.push(self.expr()?);
            value = true;
            if !self.eat_punct(';') {
                break;
            }
            value = false;
        }
        if close == 'E' {
            self.expect("END")?;
        } else {
            self.expect_punct(')')?;
        }
        self.scopes.pop();
        Ok(Expr::Block(exprs, value))
    }

    fn incr(&mut self, word: &str) -> R<Expr> {
        let name = self.name()?;
        self.scopes.push(HashMap::new());
        let slot = self.slot(8);
        let var = self.declare(
            name,
            Kind::Data {
                storage: Storage::Local(slot),
                bytes: 8,
                size: 8,
                signed: false,
                vector: None,
            },
        )?;
        let part = |p: &mut Self, w: &str| -> R<Option<Box<Expr>>> {
            Ok(if p.eat(w) {
                Some(Box::new(p.expr()?))
            } else {
                None
            })
        };
        let from = part(self, "FROM")?;
        let to = part(self, "TO")?;
        let by = part(self, "BY")?;
        self.expect("DO")?;
        let body = Box::new(self.expr()?);
        self.scopes.pop();
        Ok(Expr::Incr {
            var,
            down: word.starts_with("DECR"),
            unsigned: word.len() == 5,
            from,
            to,
            by,
            body,
        })
    }

    fn case(&mut self) -> R<Expr> {
        let sel = self.expr()?;
        self.expect("FROM")?;
        let lo = self.ctce()?;
        self.expect("TO")?;
        let hi = self.ctce()?;
        self.expect("OF")?;
        self.expect("SET")?;
        let mut arms = Vec::new();
        while self.eat_punct('[') {
            let mut labels = Vec::new();
            loop {
                labels.push(if self.eat("INRANGE") {
                    CaseLabel::Inrange
                } else if self.eat("OUTRANGE") {
                    CaseLabel::Outrange
                } else {
                    let a = self.ctce()?;
                    let b = if self.eat("TO") { self.ctce()? } else { a };
                    if a < lo || b > hi || a > b {
                        return self.err(format!("case label {a} TO {b} outside {lo} TO {hi}"));
                    }
                    CaseLabel::Range(a, b)
                });
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(']')?;
            self.expect_punct(':')?;
            arms.push((labels, self.expr()?));
            if !self.eat_punct(';') {
                break;
            }
        }
        self.expect("TES")?;
        Ok(Expr::Case {
            sel: Box::new(sel),
            lo,
            hi,
            arms,
        })
    }

    fn select(&mut self, word: &str) -> R<Expr> {
        let sel = self.expr()?;
        self.expect("OF")?;
        self.expect("SET")?;
        let mut arms = Vec::new();
        while self.eat_punct('[') {
            let mut labels = Vec::new();
            loop {
                labels.push(if self.eat("OTHERWISE") {
                    SelectLabel::Otherwise
                } else if self.eat("ALWAYS") {
                    SelectLabel::Always
                } else {
                    let a = self.expr()?;
                    let b = if self.eat("TO") {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    SelectLabel::Range(a, b)
                });
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(']')?;
            self.expect_punct(':')?;
            arms.push((labels, self.expr()?));
            if !self.eat_punct(';') {
                break;
            }
        }
        self.expect("TES")?;
        Ok(Expr::Select {
            sel: Box::new(sel),
            one: word.starts_with("SELECTONE"),
            unsigned: word.ends_with('U') || word.ends_with('A'),
            arms,
        })
    }

    /// A compile-time constant expression.
    fn ctce(&mut self) -> R<i64> {
        let line = self.line();
        let e = self.expr()?;
        fold(&e).ok_or((line, "expected a compile-time constant".into()))
    }
}

/// The size in bytes of an allocation unit.
fn unit(name: &str) -> u8 {
    match name {
        "BYTE" => 1,
        "WORD" => 2,
        "LONG" => 4,
        _ => 8,
    }
}

/// A quoted string as a value: its characters from the low byte up, at
/// most a fullword's worth.
fn string_value(s: &[u8]) -> Option<i64> {
    (s.len() <= 8).then(|| s.iter().rev().fold(0i64, |v, &c| (v << 8) | i64::from(c)))
}

/// The value of a constant expression, as the fullword arithmetic gives
/// it, or None.
pub fn fold(e: &Expr) -> Option<i64> {
    Some(match e {
        Expr::Num(n) => *n,
        Expr::Neg(a) => fold(a)?.wrapping_neg(),
        Expr::Not(a) => !fold(a)?,
        Expr::Block(es, true) if es.len() == 1 => fold(&es[0])?,
        Expr::Bin(op, a, b) => binop(*op, fold(a)?, fold(b)?)?,
        _ => return None,
    })
}

/// A binary operation on constants; None for a division by zero.
pub fn binop(op: BOp, a: i64, b: i64) -> Option<i64> {
    Some(match op {
        BOp::Add => a.wrapping_add(b),
        BOp::Sub => a.wrapping_sub(b),
        BOp::Mul => a.wrapping_mul(b),
        BOp::Div => a.checked_div(b).or((b == -1).then(|| a.wrapping_neg()))?,
        BOp::Mod => a.checked_rem(b).or((b == -1).then_some(0))?,
        BOp::Shift => shift(a, b),
        BOp::And => a & b,
        BOp::Or => a | b,
        BOp::Xor => a ^ b,
        BOp::Eqv => !(a ^ b),
        BOp::Rel(rel, unsigned) => {
            let ord = if unsigned {
                (a as u64).cmp(&(b as u64))
            } else {
                a.cmp(&b)
            };
            let yes = match rel {
                Rel::Eql => ord.is_eq(),
                Rel::Neq => ord.is_ne(),
                Rel::Lss => ord.is_lt(),
                Rel::Leq => ord.is_le(),
                Rel::Gtr => ord.is_gt(),
                Rel::Geq => ord.is_ge(),
            };
            yes as i64
        }
    })
}

/// `a ^ n`: left for a positive n, right (arithmetic) for a negative one;
/// 0 or the sign past 63, which BLISS leaves undefined.
pub fn shift(a: i64, n: i64) -> i64 {
    match n {
        0..=63 => a << n,
        -63..=-1 => a >> -n,
        n if n > 0 => 0,
        _ => a >> 63,
    }
}

/// Words that can't name anything.
const RESERVED: &[&str] = &[
    "ALWAYS",
    "AND",
    "BEGIN",
    "BY",
    "CASE",
    "DECR",
    "DECRA",
    "DECRU",
    "DO",
    "ELSE",
    "ELUDOM",
    "END",
    "EQL",
    "EQLA",
    "EQLU",
    "EQV",
    "EXITLOOP",
    "EXTERNAL",
    "FORWARD",
    "FROM",
    "GEQ",
    "GEQA",
    "GEQU",
    "GLOBAL",
    "GTR",
    "GTRA",
    "GTRU",
    "IF",
    "INCR",
    "INCRA",
    "INCRU",
    "INRANGE",
    "LABEL",
    "LEAVE",
    "LEQ",
    "LEQA",
    "LEQU",
    "LITERAL",
    "LOCAL",
    "LSS",
    "LSSA",
    "LSSU",
    "MOD",
    "MODULE",
    "NEQ",
    "NEQA",
    "NEQU",
    "NOT",
    "OF",
    "OR",
    "OTHERWISE",
    "OUTRANGE",
    "OWN",
    "RETURN",
    "ROUTINE",
    "SELECT",
    "SELECTA",
    "SELECTONE",
    "SELECTONEA",
    "SELECTONEU",
    "SELECTU",
    "SET",
    "TES",
    "THEN",
    "TO",
    "UNTIL",
    "WHILE",
    "WITH",
    "XOR",
];
