//! The IR: each routine a list of blocks of three-address instructions on
//! temporaries, as QBE's, without SSA. `vbliss --ir` prints it, and it is
//! the contract between vbliss and BLISS.EXE: both print the same IR for
//! the same source. Every value is a fullword, 64 bits (class `l`).

use std::fmt::{self, Write};

/// An operand.
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    /// Temporary `%n`.
    T(u32),
    /// Constant.
    C(i64),
    /// The address of a symbol, plus an offset: `$NAME+8`.
    Sym(String, i64),
    /// The address of frame slot `n`, plus an offset: `&n+8`.
    Slot(u32, i64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    /// Signed division, truncating.
    Div,
    /// Signed remainder, with the dividend's sign.
    Rem,
    And,
    Or,
    Xor,
    Eqv,
    /// Shifts by 0-63; `Shl` left, `Shr` logical right, `Sar` arithmetic.
    Shl,
    Shr,
    Sar,
    /// BLISS's `^`: left by a positive count, arithmetic right by a
    /// negative one, -63 to 63.
    Ash,
    /// Rotate right by 0-63.
    Ror,
    /// Comparisons, 1 if true, else 0; `u` unsigned.
    Ceq,
    Cne,
    Clt,
    Cle,
    Cgt,
    Cge,
    Cltu,
    Cleu,
    Cgtu,
    Cgeu,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Un {
    Copy,
    Neg,
    Not,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ins {
    Bin(Op, u32, V, V),
    Un(Un, u32, V),
    /// `dst = load<size><s|u> addr`: size in bytes, sign- or zero-extended.
    Load(u32, V, u8, bool),
    /// `store<size> value, addr`.
    Store(V, V, u8),
    /// `dst = ext<s|u> value, pos, size`: bits pos..pos+size of value,
    /// extended. pos and size are constants or temporaries.
    Ext(u32, V, V, V, bool),
    /// `dst = ins base, value, pos, size`: base with bits pos..pos+size
    /// replaced by the low bits of value.
    Insert(u32, V, V, V, V),
    /// `dst = arg n`: the routine's argument n, from 0.
    Arg(u32, u32),
    /// `dst = call target(args)`, by the calling standard.
    Call(Option<u32>, V, Vec<V>),
    /// `dst = jsb target(arg rN, ...) nopreserve rN...`: a JSB linkage's
    /// call, each argument in a VAX register, R0 the result; R0, R1 and
    /// the registers named after `nopreserve` aren't kept.
    Jsb(Option<u32>, V, Vec<(V, u8)>, Vec<u8>),
    /// `dst = regarg rN`: a JSB routine's parameter, in VAX register N.
    RegArg(u32, u8),
    /// `sethandler v`: the routine's condition handler, at 16(FP).
    SetHandler(V),
    /// `setenable v`: the address of the routine's enable vector, where the
    /// module's handler jacket finds it, at 32(FP).
    SetEnable(V),
    /// `dst = argcount`, `dst = argn i` (from 1) and `dst = argptr`: the
    /// routine's argument list, as a count and the arguments, a fullword
    /// each, which the prologue copies when the routine reads one.
    ArgCount(u32),
    ArgN(u32, V),
    ArgPtr(u32),
    /// `barrier`: a memory barrier.
    Barrier,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    Jmp(u32),
    /// Branches on the low bit of the value, as BLISS tests: `jlbs v, @t, @f`.
    Jlbs(V, u32, u32),
    Ret(V),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub ins: Vec<Ins>,
    pub term: Term,
}

#[derive(Clone, Debug, Default)]
pub struct Func {
    pub name: String,
    pub global: bool,
    /// Frame slots: their sizes in bytes.
    pub slots: Vec<u32>,
    pub temps: u32,
    /// Block 0 is the entry.
    pub blocks: Vec<Block>,
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let off = |f: &mut fmt::Formatter, o: i64| match o {
            0 => Ok(()),
            o if o > 0 => write!(f, "+{o}"),
            o => write!(f, "{o}"),
        };
        match self {
            V::T(t) => write!(f, "%{t}"),
            V::C(c) => write!(f, "{c}"),
            V::Sym(s, o) => {
                write!(f, "${s}")?;
                off(f, *o)
            }
            V::Slot(s, o) => {
                write!(f, "&{s}")?;
                off(f, *o)
            }
        }
    }
}

impl Op {
    pub fn name(self) -> &'static str {
        use Op::*;
        match self {
            Add => "add",
            Sub => "sub",
            Mul => "mul",
            Div => "div",
            Rem => "rem",
            And => "and",
            Or => "or",
            Xor => "xor",
            Eqv => "eqv",
            Shl => "shl",
            Shr => "shr",
            Sar => "sar",
            Ash => "ash",
            Ror => "ror",
            Ceq => "ceq",
            Cne => "cne",
            Clt => "clt",
            Cle => "cle",
            Cgt => "cgt",
            Cge => "cge",
            Cltu => "cltu",
            Cleu => "cleu",
            Cgtu => "cgtu",
            Cgeu => "cgeu",
        }
    }
}

