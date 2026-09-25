//! ARM64 instructions: operands in standard (GNU/LLVM) syntax, encoded by
//! instruction class. Every form is checked against GNU `as` in
//! `tests/encode.rs`.

use crate::expr::{self, Expr, Scope, Value};
use crate::lex::{Cursor, Result, err};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    X,
    W,
    /// SP and WSP: register 31 where the stack pointer is allowed.
    Sp,
    Wsp,
    B,
    H,
    S,
    D,
    Q,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reg {
    pub kind: Kind,
    /// 0 to 31; 31 is the zero register for `X` and `W`.
    pub n: u32,
}

impl Reg {
    fn sf(self) -> u32 {
        matches!(self.kind, Kind::X | Kind::Sp) as u32
    }
    fn is_gp(self) -> bool {
        matches!(self.kind, Kind::X | Kind::W | Kind::Sp | Kind::Wsp)
    }
}

/// `#:lo12:` and friends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modifier {
    Lo12,
    /// A 16-bit chunk for MOVZ/MOVK, and whether the rest must be zero.
    AbsG(u32, bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Reg(Reg),
    /// An immediate or an address, with or without `#`.
    Imm(Expr),
    Mod(Modifier, Expr),
    /// LSL, LSR, ASR, ROR with an amount.
    Shift(u32, Expr),
    /// UXTB..SXTX (0 to 7) with an optional amount.
    Extend(u32, Option<Expr>),
    Mem {
        base: Reg,
        index: Index,
        writeback: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Index {
    None,
    Imm(Expr),
    Mod(Modifier, Expr),
    /// Register, extend option (3 = LSL/UXTX), amount.
    Reg(Reg, u32, Option<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operand {
    pub op: Op,
    pub col: usize,
}

/// How the linker must patch an instruction (docs/object-format.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fix {
    Jump26,
    Branch19,
    Branch14,
    Adr,
    Adrp,
    AddLo12,
    /// Load or store, with log2 of the access size.
    Ldst(u32),
    Movw(u32, bool),
}

/// An encoded instruction; with a fixup, the patched field is zero.
#[derive(Debug, PartialEq)]
pub struct Encoded {
    pub word: u32,
    pub fixup: Option<(Fix, Value)>,
}

const SHIFTS: [&str; 4] = ["LSL", "LSR", "ASR", "ROR"];
const EXTENDS: [&str; 8] = [
    "UXTB", "UXTH", "UXTW", "UXTX", "SXTB", "SXTH", "SXTW", "SXTX",
];
const CONDS: [&str; 16] = [
    "EQ", "NE", "HS", "LO", "MI", "PL", "VS", "VC", "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV",
];

pub fn reg(name: &str) -> Option<Reg> {
    let (kind, n) = match name {
        "SP" => {
            return Some(Reg {
                kind: Kind::Sp,
                n: 31,
            });
        }
        "WSP" => {
            return Some(Reg {
                kind: Kind::Wsp,
                n: 31,
            });
        }
        "XZR" => {
            return Some(Reg {
                kind: Kind::X,
                n: 31,
            });
        }
        "WZR" => {
            return Some(Reg {
                kind: Kind::W,
                n: 31,
            });
        }
        "LR" => {
            return Some(Reg {
                kind: Kind::X,
                n: 30,
            });
        }
        "FP" => {
            return Some(Reg {
                kind: Kind::X,
                n: 29,
            });
        }
        _ => name.split_at_checked(1)?,
    };
    let kind = match kind {
        "X" => Kind::X,
        "W" => Kind::W,
        "B" => Kind::B,
        "H" => Kind::H,
        "S" => Kind::S,
        "D" => Kind::D,
        "Q" => Kind::Q,
        _ => return None,
    };
    let n: u32 = n
        .parse()
        .ok()
        .filter(|n: &u32| *n < 32 && !n.to_string().starts_with('+'))?;
    if n.to_string() != name[1..] || (n == 31 && matches!(kind, Kind::X | Kind::W)) {
        return None;
    }
    Some(Reg { kind, n })
}

fn cond(name: &str) -> Option<u32> {
    let name = match name {
        "CS" => "HS",
        "CC" => "LO",
        n => n,
    };
    CONDS.iter().position(|&c| c == name).map(|c| c as u32)
}

/// Parses the operands of an instruction.
pub fn operands(c: &mut Cursor, block: u32) -> Result<Vec<Operand>> {
    let mut ops = Vec::new();
    if c.at_end() {
        return Ok(ops);
    }
    loop {
        let col = c.col();
        let op = if c.eat('[') {
            memory(c, block)?
        } else {
            simple(c, block)?
        };
        ops.push(Operand { op, col });
        if !c.eat(',') {
            break;
        }
    }
    if !c.at_end() {
        return err(c.col(), "unexpected text after the operands");
    }
    Ok(ops)
}

fn simple(c: &mut Cursor, block: u32) -> Result<Op> {
    let hash = c.eat('#');
    if c.peek() == Some(':') {
        let m = modifier(c)?;
        return Ok(Op::Mod(m, expr::parse(c, block)?));
    }
    if !hash {
        let save = c.at;
        if let Some(name) = c.name() {
            if let Some(r) = reg(&name) {
                return Ok(Op::Reg(r));
            }
            if let Some(s) = SHIFTS.iter().position(|&s| s == name) {
                c.eat('#');
                return Ok(Op::Shift(s as u32, expr::parse(c, block)?));
            }
            if let Some(e) = EXTENDS.iter().position(|&e| e == name) {
                let amount = if c.peek().is_some_and(|ch| ch == '#' || ch.is_ascii_digit()) {
                    c.eat('#');
                    Some(expr::parse(c, block)?)
                } else {
                    None
                };
                return Ok(Op::Extend(e as u32, amount));
            }
        }
        c.at = save;
    }
    Ok(Op::Imm(expr::parse(c, block)?))
}

fn modifier(c: &mut Cursor) -> Result<Modifier> {
    let col = c.col();
    c.expect(':')?;
    let name = c.name().unwrap_or_default();
    c.expect(':')?;
    Ok(match name.as_str() {
        "LO12" => Modifier::Lo12,
        "ABS_G0" => Modifier::AbsG(0, true),
        "ABS_G0_NC" => Modifier::AbsG(0, false),
        "ABS_G1" => Modifier::AbsG(1, true),
        "ABS_G1_NC" => Modifier::AbsG(1, false),
        "ABS_G2" => Modifier::AbsG(2, true),
        "ABS_G2_NC" => Modifier::AbsG(2, false),
        "ABS_G3" => Modifier::AbsG(3, false),
        _ => {
            return err(
                col,
                format!("unknown relocation operator :{}:", name.to_lowercase()),
            );
        }
    })
}

fn memory(c: &mut Cursor, block: u32) -> Result<Op> {
    let col = c.col();
    let base = match c.name().and_then(|n| reg(&n)) {
        Some(r) if matches!(r.kind, Kind::X | Kind::Sp) && !(r.kind == Kind::X && r.n == 31) => r,
        _ => return err(col, "expected a 64-bit base register or SP"),
    };
    let mut index = Index::None;
    if c.eat(',') {
        let col = c.col();
        index = match simple(c, block)? {
            Op::Imm(e) => Index::Imm(e),
            Op::Mod(m, e) => Index::Mod(m, e),
            Op::Reg(r) if matches!(r.kind, Kind::X | Kind::W) => {
                let (option, amount) = if c.eat(',') {
                    match simple(c, block)? {
                        Op::Shift(0, e) => (3, Some(e)),
                        Op::Extend(e, amount) if matches!(e, 2 | 3 | 6 | 7) => (e, amount),
                        _ => return err(col, "expected LSL, UXTW, SXTW or SXTX"),
                    }
                } else {
                    (if r.kind == Kind::W { 2 } else { 3 }, None)
                };
                Index::Reg(r, option, amount)
            }
            _ => return err(col, "expected an offset or an index register"),
        };
    }
    c.expect(']')?;
    let writeback = c.eat('!');
    Ok(Op::Mem {
        base,
        index,
        writeback,
    })
}

/// Assembly context for one instruction.
pub struct Cx<'a, S: Scope> {
    pub scope: &'a S,
    /// Where the instruction goes.
    pub here: Value,
    /// Column of the mnemonic.
    pub col: usize,
}

impl<S: Scope> Cx<'_, S> {
    fn eval(&self, e: &Expr, col: usize) -> Result<Value> {
        expr::eval(e, self.scope).or_else(|msg| err(col, msg))
    }

    fn abs(&self, e: &Expr, col: usize) -> Result<i64> {
        match self.eval(e, col)? {
            Value::Abs(n) => Ok(n),
            _ => err(col, "expected a constant, not an address"),
        }
    }

    /// A branch or address target: the displacement from this instruction
    /// if it's in the same psect, otherwise a fixup.
    fn target(
        &self,
        e: &Expr,
        col: usize,
        fix: Fix,
    ) -> Result<std::result::Result<i64, (Fix, Value)>> {
        match (self.eval(e, col)?, &self.here) {
            (
                Value::Psect {
                    psect: p,
                    offset: a,
                },
                Value::Psect {
                    psect: q,
                    offset: b,
                },
            ) if p == *q && fix != Fix::Adrp => Ok(Ok(a - b)),
            (v, _) => Ok(Err((fix, v))),
        }
    }
}

fn field(v: i64, bits: u32, col: usize, what: &str) -> Result<u32> {
    if v < 0 || v >= 1 << bits {
        return err(
            col,
            format!("{what} {v} out of range 0 to {}", (1i64 << bits) - 1),
        );
    }
    Ok(v as u32)
}

fn signed(v: i64, bits: u32, col: usize, what: &str) -> Result<u32> {
    let lim = 1i64 << (bits - 1);
    if v < -lim || v >= lim {
        return err(
            col,
            format!("{what} {v} out of range {} to {}", -lim, lim - 1),
        );
    }
    Ok((v as u32) & ((1 << bits) - 1))
}

/// A displacement in instructions, for branches.
fn disp(v: i64, bits: u32, col: usize) -> Result<u32> {
    if v % 4 != 0 {
        return err(col, "branch target is not 4-byte aligned");
    }
    let range = 1i64 << (bits + 1);
    if v < -range || v >= range {
        return err(
            col,
            format!("branch target out of range (±{} KB)", range / 1024),
        );
    }
    Ok(((v / 4) as u32) & ((1 << bits) - 1))
}

fn adr_field(v: i64) -> u32 {
    let v = v as u32;
    (v & 3) << 29 | ((v >> 2) & 0x7ffff) << 5
}

/// The N:immr:imms encoding of a logical immediate, if it has one.
pub fn bitmask(value: u64, sf: u32) -> Option<u32> {
    let v = if sf == 1 {
        value
    } else {
        (value & 0xffff_ffff) | value << 32
    };
    if v == 0 || v == u64::MAX {
        return None;
    }
    let mut size = 64;
    while size > 2 {
        let half = size / 2;
        let mask = (1u64 << half) - 1;
        if v & mask != (v >> half) & mask {
            break;
        }
        size = half;
    }
    let mask = if size == 64 {
        u64::MAX
    } else {
        (1u64 << size) - 1
    };
    let elem = v & mask;
    let ones = elem.count_ones();
    let run = (1u64 << ones) - 1;
    let rotr = |x: u64, r: u32| {
        if r == 0 {
            x
        } else {
            ((x >> r) | (x << (size - r))) & mask
        }
    };
    let r = (0..size).find(|&r| rotr(elem, r) == run)?;
    let immr = (size - r) % size;
    let imms = (!((size << 1) - 1) & 0x3f) | (ones - 1);
    let n = (size == 64) as u32;
    Some(n << 12 | immr << 6 | imms)
}

/// The inverse condition, for CSET and friends.
fn invert(c: u32) -> u32 {
    c ^ 1
}

struct Args<'a, S: Scope> {
    cx: &'a Cx<'a, S>,
    ops: &'a [Operand],
    mn: &'a str,
}

