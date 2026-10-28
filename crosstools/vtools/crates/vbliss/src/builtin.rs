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
    /// CH$TRANSLATE(tab, sn, sp, fill, dn, dp): the pointer past dp's n.
    ChTranslate,
    /// `<op>_ATOMIC_<size>(p, e [, retries] [; old])`: 1 if done.
    Atomic {
        op: crate::ir::Op,
        size: u8,
        retries: bool,
        old: bool,
    },
    /// CMP_SWAP_x(addr, comparand, value): 1 if it stored value.
    CmpSwap(u8),
    /// TESTBITxx(field [, retries] [; done]): whether the bit was set
    /// (`set_was`) or clear, then set or cleared (`set`).
    TestBit {
        was_set: bool,
        set: bool,
        retries: bool,
        flag: bool,
    },
    /// ADAWI(srcaddr, dstaddr), as BLISS-32's (UM 4.1): adds the word at
    /// srcaddr to the aligned word at dstaddr; the VAX condition codes of
    /// the sum.
    Adawi,
    /// PAL_INSQUEx(pred, entry): 1 if the entry is now the only one.
    Insque {
        quad: bool,
        deferred: bool,
    },
    /// PAL_REMQUEx(entry [; addr]): -1 if the queue was empty, 0 if it is
    /// now, else 1; the entry's address stored in the output, R1 on Alpha.
    Remque {
        quad: bool,
        deferred: bool,
        addr: bool,
    },
    /// A PAL call with this function code; for CHMx, the first operand
    /// is the change mode code.
    Pal(u32),
    /// A queue call that removes an entry, R1, the entry, stored in its
    /// last operand, an output parameter.
    PalOut(u32),
    Chm(u32),
    Bpt,
    RetAddr,
    Rpcc,
    /// CODECOMMENT's text, the module's comment n: a comment in the code.
    Comment(u32),
    /// The floating point built-ins (RN 2.10.1): ADDx and the like, CMPx,
    /// and CVT[R]xy.
    FArith(crate::ir::FOp, crate::ir::Fl),
    FCmp(crate::ir::Fl),
    FCvt(crate::ir::Fl, crate::ir::Fl, bool),
    /// `%FFLOAT(a)` to `%TFLOAT(a)` as a call's actual (RN 2.10.4): the
    /// floating value at a, passed by value; as an output actual, where
    /// the routine's floating result is stored. Only in a call's actuals.
    FloatIn(crate::ir::Fl),
    FloatOut(crate::ir::Fl),
    /// a + b, a - b or a * b under the OVERFLOW switch: an overflow
    /// signals SS$_INTOVF, and the value is the wrapped one.
    Checked(crate::ir::Op),
    /// `BUILTIN R0` or `R1`: the register itself, x0 or x1, which may only
    /// be fetched or assigned (LRM 10.7.4).
    Reg(u8),
}

/// The register a register built-in names, R0 to R11.
pub fn register(name: &str) -> Option<u8> {
    let n: u8 = name.strip_prefix('R')?.parse().ok()?;
    (n <= 11 && name.len() == if n < 10 { 2 } else { 3 }).then_some(n)
}

/// The format `%FFLOAT` to `%TFLOAT` name.
pub fn float_actual(name: &str) -> Option<crate::ir::Fl> {
    use crate::ir::Fl;
    Some(match name {
        "%FFLOAT" => Fl::F,
        "%DFLOAT" => Fl::D,
        "%GFLOAT" => Fl::G,
        "%SFLOAT" => Fl::S,
        "%TFLOAT" => Fl::T,
        _ => return None,
    })
}

