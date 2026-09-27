//! What the linker computes: TIR operators, and how it patches the field each
//! ARM64 store command names (docs/object-format.md). Also what each value
//! and store means for moving the image (docs/linker.md).

use crate::obj::Tir;

/// How a value changes when the image moves by D, a multiple of 64 KB: it is
/// a constant plus k × D, for its weight k. None when it can't be written that
/// way, as for an address shifted right.
pub type Weight = Option<i64>;

/// How many values operator `op` pops, or None if it isn't an operator with
/// a result.
pub fn operands(op: &Tir) -> Option<usize> {
    use Tir::*;
    Some(match op {
        OprNeg {} | OprCom {} => 1,
        OprAdd {}
        | OprSub {}
        | OprMul {}
        | OprDiv {}
        | OprAnd {}
        | OprIor {}
        | OprEor {}
        | OprAsh {}
        | OprRot {} => 2,
        OprSel {} => 3,
        _ => return None,
    })
}

/// What operator `op` pushes, from the values it pops, first pushed first,
/// each with its weight. Arithmetic is on signed 64-bit values.
pub fn operate(op: &Tir, args: &[(u64, Weight)]) -> (u64, Weight) {
    use Tir::*;
    let v = |i: usize| args[i].0;
    let value = match op {
        OprNeg {} => v(0).wrapping_neg(),
        OprCom {} => !v(0),
        OprAdd {} => v(0).wrapping_add(v(1)),
        OprSub {} => v(0).wrapping_sub(v(1)),
        OprMul {} => v(0).wrapping_mul(v(1)),
        OprDiv {} if v(1) == 0 => 0,
        OprDiv {} => (v(0) as i64).wrapping_div(v(1) as i64) as u64,
        OprAnd {} => v(0) & v(1),
        OprIor {} => v(0) | v(1),
        OprEor {} => v(0) ^ v(1),
        // The first value pushed is the count: positive left.
        OprAsh {} => match v(0) as i64 {
            n if n >= 0 => v(1).wrapping_shl(n as u32),
            n => ((v(1) as i64).wrapping_shr(n.unsigned_abs() as u32)) as u64,
        },
        OprRot {} => match v(0) as i64 {
            n if n >= 0 => v(1).rotate_left(n as u32),
            n => v(1).rotate_right(n.unsigned_abs() as u32),
        },
        // The condition is pushed last; the first value pushed is for false.
        OprSel {} if v(2) & 1 != 0 => v(1),
        OprSel {} => v(0),
        _ => 0,
    };
    (value, weight(op, args))
}

fn weight(op: &Tir, args: &[(u64, Weight)]) -> Weight {
    match (op, args) {
        (Tir::OprAdd {}, &[(_, a), (_, b)]) => a?.checked_add(b?),
        (Tir::OprSub {}, &[(_, a), (_, b)]) => a?.checked_sub(b?),
        (Tir::OprNeg {}, &[(_, a)]) => a?.checked_neg(),
        // Still linear in D if one side is a constant.
        (Tir::OprMul {}, &[(x, a), (y, b)]) => match (a?, b?) {
            (0, b) => b.checked_mul(x as i64),
            (a, 0) => a.checked_mul(y as i64),
            _ => None,
        },
        _ => args.iter().all(|&(_, k)| k == Some(0)).then_some(0),
    }
}

/// What a store needs for the image to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    /// Nothing: the stored bits stay right wherever the image goes.
    Nothing,
    /// A fixup: the loader adds D to the stored quadword (8) or longword (4).
    Fixup(u8),
    /// Nothing can make it right.
    Impossible,
}