impl<S: Scope> Args<'_, S> {
    fn count(&self, n: std::ops::RangeInclusive<usize>) -> Result<()> {
        if !n.contains(&self.ops.len()) {
            let want = if n.start() == n.end() {
                n.start().to_string()
            } else {
                format!("{} to {}", n.start(), n.end())
            };
            return err(self.cx.col, format!("{} takes {want} operands", self.mn));
        }
        Ok(())
    }

    fn col(&self, i: usize) -> usize {
        self.ops.get(i).map_or(self.cx.col, |o| o.col)
    }

    fn reg(&self, i: usize) -> Result<Reg> {
        match self.ops.get(i).map(|o| &o.op) {
            Some(Op::Reg(r)) => Ok(*r),
            _ => err(self.col(i), "expected a register"),
        }
    }

    /// A general register; `sp` says whether SP (and not ZR) is allowed.
    fn gp(&self, i: usize, sp: bool) -> Result<Reg> {
        let r = self.reg(i)?;
        let ok = match r.kind {
            Kind::X | Kind::W => !(sp && r.n == 31),
            Kind::Sp | Kind::Wsp => sp,
            _ => false,
        };
        if !ok {
            let want = if sp {
                "a general register or SP"
            } else {
                "a general register or ZR"
            };
            return err(self.col(i), format!("expected {want}"));
        }
        Ok(r)
    }

    fn same_size(&self, regs: &[(usize, Reg)]) -> Result<u32> {
        let sf = regs[0].1.sf();
        for &(i, r) in regs {
            if r.sf() != sf {
                return err(self.col(i), "register sizes don't match");
            }
        }
        Ok(sf)
    }

    fn is_reg(&self, i: usize) -> bool {
        matches!(self.ops.get(i).map(|o| &o.op), Some(Op::Reg(_)))
    }

    fn imm(&self, i: usize) -> Result<i64> {
        match self.ops.get(i).map(|o| &o.op) {
            Some(Op::Imm(e)) => self.cx.abs(e, self.col(i)),
            _ => err(self.col(i), "expected an immediate"),
        }
    }

    fn expr(&self, i: usize) -> Result<&Expr> {
        match self.ops.get(i).map(|o| &o.op) {
            Some(Op::Imm(e)) => Ok(e),
            _ => err(self.col(i), "expected an address or label"),
        }
    }

    /// A bare name, such as a condition or a barrier option.
    fn name(&self, i: usize) -> Option<&str> {
        match self.ops.get(i).map(|o| &o.op) {
            Some(Op::Imm(Expr::Sym(s))) => Some(s),
            _ => None,
        }
    }

    fn cond(&self, i: usize) -> Result<u32> {
        match self.name(i).and_then(cond) {
            Some(c) => Ok(c),
            None => err(self.col(i), "expected a condition such as EQ or NE"),
        }
    }

    /// An optional shift operand at `i`: (type, amount).
    fn shift(&self, i: usize, sf: u32, ror: bool) -> Result<(u32, u32)> {
        match self.ops.get(i).map(|o| &o.op) {
            None => Ok((0, 0)),
            Some(Op::Shift(s, e)) if ror || *s != 3 => Ok((
                *s,
                field(
                    self.cx.abs(e, self.col(i))?,
                    5 + sf,
                    self.col(i),
                    "shift amount",
                )?,
            )),
            _ => err(self.col(i), "expected a shift"),
        }
    }
}