/// A floating point built-in by its name, if `name` is one; `i` is the
/// fullword integer's: L under BLISS-32, Q under BLISS-64.
pub fn float_builtin(name: &str, i: crate::ir::Fl) -> Option<Special> {
    use crate::ir::{FOp, Fl};
    let fl = |c: u8| match c {
        b'F' => Some(Fl::F),
        b'D' => Some(Fl::D),
        b'G' => Some(Fl::G),
        b'S' => Some(Fl::S),
        b'T' => Some(Fl::T),
        b'L' => Some(Fl::L),
        b'Q' => Some(Fl::Q),
        b'I' => Some(i),
        _ => None,
    };
    let is_float = |f: Fl| !matches!(f, Fl::L | Fl::Q);
    let n = name.as_bytes();
    if n.len() == 4 {
        let op = match &name[..3] {
            "ADD" => Some(FOp::Add),
            "SUB" => Some(FOp::Sub),
            "MUL" => Some(FOp::Mul),
            "DIV" => Some(FOp::Div),
            _ => None,
        };
        let f = fl(n[3]).filter(|&f| is_float(f) && n[3] != b'I')?;
        return match (op, &name[..3]) {
            (Some(op), _) => Some(Special::FArith(op, f)),
            (None, "CMP") => Some(Special::FCmp(f)),
            _ => None,
        };
    }
    let rest = name.strip_prefix("CVT")?;
    let (round, rest) = match rest.strip_prefix('R') {
        Some(r) if r.len() == 2 => (true, r),
        _ => (false, rest),
    };
    let [a, b] = rest.as_bytes() else {
        return None;
    };
    let (from, to) = (fl(*a)?, fl(*b)?);
    let ok = match (is_float(from), is_float(to)) {
        // CVTRxL and CVTRxQ round; CVTxL, CVTxQ and CVTxI truncate.
        (true, false) => !round || b"LQ".contains(b),
        (false, true) => !round,
        // RN's list, the provisional ones too.
        (true, true) => {
            !round
                && [
                    "FD", "DF", "FG", "GF", "ST", "TS", "DG", "GD", "FS", "SF", "DT", "TD", "GT",
                    "TG",
                ]
                .contains(&rest)
        }
        (false, false) => false,
    };
    ok.then_some(Special::FCvt(from, to, round))
}

/// The PAL calls vaxpunk's PAL has (DESIGN-0001), as BLISS-64's PAL_
/// built-ins: the name, the function code, how many arguments and whether
/// it returns a value.
const PAL: &[(&str, u32, usize, bool)] = &[
    ("PAL_HALT", 0x00, 0, false),
    ("PAL_SWPCTX", 0x05, 1, true),
    ("PAL_MFPR_FEN", 0x0B, 0, true),
    ("PAL_MTPR_FEN", 0x0C, 1, false),
    ("PAL_MTPR_IPIR", 0x0D, 1, false),
    ("PAL_MFPR_IPL", 0x0E, 0, true),
    ("PAL_MTPR_IPL", 0x0F, 1, true),
    ("PAL_MFPR_MCES", 0x10, 0, true),
    ("PAL_MTPR_MCES", 0x11, 1, false),
    ("PAL_MFPR_PCBB", 0x12, 0, true),
    ("PAL_MFPR_SCBB", 0x16, 0, true),
    ("PAL_MTPR_SCBB", 0x17, 1, false),
    ("PAL_MTPR_SIRR", 0x18, 1, false),
    ("PAL_MFPR_SISR", 0x19, 0, true),
    ("PAL_MFPR_WHAMI", 0x3F, 0, true),
    ("PAL_INSQHIL", 0x87, 2, true),
    ("PAL_INSQTIL", 0x88, 2, true),
    ("PAL_INSQHIQ", 0x89, 2, true),
    ("PAL_INSQTIQ", 0x8A, 2, true),
    ("PAL_PROBER", 0x8F, 3, true),
    ("PAL_PROBEW", 0x90, 3, true),
    ("PAL_RD_PS", 0x91, 0, true),
    // The removes take the entry removed as an output parameter too.
    ("PAL_REMQHIL", 0x93, 1, true),
    ("PAL_REMQTIL", 0x94, 1, true),
    ("PAL_REMQHIQ", 0x95, 1, true),
    ("PAL_REMQTIQ", 0x96, 1, true),
    ("PAL_INSQHILR", 0xA2, 2, true),
    ("PAL_INSQTILR", 0xA3, 2, true),
    ("PAL_INSQHIQR", 0xA4, 2, true),
    ("PAL_INSQTIQR", 0xA5, 2, true),
    ("PAL_REMQHILR", 0xA6, 1, true),
    ("PAL_REMQTILR", 0xA7, 1, true),
    ("PAL_REMQHIQR", 0xA8, 1, true),
    ("PAL_REMQTIQR", 0xA9, 1, true),
];

