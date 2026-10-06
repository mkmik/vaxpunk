//! VAX instructions, each translated to ARM64 on its own.
//! `docs/macro32.md` lists them and what doesn't carry over.

use crate::Flags;
use crate::operand::{Ext, Gen, Mode, Opnd, Place, Result, SP, Size, arm, w, x};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alu {
    Add,
    Sub,
    Mul,
    Div,
    Bis,
    Bic,
    Xor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Mov,
    Clr,
    Mcom,
    Mneg,
    Movz(Size),
    Cvt(Size),
    Mova,
    Pusha,
    Pushl,
    Arith(Alu, bool),
    Inc,
    Dec,
    Cmp,
    Tst,
    Bit,
    Bcc,
    Blb(bool),
    Br,
    Jmp,
    Jsb,
    Rsb,
    Calls,
    Callg,
    Ret,
    Pushr,
    Popr,
    Case,
    Acb,
    Aob(&'static str),
    Sob(&'static str),
    Ash,
    Rot,
    Emul,
    Ediv,
    Movc3,
    Movc5,
    /// EXTV (signed) and EXTZV.
    Ext(bool),
    Insv,
    /// Branch on bit set (or clear), then set or clear it.
    Bb(bool, Option<bool>),
    Nop,
    Halt,
    Bpt,
    Mtpr,
    Mfpr,
    Rei,
    /// CHMK, CHME, CHMS, CHMU: their PAL call.
    Chm(u32),
    /// PROBER and PROBEW: whether it is PROBEW.
    Probe(bool),
    /// Alpha's CALL_PAL: arguments in R0-R5, the result in R0.
    CallPal,
    Insque,
    Remque,
    /// EVAX_LDQ and EVAX_STQ: a register's 64 bits from or to an operand.
    Ldq,
    Stq,
}

/// A VAX mnemonic's operation and operand size, if it is one vmacro knows.
pub fn kind(mn: &str) -> Option<(Op, Size)> {
    use Size::*;
    let fixed = match mn {
        "MOVZBW" => (Op::Movz(B), W),
        "MOVZBL" => (Op::Movz(B), L),
        "MOVZWL" => (Op::Movz(W), L),
        "CVTBW" => (Op::Cvt(B), W),
        "CVTBL" => (Op::Cvt(B), L),
        "CVTWL" => (Op::Cvt(W), L),
        "CVTWB" => (Op::Cvt(W), B),
        "CVTLB" => (Op::Cvt(L), B),
        "CVTLW" => (Op::Cvt(L), W),
        "PUSHL" => (Op::Pushl, L),
        "BRB" | "BRW" => (Op::Br, L),
        "JMP" => (Op::Jmp, B),
        "JSB" | "BSBB" | "BSBW" => (Op::Jsb, B),
        "RSB" => (Op::Rsb, L),
        "CALLS" => (Op::Calls, L),
        "CALLG" => (Op::Callg, L),
        "RET" => (Op::Ret, L),
        "PUSHR" => (Op::Pushr, W),
        "POPR" => (Op::Popr, W),
        "ACBL" => (Op::Acb, L),
        "ACBW" => (Op::Acb, W),
        "ACBB" => (Op::Acb, B),
        "AOBLSS" => (Op::Aob("lt"), L),
        "AOBLEQ" => (Op::Aob("le"), L),
        "SOBGTR" => (Op::Sob("gt"), L),
        "SOBGEQ" => (Op::Sob("ge"), L),
        "ASHL" => (Op::Ash, L),
        "ASHQ" => (Op::Ash, Q),
        "ROTL" => (Op::Rot, L),
        "EMUL" => (Op::Emul, L),
        "EDIV" => (Op::Ediv, L),
        "MOVC3" => (Op::Movc3, B),
        "MOVC5" => (Op::Movc5, B),
        "EXTV" => (Op::Ext(true), L),
        "EXTZV" => (Op::Ext(false), L),
        "INSV" => (Op::Insv, L),
        "BBS" => (Op::Bb(true, None), B),
        "BBC" => (Op::Bb(false, None), B),
        "BBSS" => (Op::Bb(true, Some(true)), B),
        "BBSC" => (Op::Bb(true, Some(false)), B),
        "BBCS" => (Op::Bb(false, Some(true)), B),
        "BBCC" => (Op::Bb(false, Some(false)), B),
        "BLBS" => (Op::Blb(true), L),
        "BLBC" => (Op::Blb(false), L),
        "NOP" => (Op::Nop, L),
        "HALT" => (Op::Halt, L),
        "BPT" => (Op::Bpt, L),
        "MTPR" => (Op::Mtpr, L),
        "MFPR" => (Op::Mfpr, L),
        "REI" => (Op::Rei, L),
        "CHMK" => (Op::Chm(CHMK), W),
        "CHME" => (Op::Chm(CHME), W),
        "CHMS" => (Op::Chm(CHMS), W),
        "CHMU" => (Op::Chm(CHMU), W),
        "PROBER" => (Op::Probe(false), B),
        "PROBEW" => (Op::Probe(true), B),
        "CALL_PAL" => (Op::CallPal, L),
        "INSQUE" => (Op::Insque, B),
        "REMQUE" => (Op::Remque, B),
        "EVAX_LDQ" => (Op::Ldq, Q),
        "EVAX_STQ" => (Op::Stq, Q),
        "BNEQ" | "BNEQU" | "BEQL" | "BEQLU" | "BGTR" | "BLEQ" | "BGEQ" | "BLSS" | "BGTRU"
        | "BLEQU" | "BVC" | "BVS" | "BCC" | "BCS" | "BGEQU" | "BLSSU" => (Op::Bcc, L),
        _ => return sized(mn),
    };
    Some(fixed)
}

/// Mnemonics made of an operation, a size letter and for some an operand
/// count: MOVL, ADDB2, CASEW.
fn sized(mn: &str) -> Option<(Op, Size)> {
    const FAMILIES: [(&str, &str, Op); 13] = [
        ("MOVA", "BWLQ", Op::Mova),
        ("PUSHA", "BWLQ", Op::Pusha),
        ("MOV", "BWLQ", Op::Mov),
        ("CLR", "BWLQ", Op::Clr),
        ("MCOM", "BWL", Op::Mcom),
        ("MNEG", "BWL", Op::Mneg),
        ("INC", "BWL", Op::Inc),
        ("DEC", "BWL", Op::Dec),
        ("CMP", "BWL", Op::Cmp),
        ("TST", "BWL", Op::Tst),
        ("BIT", "BWL", Op::Bit),
        ("CASE", "BWL", Op::Case),
        ("", "", Op::Nop),
    ];
    const ARITH: [(&str, Alu); 7] = [
        ("ADD", Alu::Add),
        ("SUB", Alu::Sub),
        ("MUL", Alu::Mul),
        ("DIV", Alu::Div),
        ("BIS", Alu::Bis),
        ("BIC", Alu::Bic),
        ("XOR", Alu::Xor),
    ];
    let size = |c: u8| match c {
        b'B' => Some(Size::B),
        b'W' => Some(Size::W),
        b'L' => Some(Size::L),
        b'Q' => Some(Size::Q),
        _ => None,
    };
    for (base, sizes, op) in FAMILIES.iter().filter(|f| !f.0.is_empty()) {
        if let Some(rest) = mn.strip_prefix(base)
            && rest.len() == 1
            && sizes.contains(rest)
        {
            return Some((*op, size(rest.as_bytes()[0])?));
        }
    }
    for (base, alu) in ARITH {
        if let Some(rest) = mn.strip_prefix(base)
            && let [s @ (b'B' | b'W' | b'L'), n @ (b'2' | b'3')] = rest.as_bytes()
        {
            return Some((Op::Arith(alu, *n == b'3'), size(*s)?));
        }
    }
    None
}

/// How many operands `op` takes.
pub fn arity(op: Op) -> usize {
    match op {
        Op::Rsb | Op::Ret | Op::Nop | Op::Halt | Op::Bpt | Op::Rei => 0,
        Op::Clr | Op::Pusha | Op::Pushl | Op::Inc | Op::Dec | Op::Tst | Op::Bcc | Op::Br => 1,
        Op::Chm(_) | Op::CallPal => 1,
        Op::Probe(_) => 3,
        Op::Jmp | Op::Jsb | Op::Pushr | Op::Popr => 1,
        Op::Arith(_, true) | Op::Case | Op::Ash | Op::Rot | Op::Movc3 | Op::Aob(_) => 3,
        Op::Bb(..) => 3,
        Op::Acb | Op::Emul | Op::Ediv | Op::Ext(_) | Op::Insv => 4,
        Op::Movc5 => 5,
        _ => 2,
    }
}

/// A call's `n` arguments, from the longwords pushed at VAX SP, where
/// DESIGN-0004 puts them, sign-extended: the first eight in x0-x7, the
/// rest on the stack, which `sp` points to, below VAX SP rounded down to
/// 16. x13 is left alone.
fn args(g: &mut Gen, n: usize) {
    let stack = n.saturating_sub(8);
    if stack == 0 {
        g.emit(format!("and sp, x{SP}, #0xfffffffffffffff0"));
    } else {
        g.emit(format!("and x16, x{SP}, #0xfffffffffffffff0"));
        g.emit(format!(
            "sub sp, x16, #{}",
            (8 * stack).next_multiple_of(16)
        ));
        g.emit(format!("add x16, x{SP}, #32"));
        g.emit("mov x17, sp");
        g.emit(format!("mov x15, #{stack}"));
        let again = g.label();
        g.place_label(&again);
        g.emit("ldrsw x14, [x16], #4");
        g.emit("str x14, [x17], #8");
        g.emit("subs x15, x15, #1");
        g.emit(format!("b.ne {again}"));
    }
    for i in (0..n.min(8)).step_by(2) {
        if i + 1 < n {
            g.emit(format!("ldpsw x{i}, x{}, [x{SP}, #{}]", i + 1, 4 * i));
        } else {
            g.emit(format!("ldrsw x{i}, [x{SP}, #{}]", 4 * i));
        }
    }
}

/// The same, for x9 arguments from the list at x8, known when it runs:
/// those past the eighth copied, then a branch into the loads of x7 down
/// to x0 that loads just the ones there are, so that it reads no further
/// than the list. x13 is left alone.
fn list_args(g: &mut Gen) {
    let (regs, low, done) = (g.label(), g.label(), g.label());
    g.emit(format!("and x16, x{SP}, #0xfffffffffffffff0"));
    g.emit("subs x15, x9, #8");
    g.emit(format!("b.ls {low}"));
    g.emit("add x14, x15, #1");
    g.emit("and x14, x14, #0xfffffffffffffffe");
    g.emit("sub sp, x16, x14, lsl #3");
    g.emit("add x16, x8, #32");
    g.emit("mov x17, sp");
    let again = g.label();
    g.place_label(&again);
    g.emit("ldrsw x14, [x16], #4");
    g.emit("str x14, [x17], #8");
    g.emit("subs x15, x15, #1");
    g.emit(format!("b.ne {again}"));
    g.emit(format!("b {regs}"));
    g.place_label(&low);
    g.emit("mov sp, x16");
    g.place_label(&regs);
    g.emit("mov x15, #8");
    g.emit("cmp x9, x15");
    g.emit("csel x15, x9, x15, lo");
    g.emit(format!("adr x16, {done}"));
    g.emit("sub x16, x16, x15, lsl #2");
    g.emit("br x16");
    for i in (0..8).rev() {
        g.emit(format!("ldrsw x{i}, [x8, #{}]", 4 * i));
    }
    g.place_label(&done);
}

/// A result's flags for a branch: N and Z from its sign and zero, V and C
/// clear.
fn test(v: &str, size: Size) -> Flags {
    match size {
        Size::L => Flags::Pending(vec![format!("tst {0}, {0}", w(v))]),
        Size::Q => Flags::Pending(vec![format!("tst {0}, {0}", x(v))]),
        _ => Flags::Pending(vec![
            format!("sbfx w16, {}, #0, #{}", w(v), size.bits()),
            "tst w16, w16".into(),
        ]),
    }
}

/// A branch target: a plain expression.
fn target(op: &Opnd) -> Result<&str> {
    match op {
        Opnd::Mem(Mode::Rel(e), None) => Ok(e),
        _ => Err("expected a branch destination".into()),
    }
}

/// The ARM64 condition for a VAX conditional branch. After a subtraction,
/// ARM64's C is VAX's inverted. Some unsigned branches after an addition
/// need two ARM64 conditions: taken if either holds, or if `.0` doesn't
/// and `.1` does.
enum Cond {
    One(&'static str),
    Either(&'static str, &'static str),
    Unless(&'static str, &'static str),
}

fn cond(mn: &str, borrow: bool) -> Cond {
    use Cond::*;
    match (mn, borrow) {
        ("BNEQ" | "BNEQU", _) => One("ne"),
        ("BEQL" | "BEQLU", _) => One("eq"),
        ("BGTR", _) => One("gt"),
        ("BLEQ", _) => One("le"),
        ("BGEQ", _) => One("ge"),
        ("BLSS", _) => One("lt"),
        ("BVC", _) => One("vc"),
        ("BVS", _) => One("vs"),
        ("BCC" | "BGEQU", true) => One("hs"),
        ("BCC" | "BGEQU", false) => One("cc"),
        ("BCS" | "BLSSU", true) => One("lo"),
        ("BCS" | "BLSSU", false) => One("cs"),
        ("BGTRU", true) => One("hi"),
        ("BGTRU", false) => Unless("cs", "ne"),
        ("BLEQU", true) => One("ls"),
        _ => Either("cs", "eq"),
    }
}

/// Code for one instruction. Returns the flags it leaves, or `None` if it
/// leaves them alone.
pub fn compile(
    g: &mut Gen,
    mn: &str,
    op: Op,
    size: Size,
    ops: &[Opnd],
    flags: &Flags,
    exit: &crate::Exit,
) -> Result<Option<Flags>> {
    let live = |borrow| Ok(Some(Flags::Live { borrow }));
    match op {
        Op::Mov => {
            if let (Opnd::Imm(e), Opnd::Reg(n)) = (&ops[0], &ops[1])
                && size == Size::L
            {
                // Sign-extended, as a register holds a longword.
                g.wrote(*n);
                let r = arm(*n)?;
                match g.constant(e) {
                    Some(v) => g.imm_into(&format!("x{r}"), &(v as i32).to_string(), Size::Q)?,
                    None => {
                        g.imm_into(&format!("x{r}"), e, size)?;
                        g.sext(r);
                    }
                }
                return Ok(Some(test(&format!("w{r}"), size)));
            }
            let v = g.read(&ops[0], size, Ext::Any)?;
            let p = g.place(&ops[1], size)?;
            g.store(&p, size, &v)?;
            Ok(Some(test(&v, size)))
        }
        Op::Clr => {
            let p = g.place(&ops[0], size)?;
            g.store(&p, size, if size == Size::Q { "xzr" } else { "wzr" })?;
            Ok(Some(test("wzr", Size::L)))
        }
        Op::Mcom | Op::Mneg => {
            let v = g.read(&ops[0], size, Ext::Any)?;
            let p = g.place(&ops[1], size)?;
            let r = g.result(&p, size, &[&v])?;
            let borrow = match (op, size) {
                (Op::Mcom, _) => {
                    g.emit(format!("mvn {r}, {v}"));
                    None
                }
                (_, Size::L) => {
                    g.emit(format!("negs {r}, {v}"));
                    Some(true)
                }
                _ => {
                    g.emit(format!("neg {r}, {v}"));
                    None
                }
            };
            g.store(&p, size, &r)?;
            Ok(Some(match borrow {
                Some(b) => Flags::Live { borrow: b },
                None => test(&r, size),
            }))
        }
        Op::Movz(from) | Op::Cvt(from) => {
            let ext = match op {
                Op::Movz(_) => Ext::Zext,
                _ if from < size => Ext::Sext,
                _ => Ext::Any,
            };
            let v = g.read(&ops[0], from, ext)?;
            let p = g.place(&ops[1], size)?;
            g.store(&p, size, &v)?;
            Ok(Some(test(&v, size)))
        }
        Op::Mova | Op::Pusha => {
            let a = g.address(&ops[0], size)?;
            if op == Op::Pusha {
                g.push(&a)?;
            } else if let Opnd::Reg(n) = ops[1] {
                // Computed in longwords, as AMACRO does by default.
                g.wrote(n);
                g.emit(format!("sxtw x{}, {}", arm(n)?, w(&a)));
            } else {
                let p = g.place(&ops[1], Size::L)?;
                g.store(&p, Size::L, &a)?;
            }
            Ok(Some(test(&a, Size::L)))
        }
        Op::Pushl => {
            let v = g.read(&ops[0], size, Ext::Any)?;
            g.push(&v)?;
            Ok(Some(test(&v, size)))
        }
        Op::Arith(alu, three) => {
            let add = matches!(alu, Alu::Add | Alu::Sub);
            let ext = if alu == Alu::Div { Ext::Sext } else { Ext::Any };
            let a = if add {
                g.read2(&ops[0], size)?
            } else {
                g.read(&ops[0], size, ext)?
            };
            let (b, dst) = if three {
                let b = g.read(&ops[1], size, ext)?;
                (b, g.place(&ops[2], size)?)
            } else {
                let p = g.place(&ops[1], size)?;
                (g.load(&p, size, ext, false)?, p)
            };
            let b = if add { g.nonzero(b)? } else { b };
            let r = g.result(&dst, size, &[&a, &b])?;
            let long = size == Size::L;
            let (insn, borrow) = match alu {
                Alu::Add if long => ("adds", Some(false)),
                Alu::Add => ("add", None),
                Alu::Sub if long => ("subs", Some(true)),
                Alu::Sub => ("sub", None),
                Alu::Mul => ("mul", None),
                Alu::Div => ("sdiv", None),
                Alu::Bis => ("orr", None),
                Alu::Bic if long => ("bics", Some(false)),
                Alu::Bic => ("bic", None),
                Alu::Xor => ("eor", None),
            };
            g.emit(format!("{insn} {r}, {b}, {a}"));
            g.store(&dst, size, &r)?;
            Ok(Some(match borrow {
                Some(b) => Flags::Live { borrow: b },
                None => test(&r, size),
            }))
        }
        Op::Inc | Op::Dec => {
            let p = g.place(&ops[0], size)?;
            let v = g.load(&p, size, Ext::Any, false)?;
            let r = g.result(&p, size, &[&v])?;
            let long = size == Size::L;
            let insn = match (op, long) {
                (Op::Inc, true) => "adds",
                (Op::Inc, false) => "add",
                (_, true) => "subs",
                _ => "sub",
            };
            g.emit(format!("{insn} {r}, {v}, #1"));
            g.store(&p, size, &r)?;
            match long {
                true => live(op == Op::Dec),
                false => Ok(Some(test(&r, size))),
            }
        }
        Op::Cmp => {
            let a = g.read(&ops[0], size, Ext::Sext)?;
            let a = g.nonzero(a)?;
            let b = g.read2(&ops[1], size)?;
            g.emit(format!("cmp {a}, {b}"));
            live(true)
        }
        Op::Tst => {
            let v = g.read(&ops[0], size, Ext::Sext)?;
            g.emit(format!("tst {v}, {v}"));
            live(false)
        }
        Op::Bit => {
            let m = g.read(&ops[0], size, Ext::Sext)?;
            let v = g.read(&ops[1], size, Ext::Sext)?;
            g.emit(format!("tst {m}, {v}"));
            live(false)
        }
        Op::Bcc => {
            let t = target(&ops[0])?;
            let borrow = match flags {
                Flags::Live { borrow } => *borrow,
                Flags::Pending(lines) => {
                    for l in lines {
                        g.emit(l.clone());
                    }
                    false
                }
            };
            match cond(mn, borrow) {
                Cond::One(c) => g.emit(format!("b.{c} {t}")),
                Cond::Either(c, d) => {
                    g.emit(format!("b.{c} {t}"));
                    g.emit(format!("b.{d} {t}"));
                }
                Cond::Unless(c, d) => {
                    let skip = g.label();
                    g.emit(format!("b.{c} {skip}"));
                    g.emit(format!("b.{d} {t}"));
                    g.place_label(&skip);
                }
            }
            live(borrow)
        }
        Op::Blb(set) => {
            let v = g.read(&ops[0], size, Ext::Any)?;
            let t = target(&ops[1])?;
            g.emit(format!("{} {v}, #0, {t}", if set { "tbnz" } else { "tbz" }));
            Ok(None)
        }
        Op::Br => {
            g.emit(format!("b {}", target(&ops[0])?));
            Ok(None)
        }
        Op::Jmp => {
            match &ops[0] {
                Opnd::Mem(Mode::Rel(e), None) => g.emit(format!("b {e}")),
                o => {
                    let a = g.address(o, size)?;
                    g.emit(format!("br {a}"));
                }
            }
            Ok(None)
        }
        Op::Jsb => {
            // A native call, as AMACRO made it: the return address is in
            // x30, not on the VAX stack.
            let call = match &ops[0] {
                Opnd::Mem(Mode::Rel(e), None) => format!("bl {e}"),
                o => format!("blr {}", g.address(o, size)?),
            };
            g.emit(format!("and sp, x{SP}, #0xfffffffffffffff0"));
            g.emit(call);
            live(true)
        }
        Op::Rsb => {
            match exit {
                crate::Exit::Jsb(saved, lr) => g.out.extend(crate::jsb_epilogue(saved, *lr)),
                crate::Exit::Call(_) => return Err("RSB in a CALL routine".into()),
                crate::Exit::None => {}
            }
            g.emit("ret");
            Ok(None)
        }
        Op::Calls | Op::Callg => {
            // x8 and x13 carry the list and the target past the operands.
            g.reserve(8);
            g.reserve(13);
            let calls = op == Op::Calls;
            let n = match &ops[0] {
                Opnd::Imm(e) if calls => g.constant(e).filter(|n| (0..=255).contains(n)),
                _ => None,
            };
            let (count, list) = match n {
                Some(_) => (None, None),
                None if calls => (Some(g.read(&ops[0], Size::L, Ext::Any)?), None),
                None => (None, Some(g.address(&ops[0], Size::B)?)),
            };
            let call = match &ops[1] {
                Opnd::Mem(Mode::Rel(e), None) => format!("bl {e}"),
                o => {
                    let t = g.address(o, Size::B)?;
                    if t != "x13" {
                        g.emit(format!("mov x13, {t}"));
                    }
                    "blr x13".into()
                }
            };
            match (n, count, list) {
                (Some(n), ..) => {
                    args(g, n as usize);
                    if n > 0 {
                        g.emit(format!("add x{SP}, x{SP}, #{}", 4 * n));
                    }
                    g.emit(format!("mov x9, #{n}"));
                }
                (None, Some(count), _) => {
                    g.emit(format!("and x9, {}, #255", x(&count)));
                    g.emit(format!("mov x8, x{SP}"));
                    list_args(g);
                    g.emit(format!("add x{SP}, x{SP}, x9, lsl #2"));
                }
                (None, None, Some(list)) => {
                    // A list at 0, which a VAX callee that doesn't read AP
                    // never noticed, passes no arguments.
                    if list != "x8" {
                        g.emit(format!("mov x8, {list}"));
                    }
                    let none = g.label();
                    g.emit("mov x9, #0");
                    g.emit(format!("cbz x8, {none}"));
                    g.emit("ldr w9, [x8], #4");
                    g.emit("and x9, x9, #255");
                    g.place_label(&none);
                    list_args(g);
                }
                _ => unreachable!(),
            }
            g.emit(call);
            live(true)
        }
        Op::Ret => {
            let crate::Exit::Call(frame) = exit else {
                return Err("RET outside a CALL routine (.CALL_ENTRY or .ENTRY)".into());
            };
            g.out.extend(crate::epilogue(frame));
            Ok(None)
        }
        Op::Pushr | Op::Popr => {
            let Opnd::Imm(e) = &ops[0] else {
                return Err("expected a register mask, #^M<...>".into());
            };
            let mask = g
                .constant(e)
                .ok_or("the register mask must be a constant")?;
            let regs: Vec<u8> = (0..14).filter(|r| mask & 1 << r != 0).collect();
            if mask & 0xc000 != 0 {
                return Err("PUSHR and POPR can't save SP or PC".into());
            }
            if op == Op::Pushr {
                for r in regs.iter().rev() {
                    g.emit(format!("str w{}, [x{SP}, #-4]!", arm(*r)?));
                }
            } else {
                for r in &regs {
                    g.wrote(*r);
                    g.emit(format!("ldrsw x{}, [x{SP}], #4", arm(*r)?));
                }
            }
            Ok(None)
        }
        Op::Case => {
            let Opnd::Imm(limit) = &ops[2] else {
                return Err("the CASE limit must be an immediate, #n".into());
            };
            let s = g.read(&ops[0], size, Ext::Any)?;
            let s = g.nonzero(s)?;
            let base = if size == Size::L {
                g.read2(&ops[1], size)?
            } else {
                g.read(&ops[1], size, Ext::Any)?
            };
            let t = w(&g.tmp()?);
            g.emit(format!("sub {t}, {s}, {base}"));
            if size != Size::L {
                g.emit(format!("ubfx {t}, {t}, #0, #{}", size.bits()));
            }
            let l = g.read2(&Opnd::Imm(limit.clone()), Size::L)?;
            g.emit(format!("cmp {t}, {l}"));
            let (table, inside) = (g.label(), g.label());
            let a = g.tmp()?;
            g.emit(format!("adr {a}, {table}"));
            g.emit(format!("b.ls {inside}"));
            // Out of range: on after the table of words, aligned.
            g.emit(format!("add {a}, {a}, #((<({limit})+1>*2+3)&^C3)"));
            g.emit(format!("br {a}"));
            g.place_label(&inside);
            let d = g.tmp()?;
            g.emit(format!("ldrsh {d}, [{a}, {t}, uxtw #1]"));
            g.emit(format!("add {a}, {a}, {d}"));
            g.emit(format!("br {a}"));
            g.place_label(&table);
            live(true)
        }
        Op::Acb => {
            let limit = if size == Size::L {
                g.read2(&ops[0], size)?
            } else {
                g.read(&ops[0], size, Ext::Sext)?
            };
            let step = g.read(&ops[1], size, Ext::Sext)?;
            let p = g.place(&ops[2], size)?;
            let v = g.load(&p, size, Ext::Sext, false)?;
            let r = g.result(&p, size, &[&v])?;
            g.emit(format!("add {r}, {v}, {step}"));
            g.store(&p, size, &r)?;
            let r = if size == Size::L {
                r
            } else {
                let t = w(&g.reuse(&r)?);
                g.emit(format!("sbfx {t}, {r}, #0, #{}", size.bits()));
                t
            };
            let t = target(&ops[3])?;
            let known = match &ops[1] {
                Opnd::Imm(e) => g.constant(e),
                _ => None,
            };
            match known {
                Some(n) => {
                    g.emit(format!("cmp {r}, {limit}"));
                    g.emit(format!("b.{} {t}", if n >= 0 { "le" } else { "ge" }));
                }
                None => {
                    let (down, done) = (g.label(), g.label());
                    g.emit(format!("tbnz {step}, #31, {down}"));
                    g.emit(format!("cmp {r}, {limit}"));
                    g.emit(format!("b.le {t}"));
                    g.emit(format!("b {done}"));
                    g.place_label(&down);
                    g.emit(format!("cmp {r}, {limit}"));
                    g.emit(format!("b.ge {t}"));
                    g.place_label(&done);
                }
            }
            live(true)
        }
        Op::Aob(c) => {
            let limit = g.read2(&ops[0], size)?;
            let p = g.place(&ops[1], size)?;
            let v = g.load(&p, size, Ext::Any, false)?;
            let r = g.result(&p, size, &[&v])?;
            g.emit(format!("add {r}, {v}, #1"));
            g.store(&p, size, &r)?;
            g.emit(format!("cmp {r}, {limit}"));
            g.emit(format!("b.{c} {}", target(&ops[2])?));
            live(true)
        }
        Op::Sob(c) => {
            let p = g.place(&ops[0], size)?;
            let v = g.load(&p, size, Ext::Any, false)?;
            let r = g.result(&p, size, &[&v])?;
            g.emit(format!("subs {r}, {v}, #1"));
            g.store(&p, size, &r)?;
            g.emit(format!("b.{c} {}", target(&ops[1])?));
            live(true)
        }
        Op::Ash | Op::Rot => {
            let count = match &ops[0] {
                Opnd::Imm(e) => g.constant(e).map(|n| n as i8),
                _ => None,
            };
            let c = match count {
                Some(_) => String::new(),
                None => g.read(&ops[0], Size::B, Ext::Sext)?,
            };
            let v = g.read(&ops[1], size, Ext::Any)?;
            let p = g.place(&ops[2], size)?;
            let r = g.result(&p, size, &[&v])?;
            let bits = size.bits() as i8;
            let fit = |r: &str| if size == Size::Q { x(r) } else { w(r) };
            match (op, count) {
                (Op::Rot, Some(n)) => g.emit(format!("ror {r}, {v}, #{}", (32 - n as i32) & 31)),
                (Op::Rot, None) => {
                    let t = w(&g.tmp()?);
                    g.emit(format!("neg {t}, {c}"));
                    g.emit(format!("ror {r}, {v}, {t}"));
                }
                (_, Some(n)) if n >= bits => g.emit(format!("mov {r}, {}", fit("wzr"))),
                (_, Some(n)) if n >= 0 => g.emit(format!("lsl {r}, {v}, #{n}")),
                (_, Some(n)) => g.emit(format!(
                    "asr {r}, {v}, #{}",
                    (-(n as i32)).min(bits as i32 - 1)
                )),
                (_, None) => {
                    // ponytail: counts of 32 (64) and up wrap instead of
                    // clearing; a CMP and CSEL each way if code needs them.
                    let (right, done) = (g.label(), g.label());
                    g.emit(format!("tbnz {c}, #31, {right}"));
                    g.emit(format!("lsl {r}, {v}, {}", fit(&c)));
                    g.emit(format!("b {done}"));
                    g.place_label(&right);
                    let t = g.tmp()?;
                    g.emit(format!("neg {}, {c}", w(&t)));
                    g.emit(format!("asr {r}, {v}, {}", fit(&t)));
                    g.place_label(&done);
                }
            }
            g.store(&p, size, &r)?;
            Ok(Some(test(&r, size)))
        }
        Op::Emul => {
            let a = g.read(&ops[0], Size::L, Ext::Any)?;
            let b = g.read(&ops[1], Size::L, Ext::Any)?;
            let c = g.read(&ops[2], Size::L, Ext::Any)?;
            let p = g.place(&ops[3], Size::Q)?;
            let r = x(&g.result(&p, Size::Q, &[&a, &b])?);
            g.emit(format!("smull {r}, {a}, {b}"));
            if c != "wzr" {
                g.emit(format!("add {r}, {r}, {c}, sxtw"));
            }
            g.store(&p, Size::Q, &r)?;
            Ok(Some(test(&r, Size::Q)))
        }
        Op::Ediv => {
            let d = g.read(&ops[0], Size::L, Ext::Any)?;
            let n = g.read(&ops[1], Size::Q, Ext::Any)?;
            let pq = g.place(&ops[2], Size::L)?;
            let pr = g.place(&ops[3], Size::L)?;
            let dx = g.reuse(&d)?;
            g.emit(format!("sxtw {dx}, {d}"));
            let q = g.tmp()?;
            g.emit(format!("sdiv {q}, {n}, {dx}"));
            let rem = g.reuse(&dx)?;
            g.emit(format!("msub {rem}, {q}, {dx}, {n}"));
            g.store(&pq, Size::L, &q)?;
            g.store(&pr, Size::L, &rem)?;
            Ok(Some(test(&q, Size::L)))
        }
        Op::Movc3 => {
            (0..6).for_each(|r| g.wrote(r));
            let len = g.read(&ops[0], Size::W, Ext::Zext)?;
            let len = copy(g, &x(&len))?;
            let src = g.address(&ops[1], size)?;
            let src = copy(g, &src)?;
            let dst = g.address(&ops[2], size)?;
            let dst = copy(g, &dst)?;
            let (fwd, back, back_loop, done) = (g.label(), g.label(), g.label(), g.label());
            g.emit(format!("add x1, {src}, {len}"));
            g.emit(format!("add x20, {dst}, {len}"));
            g.emit(format!("mov x0, {len}"));
            g.emit(format!("cmp {dst}, {src}"));
            g.emit(format!("b.hi {back}"));
            g.place_label(&fwd);
            g.emit(format!("cbz x0, {done}"));
            g.emit(format!("ldrb w22, [{src}], #1"));
            g.emit(format!("strb w22, [{dst}], #1"));
            g.emit("sub x0, x0, #1");
            g.emit(format!("b {fwd}"));
            // The destination overlaps the end of the source: copy backwards.
            g.place_label(&back);
            g.emit("mov x19, x1");
            g.emit("mov x21, x20");
            g.place_label(&back_loop);
            g.emit(format!("cbz x0, {done}"));
            g.emit("ldrb w22, [x19, #-1]!");
            g.emit("strb w22, [x21, #-1]!");
            g.emit("sub x0, x0, #1");
            g.emit(format!("b {back_loop}"));
            g.place_label(&done);
            for r in [2, 4, 5] {
                g.emit(format!("mov x{}, xzr", arm(r)?));
            }
            Ok(Some(test("wzr", Size::L)))
        }
        Op::Movc5 => {
            (0..6).for_each(|r| g.wrote(r));
            let srclen = g.read(&ops[0], Size::W, Ext::Zext)?;
            let srclen = copy(g, &x(&srclen))?;
            let src = g.address(&ops[1], size)?;
            let src = copy(g, &src)?;
            let fill = g.read(&ops[2], Size::B, Ext::Any)?;
            let fill = w(&copy(g, &x(&fill))?);
            let dstlen = g.read(&ops[3], Size::W, Ext::Zext)?;
            let dstlen = copy(g, &x(&dstlen))?;
            let dst = g.address(&ops[4], size)?;
            let dst = copy(g, &dst)?;
            let (copying, filling, done) = (g.label(), g.label(), g.label());
            // ponytail: copies forwards only; MOVC3 handles overlap.
            g.emit(format!("cmp {srclen}, {dstlen}"));
            g.emit(format!("csel x19, {srclen}, {dstlen}, lo"));
            g.emit(format!("sub x0, {srclen}, x19"));
            g.emit(format!("add x1, {src}, x19"));
            g.emit(format!("add x20, {dst}, {dstlen}"));
            g.emit(format!("sub x21, {dstlen}, x19"));
            g.place_label(&copying);
            g.emit(format!("cbz x19, {filling}"));
            g.emit(format!("ldrb w22, [{src}], #1"));
            g.emit(format!("strb w22, [{dst}], #1"));
            g.emit("sub x19, x19, #1");
            g.emit(format!("b {copying}"));
            g.place_label(&filling);
            g.emit(format!("cbz x21, {done}"));
            g.emit(format!("strb {fill}, [{dst}], #1"));
            g.emit("sub x21, x21, #1");
            g.emit(format!("b {filling}"));
            g.place_label(&done);
            g.emit("mov x22, xzr");
            g.emit(format!("cmp {srclen}, {dstlen}"));
            live(true)
        }
        Op::Ext(signed) => {
            let s = field_size(g, &ops[1])?;
            let pos = position(g, &ops[0])?;
            let insn = if signed { "sbfx" } else { "ubfx" };
            let r = match (&ops[2], pos) {
                (_, _) if s == 0 => "wzr".to_string(),
                (Opnd::Reg(n), Ok(p)) => {
                    if !(0..=32 - s).contains(&p) {
                        return Err("the field must be in the register".into());
                    }
                    let t = w(&g.tmp()?);
                    g.emit(format!("{insn} {t}, w{}, #{p}, #{s}", arm(*n)?));
                    t
                }
                (Opnd::Reg(n), Err(v)) => {
                    let t = w(&g.reuse(&v)?);
                    g.emit(format!("lsr {t}, w{}, {v}", arm(*n)?));
                    g.emit(format!("{insn} {t}, {t}, #0, #{s}"));
                    t
                }
                (base, pos) => {
                    let (m, bit) = field_at(g, base, pos)?;
                    let t = g.tmp()?;
                    g.emit(format!("ldr {t}, {m}"));
                    match bit {
                        Ok(b) => g.emit(format!("{insn} {t}, {t}, #{b}, #{s}")),
                        Err(b) => {
                            g.emit(format!("lsr {t}, {t}, {}", x(&b)));
                            g.emit(format!("{insn} {t}, {t}, #0, #{s}"));
                        }
                    }
                    w(&t)
                }
            };
            let p = g.place(&ops[3], Size::L)?;
            g.store(&p, Size::L, &r)?;
            Ok(Some(test(&r, Size::L)))
        }
        Op::Insv => {
            let v = g.read(&ops[0], Size::L, Ext::Any)?;
            let v = g.nonzero(v)?;
            let pos = position(g, &ops[1])?;
            let s = field_size(g, &ops[2])?;
            if s == 0 {
                return Ok(None);
            }
            match (&ops[3], pos) {
                (Opnd::Reg(n), Ok(p)) => {
                    if !(0..=32 - s).contains(&p) {
                        return Err("the field must be in the register".into());
                    }
                    g.emit(format!("bfi w{}, {v}, #{p}, #{s}", arm(*n)?));
                    g.sext(arm(*n)?);
                    g.wrote(*n);
                }
                (Opnd::Reg(n), Err(p)) => {
                    // Rotate the field down to bit 0, insert, rotate back.
                    let r = arm(*n)?;
                    g.emit(format!("ror w{r}, w{r}, {p}"));
                    g.emit(format!("bfi w{r}, {v}, #0, #{s}"));
                    let t = w(&g.tmp()?);
                    g.emit(format!("neg {t}, {p}"));
                    g.emit(format!("ror w{r}, w{r}, {t}"));
                    g.sext(r);
                    g.wrote(*n);
                }
                (base, pos) => {
                    // ponytail: reads and writes 8 bytes around the field,
                    // not atomically, and faults if they cross into an
                    // unmapped page.
                    let (m, bit) = field_at(g, base, pos)?;
                    let t = g.tmp()?;
                    g.emit(format!("ldr {t}, {m}"));
                    match bit {
                        Ok(b) => g.emit(format!("bfi {t}, {}, #{b}, #{s}", x(&v))),
                        Err(b) => {
                            g.emit(format!("ror {t}, {t}, {}", x(&b)));
                            g.emit(format!("bfi {t}, {}, #0, #{s}", x(&v)));
                            g.emit(format!("neg {0}, {0}", w(&b)));
                            g.emit(format!("ror {t}, {t}, {}", x(&b)));
                        }
                    }
                    g.emit(format!("str {t}, {m}"));
                }
            }
            Ok(None)
        }
        Op::Bb(set, change) => {
            let pos = position(g, &ops[0])?;
            let t = target(&ops[2])?.to_string();
            let branch = |bit| match set {
                true => format!("tbnz {bit}, {t}"),
                false => format!("tbz {bit}, {t}"),
            };
            match (&ops[1], pos, change) {
                (Opnd::Reg(n), Ok(p), None) if (0..32).contains(&p) => {
                    g.emit(branch(format!("w{}, #{p}", arm(*n)?)));
                }
                (Opnd::Reg(n), pos, _) => {
                    let r = arm(*n)?;
                    let (old, mask) = (w(&g.tmp()?), w(&g.tmp()?));
                    match &pos {
                        Ok(p) if (0..32).contains(p) => {
                            g.emit(format!("lsr {old}, w{r}, #{p}"));
                            g.imm_into(&x(&mask), &(1i64 << p).to_string(), Size::L)?;
                        }
                        Ok(_) => return Err("the bit must be in the register".into()),
                        Err(p) => {
                            g.emit(format!("lsr {old}, w{r}, {p}"));
                            g.emit(format!("mov {mask}, #1"));
                            g.emit(format!("lsl {mask}, {mask}, {p}"));
                        }
                    }
                    match change {
                        Some(true) => g.emit(format!("orr w{r}, w{r}, {mask}")),
                        Some(false) => g.emit(format!("bic w{r}, w{r}, {mask}")),
                        None => {}
                    }
                    if change.is_some() {
                        g.sext(r);
                        g.wrote(*n);
                    }
                    g.emit(branch(format!("{old}, #0")));
                }
                (base, pos, _) => {
                    let (m, bit) = byte_at(g, base, pos)?;
                    let v = w(&g.tmp()?);
                    g.emit(format!("ldrb {v}, {m}"));
                    let old = w(&g.tmp()?);
                    match &bit {
                        Ok(b) => g.emit(format!("lsr {old}, {v}, #{b}")),
                        Err(b) => g.emit(format!("lsr {old}, {v}, {b}")),
                    }
                    if let Some(on) = change {
                        let mask = w(&g.tmp()?);
                        g.emit(format!("mov {mask}, #1"));
                        match &bit {
                            Ok(b) => g.emit(format!("lsl {mask}, {mask}, #{b}")),
                            Err(b) => g.emit(format!("lsl {mask}, {mask}, {b}")),
                        }
                        let insn = if on { "orr" } else { "bic" };
                        g.emit(format!("{insn} {v}, {v}, {mask}"));
                        g.emit(format!("strb {v}, {m}"));
                    }
                    g.emit(branch(format!("{old}, #0")));
                }
            }
            Ok(None)
        }
        Op::Nop => {
            g.emit("nop");
            Ok(None)
        }
        Op::Halt => {
            pal(g, HALT, None)?;
            Ok(None)
        }
        Op::Bpt => {
            g.emit("brk #0");
            Ok(None)
        }
        Op::Mtpr => {
            let v = g.read(&ops[0], size, Ext::Any)?;
            let (Some(code), _) = ipr(g, &ops[1])? else {
                return Err("MTPR can't write that processor register".into());
            };
            pal(g, code, Some(&v))?;
            Ok(Some(test(&v, size)))
        }
        Op::Mfpr => {
            let (_, Some(code)) = ipr(g, &ops[0])? else {
                return Err("MFPR can't read that processor register".into());
            };
            let p = g.place(&ops[1], size)?;
            let v = w(&pal(g, code, None)?);
            g.store(&p, size, &v)?;
            Ok(Some(test(&v, size)))
        }
        Op::Rei => {
            // The PAL resumes at the PC in the frame, with R7 from it too.
            g.emit(format!("mov x7, #{REI}"));
            g.emit("svc #0");
            Ok(None)
        }
        Op::Chm(code) => {
            // The code goes in R0, as on Alpha, and the service's status
            // comes back there: R0 isn't kept.
            let v = g.read(&ops[0], size, Ext::Sext)?;
            g.emit(format!("mov w0, {v}"));
            g.emit(format!("mov x7, #{code}"));
            g.emit("svc #0");
            Ok(Some(test("w0", Size::L)))
        }
        Op::Probe(write) => {
            // PROBEx mode, len, base: a0 = base, a1 = len, a2 = mode, as
            // Alpha's PROBER and PROBEW take them; v0 = 1 if the mode may
            // read (write) the first and last byte. Each operand is copied
            // first, since x0 and x1 are R0 and R1, which come back. Z is
            // set if it may not, as on the VAX.
            let mode = g.read(&ops[0], Size::B, Ext::Zext)?;
            let mode = copy(g, &mode)?;
            let len = g.read(&ops[1], Size::W, Ext::Zext)?;
            let len = copy(g, &len)?;
            let base = g.address(&ops[2], Size::B)?;
            let base = copy(g, &base)?;
            let (r0, r1) = (g.tmp()?, g.tmp()?);
            let code = if write { PROBEW } else { PROBER };
            for line in [
                format!("mov {r0}, x0"),
                format!("mov {r1}, x1"),
                format!("mov x0, {base}"),
                format!("mov x1, {len}"),
                format!("mov x2, {mode}"),
                format!("mov x7, #{code}"),
                "svc #0".into(),
                format!("mov {mode}, x0"),
                format!("mov x1, {r1}"),
                format!("mov x0, {r0}"),
            ] {
                g.emit(line);
            }
            Ok(Some(test(&mode, Size::L)))
        }
        Op::CallPal => {
            let code = match &ops[0] {
                Opnd::Imm(e) => g.constant(e),
                _ => None,
            };
            let code = code
                .filter(|c| (0..=0xBF).contains(c))
                .ok_or("CALL_PAL needs a function code from 0 to ^XBF, #n")?;
            // Alpha's arguments a0-a5 are R0-R5, in x0-x5 for the PAL.
            for r in 2..6 {
                g.emit(format!("mov x{r}, x{}", arm(r)?));
            }
            g.emit(format!("mov x7, #{code}"));
            g.emit("svc #0");
            Ok(Some(test("w0", Size::L)))
        }
        Op::Ldq | Op::Stq => {
            let Opnd::Reg(n) = ops[0] else {
                return Err(format!("{mn}'s first operand is a register"));
            };
            let r = format!("x{}", arm(n)?);
            if op == Op::Ldq {
                g.wrote(n);
            }
            match (op, &ops[1]) {
                (Op::Ldq, Opnd::Reg(m)) => g.emit(format!("mov {r}, x{}", arm(*m)?)),
                (Op::Ldq, Opnd::Imm(e)) => g.imm_into(&r, e, size)?,
                (Op::Ldq, o) => {
                    let Place::Mem(base, disp) = g.place(o, size)? else {
                        unreachable!("a register or an immediate");
                    };
                    let m = g.at(&base, disp, size)?;
                    g.emit(format!("ldr {r}, {m}"));
                }
                (_, Opnd::Reg(m)) => {
                    g.wrote(*m);
                    g.emit(format!("mov x{}, {r}", arm(*m)?));
                }
                (_, o) => {
                    let p = g.place(o, size)?;
                    g.store(&p, size, &r)?;
                }
            }
            Ok(None)
        }
        Op::Insque => {
            // Links are longwords: the queue must be below 4 GB.
            let e = g.address(&ops[0], size)?;
            let p = g.address(&ops[1], size)?;
            let s = w(&g.tmp()?);
            g.emit(format!("ldr {s}, [{p}]"));
            g.emit(format!("str {s}, [{e}]"));
            g.emit(format!("str {}, [{e}, #4]", w(&p)));
            g.emit(format!("str {}, [{}, #4]", w(&e), x(&s)));
            g.emit(format!("str {}, [{p}]", w(&e)));
            // Z: the entry is the only one.
            g.emit(format!("cmp {s}, {}", w(&p)));
            live(true)
        }
        Op::Remque => {
            let e = g.address(&ops[0], size)?;
            let (f, b) = (w(&g.tmp()?), w(&g.tmp()?));
            g.emit(format!("ldr {f}, [{e}]"));
            g.emit(format!("ldr {b}, [{e}, #4]"));
            g.emit(format!("str {f}, [{}]", x(&b)));
            g.emit(format!("str {b}, [{}, #4]", x(&f)));
            let p = g.place(&ops[1], Size::L)?;
            g.store(&p, Size::L, &e)?;
            // V: the queue was empty, and the "entry" its header. Z: it is
            // empty now.
            let (full, done) = (g.label(), g.label());
            g.emit(format!("cmp {}, {f}", w(&e)));
            g.emit(format!("b.ne {full}"));
            g.emit(format!("movz {b}, #0x7000, lsl #16"));
            g.emit(format!("msr nzcv, {}", x(&b)));
            g.emit(format!("b {done}"));
            g.place_label(&full);
            g.emit(format!("cmp {f}, {b}"));
            g.place_label(&done);
            live(true)
        }
    }
}

/// PAL function codes (docs/design/0001-pal-interface.md): Alpha OpenVMS's,
/// and vaxpunk's own from 0x40.
const HALT: u32 = 0x00;
const MFPR_IPL: u32 = 0x0E;
const MTPR_IPL: u32 = 0x0F;
const MFPR_PCBB: u32 = 0x12;
const MFPR_SCBB: u32 = 0x16;
const MTPR_SCBB: u32 = 0x17;
const MTPR_SIRR: u32 = 0x18;
const MFPR_SISR: u32 = 0x19;
const MTPR_TXDB: u32 = 0x40;
const MTPR_RXCS: u32 = 0x43;
const MFPR_RXCS: u32 = 0x44;
const MFPR_RXDB: u32 = 0x45;
const MTPR_DOORBELL: u32 = 0x48;
const CHME: u32 = 0x82;
const CHMK: u32 = 0x83;
const CHMS: u32 = 0x84;
const CHMU: u32 = 0x85;
const PROBER: u32 = 0x8F;
const PROBEW: u32 = 0x90;
const REI: u32 = 0x92;

/// The PAL calls that write and read a VAX processor register, by its
/// number (PR$_ in $PRDEF), which must be a constant, as in AMACRO.
fn ipr(g: &mut Gen, op: &Opnd) -> Result<(Option<u32>, Option<u32>)> {
    let n = match op {
        Opnd::Imm(e) => g.constant(e),
        _ => None,
    };
    match n.ok_or("vmacro needs the processor register as a constant, #n")? {
        16 => Ok((None, Some(MFPR_PCBB))), // PR$_PCBB, read-only
        17 => Ok((Some(MTPR_SCBB), Some(MFPR_SCBB))), // PR$_SCBB
        18 => Ok((Some(MTPR_IPL), Some(MFPR_IPL))), // PR$_IPL
        20 => Ok((Some(MTPR_SIRR), None)), // PR$_SIRR, write-only
        21 => Ok((None, Some(MFPR_SISR))), // PR$_SISR, read-only
        32 => Ok((Some(MTPR_RXCS), Some(MFPR_RXCS))), // PR$_RXCS
        33 => Ok((None, Some(MFPR_RXDB))), // PR$_RXDB, read-only
        35 => Ok((Some(MTPR_TXDB), None)), // PR$_TXDB, write-only
        64 => Ok((Some(MTPR_DOORBELL), None)), // PR$_DOORBELL, write-only
        n => Err(format!("processor register {n} has no PAL call yet")),
    }
}

/// A PAL call: `svc #0` with the function code in x7, the argument and the
/// result in x0 (docs/design/0001-pal-interface.md). x0 is VAX R0, so it
/// waits in a scratch register. Returns the result's register.
fn pal(g: &mut Gen, code: u32, arg: Option<&str>) -> Result<String> {
    let (r0, v) = (g.tmp()?, g.tmp()?);
    g.emit(format!("mov {r0}, x0"));
    if let Some(a) = arg {
        g.emit(format!("mov w0, {}", w(a)));
    }
    g.emit(format!("mov x7, #{code}"));
    g.emit("svc #0");
    g.emit(format!("mov {v}, x0"));
    g.emit(format!("mov x0, {r0}"));
    Ok(v)
}

/// `v` in a scratch register, as an x register, so that R0-R5 can change.
fn copy(g: &mut Gen, v: &str) -> Result<String> {
    if Gen::is_tmp(v) {
        return Ok(x(v));
    }
    let t = g.tmp()?;
    g.emit(format!("mov {t}, {}", x(v)));
    Ok(t)
}

/// A bit field's size, which vmacro needs to know: 0 to 32.
fn field_size(g: &mut Gen, op: &Opnd) -> Result<i64> {
    let s = match op {
        Opnd::Imm(e) => g.constant(e),
        _ => None,
    };
    match s {
        Some(s) if (0..=32).contains(&s) => Ok(s),
        Some(_) => Err("a bit field is 0 to 32 bits".into()),
        None => Err("vmacro needs the field size as a constant, #n".into()),
    }
}

/// A bit position: known now, or in a register (w).
fn position(g: &mut Gen, op: &Opnd) -> Result<std::result::Result<i64, String>> {
    if let Opnd::Imm(e) = op
        && let Some(p) = g.constant(e)
    {
        return Ok(Ok(p));
    }
    let v = g.read(op, Size::L, Ext::Any)?;
    Ok(Err(g.nonzero(v)?))
}

/// Where a field in memory starts: a quadword's address and the bit in it.
fn field_at(
    g: &mut Gen,
    base: &Opnd,
    pos: std::result::Result<i64, String>,
) -> Result<(String, std::result::Result<i64, String>)> {
    access_at(g, base, pos, Size::Q)
}

/// The byte a bit in memory is in, and the bit in the byte.
fn byte_at(
    g: &mut Gen,
    base: &Opnd,
    pos: std::result::Result<i64, String>,
) -> Result<(String, std::result::Result<i64, String>)> {
    access_at(g, base, pos, Size::B)
}

fn access_at(
    g: &mut Gen,
    base: &Opnd,
    pos: std::result::Result<i64, String>,
    size: Size,
) -> Result<(String, std::result::Result<i64, String>)> {
    let a = g.address(base, Size::B)?;
    match pos {
        Ok(p) => {
            let m = g.at(&a, p.div_euclid(8), size)?;
            Ok((m, Ok(p.rem_euclid(8))))
        }
        Err(p) => {
            // Byte offset, arithmetic: a bit position is signed.
            let t = g.tmp()?;
            g.emit(format!("asr {}, {p}, #3", w(&t)));
            g.emit(format!("add {t}, {a}, {}, sxtw", w(&t)));
            let b = w(&g.reuse(&p)?);
            g.emit(format!("and {b}, {p}, #7"));
            Ok((format!("[{t}]"), Err(b)))
        }
    }
}