fn fixed(word: u32) -> Result<Encoded> {
    Ok(Encoded { word, fixup: None })
}

/// Encodes one instruction. `mn` is the upper-case mnemonic.
pub fn encode<S: Scope>(mn: &str, ops: &[Operand], cx: &Cx<S>) -> Result<Encoded> {
    let a = Args { cx, ops, mn };
    match mn {
        "ADD" | "ADDS" | "SUB" | "SUBS" => {
            a.count(3..=4)?;
            let op = mn.starts_with("SUB") as u32;
            let s = mn.ends_with('S') as u32;
            addsub(&a, op, s, 0)
        }
        "CMP" | "CMN" => {
            a.count(2..=3)?;
            let op = (mn == "CMP") as u32;
            addsub(&a, op, 1, 1)
        }
        "NEG" | "NEGS" => {
            a.count(2..=3)?;
            let rd = a.gp(0, false)?;
            let rm = a.gp(1, false)?;
            let sf = a.same_size(&[(0, rd), (1, rm)])?;
            let (shift, amount) = a.shift(2, sf, false)?;
            let s = (mn == "NEGS") as u32;
            fixed(
                0x4b00_0000
                    | sf << 31
                    | s << 29
                    | shift << 22
                    | rm.n << 16
                    | amount << 10
                    | 31 << 5
                    | rd.n,
            )
        }
        "AND" | "ORR" | "EOR" | "ANDS" | "BIC" | "ORN" | "EON" | "BICS" => {
            a.count(3..=4)?;
            let opc = match mn {
                "AND" | "BIC" => 0,
                "ORR" | "ORN" => 1,
                "EOR" | "EON" => 2,
                _ => 3,
            };
            let invert = matches!(mn, "BIC" | "ORN" | "EON" | "BICS") as u32;
            logical(&a, opc, invert, 0)
        }
        "TST" => {
            a.count(2..=3)?;
            logical(&a, 3, 0, 1)
        }
        "MOV" => {
            a.count(2..=2)?;
            let rd = a.gp(0, true)?;
            if a.is_reg(1) {
                let rm = a.gp(1, true)?;
                let sf = a.same_size(&[(0, rd), (1, rm)])?;
                if matches!(rd.kind, Kind::Sp | Kind::Wsp)
                    || matches!(rm.kind, Kind::Sp | Kind::Wsp)
                {
                    return fixed(0x1100_0000 | sf << 31 | rm.n << 5 | rd.n);
                }
                return fixed(0x2a00_03e0 | sf << 31 | rm.n << 16 | rd.n);
            }
            let v = a.imm(1)? as u64;
            mov_imm(rd, v).ok_or(()).or_else(|()| {
                err(
                    a.col(1),
                    format!("{v:#x} can't be loaded with one MOV; use MOVZ and MOVK"),
                )
            })
        }
        "MVN" => {
            a.count(2..=3)?;
            let rd = a.gp(0, false)?;
            let rm = a.gp(1, false)?;
            let sf = a.same_size(&[(0, rd), (1, rm)])?;
            let (shift, amount) = a.shift(2, sf, true)?;
            fixed(0x2a20_03e0 | sf << 31 | shift << 22 | rm.n << 16 | amount << 10 | rd.n)
        }
        "MOVZ" | "MOVN" | "MOVK" => {
            a.count(2..=3)?;
            let rd = a.gp(0, false)?;
            let sf = rd.sf();
            let opc = match mn {
                "MOVN" => 0,
                "MOVZ" => 2,
                _ => 3,
            };
            let base = 0x1280_0000 | sf << 31 | opc << 29 | rd.n;
            if let Some(Op::Mod(Modifier::AbsG(g, check), e)) = a.ops.get(1).map(|o| &o.op) {
                a.count(2..=2)?;
                if sf == 0 && *g > 1 {
                    return err(a.col(1), "a 32-bit register only has chunks 0 and 1");
                }
                let v = cx.eval(e, a.col(1))?;
                return Ok(match v {
                    Value::Abs(n) => {
                        let n = n as u64;
                        if *check && n >> (16 * (g + 1)) != 0 {
                            return err(
                                a.col(1),
                                format!("{n:#x} doesn't fit in {} bits", 16 * (g + 1)),
                            );
                        }
                        Encoded {
                            word: base | g << 21 | ((n >> (16 * g)) as u32 & 0xffff) << 5,
                            fixup: None,
                        }
                    }
                    v => Encoded {
                        word: base | g << 21,
                        fixup: Some((Fix::Movw(*g, *check), v)),
                    },
                });
            }
            let imm = field(a.imm(1)?, 16, a.col(1), "immediate")?;
            let hw = match a.ops.get(2).map(|o| &o.op) {
                None => 0,
                Some(Op::Shift(0, e)) => {
                    let s = cx.abs(e, a.col(2))?;
                    if s % 16 != 0 || s < 0 || s >= 32 << sf {
                        return err(a.col(2), "shift must be 0, 16, 32 or 48 (0 or 16 for W)");
                    }
                    s as u32 / 16
                }
                _ => return err(a.col(2), "expected LSL #n"),
            };
            fixed(base | hw << 21 | imm << 5)
        }
        "ADR" | "ADRP" => {
            a.count(2..=2)?;
            let rd = a.gp(0, false)?;
            if rd.kind != Kind::X {
                return err(a.col(0), "expected a 64-bit register");
            }
            let (base, fix) = if mn == "ADR" {
                (0x1000_0000, Fix::Adr)
            } else {
                (0x9000_0000, Fix::Adrp)
            };
            match cx.target(a.expr(1)?, a.col(1), fix)? {
                Ok(d) => {
                    signed(d, 21, a.col(1), "ADR offset")?;
                    fixed(base | adr_field(d) | rd.n)
                }
                Err(fixup) => Ok(Encoded {
                    word: base | rd.n,
                    fixup: Some(fixup),
                }),
            }
        }
        "LSL" | "LSR" | "ASR" | "ROR" => {
            a.count(3..=3)?;
            let rd = a.gp(0, false)?;
            let rn = a.gp(1, false)?;
            if a.is_reg(2) {
                let rm = a.gp(2, false)?;
                let sf = a.same_size(&[(0, rd), (1, rn), (2, rm)])?;
                let op = ["LSL", "LSR", "ASR", "ROR"]
                    .iter()
                    .position(|&m| m == mn)
                    .unwrap() as u32;
                return fixed(0x1ac0_2000 | sf << 31 | rm.n << 16 | op << 10 | rn.n << 5 | rd.n);
            }
            let sf = a.same_size(&[(0, rd), (1, rn)])?;
            let size = 32 << sf;
            let n = field(a.imm(2)?, 5 + sf, a.col(2), "shift")?;
            let (opc, immr, imms) = match mn {
                "LSL" => (2, (size - n) % size, size - 1 - n),
                "LSR" => (2, n, size - 1),
                "ASR" => (0, n, size - 1),
                _ => {
                    return fixed(
                        0x1380_0000 | sf << 31 | sf << 22 | rn.n << 16 | n << 10 | rn.n << 5 | rd.n,
                    );
                }
            };
            fixed(bitfield(sf, opc, immr, imms, rn.n, rd.n))
        }
        "UBFM" | "SBFM" | "BFM" | "UBFX" | "SBFX" | "UBFIZ" | "SBFIZ" | "BFI" | "BFXIL" => {
            a.count(4..=4)?;
            let rd = a.gp(0, false)?;
            let rn = a.gp(1, false)?;
            let sf = a.same_size(&[(0, rd), (1, rn)])?;
            let size = 32i64 << sf;
            let (x, y) = (a.imm(2)?, a.imm(3)?);
            let opc = match mn {
                "SBFM" | "SBFX" | "SBFIZ" => 0,
                "BFM" | "BFI" | "BFXIL" => 1,
                _ => 2,
            };
            let (immr, imms) = match mn {
                "UBFM" | "SBFM" | "BFM" => (x, y),
                // lsb, width
                "UBFX" | "SBFX" | "BFXIL" if x >= 0 && y > 0 && x + y <= size => (x, x + y - 1),
                "UBFIZ" | "SBFIZ" | "BFI" if x >= 0 && y > 0 && x + y <= size => {
                    ((size - x) % size, y - 1)
                }
                _ => return err(a.col(3), "bit field doesn't fit in the register"),
            };
            let immr = field(immr, 5 + sf, a.col(2), "bit position")?;
            let imms = field(imms, 5 + sf, a.col(3), "bit position")?;
            fixed(bitfield(sf, opc, immr, imms, rn.n, rd.n))
        }
        "SXTB" | "SXTH" | "SXTW" | "UXTB" | "UXTH" => {
            a.count(2..=2)?;
            let rd = a.gp(0, false)?;
            let rn = a.gp(1, false)?;
            if rn.kind != Kind::W {
                return err(a.col(1), "expected a 32-bit source register");
            }
            let dest_ok = match mn {
                "SXTW" => rd.kind == Kind::X,
                "UXTB" | "UXTH" => rd.kind == Kind::W,
                _ => true,
            };
            if !dest_ok {
                return err(a.col(0), "wrong destination register size");
            }
            let sf = rd.sf();
            let imms = match &mn[3..] {
                "B" => 7,
                "H" => 15,
                _ => 31,
            };
            let opc = if mn.starts_with('S') { 0 } else { 2 };
            fixed(bitfield(sf, opc, 0, imms, rn.n, rd.n))
        }
        "EXTR" => {
            a.count(4..=4)?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            let sf = a.same_size(&[(0, rd), (1, rn), (2, rm)])?;
            let lsb = field(a.imm(3)?, 5 + sf, a.col(3), "bit position")?;
            fixed(0x1380_0000 | sf << 31 | sf << 22 | rm.n << 16 | lsb << 10 | rn.n << 5 | rd.n)
        }
        "UDIV" | "SDIV" | "LSLV" | "LSRV" | "ASRV" | "RORV" => {
            a.count(3..=3)?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            let sf = a.same_size(&[(0, rd), (1, rn), (2, rm)])?;
            let opcode = match mn {
                "UDIV" => 2,
                "SDIV" => 3,
                "LSLV" => 8,
                "LSRV" => 9,
                "ASRV" => 10,
                _ => 11,
            };
            fixed(0x1ac0_0000 | sf << 31 | rm.n << 16 | opcode << 10 | rn.n << 5 | rd.n)
        }
        "MADD" | "MSUB" | "MUL" | "MNEG" => {
            let three = matches!(mn, "MUL" | "MNEG");
            a.count(if three { 3..=3 } else { 4..=4 })?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            let ra = if three {
                Reg {
                    kind: rd.kind,
                    n: 31,
                }
            } else {
                a.gp(3, false)?
            };
            let sf = a.same_size(&[(0, rd), (1, rn), (2, rm), (3, ra)])?;
            let o0 = matches!(mn, "MSUB" | "MNEG") as u32;
            fixed(0x1b00_0000 | sf << 31 | rm.n << 16 | o0 << 15 | ra.n << 10 | rn.n << 5 | rd.n)
        }
        "SMADDL" | "UMADDL" | "SMSUBL" | "UMSUBL" | "SMULL" | "UMULL" => {
            let three = mn.ends_with("MULL");
            a.count(if three { 3..=3 } else { 4..=4 })?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            let ra = if three {
                Reg {
                    kind: Kind::X,
                    n: 31,
                }
            } else {
                a.gp(3, false)?
            };
            if rd.kind != Kind::X || rn.kind != Kind::W || rm.kind != Kind::W || ra.kind != Kind::X
            {
                return err(
                    cx.col,
                    format!("{mn} takes Xd, Wn, Wm{}", if three { "" } else { ", Xa" }),
                );
            }
            let u = mn.starts_with('U') as u32;
            let o0 = mn.contains("SUB") as u32;
            fixed(0x9b20_0000 | u << 23 | rm.n << 16 | o0 << 15 | ra.n << 10 | rn.n << 5 | rd.n)
        }
        "SMULH" | "UMULH" => {
            a.count(3..=3)?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            if a.same_size(&[(0, rd), (1, rn), (2, rm)])? != 1 {
                return err(a.col(0), "expected 64-bit registers");
            }
            let u = (mn == "UMULH") as u32;
            fixed(0x9b40_7c00 | u << 23 | rm.n << 16 | rn.n << 5 | rd.n)
        }
        "CSEL" | "CSINC" | "CSINV" | "CSNEG" => {
            a.count(4..=4)?;
            let (rd, rn, rm) = (a.gp(0, false)?, a.gp(1, false)?, a.gp(2, false)?);
            let sf = a.same_size(&[(0, rd), (1, rn), (2, rm)])?;
            let (op, op2) = match mn {
                "CSEL" => (0, 0),
                "CSINC" => (0, 1),
                "CSINV" => (1, 0),
                _ => (1, 1),
            };
            fixed(condsel(sf, op, op2, rm.n, a.cond(3)?, rn.n, rd.n))
        }
        "CSET" | "CSETM" => {
            a.count(2..=2)?;
            let rd = a.gp(0, false)?;
            let c = a.cond(1)?;
            if c >= 14 {
                return err(a.col(1), "AL and NV can't be inverted");
            }
            // CSINC or CSINV from the zero register, on the opposite condition.
            let op = (mn == "CSETM") as u32;
            fixed(condsel(rd.sf(), op, 1 - op, 31, invert(c), 31, rd.n))
        }
        "CINC" | "CINV" | "CNEG" => {
            a.count(3..=3)?;
            let (rd, rn) = (a.gp(0, false)?, a.gp(1, false)?);
            let sf = a.same_size(&[(0, rd), (1, rn)])?;
            let c = a.cond(2)?;
            if c >= 14 {
                return err(a.col(2), "AL and NV can't be inverted");
            }
            let (op, op2) = match mn {
                "CINC" => (0, 1),
                "CINV" => (1, 0),
                _ => (1, 1),
            };
            fixed(condsel(sf, op, op2, rn.n, invert(c), rn.n, rd.n))
        }
        "RBIT" | "REV16" | "REV32" | "REV" | "CLZ" | "CLS" => {
            a.count(2..=2)?;
            let (rd, rn) = (a.gp(0, false)?, a.gp(1, false)?);
            let sf = a.same_size(&[(0, rd), (1, rn)])?;
            let opcode = match (mn, sf) {
                ("RBIT", _) => 0,
                ("REV16", _) => 1,
                ("REV32", 1) | ("REV", 0) => 2,
                ("REV", _) => 3,
                ("CLZ", _) => 4,
                ("CLS", _) => 5,
                _ => return err(a.col(0), "REV32 needs 64-bit registers"),
            };
            fixed(0x5ac0_0000 | sf << 31 | opcode << 10 | rn.n << 5 | rd.n)
        }
        "B" | "BL" => {
            a.count(1..=1)?;
            let base = if mn == "B" { 0x1400_0000 } else { 0x9400_0000 };
            branch(&a, base, 0, Fix::Jump26, 26, a.expr(0)?, 0)
        }
        _ if mn.starts_with("B.") => {
            a.count(1..=1)?;
            let c = match cond(&mn[2..]) {
                Some(c) => c,
                None => return err(cx.col, format!("unknown condition in {mn}")),
            };
            branch(&a, 0x5400_0000 | c, 0, Fix::Branch19, 19, a.expr(0)?, 5)
        }
        "CBZ" | "CBNZ" => {
            a.count(2..=2)?;
            let rt = a.gp(0, false)?;
            let base = 0x3400_0000 | rt.sf() << 31 | ((mn == "CBNZ") as u32) << 24 | rt.n;
            branch(&a, base, 1, Fix::Branch19, 19, a.expr(1)?, 5)
        }
        "TBZ" | "TBNZ" => {
            a.count(3..=3)?;
            let rt = a.gp(0, false)?;
            let bit = field(a.imm(1)?, 5 + rt.sf(), a.col(1), "bit number")?;
            let base = 0x3600_0000
                | (bit >> 5) << 31
                | ((mn == "TBNZ") as u32) << 24
                | (bit & 31) << 19
                | rt.n;
            branch(&a, base, 2, Fix::Branch14, 14, a.expr(2)?, 5)
        }
        "BR" | "BLR" | "RET" => {
            let rn = if mn == "RET" && ops.is_empty() {
                30
            } else {
                a.count(1..=1)?;
                let r = a.gp(0, false)?;
                if r.kind != Kind::X {
                    return err(a.col(0), "expected a 64-bit register");
                }
                r.n
            };
            let base = match mn {
                "BR" => 0xd61f_0000,
                "BLR" => 0xd63f_0000,
                _ => 0xd65f_0000,
            };
            fixed(base | rn << 5)
        }
        "ERET" => {
            a.count(0..=0)?;
            fixed(0xd69f_03e0)
        }
        "SVC" | "HVC" | "SMC" | "BRK" | "HLT" | "UDF" => {
            a.count(1..=1)?;
            let imm = field(a.imm(0)?, 16, a.col(0), "immediate")?;
            let base = match mn {
                "SVC" => 0xd400_0001,
                "HVC" => 0xd400_0002,
                "SMC" => 0xd400_0003,
                "BRK" => 0xd420_0000,
                "HLT" => 0xd440_0000,
                _ => 0,
            };
            fixed(base | if mn == "UDF" { imm } else { imm << 5 })
        }
        "NOP" | "YIELD" | "WFE" | "WFI" | "SEV" | "SEVL" | "HINT" => {
            let n = match mn {
                "HINT" => {
                    a.count(1..=1)?;
                    field(a.imm(0)?, 7, a.col(0), "hint")?
                }
                _ => {
                    a.count(0..=0)?;
                    ["NOP", "YIELD", "WFE", "WFI", "SEV", "SEVL"]
                        .iter()
                        .position(|&h| h == mn)
                        .unwrap() as u32
                }
            };
            fixed(0xd503_201f | n << 5)
        }
        "DMB" | "DSB" | "ISB" => {
            let crm = if mn == "ISB" && ops.is_empty() {
                15
            } else {
                a.count(1..=1)?;
                const OPTIONS: [(&str, u32); 12] = [
                    ("SY", 15),
                    ("ST", 14),
                    ("LD", 13),
                    ("ISH", 11),
                    ("ISHST", 10),
                    ("ISHLD", 9),
                    ("NSH", 7),
                    ("NSHST", 6),
                    ("NSHLD", 5),
                    ("OSH", 3),
                    ("OSHST", 2),
                    ("OSHLD", 1),
                ];
                match a.name(0).and_then(|n| OPTIONS.iter().find(|o| o.0 == n)) {
                    Some(&(_, v)) => v,
                    None => field(a.imm(0)?, 4, a.col(0), "barrier option")?,
                }
            };
            let op2 = match mn {
                "DSB" => 4,
                "DMB" => 5,
                _ => 6,
            };
            fixed(0xd503_301f | crm << 8 | op2 << 5)
        }
        "MRS" => {
            a.count(2..=2)?;
            let rt = a.gp(0, false)?;
            let sr = sysreg(&a, 1)?;
            fixed(0xd530_0000 | (sr & 0x7fff) << 5 | rt.n)
        }
        "MSR" => {
            a.count(2..=2)?;
            const PSTATE: [(&str, u32, u32); 3] =
                [("SPSEL", 0, 5), ("DAIFSET", 3, 6), ("DAIFCLR", 3, 7)];
            if let Some(&(_, op1, op2)) = a.name(0).and_then(|n| PSTATE.iter().find(|p| p.0 == n)) {
                let imm = field(a.imm(1)?, 4, a.col(1), "immediate")?;
                return fixed(0xd500_401f | op1 << 16 | imm << 8 | op2 << 5);
            }
            let sr = sysreg(&a, 0)?;
            let rt = a.gp(1, false)?;
            fixed(0xd510_0000 | (sr & 0x7fff) << 5 | rt.n)
        }
        "LDR" | "STR" | "LDRB" | "STRB" | "LDRH" | "STRH" | "LDRSB" | "LDRSH" | "LDRSW"
        | "LDUR" | "STUR" | "LDURB" | "STURB" | "LDURH" | "STURH" | "LDURSB" | "LDURSH"
        | "LDURSW" => load_store(&a),
        "LDP" | "STP" | "LDPSW" => pair(&a),
        "LDXR" | "LDAXR" | "LDAR" | "STLR" => {
            a.count(2..=2)?;
            let rt = a.gp(0, false)?;
            let rn = bare_base(&a, 1)?;
            let base = match mn {
                "LDXR" => 0x885f_7c00,
                "LDAXR" => 0x885f_fc00,
                "LDAR" => 0x88df_fc00,
                _ => 0x889f_fc00,
            };
            fixed(base | rt.sf() << 30 | rn << 5 | rt.n)
        }
        "STXR" | "STLXR" => {
            a.count(3..=3)?;
            let rs = a.gp(0, false)?;
            let rt = a.gp(1, false)?;
            if rs.kind != Kind::W {
                return err(a.col(0), "the status register must be 32-bit");
            }
            let rn = bare_base(&a, 2)?;
            let base = if mn == "STXR" {
                0x8800_7c00
            } else {
                0x8800_fc00
            };
            fixed(base | rt.sf() << 30 | rs.n << 16 | rn << 5 | rt.n)
        }
        _ => err(cx.col, format!("unknown instruction {}", mn.to_lowercase())),
    }
}