/// BLISS-64's PAL built-ins vaxpunk's PAL doesn't have (RN 2.9.2): a
/// compile error, but known names for BUILTIN.
const NO_PAL: &[&str] = &[
    "PAL_BUGCHK",
    "PAL_IMB",
    "PAL_SWASTEN",
    "PAL_WR_PS_SW",
    "PAL_CFLUSH",
    "PAL_DRAINA",
    "PAL_LDQP",
    "PAL_STQP",
    "PAL_GENTRAP",
    "PAL_RSCC",
    "PAL_READ_UNQ",
    "PAL_WRITE_UNQ",
    "PAL_MTPR_ASTSR",
    "PAL_MTPR_PRBR",
    "PAL_MTPR_TBIAP",
    "PAL_MTPR_TBIS",
    "PAL_MTPR_SSP",
    "PAL_MTPR_ASTEN",
    "PAL_MTPR_TBIA",
    "PAL_MTPR_ESP",
    "PAL_MTPR_USP",
    "PAL_MTPR_VPTB",
    "PAL_MTPR_TBISD",
    "PAL_MTPR_TBISI",
    "PAL_MTPR_DATFX",
    "PAL_MTPR_ASN",
    "PAL_MTPR_PERFMON",
    "PAL_MFPR_SSP",
    "PAL_MFPR_PTBR",
    "PAL_MFPR_TBCHK",
    "PAL_MFPR_ESP",
    "PAL_MFPR_USP",
    "PAL_MFPR_PRBR",
    "PAL_MFPR_ASTEN",
    "PAL_MFPR_ASTSR",
    "PAL_MFPR_VPTB",
    "PAL_MFPR_ASN",
    "WRITE_MBX",
    "CMP_STORE_LONG",
    "CMP_STORE_QUAD",
];

/// The other machine-specific built-ins vbliss has (RN 2.9), which a
/// module must declare BUILTIN, as BLISSA64 has it.
const MACHINE: &[&str] = &[
    "CALL_PAL",
    "PAL_CHME",
    "PAL_CHMK",
    "PAL_CHMS",
    "PAL_CHMU",
    "PAL_BPT",
    "PAL_INSQUEL",
    "PAL_INSQUEQ",
    "PAL_INSQUEL_D",
    "PAL_INSQUEQ_D",
    "PAL_REMQUEL",
    "PAL_REMQUEQ",
    "PAL_REMQUEL_D",
    "PAL_REMQUEQ_D",
    "ADD_ATOMIC_LONG",
    "ADD_ATOMIC_QUAD",
    "AND_ATOMIC_LONG",
    "AND_ATOMIC_QUAD",
    "OR_ATOMIC_LONG",
    "OR_ATOMIC_QUAD",
    "CMP_SWAP_LONG",
    "CMP_SWAP_QUAD",
    "TESTBITSS",
    "TESTBITSC",
    "TESTBITCS",
    "TESTBITCC",
    "TESTBITSSI",
    "TESTBITCCI",
    "ADAWI",
    "UMULH",
    "CMPBGE",
    "ZAP",
    "ZAPNOT",
    "RPCC",
    "TRAPB",
    "DRAINT",
    "RETURNADDRESS",
];

/// Whether `name` is a built-in vbliss knows, for BUILTIN.
fn known(name: &str) -> bool {
    register(name).is_some()
        || float_builtin(name, crate::ir::Fl::Q).is_some()
        || BUILTINS.contains(&name)
        || MACHINE.contains(&name)
        || NO_PAL.contains(&name)
        || PAL.iter().any(|p| p.0 == name)
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
    "CH$TRANSTABLE",
    "CH$TRANSLATE",
    "SIGNALREF",
    "SIGNALREF_STOP",
];

