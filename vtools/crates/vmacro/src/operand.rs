//! VAX operand specifiers, and the ARM64 code that reads and writes them.
//! `docs/macro32.md` has the register map.

/// Operand sizes: byte, word, longword, quadword.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    B,
    W,
    L,
    Q,
}

impl Size {
    pub fn bytes(self) -> i64 {
        1 << self.shift()
    }
    pub fn shift(self) -> u8 {
        match self {
            Size::B => 0,
            Size::W => 1,
            Size::L => 2,
            Size::Q => 3,
        }
    }
    pub fn bits(self) -> i64 {
        8 * self.bytes()
    }
}

/// How a byte or word value is widened to 32 bits: not at all (only its
/// low bits count), with its sign, or with zeros.
#[derive(Clone, Copy, PartialEq)]
pub enum Ext {
    Any,
    Sext,
    Zext,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Opnd {
    Reg(u8),
    /// `#expr`, `S^#expr`, `I^#expr`.
    Imm(String),
    /// A memory mode, maybe indexed by a register: `mode[Rx]`.
    Mem(Mode, Option<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// `(Rn)`
    Def(u8),
    /// `(Rn)+`
    Inc(u8),
    /// `-(Rn)`
    Dec(u8),
    /// `@(Rn)+`
    IncDef(u8),
    /// `d(Rn)`
    Disp(String, u8),
    /// `@d(Rn)`
    DispDef(String, u8),
    /// `@#address`
    Abs(String),
    /// `address`, PC-relative on the VAX.
    Rel(String),
    /// `G^address`: an address anywhere, as in another image; the same as
    /// `address`, but a jump or call to it reaches past `bl`'s ±128 MB.
    Gen(String),
    /// `@address`
    RelDef(String),
}

/// A VAX register by name: R0-R15, AP, FP, SP, PC.
pub fn reg(text: &str) -> Option<u8> {
    let t = text.trim().to_ascii_uppercase();
    match t.as_str() {
        "AP" => Some(12),
        "FP" => Some(13),
        "SP" => Some(14),
        "PC" => Some(15),
        _ => (0..16).find(|n| t == format!("R{n}")),
    }
}

/// The ARM64 register that holds VAX register `n` (DESIGN-0004): R0 and R1
/// in x0 and x1, R2-R11 in x19-x28, AAPCS64's saved registers, AP in x12, FP
/// in x29 and SP in x18.
pub fn arm(n: u8) -> Result<u8> {
    match n {
        0 | 1 | 12 => Ok(n),
        2..=11 => Ok(n + 17),
        13 => Ok(29),
        14 => Ok(SP),
        _ => Err("PC can't be used as a register".into()),
    }
}

/// The VAX stack pointer.
pub const SP: u8 = 18;

/// Splits operands at commas outside `()`, `[]` and `<>`.
pub fn split(text: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth) = (Vec::new(), String::new(), 0i32);
    for c in text.chars() {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Parses one operand specifier.
pub fn parse(text: &str) -> Result<Opnd> {
    let t = text.trim();
    if t.is_empty() {
        return Err("missing operand".into());
    }
    if let Some(inner) = t.strip_suffix(']') {
        let open = inner.rfind('[').ok_or("missing [")?;
        let index = reg(&inner[open + 1..]).ok_or("expected an index register")?;
        return match parse(&inner[..open])? {
            Opnd::Mem(mode, None) => Ok(Opnd::Mem(mode, Some(index))),
            _ => Err("only a memory operand can be indexed".into()),
        };
    }
    if let Some(n) = reg(t) {
        return Ok(Opnd::Reg(n));
    }
    let upper = t.to_ascii_uppercase();
    for p in ["S^#", "I^#", "#"] {
        if upper.starts_with(p) {
            return Ok(Opnd::Imm(t[p.len()..].trim().to_string()));
        }
    }
    let (deferred, t) = match t.strip_prefix('@') {
        Some(rest) => (true, rest.trim()),
        None => (false, t),
    };
    if deferred && let Some(e) = t.strip_prefix('#') {
        return Ok(Opnd::Mem(Mode::Abs(e.trim().to_string()), None));
    }
    let general = !deferred && t.get(..2).is_some_and(|q| q.eq_ignore_ascii_case("G^"));
    let t = ["B^", "W^", "L^", "G^"]
        .iter()
        .find(|p| t.get(..2).is_some_and(|q| q.eq_ignore_ascii_case(p)))
        .map_or(t, |_| t[2..].trim());
    let inner_reg = |s: &str| {
        s.strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .and_then(reg)
    };
    if let Some(n) = t.strip_prefix('-').and_then(inner_reg) {
        return match deferred {
            false => Ok(Opnd::Mem(Mode::Dec(n), None)),
            true => Err("autodecrement can't be deferred".into()),
        };
    }
    if let Some(n) = t.strip_suffix('+').and_then(inner_reg) {
        let mode = if deferred {
            Mode::IncDef(n)
        } else {
            Mode::Inc(n)
        };
        return Ok(Opnd::Mem(mode, None));
    }
    if t.ends_with(')')
        && let Some(open) = t.rfind('(')
        && let Some(n) = reg(&t[open + 1..t.len() - 1])
    {
        let disp = t[..open].trim();
        let mode = match (disp.is_empty(), deferred) {
            (true, false) => Mode::Def(n),
            (true, true) => Mode::DispDef("0".into(), n),
            (false, false) => Mode::Disp(disp.into(), n),
            (false, true) => Mode::DispDef(disp.into(), n),
        };
        return Ok(Opnd::Mem(mode, None));
    }
    let e = t.to_string();
    Ok(Opnd::Mem(
        if deferred {
            Mode::RelDef(e)
        } else if general {
            Mode::Gen(e)
        } else {
            Mode::Rel(e)
        },
        None,
    ))
}

/// Where an operand is, once its specifier is evaluated.
#[derive(Clone, Debug)]
pub enum Place {
    Reg(u8),
    Imm(String),
    /// Base register (an ARM64 x register) and displacement.
    Mem(String, i64),
}

/// Scratch registers, free for any instruction to use, taken from the end.
/// CALLS and CALLG pass the argument list in x13 once their operands are
/// read, so it goes last. x2-x7 are left for PAL calls' arguments.
const POOL: [u8; 9] = [13, 9, 8, 11, 10, 17, 16, 15, 14];

/// Code for one VAX instruction.
pub struct Gen<'a> {
    pub out: Vec<String>,
    /// The VAX registers R0-R11 it writes, bit n for Rn.
    pub written: u16,
    free: Vec<u8>,
    /// Registers that operand side effects change, so reading them as
    /// operands must take a copy first.
    side: Vec<u8>,
    constant: &'a dyn Fn(&str) -> Option<i64>,
    next: &'a mut u32,
}

pub type Result<T> = std::result::Result<T, String>;

pub fn w(r: &str) -> String {
    r.replacen('x', "w", 1)
}

pub fn x(r: &str) -> String {
    r.replacen('w', "x", 1)
}

impl<'a> Gen<'a> {
    pub fn new(ops: &[Opnd], constant: &'a dyn Fn(&str) -> Option<i64>, next: &'a mut u32) -> Self {
        let side = ops
            .iter()
            .filter_map(|o| match o {
                Opnd::Mem(Mode::Inc(n) | Mode::Dec(n) | Mode::IncDef(n), _) => Some(*n),
                _ => None,
            })
            .collect();
        Gen {
            out: Vec::new(),
            written: 0,
            free: POOL.to_vec(),
            side,
            constant,
            next,
        }
    }

    pub fn emit(&mut self, line: impl Into<String>) {
        let line = line.into();
        // vasm's MOV doesn't take the zero register.
        let line = match line.strip_suffix("wzr").or(line.strip_suffix("xzr")) {
            Some(head) if line.starts_with("mov ") => format!("{head}#0"),
            _ => line,
        };
        self.out.push(format!("\t{line}"));
    }

    /// Pushes the longword in `v` on the VAX stack. SP itself goes through
    /// a scratch register: a writeback store of its own base is
    /// CONSTRAINED UNPREDICTABLE, which Apple's cores trap.
    pub fn push(&mut self, v: &str) -> Result<()> {
        let mut v = w(v);
        if v == format!("w{SP}") {
            let t = w(&self.tmp()?);
            self.emit(format!("mov {t}, w{SP}"));
            v = t;
        }
        self.emit(format!("str {v}, [x{SP}, #-4]!"));
        Ok(())
    }

    /// A new local label, unique in the module.
    pub fn label(&mut self) -> String {
        *self.next += 1;
        format!("{}$", 90000 + *self.next)
    }

    pub fn place_label(&mut self, label: &str) {
        self.out.push(format!("{label}:"));
    }

    pub fn constant(&self, e: &str) -> Option<i64> {
        (self.constant)(e)
    }

    /// A scratch register, as an x register.
    pub fn tmp(&mut self) -> Result<String> {
        let n = self
            .free
            .pop()
            .ok_or("too many operands in memory for vmacro")?;
        Ok(format!("x{n}"))
    }

    pub fn is_tmp(r: &str) -> bool {
        r.starts_with(['x', 'w']) && r[1..].parse::<u8>().is_ok_and(|n| POOL.contains(&n))
    }

    /// A scratch register: `r` if it is one, else a new one.
    pub fn reuse(&mut self, r: &str) -> Result<String> {
        if Self::is_tmp(r) {
            Ok(x(r))
        } else {
            self.tmp()
        }
    }

    /// Evaluates an operand specifier, with its side effects.
    pub fn place(&mut self, op: &Opnd, size: Size) -> Result<Place> {
        let (mode, index) = match op {
            Opnd::Reg(n) => return Ok(Place::Reg(*n)),
            Opnd::Imm(e) => return Ok(Place::Imm(e.clone())),
            Opnd::Mem(mode, index) => (mode, index),
        };
        let (base, disp) = self.mode(mode, size)?;
        let Some(i) = index else {
            return Ok(Place::Mem(base, disp));
        };
        let base = self.fold(&base, disp)?;
        let t = self.reuse(&base)?;
        let i = arm(*i)?;
        match size.shift() {
            0 => self.emit(format!("add {t}, {base}, w{i}, sxtw")),
            s => self.emit(format!("add {t}, {base}, w{i}, sxtw #{s}")),
        }
        Ok(Place::Mem(t, 0))
    }

    fn mode(&mut self, mode: &Mode, size: Size) -> Result<(String, i64)> {
        Ok(match mode {
            Mode::Def(n) => (format!("x{}", arm(*n)?), 0),
            Mode::Inc(n) => {
                self.wrote(*n);
                let (r, t) = (arm(*n)?, self.tmp()?);
                self.emit(format!("mov {t}, x{r}"));
                self.emit(format!("add x{r}, x{r}, #{}", size.bytes()));
                (t, 0)
            }
            Mode::Dec(n) => {
                self.wrote(*n);
                let r = arm(*n)?;
                self.emit(format!("sub x{r}, x{r}, #{}", size.bytes()));
                (format!("x{r}"), 0)
            }
            Mode::IncDef(n) => {
                self.wrote(*n);
                let (r, t) = (arm(*n)?, self.tmp()?);
                self.emit(format!("ldr {}, [x{r}], #4", w(&t)));
                (t, 0)
            }
            Mode::Disp(e, n) => self.disp(e, *n)?,
            Mode::DispDef(e, n) => {
                let (base, d) = self.disp(e, *n)?;
                let t = self.reuse(&base)?;
                let m = self.at(&base, d, Size::L)?;
                self.emit(format!("ldr {}, {m}", w(&t)));
                (t, 0)
            }
            Mode::Abs(e) => {
                let t = self.tmp()?;
                self.imm_into(&t, e, Size::L)?;
                (t, 0)
            }
            Mode::Rel(e) | Mode::Gen(e) => (self.rel(e)?, 0),
            Mode::RelDef(e) => {
                let t = self.rel(e)?;
                self.emit(format!("ldr {}, [{t}]", w(&t)));
                (t, 0)
            }
        })
    }

    /// `d(Rn)`: a displacement known now stays one; any other is added in
    /// a register, sign-extended from 32 bits.
    fn disp(&mut self, e: &str, n: u8) -> Result<(String, i64)> {
        let r = format!("x{}", arm(n)?);
        if let Some(d) = self.constant(e) {
            return Ok((r, d));
        }
        let t = self.tmp()?;
        self.imm_into(&t, e, Size::L)?;
        self.emit(format!("add {t}, {r}, {}, sxtw", w(&t)));
        Ok((t, 0))
    }

    /// The address of `e`: PC-relative, or a constant.
    fn rel(&mut self, e: &str) -> Result<String> {
        let t = self.tmp()?;
        if self.constant(e).is_some() {
            self.imm_into(&t, e, Size::L)?;
        } else {
            self.emit(format!("adrp {t}, {e}"));
            self.emit(format!("add {t}, {t}, #:lo12:{e}"));
        }
        Ok(t)
    }

    /// `[base, #disp]` if a load or store of `size` can encode it, else the
    /// address in a register.
    pub fn at(&mut self, base: &str, disp: i64, size: Size) -> Result<String> {
        let scaled = disp >= 0 && disp % size.bytes() == 0 && disp / size.bytes() < 4096;
        if disp == 0 {
            Ok(format!("[{base}]"))
        } else if (-256..256).contains(&disp) || scaled {
            Ok(format!("[{base}, #{disp}]"))
        } else {
            Ok(format!("[{}]", self.fold(base, disp)?))
        }
    }

    /// `base + disp` in one register.
    pub fn fold(&mut self, base: &str, disp: i64) -> Result<String> {
        if disp == 0 {
            return Ok(base.to_string());
        }
        let t = self.reuse(base)?;
        if disp.unsigned_abs() < 4096 {
            let op = if disp < 0 { "sub" } else { "add" };
            self.emit(format!("{op} {t}, {base}, #{}", disp.unsigned_abs()));
        } else {
            let d = self.tmp()?;
            self.imm_into(&d, &disp.to_string(), Size::L)?;
            self.emit(format!("add {t}, {base}, {}, sxtw", w(&d)));
            self.free.push(d[1..].parse().unwrap());
        }
        Ok(t)
    }

    /// The address an operand names, in an x register.
    pub fn address(&mut self, op: &Opnd, size: Size) -> Result<String> {
        match self.place(op, size)? {
            Place::Mem(base, disp) => self.fold(&base, disp),
            _ => Err("expected an operand in memory".into()),
        }
    }

    /// Loads `e` into register `t` (an x register) as a value of `size`.
    pub fn imm_into(&mut self, t: &str, e: &str, size: Size) -> Result<()> {
        let quad = size == Size::Q;
        let r = if quad { x(t) } else { w(t) };
        let Some(n) = self.constant(e) else {
            // Not known yet: maybe an address. From a slot in $LINK$, which
            // the loader fixes up if the image moves.
            let slot = self.label();
            let (align, data) = if quad {
                ("QUAD", "QUAD")
            } else {
                ("LONG", "LONG")
            };
            self.out.extend([
                "\t.SAVE_PSECT LOCAL_BLOCK".into(),
                "\t.PSECT $LINK$".into(),
                format!("\t.ALIGN {align}"),
                format!("{slot}:\t.{data} {e}"),
                "\t.RESTORE_PSECT".into(),
            ]);
            self.emit(format!("adrp {}, {slot}", x(t)));
            self.emit(format!("ldr {r}, [{}, #:lo12:{slot}]", x(t)));
            return Ok(());
        };
        let v = if quad { n as u64 } else { u64::from(n as u32) };
        let chunks = if quad { 4 } else { 2 };
        let parts: Vec<u64> = (0..chunks).map(|i| v >> (16 * i) & 0xffff).collect();
        let zeros = parts.iter().filter(|&&p| p == 0).count();
        let ones = parts.iter().filter(|&&p| p == 0xffff).count();
        // Mostly ones: MOVN, then MOVK for the chunks that aren't.
        let (skip, first_op) = if ones > zeros {
            (0xffff, "movn")
        } else {
            (0, "movz")
        };
        let first = parts.iter().position(|&p| p != skip).unwrap_or(0);
        let lsl = |i: usize| match i {
            0 => String::new(),
            _ => format!(", lsl #{}", 16 * i),
        };
        let imm = if skip == 0 {
            parts[first]
        } else {
            !parts[first] & 0xffff
        };
        self.emit(format!("{first_op} {r}, #{imm}{}", lsl(first)));
        for (i, p) in parts.iter().enumerate().skip(first + 1) {
            if *p != skip {
                self.emit(format!("movk {r}, #{p}{}", lsl(i)));
            }
        }
        Ok(())
    }

    /// Reads an operand: evaluates it and loads its value.
    pub fn read(&mut self, op: &Opnd, size: Size, ext: Ext) -> Result<String> {
        let p = self.place(op, size)?;
        self.load(&p, size, ext, true)
    }

    /// An operand for ADD, SUB and CMP: a 12-bit immediate if it is one.
    pub fn read2(&mut self, op: &Opnd, size: Size) -> Result<String> {
        if let Opnd::Imm(e) = op
            && let Some(n) = self.constant(e)
            && size == Size::L
            && (0..4096).contains(&n)
        {
            return Ok(format!("#{n}"));
        }
        self.read(op, size, Ext::Sext)
    }

    /// The value at `p`, in a register: w for byte to longword, x for
    /// quadword. `consume` lets the load reuse the address register.
    pub fn load(&mut self, p: &Place, size: Size, ext: Ext, consume: bool) -> Result<String> {
        match p {
            Place::Reg(n) => {
                let r = arm(*n)?;
                if size == Size::Q {
                    let r1 = pair(*n)?;
                    let t = self.tmp()?;
                    self.emit(format!("mov {}, w{r}", w(&t)));
                    self.emit(format!("bfi {t}, x{r1}, #32, #32"));
                    return Ok(t);
                }
                let sub = size != Size::L && ext != Ext::Any;
                if !sub && !self.side.contains(n) {
                    return Ok(format!("w{r}"));
                }
                let t = w(&self.tmp()?);
                match (sub, ext) {
                    (false, _) | (_, Ext::Any) => self.emit(format!("mov {t}, w{r}")),
                    (true, Ext::Sext) => self.emit(format!("sbfx {t}, w{r}, #0, #{}", size.bits())),
                    (true, Ext::Zext) => self.emit(format!("ubfx {t}, w{r}, #0, #{}", size.bits())),
                }
                Ok(t)
            }
            Place::Imm(e) => {
                let zero = if size == Size::Q { "xzr" } else { "wzr" };
                let Some(mut n) = self.constant(e) else {
                    let t = self.tmp()?;
                    self.imm_into(&t, e, size)?;
                    if size < Size::L && ext != Ext::Any {
                        let op = if ext == Ext::Sext { "sbfx" } else { "ubfx" };
                        self.emit(format!("{op} {0}, {0}, #0, #{1}", w(&t), size.bits()));
                    }
                    return Ok(if size == Size::Q { t } else { w(&t) });
                };
                if size < Size::L {
                    let bits = size.bits();
                    n &= (1 << bits) - 1;
                    if ext == Ext::Sext && n >> (bits - 1) != 0 {
                        n -= 1 << bits;
                    }
                }
                if n == 0 {
                    return Ok(zero.into());
                }
                let t = self.tmp()?;
                self.imm_into(&t, &n.to_string(), size)?;
                Ok(if size == Size::Q { t } else { w(&t) })
            }
            Place::Mem(base, disp) => {
                let t = if consume {
                    self.reuse(base)?
                } else {
                    self.tmp()?
                };
                let m = self.at(base, *disp, size)?;
                let (op, r) = match (size, ext) {
                    (Size::B, Ext::Sext) => ("ldrsb", w(&t)),
                    (Size::B, _) => ("ldrb", w(&t)),
                    (Size::W, Ext::Sext) => ("ldrsh", w(&t)),
                    (Size::W, _) => ("ldrh", w(&t)),
                    (Size::L, _) => ("ldr", w(&t)),
                    (Size::Q, _) => ("ldr", t.clone()),
                };
                self.emit(format!("{op} {r}, {m}"));
                Ok(r)
            }
        }
    }

    /// Writes `v`, a register or `wzr`, to `p`. A register gets its new
    /// longword sign-extended, as every VAX register holds it.
    pub fn store(&mut self, p: &Place, size: Size, v: &str) -> Result<()> {
        match p {
            Place::Reg(n) => {
                let r = arm(*n)?;
                self.wrote(*n);
                if size == Size::Q {
                    self.wrote(n + 1);
                }
                match size {
                    Size::L if v.ends_with("zr") => self.emit(format!("mov x{r}, xzr")),
                    Size::L => self.emit(format!("sxtw x{r}, {}", w(v))),
                    Size::Q => {
                        let r1 = pair(*n)?;
                        let v = self.nonzero(x(v))?;
                        self.emit(format!("sxtw x{r}, {}", w(&v)));
                        self.emit(format!("asr x{r1}, {v}, #32"));
                    }
                    _ => {
                        self.emit(format!("bfi w{r}, {}, #0, #{}", w(v), size.bits()));
                        self.sext(r);
                    }
                }
            }
            Place::Imm(_) => return Err("can't write to an immediate".into()),
            Place::Mem(base, disp) => {
                let m = self.at(base, *disp, size)?;
                match size {
                    Size::B => self.emit(format!("strb {}, {m}", w(v))),
                    Size::W => self.emit(format!("strh {}, {m}", w(v))),
                    Size::L => self.emit(format!("str {}, {m}", w(v))),
                    Size::Q => self.emit(format!("str {}, {m}", x(v))),
                }
            }
        }
        Ok(())
    }

    /// Where to put a result for `dst`: its own register for a longword,
    /// else a scratch register, reusing one of `from` if it can.
    pub fn result(&mut self, dst: &Place, size: Size, from: &[&str]) -> Result<String> {
        if let Place::Reg(n) = dst
            && size == Size::L
        {
            return Ok(format!("w{}", arm(*n)?));
        }
        let t = match from.iter().find(|r| Self::is_tmp(r)) {
            Some(r) => x(r),
            None => self.tmp()?,
        };
        Ok(if size == Size::Q { t } else { w(&t) })
    }

    /// A register that isn't the zero register, for instructions whose
    /// first operand can't be one.
    pub fn nonzero(&mut self, v: String) -> Result<String> {
        if !v.ends_with("zr") {
            return Ok(v);
        }
        let t = self.tmp()?;
        let t = if v.starts_with('x') { t } else { w(&t) };
        self.emit(format!("mov {t}, {v}"));
        Ok(t)
    }
}

impl Gen<'_> {
    /// Notes that the instruction writes VAX register `n`.
    pub fn wrote(&mut self, n: u8) {
        if n < 12 {
            self.written |= 1 << n;
        }
    }

    /// Sign-extends register x`r`'s longword.
    pub fn sext(&mut self, r: u8) {
        self.emit(format!("sxtw x{r}, w{r}"));
    }
}

/// The register that holds the high longword of a quadword in Rn and Rn+1.
fn pair(n: u8) -> Result<u8> {
    match n {
        0..=10 => arm(n + 1),
        _ => Err("a quadword needs two registers, up to R10 and R11".into()),
    }
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.bytes().partial_cmp(&other.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(mode: Mode) -> Opnd {
        Opnd::Mem(mode, None)
    }

    #[test]
    fn operands() {
        assert_eq!(parse("R5"), Ok(Opnd::Reg(5)));
        assert_eq!(parse("ap"), Ok(Opnd::Reg(12)));
        assert_eq!(parse("S^#10"), Ok(Opnd::Imm("10".into())));
        assert_eq!(parse("(R1)"), Ok(mem(Mode::Def(1))));
        assert_eq!(parse("(SP)+"), Ok(mem(Mode::Inc(14))));
        assert_eq!(parse("-(SP)"), Ok(mem(Mode::Dec(14))));
        assert_eq!(parse("@(R2)+"), Ok(mem(Mode::IncDef(2))));
        assert_eq!(parse("B^4(AP)"), Ok(mem(Mode::Disp("4".into(), 12))));
        assert_eq!(parse("@8(FP)"), Ok(mem(Mode::DispDef("8".into(), 13))));
        assert_eq!(parse("<A+B>(R3)"), Ok(mem(Mode::Disp("<A+B>".into(), 3))));
        assert_eq!(parse("@#^X200"), Ok(mem(Mode::Abs("^X200".into()))));
        assert_eq!(parse("G^TABLE"), Ok(mem(Mode::Gen("TABLE".into()))));
        assert_eq!(parse("@PTR"), Ok(mem(Mode::RelDef("PTR".into()))));
        assert_eq!(parse("RX"), Ok(mem(Mode::Rel("RX".into()))));
        assert_eq!(
            parse("TABLE[R4]"),
            Ok(Opnd::Mem(Mode::Rel("TABLE".into()), Some(4)))
        );
        assert!(parse("R1[R2]").is_err());
        assert_eq!(split("#1, 4(R1)[R2], <A,B>"), ["#1", "4(R1)[R2]", "<A,B>"]);
    }
}