fn bitfield(sf: u32, opc: u32, immr: u32, imms: u32, rn: u32, rd: u32) -> u32 {
    0x1300_0000 | sf << 31 | opc << 29 | sf << 22 | immr << 16 | imms << 10 | rn << 5 | rd
}

fn condsel(sf: u32, op: u32, op2: u32, rm: u32, cond: u32, rn: u32, rd: u32) -> u32 {
    0x1a80_0000 | sf << 31 | op << 30 | rm << 16 | cond << 12 | op2 << 10 | rn << 5 | rd
}

/// ADD, SUB and their S forms; `zr` operands are implied (CMP, CMN).
fn addsub<S: Scope>(a: &Args<S>, op: u32, s: u32, implied: usize) -> Result<Encoded> {
    // Operand positions, with Rd missing for CMP/CMN.
    let (rd, n) = if implied == 1 {
        (None, 0)
    } else {
        (Some(a.gp(0, s == 0)?), 1)
    };
    let rn = a.gp(n, true)?;
    let rd = rd.unwrap_or(Reg {
        kind: if rn.sf() == 1 { Kind::X } else { Kind::W },
        n: 31,
    });
    let sf = a.same_size(&[(0, rd), (n, rn)])?;
    let base = sf << 31 | op << 30 | s << 29 | rn.n << 5 | rd.n;
    let sp = matches!(rd.kind, Kind::Sp | Kind::Wsp) || matches!(rn.kind, Kind::Sp | Kind::Wsp);
    match a.ops.get(n + 1).map(|o| &o.op) {
        Some(Op::Reg(_)) => {
            let rm = a.gp(n + 1, false)?;
            let extend = matches!(a.ops.get(n + 2).map(|o| &o.op), Some(Op::Extend(..)));
            if extend || sp {
                // Extended register: needed for SP, or when an extend is given.
                let (option, amount) = match a.ops.get(n + 2).map(|o| &o.op) {
                    Some(Op::Extend(e, amount)) => (*e, amount.as_ref()),
                    Some(Op::Shift(0, amount)) => (2 + sf, Some(amount)),
                    None => (2 + sf, None),
                    _ => return err(a.col(n + 2), "expected an extend such as UXTW"),
                };
                let amount = match amount {
                    Some(e) => field(a.cx.abs(e, a.col(n + 2))?, 3, a.col(n + 2), "extend amount")?,
                    None => 0,
                };
                if amount > 4 {
                    return err(a.col(n + 2), "extend amount must be 0 to 4");
                }
                let want_x = option & 3 == 3;
                if (rm.kind == Kind::X) != want_x {
                    return err(a.col(n + 1), "register size doesn't match the extend");
                }
                return fixed(0x0b20_0000 | base | rm.n << 16 | option << 13 | amount << 10);
            }
            a.same_size(&[(0, rd), (n + 1, rm)])?;
            let (shift, amount) = a.shift(n + 2, sf, false)?;
            fixed(0x0b00_0000 | base | shift << 22 | rm.n << 16 | amount << 10)
        }
        Some(Op::Mod(Modifier::Lo12, e)) => {
            if op != 0 || s != 0 {
                return err(a.col(n + 1), ":lo12: only works with ADD");
            }
            match a.cx.eval(e, a.col(n + 1))? {
                Value::Abs(v) => fixed(0x1100_0000 | base | ((v as u32) & 0xfff) << 10),
                v => Ok(Encoded {
                    word: 0x1100_0000 | base,
                    fixup: Some((Fix::AddLo12, v)),
                }),
            }
        }
        _ => {
            let mut imm = a.imm(n + 1)?;
            let mut sh = 0;
            match a.ops.get(n + 2).map(|o| &o.op) {
                None => {}
                Some(Op::Shift(0, e)) if matches!(a.cx.abs(e, a.col(n + 2))?, 0 | 12) => {
                    sh = (a.cx.abs(e, a.col(n + 2))? == 12) as u32;
                }
                _ => return err(a.col(n + 2), "expected LSL #0 or LSL #12"),
            }
            // A negative immediate is the opposite operation.
            let mut op = op;
            if imm < 0 {
                imm = -imm;
                op ^= 1;
            }
            if sh == 0 && imm >= 1 << 12 && imm & 0xfff == 0 {
                imm >>= 12;
                sh = 1;
            }
            let imm = field(imm, 12, a.col(n + 1), "immediate")?;
            fixed(0x1100_0000 | (base & !(1 << 30)) | op << 30 | sh << 22 | imm << 10)
        }
    }
}