/// The built-ins a module must declare BUILTIN to use (BLISSA64), with the
/// machine-specific ones.
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
    /// library's, for SIGNAL and the like. Its calls pass the count, as
    /// LIB$SIGNAL needs it, under NOCOUNT too.
    fn library_routine(&mut self, name: &str) -> usize {
        let counted = |s: &Sym| {
            matches!(&s.kind, Kind::Routine { external: true, linkage: Some(l), .. }
                if l.count == Some(true) && !l.jsb)
        };
        if let Some(id) = self.m.syms.iter().position(|s| s.asm == name && counted(s)) {
            return id;
        }
        let linkage = crate::linkage::Linkage {
            jsb: false,
            interrupt: false,
            count: Some(true),
            params: Vec::new(),
            outs: Vec::new(),
            globals: Vec::new(),
            nopreserve: Vec::new(),
        };
        self.m.syms.push(Sym {
            name: name.into(),
            asm: name.into(),
            kind: Kind::Routine {
                global: false,
                external: true,
                novalue: false,
                linkage: Some(std::rc::Rc::new(linkage)),
            },
        });
        self.m.syms.len() - 1
    }

    /// The actuals of a built-in, `(e, ... [; out, ...])`: the inputs and
    /// the outputs, places the built-in stores in.
    fn builtin_args(&mut self) -> R<(Vec<Expr>, Vec<Expr>)> {
        self.expect_punct('(')?;
        let (mut args, mut outs) = (Vec::new(), Vec::new());
        if !self.eat_punct(')') {
            if !self.at_punct(';') {
                loop {
                    args.push(self.value()?);
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            if self.eat_punct(';') {
                loop {
                    outs.push(self.expr()?);
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            self.expect_punct(')')?;
        }
        Ok((args, outs))
    }

    /// `CH$TRANSTABLE(items)`, after its name: a PLIT of the translation
    /// table's bytes (LRM 20.2.8).
    fn transtable(&mut self) -> R<Expr> {
        self.expect_punct('(')?;
        let mut bytes = Vec::new();
        self.trans_items(&mut bytes)?;
        self.expect_punct(')')?;
        let items = bytes
            .into_iter()
            .map(|b| crate::data::Init::Val(Expr::Num(b), 1))
            .collect();
        self.lets += 1;
        Ok(Expr::Plit(self.lets - 1, false, items))
    }

    /// A translation string: codes and `REP n OF (items)`.
    fn trans_items(&mut self, out: &mut Vec<i64>) -> R<()> {
        loop {
            if self.eat("REP") {
                let n = self.ctce()?;
                self.expect("OF")?;
                self.expect_punct('(')?;
                let mut group = Vec::new();
                self.trans_items(&mut group)?;
                self.expect_punct(')')?;
                for _ in 0..n.max(0) {
                    out.extend(&group);
                }
            } else {
                out.push(self.ctce()? & 255);
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// A temporary for a value used more than once.
    fn temp(&mut self) -> u32 {
        self.lets += 1;
        self.lets - 1
    }

    /// The call of built-in `name`, after its name.
    pub(crate) fn builtin(&mut self, name: &str) -> R<Expr> {
        if name == "CH$TRANSTABLE" {
            return self.transtable();
        }
        if NO_PAL.contains(&name) {
            return self.err(format!(
                "{name}: vaxpunk's PAL has no such call (DESIGN-0001), or ARM64 no such operation"
            ));
        }
        // The linkage functions read the count a NOCOUNT call doesn't pass
        // (RN 2.8.5).
        if matches!(
            name,
            "ACTUALCOUNT" | "ACTUALPARAMETER" | "NULLPARAMETER" | "ARGPTR"
        ) && !self
            .linkage_now()
            .and_then(|l| l.count)
            .unwrap_or(!self.m.nocount)
        {
            return self.err(format!(
                "{name} can't be used in a routine whose linkage is NOCOUNT"
            ));
        }
        let (mut args, mut outs) = self.builtin_args()?;
        let n = args.len();
        let takes_outputs = name.contains("_ATOMIC_")
            || name.starts_with("TESTBIT")
            || name.starts_with("PAL_REMQ");
        // BLISSA64 warns, and goes on without them (probe).
        let most = usize::from(takes_outputs);
        if outs.len() > most {
            let at = self.last();
            let msg = "Extra output parameter in call to built-in function";
            self.diag('W', &at, msg.into());
            outs.truncate(most);
        }
        let want = |p: &mut Self, k: usize| -> R<()> {
            match n.cmp(&k) {
                std::cmp::Ordering::Equal => Ok(()),
                std::cmp::Ordering::Greater => {
                    p.err(format!("Extra actual parameters in call to {name}"))
                }
                std::cmp::Ordering::Less => p.err(format!(
                    "Omitted actual parameter in call to {name} has no default value"
                )),
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
                Expr::Call(Box::new(Expr::Name(r)), args, Vec::new(), None)
            }
            "SETUNWIND" => {
                want(self, 0)?;
                let r = self.library_routine("SYS$UNWIND");
                Expr::Call(
                    Box::new(Expr::Name(r)),
                    vec![Expr::Num(0), Expr::Num(0)],
                    Vec::new(),
                    None,
                )
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
                    // A pointer, not a name: the dot lint leaves it alone.
                    None => Expr::Block(vec![addr], true),
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
            "CH$TRANSLATE" => {
                want(self, 6)?;
                special(Special::ChTranslate, args)
            }
            "SIGNALREF" | "SIGNALREF_STOP" => {
                if n == 0 {
                    return self.err(format!("{name} takes a condition value's address"));
                }
                let r = self.library_routine(if name == "SIGNALREF" {
                    "LIB$SIGNAL"
                } else {
                    "LIB$STOP"
                });
                args[0] = Expr::Fetch(Box::new(word(args.remove(0))));
                Expr::Call(Box::new(Expr::Name(r)), args, Vec::new(), None)
            }
            "ADD_ATOMIC_LONG" | "ADD_ATOMIC_QUAD" | "AND_ATOMIC_LONG" | "AND_ATOMIC_QUAD"
            | "OR_ATOMIC_LONG" | "OR_ATOMIC_QUAD" => {
                if !(2..=3).contains(&n) {
                    return self.err(format!(
                        "{name} takes an address, a value and a retry count"
                    ));
                }
                let op = match &name[..2] {
                    "AD" => crate::ir::Op::Add,
                    "AN" => crate::ir::Op::And,
                    _ => crate::ir::Op::Or,
                };
                let s = Special::Atomic {
                    op,
                    size: if name.ends_with("QUAD") { 8 } else { 4 },
                    retries: n == 3,
                    old: !outs.is_empty(),
                };
                args.append(&mut outs);
                special(s, args)
            }
            "CMP_SWAP_LONG" | "CMP_SWAP_QUAD" => {
                want(self, 3)?;
                special(
                    Special::CmpSwap(if name.ends_with("QUAD") { 8 } else { 4 }),
                    args,
                )
            }
            "TESTBITSS" | "TESTBITSC" | "TESTBITCS" | "TESTBITCC" | "TESTBITSSI" | "TESTBITCCI" => {
                if !(1..=2).contains(&n) {
                    return self.err(format!("{name} takes a bit field and a retry count"));
                }
                let b = name.as_bytes();
                let s = Special::TestBit {
                    was_set: b[7] == b'S',
                    set: b[8] == b'S',
                    retries: n == 2,
                    flag: !outs.is_empty(),
                };
                args.append(&mut outs);
                special(s, args)
            }
            "ADAWI" => {
                want(self, 2)?;
                special(Special::Adawi, args)
            }
            "PAL_INSQUEL" | "PAL_INSQUEQ" | "PAL_INSQUEL_D" | "PAL_INSQUEQ_D" => {
                want(self, 2)?;
                let s = Special::Insque {
                    quad: name.as_bytes()[10] == b'Q',
                    deferred: name.ends_with("_D"),
                };
                special(s, args)
            }
            "PAL_REMQUEL" | "PAL_REMQUEQ" | "PAL_REMQUEL_D" | "PAL_REMQUEQ_D" => {
                want(self, 1)?;
                let s = Special::Remque {
                    quad: name.as_bytes()[10] == b'Q',
                    deferred: name.ends_with("_D"),
                    addr: !outs.is_empty(),
                };
                args.append(&mut outs);
                special(s, args)
            }
            "CALL_PAL" => {
                if !(1..=7).contains(&n) {
                    return self.err("CALL_PAL takes a function code and up to six arguments");
                }
                let code = args.remove(0);
                match fold(&code) {
                    Some(c @ 0..=0xBF) => special(Special::Pal(c as u32), args),
                    _ => return self.err("CALL_PAL's function code must be a constant, 0 to ^XBF"),
                }
            }
            "PAL_CHME" | "PAL_CHMK" | "PAL_CHMS" | "PAL_CHMU" => {
                if !(1..=7).contains(&n) {
                    return self.err(format!("{name} takes a code and up to six arguments"));
                }
                let func = match name {
                    "PAL_CHME" => 0x82,
                    "PAL_CHMK" => 0x83,
                    "PAL_CHMS" => 0x84,
                    _ => 0x85,
                };
                special(Special::Chm(func), args)
            }
            "PAL_BPT" => {
                want(self, 0)?;
                special(Special::Bpt, args)
            }
            "RETURNADDRESS" => {
                want(self, 0)?;
                special(Special::RetAddr, args)
            }
            "RPCC" => {
                want(self, 0)?;
                special(Special::Rpcc, args)
            }
            "TRAPB" | "DRAINT" => {
                // No pending arithmetic traps on ARM64.
                want(self, 0)?;
                Expr::Num(0)
            }
            n if let Some(s) = float_builtin(n, self.int_fl()) => {
                if self.m.nofp {
                    let at = self.last();
                    let msg =
                        "Illegal use of floating point built-in function with ENVIRONMENT(NOFP)";
                    self.diag('W', &at, msg.into());
                }
                // Operands by address; only CMPx has a value (RN 2.10.1).
                let k = match s {
                    Special::FArith(..) => 3,
                    _ => 2,
                };
                want(self, k)?;
                special(s, args)
            }
            "UMULH" => {
                want(self, 2)?;
                Expr::Op(
                    crate::ir::Op::Umulh,
                    Box::new(args.remove(0)),
                    Box::new(args.remove(0)),
                )
            }
            "CMPBGE" | "ZAP" | "ZAPNOT" => {
                want(self, 2)?;
                self.byte_op(name, args.remove(0), args.remove(0))
            }
            _ => match PAL.iter().find(|p| p.0 == name) {
                Some(&(_, code, k, _)) if !outs.is_empty() => {
                    want(self, k)?;
                    args.append(&mut outs);
                    special(Special::PalOut(code), args)
                }
                Some(&(_, code, k, _)) => {
                    want(self, k)?;
                    special(Special::Pal(code), args)
                }
                None => return self.err(format!("{name} is not supported yet")),
            },
        })
    }

    /// Alpha's byte operations, a byte at a time: CMPBGE(a, b) has bit i
    /// set if byte i of a is at least byte i of b, unsigned; ZAP(a, m)
    /// clears the bytes of a that m's bits 0-7 select, ZAPNOT the others.
    fn byte_op(&mut self, name: &str, a: Expr, b: Expr) -> Expr {
        let (ta, tb) = (self.temp(), self.temp());
        let byte = |t: u32, i: i64| {
            Expr::Bin(
                BOp::And,
                Box::new(Expr::Op(
                    crate::ir::Op::Shr,
                    Box::new(Expr::Temp(t)),
                    Box::new(Expr::Num(i)),
                )),
                Box::new(Expr::Num(255)),
            )
        };
        let mut e = Expr::Num(0);
        for i in 0..8 {
            let part = if name == "CMPBGE" {
                let ge = Expr::Bin(
                    BOp::Rel(Rel::Geq, true),
                    Box::new(byte(ta, 8 * i)),
                    Box::new(byte(tb, 8 * i)),
                );
                Expr::Op(crate::ir::Op::Shl, Box::new(ge), Box::new(Expr::Num(i)))
            } else {
                // The byte's mask where m's bit i says keep it.
                let bit = Expr::Bin(
                    BOp::And,
                    Box::new(Expr::Op(
                        crate::ir::Op::Shr,
                        Box::new(Expr::Temp(tb)),
                        Box::new(Expr::Num(i)),
                    )),
                    Box::new(Expr::Num(1)),
                );
                let keep = if name == "ZAP" {
                    Expr::Bin(BOp::Xor, Box::new(bit), Box::new(Expr::Num(1)))
                } else {
                    bit
                };
                Expr::Bin(
                    BOp::And,
                    Box::new(Expr::Neg(Box::new(keep))),
                    Box::new(Expr::Num(255i64 << (8 * i))),
                )
            };
            e = Expr::Bin(BOp::Or, Box::new(e), Box::new(part));
        }
        if name != "CMPBGE" {
            e = Expr::Bin(BOp::And, Box::new(Expr::Temp(ta)), Box::new(e));
        }
        Expr::Let(
            ta,
            Box::new(a),
            Box::new(Expr::Let(tb, Box::new(b), Box::new(e))),
        )
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

    /// The fullword integer the floating point built-ins' I means.
    fn int_fl(&self) -> crate::ir::Fl {
        if self.m.dialect.a32 {
            crate::ir::Fl::L
        } else {
            crate::ir::Fl::Q
        }
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
            if !known(&name) {
                return Err(self.error_at(&at, format!("{name} is not a built-in vbliss knows")));
            }
            if !matches!(name.as_str(), "ACTUALCOUNT" | "ACTUALPARAMETER") {
                self.transport(&name, &at, true);
            }
            // R2 to R11 in a routine are its register data, as `REGISTER
            // R2 = 2` declares it (docs/bliss64.md, *Registers*).
            if register(&name).is_some_and(|r| r >= 2) && self.slots_open() {
                if !self.scopes.last().unwrap().contains_key(&name) {
                    let kind =
                        self.data_kind(Storage::Local(0), &mut crate::data::Attrs::default())?;
                    self.declare(name, kind)?;
                }
                if !self.eat_punct(',') {
                    return Ok(());
                }
                continue;
            }
            // Again in the same block is no new declaration: BLISSA64 takes
            // it, as DEC's VAX built-in macros need (t8y:/host/vdefs).
            let again = self
                .scopes
                .last()
                .unwrap()
                .get(&name)
                .is_some_and(|&id| self.m.syms[id].kind == Kind::Builtin);
            if !again {
                self.declare(name, Kind::Builtin)?;
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }
}
