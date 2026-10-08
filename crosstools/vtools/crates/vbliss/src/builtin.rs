//! Built-in functions (`docs/bliss64.md` *Built-ins*) and ENABLE (LRM
//! chapter 17). Most become expressions of the parser's own; the rest are
//! `Expr::Special`, which irgen turns into IR instructions of their own.
//!
//! ENABLE keeps its enable vector in the routine's frame, the handler's
//! address first, and writes its address at 32(FP) and the module's
//! handler jacket, `BLI$HANDLER`, at 16(FP): the jacket finds the vector
//! through the mechanism array's frame and calls the handler with it as
//! the third argument, as BLISS's `OTS$BLISS_STATIC_HANDLER` does.

use crate::lex::Tok;
use crate::parse::{BOp, Expr, Kind, Parser, R, Rel, Storage, Sym, fold};

/// What irgen makes an instruction of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Special {
    ArgCount,
    /// The argument its operand numbers, from 1.
    ArgN,
    ArgPtr,
    SetHandler,
    SetEnable,
    Barrier,
    /// CH$MOVE(n, src, dst), CH$FILL(fill, n, dst), CH$COPY(n, src, ...,
    /// fill, n, dst): the pointer past what they write.
    ChMove,
    ChFill,
    ChCopy,
    /// CH$COMPARE(n1, p1, n2, p2, fill): -1, 0 or 1.
    ChCompare,
    /// CH$FIND_CH(n, p, c), CH$FIND_NOT_CH and CH$FIND_SUB(n, p, n, p):
    /// a pointer to what they find, or 0.
    ChFind,
    ChFindNot,
    ChFindSub,
}

/// The built-ins vbliss knows, by name.
pub const BUILTINS: &[&str] = &[
    "ACTUALCOUNT",
    "ACTUALPARAMETER",
    "NULLPARAMETER",
    "ARGPTR",
    "MAX",
    "MIN",
    "MAXU",
    "MINU",
    "MAXA",
    "MINA",
    "ABS",
    "SIGN",
    "%REF",
    "ROT",
    "SLL",
    "SRL",
    "SRA",
    "BARRIER",
    "SIGNAL",
    "SIGNAL_STOP",
    "SETUNWIND",
    "ESTABLISH",
    "REVERT",
    "CH$ALLOCATION",
    "CH$SIZE",
    "CH$PTR",
    "CH$PLUS",
    "CH$DIFF",
    "CH$RCHAR",
    "CH$A_RCHAR",
    "CH$RCHAR_A",
    "CH$WCHAR",
    "CH$A_WCHAR",
    "CH$WCHAR_A",
    "CH$MOVE",
    "CH$FILL",
    "CH$COPY",
    "CH$EQL",
    "CH$NEQ",
    "CH$LSS",
    "CH$LEQ",
    "CH$GTR",
    "CH$GEQ",
    "CH$COMPARE",
    "CH$FIND_CH",
    "CH$FIND_NOT_CH",
    "CH$FIND_SUB",
    "CH$FAIL",
];

/// The built-ins a module must declare BUILTIN to use (BLISSA64).
const DECLARED: &[&str] = &[
    "ACTUALCOUNT",
    "ACTUALPARAMETER",
    "NULLPARAMETER",
    "ARGPTR",
    "ROT",
    "SLL",
    "SRL",
    "SRA",
    "BARRIER",
    "ESTABLISH",
    "REVERT",
];

/// A byte at the address `e` gives.
fn byte(e: Expr) -> Expr {
    Expr::Field(
        Box::new(e),
        Box::new(Expr::Num(0)),
        Box::new(Expr::Num(8)),
        Box::new(Expr::Num(0)),
    )
}

/// The fullword at the address `e` gives.
fn word(e: Expr) -> Expr {
    Expr::Field(
        Box::new(e),
        Box::new(Expr::Num(0)),
        Box::new(Expr::Num(64)),
        Box::new(Expr::Num(0)),
    )
}

fn add(a: Expr, b: Expr) -> Expr {
    Expr::Bin(BOp::Add, Box::new(a), Box::new(b))
}