/// AND, ORR, EOR, ANDS and the inverted forms; TST has an implied ZR.
fn logical<S: Scope>(a: &Args<S>, opc: u32, invert: u32, implied: usize) -> Result<Encoded> {
    let (rd, n) = if implied == 1 {
        (None, 0)
    } else {
        (Some(a.gp(0, opc != 3 && !a.is_reg(2))?), 1)
    };
    let rn = a.gp(n, false)?;
    let rd = rd.unwrap_or(Reg {
        kind: if rn.sf() == 1 { Kind::X } else { Kind::W },
        n: 31,
    });
    let sf = a.same_size(&[(0, rd), (n, rn)])?;
    let base = sf << 31 | opc << 29 | rn.n << 5 | rd.n;
    if a.is_reg(n + 1) {
        let rm = a.gp(n + 1, false)?;
        a.same_size(&[(0, rd), (n + 1, rm)])?;
        let (shift, amount) = a.shift(n + 2, sf, true)?;
        return fixed(0x0a00_0000 | base | shift << 22 | invert << 21 | rm.n << 16 | amount << 10);
    }
    a.count(n + 2..=n + 2)?;
    let mut v = a.imm(n + 1)? as u64;
    if invert == 1 {
        v = !v;
    }
    match bitmask(v, sf) {
        Some(bits) if sf == 1 || bits & (1 << 12) == 0 => fixed(0x1200_0000 | base | bits << 10),
        _ => err(
            a.col(n + 1),
            format!("{v:#x} is not a valid logical immediate"),
        ),
    }
}