/// What store command `cmd` needs when the value it stores has weight `k`.
/// Where it stores, P, has weight 1, so a PC-relative field, S − P, has
/// weight k − 1. Commands that store no computed value need nothing.
pub fn need(cmd: &Tir, k: Weight) -> Need {
    use Need::*;
    let data = |address| match k {
        Some(0) => Nothing,
        Some(1) => address,
        _ => Impossible,
    };
    match cmd {
        Tir::StoQw {} | Tir::StoOff {} | Tir::StoGbl { .. } | Tir::StoCa { .. } => data(Fixup(8)),
        Tir::StoLw {} => data(Fixup(4)),
        Tir::StoW {} | Tir::StoB {} | Tir::StoImmr { .. } => data(Impossible),
        _ => {
            let Some((kind, _)) = Kind::of(cmd) else {
                return Nothing;
            };
            let ok = match kind {
                Kind::Jump26 | Kind::Branch19 | Kind::Branch14 | Kind::Adr | Kind::Adrp => {
                    k == Some(1)
                }
                // D is a multiple of 64 KB, so the low 12 and 16 bits never
                // change. The checked low chunk also promises that the rest is
                // zero, which moving breaks.
                Kind::AddLo12
                | Kind::Ldst(_)
                | Kind::Movw {
                    chunk: 0,
                    check: false,
                } => matches!(k, Some(0 | 1)),
                Kind::Movw { .. } => k == Some(0),
            };
            if ok { Nothing } else { Impossible }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Jump26,
    Branch19,
    Branch14,
    Adr,
    Adrp,
    AddLo12,
    /// A load or store, with log2 of its access size.
    Ldst(u32),
    /// A 16-bit chunk for MOVZ/MOVK, and whether the higher bits must be zero.
    Movw {
        chunk: u32,
        check: bool,
    },
}

impl Kind {
    /// The relocation an ARM64 store command applies, with its instruction.
    pub fn of(cmd: &Tir) -> Option<(Kind, u32)> {
        use Kind::*;
        Some(match *cmd {
            Tir::StoA64Jump26 { insn } => (Jump26, insn),
            Tir::StoA64Branch19 { insn } => (Branch19, insn),
            Tir::StoA64Branch14 { insn } => (Branch14, insn),
            Tir::StoA64Adr { insn } => (Adr, insn),
            Tir::StoA64Adrp { insn } => (Adrp, insn),
            Tir::StoA64AddLo12 { insn } => (AddLo12, insn),
            Tir::StoA64Ldst8Lo12 { insn } => (Ldst(0), insn),
            Tir::StoA64Ldst16Lo12 { insn } => (Ldst(1), insn),
            Tir::StoA64Ldst32Lo12 { insn } => (Ldst(2), insn),
            Tir::StoA64Ldst64Lo12 { insn } => (Ldst(3), insn),
            Tir::StoA64Ldst128Lo12 { insn } => (Ldst(4), insn),
            Tir::StoA64MovwG0 { insn } => (
                Movw {
                    chunk: 0,
                    check: true,
                },
                insn,
            ),
            Tir::StoA64MovwG0Nc { insn } => (
                Movw {
                    chunk: 0,
                    check: false,
                },
                insn,
            ),
            Tir::StoA64MovwG1 { insn } => (
                Movw {
                    chunk: 1,
                    check: true,
                },
                insn,
            ),
            Tir::StoA64MovwG1Nc { insn } => (
                Movw {
                    chunk: 1,
                    check: false,
                },
                insn,
            ),
            Tir::StoA64MovwG2 { insn } => (
                Movw {
                    chunk: 2,
                    check: true,
                },
                insn,
            ),
            Tir::StoA64MovwG2Nc { insn } => (
                Movw {
                    chunk: 2,
                    check: false,
                },
                insn,
            ),
            Tir::StoA64MovwG3 { insn } => (
                Movw {
                    chunk: 3,
                    check: false,
                },
                insn,
            ),
            _ => return None,
        })
    }
}

/// Patches `insn`, stored at address `p`, to refer to `s`, or says why it
/// can't.
pub fn apply(kind: Kind, insn: u32, s: u64, p: u64) -> Result<u32, &'static str> {
    let d = s.wrapping_sub(p) as i64;
    let (mask, field) = match kind {
        Kind::Jump26 => (
            0x03ff_ffff,
            branch(d, 26, "branch target out of range (±128 MB)")?,
        ),
        Kind::Branch19 => (
            0x00ff_ffe0,
            branch(d, 19, "branch target out of range (±1 MB)")? << 5,
        ),
        Kind::Branch14 => (
            0x0007_ffe0,
            branch(d, 14, "branch target out of range (±32 KB)")? << 5,
        ),
        Kind::Adr => {
            if !(-(1 << 20)..1 << 20).contains(&d) {
                return Err("ADR target out of range (±1 MB)");
            }
            (0x60ff_ffe0, adr(d))
        }
        Kind::Adrp => {
            let pages = ((s & !0xfff) as i64).wrapping_sub((p & !0xfff) as i64) >> 12;
            if !(-(1 << 20)..1 << 20).contains(&pages) {
                return Err("ADRP target out of range (±4 GB)");
            }
            (0x60ff_ffe0, adr(pages))
        }
        Kind::AddLo12 => (0x003f_fc00, ((s & 0xfff) as u32) << 10),
        Kind::Ldst(scale) => {
            let off = s & 0xfff;
            if !off.is_multiple_of(1 << scale) {
                return Err("address not aligned to the access size");
            }
            (0x003f_fc00, ((off >> scale) as u32) << 10)
        }
        Kind::Movw { chunk, check } => {
            if check && s >> (16 * (chunk + 1)) != 0 {
                return Err("address too large for this MOVZ/MOVK chunk");
            }
            (0x001f_ffe0, (((s >> (16 * chunk)) & 0xffff) as u32) << 5)
        }
    };
    Ok(insn & !mask | field)
}

fn branch(d: i64, bits: u32, range: &'static str) -> Result<u32, &'static str> {
    if d % 4 != 0 {
        return Err("branch target not 4-byte aligned");
    }
    let limit = 1i64 << (bits + 1);
    if d < -limit || d >= limit {
        return Err(range);
    }
    Ok(((d >> 2) as u32) & ((1 << bits) - 1))
}

/// ADR's split immediate: immlo in bits 29-30, immhi in bits 5-23.
fn adr(v: i64) -> u32 {
    let v = v as u32;
    (v & 3) << 29 | ((v >> 2) & 0x7ffff) << 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn patches() {
        assert_eq!(
            apply(Kind::Jump26, 0x9400_0000, 0x10008, 0x10000),
            Ok(0x9400_0002)
        ); // bl .+8
        assert_eq!(
            apply(Kind::Jump26, 0x1400_0000, 0x10000, 0x10008),
            Ok(0x17ff_fffe)
        ); // b .-8
        assert_eq!(
            apply(Kind::Branch19, 0x5400_0000, 0x10010, 0x10000),
            Ok(0x5400_0080)
        ); // b.eq .+16
        assert_eq!(
            apply(Kind::Adr, 0x1000_0000, 0x10014, 0x10000),
            Ok(0x1000_00a0)
        ); // adr x0, .+20
        // adrp x0: 17 pages up, immlo 1 (bit 29) and immhi 4.
        assert_eq!(
            apply(Kind::Adrp, 0x9000_0000, 0x21234, 0x10000),
            Ok(0xb000_0080)
        );
        assert_eq!(
            apply(Kind::AddLo12, 0x9100_0000, 0x21234, 0),
            Ok(0x9108_d000)
        ); // #0x234
        assert_eq!(
            apply(Kind::Ldst(3), 0xf940_0000, 0x21238, 0),
            Ok(0xf941_1c00)
        ); // #0x238
        assert_eq!(
            apply(
                Kind::Movw {
                    chunk: 1,
                    check: true
                },
                0xd2a0_0000,
                0x1234_5678,
                0
            ),
            Ok(0xd2a2_4680)
        );

        assert!(
            apply(Kind::Jump26, 0x1400_0000, 1 << 27, 0).is_err(),
            "just out of range"
        );
        assert!(
            apply(Kind::Jump26, 0x1400_0000, (1 << 27) - 4, 0).is_ok(),
            "just in range"
        );
        assert!(
            apply(Kind::Ldst(3), 0xf940_0000, 0x21234, 0).is_err(),
            "misaligned"
        );
        assert!(
            apply(
                Kind::Movw {
                    chunk: 0,
                    check: true
                },
                0xd280_0000,
                0x10000,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn weights() {
        let (lw, addr, unknown) = ((3, Some(0)), (0x10000, Some(1)), (0, None));
        assert_eq!(
            operate(&Tir::OprSub {}, &[addr, addr]).1,
            Some(0),
            "label - ."
        );
        assert_eq!(operate(&Tir::OprAdd {}, &[addr, lw]).1, Some(1));
        assert_eq!(operate(&Tir::OprNeg {}, &[addr]).1, Some(-1));
        assert_eq!(operate(&Tir::OprMul {}, &[lw, addr]), (0x30000, Some(3)));
        assert_eq!(operate(&Tir::OprMul {}, &[addr, addr]).1, None);
        assert_eq!(
            operate(&Tir::OprAsh {}, &[(-12i64 as u64, Some(0)), addr]),
            (0x10, None)
        );
        assert_eq!(operate(&Tir::OprAnd {}, &[lw, lw]), (3, Some(0)));
        assert_eq!(operate(&Tir::OprAdd {}, &[unknown, lw]).1, None);
        assert_eq!(
            operate(&Tir::OprSel {}, &[(1, Some(0)), (2, Some(0)), (1, Some(0))]).0,
            2
        );

        let adrp = Tir::StoA64Adrp { insn: 0 };
        let g0 = Tir::StoA64MovwG0Nc { insn: 0 };
        let cases = [
            (Tir::StoOff {}, Some(1), Need::Fixup(8)),
            (Tir::StoLw {}, Some(1), Need::Fixup(4)),
            (Tir::StoQw {}, Some(0), Need::Nothing),
            (Tir::StoQw {}, Some(2), Need::Impossible),
            (Tir::StoW {}, Some(1), Need::Impossible),
            (adrp.clone(), Some(1), Need::Nothing),
            (adrp, Some(0), Need::Impossible),
            (Tir::StoA64Ldst64Lo12 { insn: 0 }, Some(1), Need::Nothing),
            (g0.clone(), Some(1), Need::Nothing),
            (g0, None, Need::Impossible),
            (Tir::StoA64MovwG0 { insn: 0 }, Some(1), Need::Impossible),
            (Tir::StoA64MovwG1 { insn: 0 }, Some(1), Need::Impossible),
            (Tir::StoA64MovwG1 { insn: 0 }, Some(0), Need::Nothing),
            (Tir::StoImm { data: vec![] }, None, Need::Nothing),
        ];
        for (cmd, k, want) in cases {
            assert_eq!(need(&cmd, k), want, "{} with weight {k:?}", cmd.name());
        }
    }
}
