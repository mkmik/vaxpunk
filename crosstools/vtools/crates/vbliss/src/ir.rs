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
    /// The high 64 bits of the unsigned 128-bit product.
    Umulh,
    /// 1 if a + b, a - b or a * b overflows 64 bits, else 0: for the
    /// OVERFLOW switch.
    Addv,
    Subv,
    Mulv,
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
    /// `dst = stackarg n`: an INTERRUPT routine's formal n, from the stack
    /// at entry: the PAL's parameters, padded to 16 bytes, then its
    /// frame's PC and PSL.
    StackArg(u32, u32),
    /// `dst = call target(args)`, by the calling standard.
    Call(Option<u32>, V, Vec<V>),
    /// `dst = jsb target(arg rN, ...) -> rN %t, ... nopreserve rN...`: a
    /// call by a JSB linkage, or by a CALL linkage with registers in it
    /// (`dst = call target(args; arg rN, ...) ...`): arguments in VAX
    /// registers, standard ones for CALL, R0 the result, and registers
    /// read into temporaries after it; R0, R1, the output registers and
    /// those named after `nopreserve` aren't kept.
    Link(Box<LinkCall>),
    /// `setreg rN, v`: register N holds v when the routine returns, as an
    /// output parameter or EXTERNAL REGISTER; just before `ret`.
    SetReg(u8, V),
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
    /// `dst = casN addr, expected, new`: one try at storing new, N bytes,
    /// at addr if what is there is expected, by load- and
    /// store-exclusive with acquire and release; 1 if it stored. addr is
    /// naturally aligned.
    Cas(u32, V, V, V, u8),
    /// `dst = pal func(args)`: a PAL call (DESIGN-0001), the function
    /// code (x7) and up to six arguments (x0-x5); the result is x0.
    Pal(u32, V, Vec<V>),
    /// `dst = palr1`: x1 as the PAL call just before left it, the entry a
    /// queue call removed.
    PalR1(u32),
    /// `dst = retaddr`: the routine's return address, in its frame.
    RetAddr(u32),
    /// `dst = rpcc`: the cycle counter, CNTVCT_EL0.
    Rpcc(u32),
    /// `bpt`: a breakpoint, `brk`.
    Bpt,
    /// `comment "text"`: CODECOMMENT's text, a comment in the code.
    Comment(String),
    /// `fadd<f> a, b, dst` and the like: the floating point built-ins
    /// (RN 2.10.1), on operands in memory at a and b, the result stored at
    /// dst, all addresses.
    FArith(FOp, Fl, V, V, V),
    /// `dst = fcmp<f> a, b`: -1, 0 or 1.
    FCmp(u32, Fl, V, V),
    /// `cvt[r] <from> <to> a, dst`: a conversion, from memory at a to
    /// memory at dst; `r` rounds to the nearest, ties away, else an
    /// integer is truncated.
    FCvt(Fl, Fl, bool, V, V),
    /// `dst = fcall target(args) -> results`: a call by the calling
    /// standard with floating arguments or results (RN 2.10.4,
    /// DESIGN-0004): an argument `t:addr` is the value of that format at
    /// addr, S and T in v0-v7 with their class in x9, VAX ones general;
    /// a result `t:addr` is stored at addr, from v0 and v1 or x0 and x1.
    FCall(Box<FCall>),
}

/// A call with floating arguments or results (`Ins::FCall`).
#[derive(Clone, Debug, PartialEq)]
pub struct FCall {
    pub dst: u32,
    pub target: V,
    /// Each argument: a value, or a floating value's format and address.
    pub args: Vec<(V, Option<Fl>)>,
    pub results: Vec<(Fl, V)>,
}

/// A floating point built-in's arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// What the floating point built-ins work on: VAX F, D and G floating,
/// IEEE S and T, and longword and quadword integers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fl {
    F,
    D,
    G,
    S,
    T,
    L,
    Q,
}

impl Fl {
    /// The letter the built-ins' names have for it.
    pub fn letter(self) -> char {
        match self {
            Fl::F => 'f',
            Fl::D => 'd',
            Fl::G => 'g',
            Fl::S => 's',
            Fl::T => 't',
            Fl::L => 'l',
            Fl::Q => 'q',
        }
    }
}