/// MOV with an immediate: MOVZ, MOVN or ORR, as GNU `as` picks.
fn mov_imm(rd: Reg, v: u64) -> Option<Encoded> {
    let sf = rd.sf();
    let v = if sf == 1 { v } else { v & 0xffff_ffff };
    let chunks = if sf == 1 { 4 } else { 2 };
    let mask = if sf == 1 { u64::MAX } else { 0xffff_ffff };
    let single = |x: u64| (0..chunks).find(|&h| x & !(0xffff << (16 * h)) & mask == 0);
    // MOVZ and MOVN can't write SP: register 31 is the zero register there.
    let sp = matches!(rd.kind, Kind::Sp | Kind::Wsp);
    let word = if let Some(h) = single(v).filter(|_| !sp) {
        0x5280_0000 | h << 21 | ((v >> (16 * h)) as u32 & 0xffff) << 5
    } else if let Some(h) = single(!v & mask).filter(|_| !sp) {
        0x1280_0000 | h << 21 | ((!v >> (16 * h)) as u32 & 0xffff) << 5
    } else {
        0x3200_03e0 | bitmask(v, sf)? << 10
    };
    Some(Encoded {
        word: word | sf << 31 | rd.n,
        fixup: None,
    })
}

/// A branch: direct if the target is in this psect, otherwise a fixup.
fn branch<S: Scope>(
    a: &Args<S>,
    base: u32,
    i: usize,
    fix: Fix,
    bits: u32,
    target: &Expr,
    shift: u32,
) -> Result<Encoded> {
    match a.cx.target(target, a.col(i), fix)? {
        Ok(d) => fixed(base | disp(d, bits, a.col(i))? << shift),
        Err(fixup) => Ok(Encoded {
            word: base,
            fixup: Some(fixup),
        }),
    }
}