impl Parser<'_> {
    /// A routine the module calls without declaring it: the run-time
    /// library's, for SIGNAL and the like.
    fn library_routine(&mut self, name: &str) -> usize {
        if let Some(id) =
            self.m.syms.iter().position(|s| {
                s.asm == name && matches!(s.kind, Kind::Routine { external: true, .. })
            })
        {
            return id;
        }
        self.m.syms.push(Sym {
            name: name.into(),
            asm: name.into(),
            kind: Kind::Routine {
                global: false,
                external: true,
                novalue: false,
                linkage: None,
            },
        });
        self.m.syms.len() - 1
    }

    /// The actuals of a built-in, `(e, ...)`.
    fn builtin_args(&mut self) -> R<Vec<Expr>> {
        self.expect_punct('(')?;
        let mut args = Vec::new();
        if !self.eat_punct(')') {
            loop {
                args.push(self.value()?);
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
        }
        Ok(args)
    }

    /// A temporary for a value used more than once.
    fn temp(&mut self) -> u32 {
        self.lets += 1;
        self.lets - 1
    }

    /// The call of built-in `name`, after its name.
    pub(crate) fn builtin(&mut self, name: &str) -> R<Expr> {
        let mut args = self.builtin_args()?;
        let n = args.len();
        let want = |p: &mut Self, k: usize| -> R<()> {
            if n == k {
                Ok(())
            } else {
                p.err(format!("{name} takes {k} parameters, not {n}"))
            }
        };
        let special = |s: Special, args: Vec<Expr>| Expr::Special(s, args);
        Ok(match name {
            "ACTUALCOUNT" => {
                want(self, 0)?;
                special(Special::ArgCount, args)
            }
            "ACTUALPARAMETER" => {
                want(self, 1)?;
                special(Special::ArgN, args)
            }
            "NULLPARAMETER" => {
                want(self, 1)?;
                // Past the count, or 0.
                let t = self.temp();
                let i = Expr::Temp(t);
                let count = special(Special::ArgCount, Vec::new());
                let past = Expr::Bin(
                    BOp::Rel(Rel::Gtr, false),
                    Box::new(i.clone()),
                    Box::new(count),
                );
                let zero = Expr::Bin(
                    BOp::Rel(Rel::Eql, false),
                    Box::new(special(Special::ArgN, vec![i])),
                    Box::new(Expr::Num(0)),
                );
                let test = Expr::Bin(BOp::Or, Box::new(past), Box::new(zero));
                Expr::Let(t, Box::new(args.remove(0)), Box::new(test))
            }
            "ARGPTR" => {
                want(self, 0)?;
                special(Special::ArgPtr, args)
            }
            "BARRIER" => {
                want(self, 0)?;
                special(Special::Barrier, args)
            }
            "MAX" | "MIN" | "MAXU" | "MINU" | "MAXA" | "MINA" => {
                if n == 0 {
                    return self.err(format!("{name} takes at least one parameter"));
                }
                let rel = if name.starts_with("MAX") {
                    Rel::Gtr
                } else {
                    Rel::Lss
                };
                let unsigned = name.len() == 4;
                let mut args = args.into_iter();
                let mut e = args.next().unwrap();
                for b in args {
                    let (ta, tb) = (self.temp(), self.temp());
                    let test = Expr::Bin(
                        BOp::Rel(rel, unsigned),
                        Box::new(Expr::Temp(ta)),
                        Box::new(Expr::Temp(tb)),
                    );
                    let pick = Expr::If(
                        Box::new(test),
                        Box::new(Expr::Temp(ta)),
                        Some(Box::new(Expr::Temp(tb))),
                    );
                    e = Expr::Let(
                        ta,
                        Box::new(e),
                        Box::new(Expr::Let(tb, Box::new(b), Box::new(pick))),
                    );
                }
                e
            }
            "ABS" | "SIGN" => {
                want(self, 1)?;
                let t = self.temp();
                let x = || Box::new(Expr::Temp(t));
                let lss = Expr::Bin(BOp::Rel(Rel::Lss, false), x(), Box::new(Expr::Num(0)));
                let body = if name == "ABS" {
                    Expr::If(Box::new(lss), Box::new(Expr::Neg(x())), Some(x()))
                } else {
                    let gtr = Expr::Bin(BOp::Rel(Rel::Gtr, false), x(), Box::new(Expr::Num(0)));
                    Expr::If(
                        Box::new(gtr),
                        Box::new(Expr::Num(1)),
                        Some(Box::new(Expr::If(
                            Box::new(lss),
                            Box::new(Expr::Num(-1)),
                            Some(Box::new(Expr::Num(0))),
                        ))),
                    )
                };
                Expr::Let(t, Box::new(args.remove(0)), Box::new(body))
            }
            "ROT" | "SLL" | "SRL" | "SRA" => {
                want(self, 2)?;
                let (v, k) = (args.remove(0), args.remove(0));
                let op = match name {
                    "SLL" => crate::ir::Op::Shl,
                    "SRL" => crate::ir::Op::Shr,
                    "SRA" => crate::ir::Op::Sar,
                    _ => crate::ir::Op::Ror,
                };
                // ROT rotates left: right by 64 - k.
                let k = if name == "ROT" {
                    Expr::Bin(
                        BOp::And,
                        Box::new(Expr::Bin(BOp::Sub, Box::new(Expr::Num(64)), Box::new(k))),
                        Box::new(Expr::Num(63)),
                    )
                } else {
                    Expr::Bin(BOp::And, Box::new(k), Box::new(Expr::Num(63)))
                };
                let k = fold(&k).map_or(k, Expr::Num);
                Expr::Op(op, Box::new(v), Box::new(k))
            }
            "%REF" => {
                want(self, 1)?;
                let h = self.hidden(8)?;
                Expr::Block(
                    vec![
                        Expr::Assign(Box::new(Expr::Name(h)), Box::new(args.remove(0))),
                        Expr::Name(h),
                    ],
                    true,
                )
            }
            "SIGNAL" | "SIGNAL_STOP" => {
                if n == 0 {
                    return self.err(format!("{name} takes a condition value"));
                }
                let r = self.library_routine(if name == "SIGNAL" {
                    "LIB$SIGNAL"
                } else {
                    "LIB$STOP"
                });
                Expr::Call(Box::new(Expr::Name(r)), args)
            }
            "SETUNWIND" => {
                want(self, 0)?;
                let r = self.library_routine("SYS$UNWIND");
                Expr::Call(Box::new(Expr::Name(r)), vec![Expr::Num(0), Expr::Num(0)])
            }
            "ESTABLISH" => {
                want(self, 1)?;
                special(Special::SetHandler, args)
            }
            "REVERT" => {
                want(self, 0)?;
                special(Special::SetHandler, vec![Expr::Num(0)])
            }
            "CH$ALLOCATION" => {
                if !(1..=2).contains(&n) {
                    return self.err("CH$ALLOCATION takes a length and a character size");
                }
                self.char_size(args.get(1))?;
                // Fullwords for n bytes.
                let n = args.remove(0);
                Expr::Bin(
                    BOp::Div,
                    Box::new(add(n, Expr::Num(7))),
                    Box::new(Expr::Num(8)),
                )
            }
            "CH$SIZE" => {
                if n > 1 {
                    return self.err("CH$SIZE takes a pointer");
                }
                Expr::Num(8)
            }
            "CH$PTR" => {
                if !(1..=3).contains(&n) {
                    return self.err("CH$PTR takes an address, an index and a character size");
                }
                self.char_size(args.get(2))?;
                args.truncate(2);
                let addr = args.remove(0);
                match args.pop() {
                    Some(i) => add(addr, i),
                    None => addr,
                }
            }
            "CH$PLUS" => {
                want(self, 2)?;
                add(args.remove(0), args.remove(0))
            }
            "CH$DIFF" => {
                want(self, 2)?;
                Expr::Bin(BOp::Sub, Box::new(args.remove(0)), Box::new(args.remove(0)))
            }
            "CH$FAIL" => {
                want(self, 1)?;
                Expr::Bin(
                    BOp::Rel(Rel::Eql, false),
                    Box::new(args.remove(0)),
                    Box::new(Expr::Num(0)),
                )
            }
            "CH$RCHAR" => {
                want(self, 1)?;
                Expr::Fetch(Box::new(byte(args.remove(0))))
            }
            "CH$WCHAR" => {
                want(self, 2)?;
                let c = args.remove(0);
                Expr::Block(
                    vec![Expr::Assign(Box::new(byte(args.remove(0))), Box::new(c))],
                    false,
                )
            }
            "CH$A_RCHAR" | "CH$RCHAR_A" | "CH$A_WCHAR" | "CH$WCHAR_A" => {
                let write = name.contains("WCHAR");
                want(self, if write { 2 } else { 1 })?;
                let c = if write { Some(args.remove(0)) } else { None };
                // The pointer's address, then the pointer, before or after.
                let (ta, tp) = (self.temp(), self.temp());
                let before = name.starts_with("CH$A_");
                let ptr = word(Expr::Temp(ta));
                let next = add(Expr::Fetch(Box::new(ptr.clone())), Expr::Num(1));
                let at = if before {
                    add(Expr::Temp(tp), Expr::Num(1))
                } else {
                    Expr::Temp(tp)
                };
                let step = Expr::Assign(Box::new(ptr.clone()), Box::new(next));
                let body = match c {
                    Some(c) => Expr::Block(
                        vec![Expr::Assign(Box::new(byte(at)), Box::new(c)), step],
                        false,
                    ),
                    None => {
                        let tc = self.temp();
                        Expr::Let(
                            tc,
                            Box::new(Expr::Fetch(Box::new(byte(at)))),
                            Box::new(Expr::Block(vec![step, Expr::Temp(tc)], true)),
                        )
                    }
                };
                Expr::Let(
                    ta,
                    Box::new(args.remove(0)),
                    Box::new(Expr::Let(
                        tp,
                        Box::new(Expr::Fetch(Box::new(ptr))),
                        Box::new(body),
                    )),
                )
            }
            "CH$MOVE" => {
                want(self, 3)?;
                special(Special::ChMove, args)
            }
            "CH$FILL" => {
                want(self, 3)?;
                special(Special::ChFill, args)
            }
            "CH$COPY" => {
                if n < 3 || n % 2 == 0 {
                    return self
                        .err("CH$COPY takes lengths and pointers, a fill, a length and a pointer");
                }
                special(Special::ChCopy, args)
            }
            "CH$EQL" | "CH$NEQ" | "CH$LSS" | "CH$LEQ" | "CH$GTR" | "CH$GEQ" | "CH$COMPARE" => {
                if !(4..=5).contains(&n) {
                    return self.err(format!("{name} takes two lengths and pointers and a fill"));
                }
                if n == 4 {
                    args.push(Expr::Num(0));
                }
                let cmp = special(Special::ChCompare, args);
                let rel = match name {
                    "CH$EQL" => Rel::Eql,
                    "CH$NEQ" => Rel::Neq,
                    "CH$LSS" => Rel::Lss,
                    "CH$LEQ" => Rel::Leq,
                    "CH$GTR" => Rel::Gtr,
                    "CH$GEQ" => Rel::Geq,
                    _ => return Ok(cmp),
                };
                Expr::Bin(BOp::Rel(rel, false), Box::new(cmp), Box::new(Expr::Num(0)))
            }
            "CH$FIND_CH" | "CH$FIND_NOT_CH" => {
                want(self, 3)?;
                let s = if name == "CH$FIND_CH" {
                    Special::ChFind
                } else {
                    Special::ChFindNot
                };
                special(s, args)
            }
            "CH$FIND_SUB" => {
                want(self, 4)?;
                special(Special::ChFindSub, args)
            }
            _ => return self.err(format!("{name} is not supported yet")),
        })
    }

    /// A CH$ function's character size, which must be 8.
    fn char_size(&mut self, cs: Option<&Expr>) -> R<()> {
        match cs.map(fold) {
            None | Some(Some(8)) => Ok(()),
            _ => self.err("the character size must be 8"),
        }
    }

    /// A local of `bytes` the source doesn't name.
    pub(crate) fn hidden(&mut self, bytes: u32) -> R<usize> {
        if !self.slots_open() {
            return self.err("this needs a routine's frame");
        }
        let slot = self.slot(bytes);
        self.m.syms.push(Sym {
            name: String::new(),
            asm: String::new(),
            kind: Kind::Data {
                storage: Storage::Local(slot),
                bytes,
                size: 8,
                signed: false,
                structure: None,
            },
        });
        Ok(self.m.syms.len() - 1)
    }

    /// `ENABLE handler (actuals)`, after the word: the enable vector built
    /// and the jacket made the handler as the block starts.
    pub(crate) fn enable(&mut self) -> R<()> {
        let name = self.name()?;
        let handler = match self.lookup(&name) {
            Some(id) if matches!(self.m.syms[id].kind, Kind::Routine { .. }) => id,
            _ => return self.err(format!("ENABLE of {name}, which isn't a routine")),
        };
        let mut actuals = Vec::new();
        if self.eat_punct('(') {
            loop {
                let at = self.here();
                let a = self.name()?;
                match self.lookup(&a) {
                    Some(id) if matches!(self.m.syms[id].kind, Kind::Data { .. }) => {
                        if !self.volatile.contains(&id) {
                            let msg = format!(
                                "Name used as ENABLE actual parameter must be VOLATILE:  {a}"
                            );
                            self.diag('W', &at, msg);
                        }
                        actuals.push(id)
                    }
                    _ => return self.err(format!("enable actual {a} isn't data")),
                }
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
        }
        let ev = self.hidden(8 * (2 + actuals.len() as u32))?;
        let at = |k: usize| {
            Box::new(Expr::Field(
                Box::new(Expr::Bin(
                    BOp::Add,
                    Box::new(Expr::Name(ev)),
                    Box::new(Expr::Num(8 * k as i64)),
                )),
                Box::new(Expr::Num(0)),
                Box::new(Expr::Num(64)),
                Box::new(Expr::Num(0)),
            ))
        };
        // LOCAL enable actuals start as zeros.
        for &a in &actuals {
            if let Kind::Data {
                storage: Storage::Local(_),
                bytes,
                ..
            } = self.m.syms[a].kind
            {
                for off in (0..bytes).step_by(8) {
                    let n = (bytes - off).min(8);
                    let place = Expr::Field(
                        Box::new(Expr::Bin(
                            BOp::Add,
                            Box::new(Expr::Name(a)),
                            Box::new(Expr::Num(off.into())),
                        )),
                        Box::new(Expr::Num(0)),
                        Box::new(Expr::Num(8 * i64::from(n))),
                        Box::new(Expr::Num(0)),
                    );
                    self.inits
                        .push(Expr::Assign(Box::new(place), Box::new(Expr::Num(0))));
                }
            }
        }
        self.inits
            .push(Expr::Assign(at(0), Box::new(Expr::Name(handler))));
        self.inits.push(Expr::Assign(
            at(1),
            Box::new(Expr::Num(actuals.len() as i64)),
        ));
        for (i, &a) in actuals.iter().enumerate() {
            self.inits
                .push(Expr::Assign(at(2 + i), Box::new(Expr::Name(a))));
        }
        self.inits
            .push(Expr::Special(Special::SetEnable, vec![Expr::Name(ev)]));
        self.inits
            .push(Expr::Special(Special::SetHandler, vec![Expr::Jacket]));
        self.m.jacket = true;
        Ok(())
    }

    /// Whether `name` is a predeclared built-in, when no declaration
    /// hides it; the others must be declared BUILTIN, as in BLISSA64.
    pub(crate) fn is_builtin(&self, name: &str) -> bool {
        BUILTINS.contains(&name) && !DECLARED.contains(&name) && self.lookup(name).is_none()
    }

    /// `BUILTIN` names, after the word: built-ins vbliss knows.
    pub(crate) fn builtins(&mut self) -> R<()> {
        loop {
            let at = self.here();
            let name = match self.next() {
                Tok::Name(n) => n,
                _ => return self.err("expected a built-in's name"),
            };
            if !BUILTINS.contains(&name.as_str()) {
                return Err(self.error_at(&at, format!("{name} is not a built-in vbliss knows")));
            }
            self.declare(name, Kind::Builtin)?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }
}