impl fmt::Display for Ins {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let su = |s: bool| if s { "s" } else { "u" };
        match self {
            Ins::Bin(op, d, a, b) => write!(f, "%{d} =l {} {a}, {b}", op.name()),
            Ins::Un(op, d, a) => {
                let name = match op {
                    Un::Copy => "copy",
                    Un::Neg => "neg",
                    Un::Not => "not",
                };
                write!(f, "%{d} =l {name} {a}")
            }
            Ins::Load(d, a, size, s) => write!(f, "%{d} =l load{size}{} {a}", su(*s)),
            Ins::Store(v, a, size) => write!(f, "store{size} {v}, {a}"),
            Ins::Ext(d, v, p, s, e) => write!(f, "%{d} =l ext{} {v}, {p}, {s}", su(*e)),
            Ins::Insert(d, b, v, p, s) => write!(f, "%{d} =l ins {b}, {v}, {p}, {s}"),
            Ins::Arg(d, n) => write!(f, "%{d} =l arg {n}"),
            Ins::Call(d, t, args) => {
                if let Some(d) = d {
                    write!(f, "%{d} =l ")?;
                }
                write!(f, "call {t}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a}")?;
                }
                f.write_str(")")
            }
            Ins::Jsb(d, t, args, nopreserve) => {
                if let Some(d) = d {
                    write!(f, "%{d} =l ")?;
                }
                write!(f, "jsb {t}(")?;
                for (i, (a, r)) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a} r{r}")?;
                }
                f.write_str(")")?;
                if !nopreserve.is_empty() {
                    f.write_str(" nopreserve")?;
                    for r in nopreserve {
                        write!(f, " r{r}")?;
                    }
                }
                Ok(())
            }
            Ins::RegArg(d, r) => write!(f, "%{d} =l regarg r{r}"),
            Ins::SetHandler(v) => write!(f, "sethandler {v}"),
            Ins::SetEnable(v) => write!(f, "setenable {v}"),
            Ins::ArgCount(d) => write!(f, "%{d} =l argcount"),
            Ins::ArgN(d, i) => write!(f, "%{d} =l argn {i}"),
            Ins::ArgPtr(d) => write!(f, "%{d} =l argptr"),
            Ins::Barrier => f.write_str("barrier"),
        }
    }
}

impl fmt::Display for Func {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let g = if self.global { "global " } else { "" };
        writeln!(f, "{g}routine ${} {{", self.name)?;
        for (i, size) in self.slots.iter().enumerate() {
            writeln!(f, "    &{i} = slot {size}")?;
        }
        for (i, b) in self.blocks.iter().enumerate() {
            writeln!(f, "@{i}")?;
            for ins in &b.ins {
                writeln!(f, "    {ins}")?;
            }
            match &b.term {
                Term::Jmp(t) => writeln!(f, "    jmp @{t}")?,
                Term::Jlbs(v, t, e) => writeln!(f, "    jlbs {v}, @{t}, @{e}")?,
                Term::Ret(v) => writeln!(f, "    ret {v}")?,
            }
        }
        writeln!(f, "}}")
    }
}

/// Static data, as the IR prints it and the back end writes it.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A value of 1, 2, 4 or 8 bytes: a constant or an address.
    Val(V, u8),
    Bytes(Vec<u8>),
    Zero(u32),
}

#[derive(Clone, Debug)]
pub struct Data {
    pub psect: String,
    pub name: String,
    pub global: bool,
    /// Alignment, as a power of 2.
    pub align: u8,
    pub items: Vec<Item>,
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let g = if self.global { "global " } else { "" };
        write!(
            f,
            "{g}data ${} in {} align {} {{",
            self.name,
            self.psect,
            1 << self.align
        )?;
        for (i, item) in self.items.iter().enumerate() {
            f.write_str(if i > 0 { ", " } else { " " })?;
            match item {
                Item::Val(v, size) => write!(f, "{size} {v}")?,
                Item::Bytes(b) => {
                    f.write_char('"')?;
                    for &c in b {
                        if c == b'"' || c == b'\\' || !(32..127).contains(&c) {
                            write!(f, "\\x{c:02x}")?;
                        } else {
                            f.write_char(c as char)?;
                        }
                    }
                    f.write_char('"')?;
                }
                Item::Zero(n) => write!(f, "z {n}")?,
            }
        }
        writeln!(f, " }}")
    }
}

/// A compiled module.
#[derive(Clone, Debug, Default)]
pub struct Module {
    pub name: String,
    pub ident: Option<String>,
    /// The main routine, the image's transfer address.
    pub main: Option<String>,
    pub externals: Vec<String>,
    pub data: Vec<Data>,
    pub funcs: Vec<Func>,
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "module {}", self.name)?;
        if let Some(m) = &self.main {
            writeln!(f, "main ${m}")?;
        }
        for e in &self.externals {
            writeln!(f, "external ${e}")?;
        }
        for d in &self.data {
            write!(f, "{d}")?;
        }
        for func in &self.funcs {
            write!(f, "{func}")?;
        }
        Ok(())
    }
}