/// `[Xn]` with no offset, for exclusive and ordered accesses.
fn bare_base<S: Scope>(a: &Args<S>, i: usize) -> Result<u32> {
    match a.ops.get(i).map(|o| &o.op) {
        Some(Op::Mem {
            base,
            index: Index::None,
            writeback: false,
        }) => Ok(base.n),
        Some(Op::Mem {
            index: Index::Imm(Expr::Num(0)),
            base,
            writeback: false,
        }) => Ok(base.n),
        _ => err(a.col(i), "expected [Xn]"),
    }
}

/// The system register at operand `i`: o0:op1:CRn:CRm:op2 in 16 bits.
fn sysreg<S: Scope>(a: &Args<S>, i: usize) -> Result<u32> {
    const REGS: [(&str, u32, u32, u32, u32, u32); 31] = [
        ("NZCV", 3, 3, 4, 2, 0),
        ("DAIF", 3, 3, 4, 2, 1),
        ("FPCR", 3, 3, 4, 4, 0),
        ("FPSR", 3, 3, 4, 4, 1),
        ("CTR_EL0", 3, 3, 0, 0, 1),
        ("DCZID_EL0", 3, 3, 0, 0, 7),
        ("TPIDR_EL0", 3, 3, 13, 0, 2),
        ("TPIDRRO_EL0", 3, 3, 13, 0, 3),
        ("CNTFRQ_EL0", 3, 3, 14, 0, 0),
        ("CNTPCT_EL0", 3, 3, 14, 0, 1),
        ("CNTVCT_EL0", 3, 3, 14, 0, 2),
        ("MIDR_EL1", 3, 0, 0, 0, 0),
        ("MPIDR_EL1", 3, 0, 0, 0, 5),
        ("ID_AA64MMFR0_EL1", 3, 0, 0, 7, 0),
        ("SCTLR_EL1", 3, 0, 1, 0, 0),
        ("CPACR_EL1", 3, 0, 1, 0, 2),
        ("TTBR0_EL1", 3, 0, 2, 0, 0),
        ("TTBR1_EL1", 3, 0, 2, 0, 1),
        ("TCR_EL1", 3, 0, 2, 0, 2),
        ("SPSR_EL1", 3, 0, 4, 0, 0),
        ("ELR_EL1", 3, 0, 4, 0, 1),
        ("SP_EL0", 3, 0, 4, 1, 0),
        ("SPSEL", 3, 0, 4, 2, 0),
        ("CURRENTEL", 3, 0, 4, 2, 2),
        ("ESR_EL1", 3, 0, 5, 2, 0),
        ("FAR_EL1", 3, 0, 6, 0, 0),
        ("MAIR_EL1", 3, 0, 10, 2, 0),
        ("VBAR_EL1", 3, 0, 12, 0, 0),
        ("TPIDR_EL1", 3, 0, 13, 0, 4),
        ("CNTKCTL_EL1", 3, 0, 14, 1, 0),
        ("CNTV_CTL_EL0", 3, 3, 14, 3, 1),
    ];
    let pack = |op0: u32, op1: u32, crn: u32, crm: u32, op2: u32| {
        (op0 - 2) << 14 | op1 << 11 | crn << 7 | crm << 3 | op2
    };
    let Some(name) = a.name(i) else {
        return err(a.col(i), "expected a system register");
    };
    if let Some(&(_, op0, op1, crn, crm, op2)) = REGS.iter().find(|r| r.0 == name) {
        return Ok(pack(op0, op1, crn, crm, op2));
    }
    // The generic form, S<op0>_<op1>_C<n>_C<m>_<op2>.
    let parts: Vec<&str> = name.strip_prefix('S').unwrap_or("").split('_').collect();
    let num = |s: &str, max: u32| s.parse::<u32>().ok().filter(|&v| v <= max);
    if let [op0, op1, crn, crm, op2] = parts[..]
        && let (Some(op0 @ 2..=3), Some(op1), Some(crn), Some(crm), Some(op2)) = (
            num(op0, 3),
            num(op1, 7),
            crn.strip_prefix('C').and_then(|c| num(c, 15)),
            crm.strip_prefix('C').and_then(|c| num(c, 15)),
            num(op2, 7),
        )
    {
        return Ok(pack(op0, op1, crn, crm, op2));
    }
    err(
        a.col(i),
        format!("unknown system register {}", name.to_lowercase()),
    )
}

/// The size, V bit and opc of a load or store, and log2 of the access size.
fn ldst_spec(mn: &str, rt: Reg, col: usize) -> Result<(u32, u32, u32, u32)> {
    let load = mn.starts_with("LD") as u32;
    let base = mn.replace("UR", "R");
    let (size, v, opc) = match (base.as_str(), rt.kind) {
        ("LDR" | "STR", Kind::X) => (3, 0, load),
        ("LDR" | "STR", Kind::W) => (2, 0, load),
        ("LDR" | "STR", Kind::B) => (0, 1, load),
        ("LDR" | "STR", Kind::H) => (1, 1, load),
        ("LDR" | "STR", Kind::S) => (2, 1, load),
        ("LDR" | "STR", Kind::D) => (3, 1, load),
        ("LDR" | "STR", Kind::Q) => (0, 1, 2 | load),
        ("LDRB" | "STRB", Kind::W) => (0, 0, load),
        ("LDRH" | "STRH", Kind::W) => (1, 0, load),
        ("LDRSB", Kind::W) => (0, 0, 3),
        ("LDRSB", Kind::X) => (0, 0, 2),
        ("LDRSH", Kind::W) => (1, 0, 3),
        ("LDRSH", Kind::X) => (1, 0, 2),
        ("LDRSW", Kind::X) => (2, 0, 2),
        _ => {
            return err(
                col,
                format!("{} can't use this register", mn.to_lowercase()),
            );
        }
    };
    let scale = if rt.kind == Kind::Q { 4 } else { size };
    Ok((size, v, opc, scale))
}

