//! ARM64 instruction relocations: how the linker patches the field each ARM64
//! store command names (docs/object-format.md).

use crate::obj::Tir;

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
}