/// A call by a linkage with registers (`Ins::Link`).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkCall {
    pub dst: Option<u32>,
    pub target: V,
    pub jsb: bool,
    /// Whether a CALL passes the argument count, as COUNT linkages do.
    pub count: bool,
    /// The standard arguments, for CALL.
    pub args: Vec<V>,
    pub ins: Vec<(V, u8)>,
    pub outs: Vec<(u8, u32)>,
    pub nopreserve: Vec<u8>,
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
    /// The psect its code goes in.
    pub psect: String,
    /// Whether its argument list (argcount, argn, argptr) has longwords,
    /// for BLISS-32 and LONG_DEFAULT, rather than quadwords.
    pub long_args: bool,
    /// An INTERRUPT routine: it pops the PAL's parameters, this many, and
    /// returns with REI.
    pub rei: Option<u32>,
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
            Umulh => "umulh",
            Addv => "addv",
            Subv => "subv",
            Mulv => "mulv",
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
            Ins::StackArg(d, n) => write!(f, "%{d} =l stackarg {n}"),
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
            Ins::Link(c) => {
                if let Some(d) = c.dst {
                    write!(f, "%{d} =l ")?;
                }
                let call = match (c.jsb, c.count) {
                    (true, _) => "jsb",
                    (false, true) => "call",
                    (false, false) => "call nocount",
                };
                write!(f, "{call} {}(", c.target)?;
                for (i, a) in c.args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a}")?;
                }
                if !c.jsb {
                    f.write_str(";")?;
                }
                for (i, (a, r)) in c.ins.iter().enumerate() {
                    f.write_str(if i > 0 || !c.jsb { " " } else { "" })?;
                    write!(f, "{a} r{r}")?;
                    if i + 1 < c.ins.len() {
                        f.write_str(",")?;
                    }
                }
                f.write_str(")")?;
                for (i, (r, t)) in c.outs.iter().enumerate() {
                    f.write_str(if i > 0 { ", " } else { " -> " })?;
                    write!(f, "r{r} %{t}")?;
                }
                if !c.nopreserve.is_empty() {
                    f.write_str(" nopreserve")?;
                    for r in &c.nopreserve {
                        write!(f, " r{r}")?;
                    }
                }
                Ok(())
            }
            Ins::SetReg(r, v) => write!(f, "setreg r{r}, {v}"),
            Ins::RegArg(d, r) => write!(f, "%{d} =l regarg r{r}"),
            Ins::SetHandler(v) => write!(f, "sethandler {v}"),
            Ins::SetEnable(v) => write!(f, "setenable {v}"),
            Ins::ArgCount(d) => write!(f, "%{d} =l argcount"),
            Ins::ArgN(d, i) => write!(f, "%{d} =l argn {i}"),
            Ins::ArgPtr(d) => write!(f, "%{d} =l argptr"),
            Ins::Barrier => f.write_str("barrier"),
            Ins::Cas(d, a, e, n, size) => write!(f, "%{d} =l cas{size} {a}, {e}, {n}"),
            Ins::Pal(d, func, args) => {
                write!(f, "%{d} =l pal {func}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a}")?;
                }
                f.write_str(")")
            }
            Ins::RetAddr(d) => write!(f, "%{d} =l retaddr"),
            Ins::PalR1(d) => write!(f, "%{d} =l palr1"),
            Ins::Rpcc(d) => write!(f, "%{d} =l rpcc"),
            Ins::Bpt => f.write_str("bpt"),
            Ins::Comment(s) => write!(f, "comment {s:?}"),
            Ins::FArith(op, t, a, b, d) => {
                let op = match op {
                    FOp::Add => "add",
                    FOp::Sub => "sub",
                    FOp::Mul => "mul",
                    FOp::Div => "div",
                };
                write!(f, "f{op}{} {a}, {b}, {d}", t.letter())
            }
            Ins::FCmp(d, t, a, b) => write!(f, "%{d} =l fcmp{} {a}, {b}", t.letter()),
            Ins::FCall(c) => {
                write!(f, "%{} =l fcall {}(", c.dst, c.target)?;
                for (i, (v, fl)) in c.args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    if let Some(fl) = fl {
                        write!(f, "{}:", fl.letter())?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str(")")?;
                for (i, (fl, v)) in c.results.iter().enumerate() {
                    write!(
                        f,
                        "{} {}:{v}",
                        if i == 0 { " ->" } else { "," },
                        fl.letter()
                    )?;
                }
                Ok(())
            }
            Ins::FCvt(from, to, r, a, d) => write!(
                f,
                "cvt{} {} {} {a}, {d}",
                if *r { "r" } else { "" },
                from.letter(),
                to.letter()
            ),
        }
    }
}

impl fmt::Display for Func {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let g = if self.global { "global " } else { "" };
        let p = if self.psect == "$CODE$" || self.psect.is_empty() {
            String::new()
        } else {
            format!(" in {}", self.psect)
        };
        writeln!(f, "{g}routine ${}{p} {{", self.name)?;
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
                Term::Ret(v) => match self.rei {
                    Some(n) => writeln!(f, "    rei {n}")?,
                    None => writeln!(f, "    ret {v}")?,
                },
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
    /// Psects' attributes, as `.PSECT` takes them, for those not BLISS's
    /// own; and global literals and GLOBAL BINDs, `name == value`.
    pub psects: Vec<(String, String)>,
    pub globals: Vec<(String, V)>,
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
        for (p, attrs) in &self.psects {
            writeln!(f, "psect {p} ({attrs})")?;
        }
        for (g, v) in &self.globals {
            writeln!(f, "global ${g} = {v}")?;
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