fn load_store<S: Scope>(a: &Args<S>) -> Result<Encoded> {
    a.count(2..=3)?;
    let rt = a.reg(0)?;
    if rt.is_gp() && matches!(rt.kind, Kind::Sp | Kind::Wsp) {
        return err(a.col(0), "SP can't be loaded or stored");
    }
    let (size, v, opc, scale) = ldst_spec(a.mn, rt, a.col(0))?;
    let unscaled = a.mn.contains("UR");
    let head = size << 30 | v << 26 | opc << 22 | rt.n;
    let Some(Op::Mem {
        base,
        index,
        writeback,
    }) = a.ops.get(1).map(|o| &o.op)
    else {
        // LDR (literal): a PC-relative address.
        let lit = match (a.mn, rt.kind) {
            ("LDR", Kind::W) => 0x1800_0000,
            ("LDR", Kind::X) => 0x5800_0000,
            ("LDRSW", Kind::X) => 0x9800_0000,
            ("LDR", Kind::S) => 0x1c00_0000,
            ("LDR", Kind::D) => 0x5c00_0000,
            ("LDR", Kind::Q) => 0x9c00_0000,
            _ => return err(a.col(1), "expected a memory operand like [x1, #8]"),
        };
        return branch(a, lit | rt.n, 1, Fix::Branch19, 19, a.expr(1)?, 5);
    };
    let rn = base.n << 5;
    if let Some(post) = a.ops.get(2) {
        // Post-index: [Xn], #imm
        if *writeback || *index != Index::None || unscaled {
            return err(post.col, "post-index needs [Xn], #imm");
        }
        let imm = signed(a.imm(2)?, 9, post.col, "offset")?;
        return fixed(0x3800_0400 | head | imm << 12 | rn);
    }
    match index {
        Index::None | Index::Imm(_) => {
            let off = match index {
                Index::Imm(e) => a.cx.abs(e, a.col(1))?,
                _ => 0,
            };
            if *writeback {
                let imm = signed(off, 9, a.col(1), "offset")?;
                return fixed(0x3800_0c00 | head | imm << 12 | rn);
            }
            let fits = off >= 0 && off % (1 << scale) == 0 && off >> scale < 1 << 12;
            if !unscaled && fits {
                return fixed(0x3900_0000 | head | ((off >> scale) as u32) << 10 | rn);
            }
            let imm = signed(off, 9, a.col(1), "offset")?;
            fixed(0x3800_0000 | head | imm << 12 | rn)
        }
        Index::Mod(Modifier::Lo12, e) if !unscaled && !writeback => match a.cx.eval(e, a.col(1))? {
            Value::Abs(off) => {
                let off = off & 0xfff;
                if off % (1 << scale) != 0 {
                    return err(a.col(1), "offset is not aligned to the access size");
                }
                fixed(0x3900_0000 | head | ((off >> scale) as u32) << 10 | rn)
            }
            v => Ok(Encoded {
                word: 0x3900_0000 | head | rn,
                fixup: Some((Fix::Ldst(scale), v)),
            }),
        },
        Index::Reg(rm, option, amount) if !unscaled && !writeback => {
            let s = match amount {
                None => 0,
                Some(e) => match a.cx.abs(e, a.col(1))? {
                    0 if scale == 0 => 1,
                    0 => 0,
                    n if n == scale as i64 => 1,
                    _ => return err(a.col(1), format!("shift must be 0 or {scale}")),
                },
            };
            if (rm.kind == Kind::X) != (option & 1 == 1) {
                return err(a.col(1), "index register size doesn't match the extend");
            }
            fixed(0x3820_0800 | head | rm.n << 16 | option << 13 | s << 12 | rn)
        }
        _ => err(a.col(1), "this addressing mode doesn't fit the instruction"),
    }
}

fn pair<S: Scope>(a: &Args<S>) -> Result<Encoded> {
    a.count(3..=4)?;
    let (rt, rt2) = (a.reg(0)?, a.reg(1)?);
    if rt.kind != rt2.kind || matches!(rt.kind, Kind::Sp | Kind::Wsp | Kind::B | Kind::H) {
        return err(a.col(1), "registers must be the same kind: X, W, S, D or Q");
    }
    let (opc, v, scale) = match (a.mn, rt.kind) {
        ("LDPSW", Kind::X) => (1, 0, 2),
        ("LDPSW", _) => return err(a.col(0), "LDPSW loads X registers"),
        (_, Kind::W) => (0, 0, 2),
        (_, Kind::X) => (2, 0, 3),
        (_, Kind::S) => (0, 1, 2),
        (_, Kind::D) => (1, 1, 3),
        _ => (2, 1, 4),
    };
    let l = (a.mn != "STP") as u32;
    let Some(Op::Mem {
        base,
        index,
        writeback,
    }) = a.ops.get(2).map(|o| &o.op)
    else {
        return err(a.col(2), "expected a memory operand like [sp, #16]");
    };
    let (mode, off) = match (a.ops.get(3), index, writeback) {
        (Some(_), Index::None, false) => (1, a.imm(3)?),
        (None, Index::None, w) => (if *w { 3 } else { 2 }, 0),
        (None, Index::Imm(e), w) => (if *w { 3 } else { 2 }, a.cx.abs(e, a.col(2))?),
        _ => return err(a.col(2), "expected [Xn, #imm], [Xn, #imm]! or [Xn], #imm"),
    };
    if off % (1 << scale) != 0 {
        return err(a.col(2), "offset is not aligned to the register size");
    }
    let imm7 = signed(off >> scale, 7, a.col(2), "offset")?;
    fixed(
        0x2800_0000
            | opc << 30
            | v << 26
            | mode << 23
            | l << 22
            | imm7 << 15
            | rt2.n << 10
            | base.n << 5
            | rt.n,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_immediates() {
        // N:immr:imms; tests/encode.rs checks the same values against GNU as.
        assert_eq!(bitmask(0xff, 1), Some(0x1007));
        assert_eq!(bitmask(0x8000_0000, 0), Some(0x0040));
        assert_eq!(bitmask(0x5555_5555_5555_5555, 1), Some(0x003c));
        assert_eq!(bitmask(0xffff_ffff_ffff_fff0, 1), Some(0x1f3b));
        assert_eq!(bitmask(0, 1), None);
        assert_eq!(bitmask(u64::MAX, 1), None);
        assert_eq!(bitmask(0x1234, 1), None);
    }
}
