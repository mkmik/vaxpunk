//! Object modules (OBJ): the Alpha object language with the ARM64 changes in
//! `docs/object-format.md`. A file is a sequence of records.

use alloc::string::String;
use alloc::vec::Vec;

use crate::record::{Field, Reader, record};
use crate::{ARCH_ARM64, Error};

/// Record types (`EOBJ$C_*`).
pub const EMH: u16 = 8;
pub const EEOM: u16 = 9;
pub const EGSD: u16 = 10;
pub const ETIR: u16 = 11;
pub const EDBG: u16 = 12;
pub const ETBT: u16 = 13;

/// Maximum record size (`EOBJ$C_MAXRECSIZ`).
pub const MAX_RECORD: usize = 8192;
/// Structure level (`EOBJ$C_STRLVL`).
pub const STRLVL: u8 = 2;

/// Module header subtypes (`EMH$C_*`).
pub const MHD: u16 = 0;
pub const LNM: u16 = 1;

/// Psect flags (`EGPS$V_*`).
pub mod psc {
    pub const PIC: u16 = 1 << 0;
    pub const LIB: u16 = 1 << 1;
    pub const OVR: u16 = 1 << 2;
    pub const REL: u16 = 1 << 3;
    pub const GBL: u16 = 1 << 4;
    pub const SHR: u16 = 1 << 5;
    pub const EXE: u16 = 1 << 6;
    pub const RD: u16 = 1 << 7;
    pub const WRT: u16 = 1 << 8;
    pub const VEC: u16 = 1 << 9;
    pub const NOMOD: u16 = 1 << 10;
    pub const COM: u16 = 1 << 11;
    pub const ALLOC_64BIT: u16 = 1 << 12;
    /// Names of the flags, bit 0 first.
    pub const NAMES: [&str; 13] = [
        "PIC",
        "LIB",
        "OVR",
        "REL",
        "GBL",
        "SHR",
        "EXE",
        "RD",
        "WRT",
        "VEC",
        "NOMOD",
        "COM",
        "ALLOC_64BIT",
    ];
}

/// Symbol flags (`EGSY$V_*`).
pub mod sym {
    pub const WEAK: u16 = 1 << 0;
    pub const DEF: u16 = 1 << 1;
    pub const REL: u16 = 1 << 3;
    pub const COMM: u16 = 1 << 4;
    pub const NORM: u16 = 1 << 6;
    pub const QUAD_VAL: u16 = 1 << 7;
    /// Names of the flags, bit 0 first.
    pub const NAMES: [&str; 8] = [
        "WEAK", "DEF", "UNI", "REL", "COMM", "VECEP", "NORM", "QUAD_VAL",
    ];
}

/// One object record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// Main module header (`EMH$C_MHD`).
    Mhd(Mhd),
    /// Another module header subtype (`EMH$C_LNM`, `SRC`, `TTL`, ...): text up
    /// to the end of the record.
    Text { subtype: u16, text: Vec<u8> },
    /// Global symbol directory (`EOBJ$C_EGSD`).
    Gsd(Vec<Gsd>),
    /// Text, information and relocation (`EOBJ$C_ETIR`).
    Tir(Vec<Tir>),
    /// Debugger information (`EOBJ$C_EDBG`): TIR commands.
    Dbg(Vec<Tir>),
    /// Traceback information (`EOBJ$C_ETBT`): TIR commands.
    Tbt(Vec<Tir>),
    /// End of module (`EOBJ$C_EEOM`), with the transfer address if any.
    Eom(Eom, Option<Transfer>),
}

record! {
    /// Main module header, after the record type, size and subtype.
    pub struct Mhd {
        pub strlvl: u8,
        pub temp: u8,
        /// Architecture code, [`ARCH_ARM64`]. Unused on Alpha.
        pub arch1: u32,
        pub arch2: u32,
        /// Size of the longest record in the module.
        pub recsiz: u32,
        pub name: String,
        pub version: String,
        /// Creation time, `dd-mmm-yyyy hh:mm`.
        pub date: [u8; 17],
    }
}

/// A GSD subrecord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gsd {
    /// Program section definition (`EGSD$C_PSC`).
    Psc(Psc),
    /// Symbol definition (`EGSD$C_SYM` with `EGSY$V_DEF`).
    Def(SymDef),
    /// Symbol reference (`EGSD$C_SYM` without `EGSY$V_DEF`).
    Ref(SymRef),
    /// Any other subrecord, kept as bytes (`IDC`, `PSC64`, the linker's own).
    Other { gsdtyp: u16, data: Vec<u8> },
}

record! {
    /// Program section definition (`EGPS$`).
    pub struct Psc {
        /// Alignment, as a power of two.
        pub align: u8,
        pub temp: u8,
        pub flags: u16,
        /// Size of this module's contribution.
        pub alloc: u32,
        pub name: String,
    }
}

record! {
    /// Symbol definition (`EGSY$` then `ESDF$`).
    pub struct SymDef {
        pub datyp: u8,
        pub temp: u8,
        pub flags: u16,
        /// Offset in the psect, or the value of an absolute symbol. For a
        /// procedure (`NORM`), the offset of its procedure descriptor.
        pub value: u64,
        /// For a procedure, the offset of its entry point in `ca_psindx`.
        pub code_address: u64,
        pub ca_psindx: u32,
        pub psindx: u32,
        pub name: String,
    }
}

record! {
    /// Symbol reference (`EGSY$` then `ESRF$`).
    pub struct SymRef {
        pub datyp: u8,
        pub temp: u8,
        pub flags: u16,
        pub name: String,
    }
}

record! {
    /// End of module, without the transfer address.
    pub struct Eom {
        pub total_lps: u32,
        /// 0 success, 1 warning, 2 error, 3 abort.
        pub comcod: u16,
    }
}

record! {
    /// Transfer address at the end of a module.
    pub struct Transfer {
        /// Bit 0: weak transfer address.
        pub tfrflg: u8,
        pub temp: u8,
        pub psindx: u32,
        pub tfradr: u64,
    }
}

/// Declares the TIR commands: code, VMS name and arguments, once each.
macro_rules! commands {
    ($($(#[$m:meta])* $code:literal $name:literal $v:ident { $($f:ident: $t:ty),* },)*) => {
        /// A text, information and relocation command (`ETIR$C_*`).
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Tir {
            $($(#[$m])* $v { $($f: $t),* },)*
            /// A command this crate doesn't know, with its argument bytes.
            Other { code: u16, args: Vec<u8> },
        }

        impl Tir {
            pub fn code(&self) -> u16 {
                match self {
                    $(Tir::$v { .. } => $code,)*
                    Tir::Other { code, .. } => *code,
                }
            }

            /// The VMS name, without the `ETIR$C_` prefix.
            pub fn name(&self) -> &'static str {
                match self {
                    $(Tir::$v { .. } => $name,)*
                    Tir::Other { .. } => "?",
                }
            }

            fn read_args(code: u16, r: &mut Reader<'_>) -> Result<Tir, Error> {
                Ok(match code {
                    $($code => Tir::$v { $($f: Field::read(r)?),* },)*
                    _ => Tir::Other { code, args: r.0.to_vec() },
                })
            }

            fn write_args(&self, out: &mut Vec<u8>) {
                match self {
                    $(Tir::$v { $($f),* } => { $(Field::put($f, out);)* })*
                    Tir::Other { args, .. } => out.extend_from_slice(args),
                }
            }
        }
    };
}

commands! {
    /// Push a symbol's value.
    0 "STA_GBL" StaGbl { name: String },
    /// Push a longword, sign-extended.
    1 "STA_LW" StaLw { value: u32 },
    2 "STA_QW" StaQw { value: u64 },
    /// Push a psect base plus an offset.
    3 "STA_PQ" StaPq { psect: u32, offset: u64 },
    /// Pop and store a byte, word, longword or quadword.
    50 "STO_B" StoB {},
    51 "STO_W" StoW {},
    52 "STO_LW" StoLw {},
    53 "STO_QW" StoQw {},
    /// Pop a repeat count and store the data that many times.
    54 "STO_IMMR" StoImmr { data: Vec<u8> },
    /// Store a symbol's value as a quadword.
    55 "STO_GBL" StoGbl { name: String },
    /// Store a procedure's entry point as a quadword.
    56 "STO_CA" StoCa { name: String },
    /// Pop a psect base plus offset and store it as a quadword address.
    59 "STO_OFF" StoOff {},
    /// Store the data.
    61 "STO_IMM" StoImm { data: Vec<u8> },
    /// ARM64 instruction stores: pop the target and store `insn` with the
    /// field patched.
    80 "STO_A64_JUMP26" StoA64Jump26 { insn: u32 },
    81 "STO_A64_BRANCH19" StoA64Branch19 { insn: u32 },
    82 "STO_A64_BRANCH14" StoA64Branch14 { insn: u32 },
    83 "STO_A64_ADR" StoA64Adr { insn: u32 },
    84 "STO_A64_ADRP" StoA64Adrp { insn: u32 },
    85 "STO_A64_ADD_LO12" StoA64AddLo12 { insn: u32 },
    86 "STO_A64_LDST8_LO12" StoA64Ldst8Lo12 { insn: u32 },
    87 "STO_A64_LDST16_LO12" StoA64Ldst16Lo12 { insn: u32 },
    88 "STO_A64_LDST32_LO12" StoA64Ldst32Lo12 { insn: u32 },
    89 "STO_A64_LDST64_LO12" StoA64Ldst64Lo12 { insn: u32 },
    90 "STO_A64_LDST128_LO12" StoA64Ldst128Lo12 { insn: u32 },
    91 "STO_A64_MOVW_G0" StoA64MovwG0 { insn: u32 },
    92 "STO_A64_MOVW_G0_NC" StoA64MovwG0Nc { insn: u32 },
    93 "STO_A64_MOVW_G1" StoA64MovwG1 { insn: u32 },
    94 "STO_A64_MOVW_G1_NC" StoA64MovwG1Nc { insn: u32 },
    95 "STO_A64_MOVW_G2" StoA64MovwG2 { insn: u32 },
    96 "STO_A64_MOVW_G2_NC" StoA64MovwG2Nc { insn: u32 },
    97 "STO_A64_MOVW_G3" StoA64MovwG3 { insn: u32 },
    /// Operators: pop operands, push the result.
    100 "OPR_NOP" OprNop {},
    101 "OPR_ADD" OprAdd {},
    102 "OPR_SUB" OprSub {},
    103 "OPR_MUL" OprMul {},
    104 "OPR_DIV" OprDiv {},
    105 "OPR_AND" OprAnd {},
    106 "OPR_IOR" OprIor {},
    107 "OPR_EOR" OprEor {},
    108 "OPR_NEG" OprNeg {},
    109 "OPR_COM" OprCom {},
    111 "OPR_ASH" OprAsh {},
    113 "OPR_ROT" OprRot {},
    114 "OPR_SEL" OprSel {},
    /// Pop and set the location counter.
    150 "CTL_SETRB" CtlSetrb {},
    /// Add a signed longword to the location counter.
    151 "CTL_AUGRB" CtlAugrb { offset: u32 },
    152 "CTL_DFLOC" CtlDfloc {},
    153 "CTL_STLOC" CtlStloc {},
    154 "CTL_STKDL" CtlStkdl {},
}

/// Parses an object file. Every module must be for ARM64.
pub fn parse(file: &[u8]) -> Result<Vec<Record>, Error> {
    let mut r = Reader(file);
    let mut records = Vec::new();
    while !r.0.is_empty() {
        let rectyp = u16::read(&mut r)?;
        let size = u16::read(&mut r)? as usize;
        let body = r.take(size.checked_sub(4).ok_or(Error::Invalid("record size"))?)?;
        records.push(parse_record(rectyp, body)?);
    }
    Ok(records)
}

fn parse_record(rectyp: u16, body: &[u8]) -> Result<Record, Error> {
    let mut r = Reader(body);
    Ok(match rectyp {
        EMH => match u16::read(&mut r)? {
            MHD => {
                let mhd = Mhd::read(&mut r)?;
                if mhd.arch1 != ARCH_ARM64 {
                    return Err(Error::Invalid("architecture"));
                }
                Record::Mhd(mhd)
            }
            subtype => Record::Text {
                subtype,
                text: r.0.to_vec(),
            },
        },
        EEOM => {
            let eom = Eom::read(&mut r)?;
            let transfer = if r.0.is_empty() {
                None
            } else {
                Some(Transfer::read(&mut r)?)
            };
            Record::Eom(eom, transfer)
        }
        EGSD => {
            u32::read(&mut r)?; // EGSD$L_ALIGNLW
            let mut subrecords = Vec::new();
            while !r.0.is_empty() {
                let gsdtyp = u16::read(&mut r)?;
                let size = u16::read(&mut r)? as usize;
                let body = r.take(size.checked_sub(4).ok_or(Error::Invalid("GSD size"))?)?;
                subrecords.push(match gsdtyp {
                    0 => Gsd::Psc(Psc::parse(body)?),
                    1 if u16::read(&mut Reader(body.get(2..).unwrap_or(&[])))? & sym::DEF != 0 => {
                        Gsd::Def(SymDef::parse(body)?)
                    }
                    1 => Gsd::Ref(SymRef::parse(body)?),
                    _ => Gsd::Other {
                        gsdtyp,
                        data: body.to_vec(),
                    },
                });
            }
            Record::Gsd(subrecords)
        }
        ETIR | EDBG | ETBT => {
            let mut commands = Vec::new();
            while !r.0.is_empty() {
                let code = u16::read(&mut r)?;
                let size = u16::read(&mut r)? as usize;
                let args = r.take(size.checked_sub(4).ok_or(Error::Invalid("TIR size"))?)?;
                commands.push(Tir::read_args(code, &mut Reader(args))?);
            }
            match rectyp {
                ETIR => Record::Tir(commands),
                EDBG => Record::Dbg(commands),
                _ => Record::Tbt(commands),
            }
        }
        _ => return Err(Error::Invalid("record type")),
    })
}

/// Serializes records. Panics if one is longer than [`MAX_RECORD`].
pub fn write(records: &[Record]) -> Vec<u8> {
    let mut out = Vec::new();
    for record in records {
        let (rectyp, body) = match record {
            Record::Mhd(mhd) => (
                EMH,
                sized(|v| {
                    MHD.put(v);
                    mhd.write(v);
                }),
            ),
            Record::Text { subtype, text } => (
                EMH,
                sized(|v| {
                    subtype.put(v);
                    v.extend_from_slice(text);
                }),
            ),
            Record::Eom(eom, transfer) => (
                EEOM,
                sized(|v| {
                    eom.write(v);
                    if let Some(t) = transfer {
                        t.write(v);
                    }
                }),
            ),
            Record::Gsd(subrecords) => (
                EGSD,
                sized(|v| {
                    0u32.put(v); // EGSD$L_ALIGNLW
                    for sub in subrecords {
                        let start = v.len();
                        let gsdtyp: u16 = match sub {
                            Gsd::Psc(_) => 0,
                            Gsd::Def(_) | Gsd::Ref(_) => 1,
                            Gsd::Other { gsdtyp, .. } => *gsdtyp,
                        };
                        v.extend([0; 4]);
                        match sub {
                            Gsd::Psc(p) => p.write(v),
                            Gsd::Def(d) => d.write(v),
                            Gsd::Ref(r) => r.write(v),
                            Gsd::Other { data, .. } => v.extend_from_slice(data),
                        }
                        // Every subrecord ends on a quadword boundary.
                        v.resize((v.len() - start).next_multiple_of(8) + start, 0);
                        header(&mut v[start..], gsdtyp);
                    }
                }),
            ),
            Record::Tir(commands) | Record::Dbg(commands) | Record::Tbt(commands) => {
                let rectyp = match record {
                    Record::Tir(_) => ETIR,
                    Record::Dbg(_) => EDBG,
                    _ => ETBT,
                };
                (
                    rectyp,
                    sized(|v| {
                        for cmd in commands {
                            let start = v.len();
                            v.extend([0; 4]);
                            cmd.write_args(v);
                            header(&mut v[start..], cmd.code());
                        }
                    }),
                )
            }
        };
        let start = out.len();
        out.extend([0; 4]);
        out.extend_from_slice(&body);
        assert!(
            out.len() - start <= MAX_RECORD,
            "record longer than {MAX_RECORD} bytes"
        );
        header(&mut out[start..], rectyp);
    }
    out
}

fn sized(write: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut v = Vec::new();
    write(&mut v);
    v
}

/// Fills in the type and size at the start of a record, subrecord or
/// command that spans all of `b`.
fn header(b: &mut [u8], kind: u16) {
    let size = u16::try_from(b.len()).expect("record size");
    b[..2].copy_from_slice(&kind.to_le_bytes());
    b[2..4].copy_from_slice(&size.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// A module using every record type and every kind of argument.
    fn module() -> Vec<Record> {
        vec![
            Record::Mhd(Mhd {
                strlvl: STRLVL,
                temp: 0,
                arch1: ARCH_ARM64,
                arch2: 0,
                recsiz: MAX_RECORD as u32,
                name: "HELLO".into(),
                version: "V1.0".into(),
                date: *b"25-SEP-2026 17:00",
            }),
            Record::Text {
                subtype: LNM,
                text: b"vasm 0.1".to_vec(),
            },
            Record::Gsd(vec![
                Gsd::Psc(Psc {
                    align: 2,
                    temp: 0,
                    flags: psc::PIC | psc::REL | psc::SHR | psc::EXE,
                    alloc: 12,
                    name: "$CODE$".into(),
                }),
                Gsd::Def(SymDef {
                    datyp: 0,
                    temp: 0,
                    flags: sym::DEF | sym::REL,
                    value: 0,
                    code_address: 0,
                    ca_psindx: 0,
                    psindx: 0,
                    name: "HELLO".into(),
                }),
                Gsd::Ref(SymRef {
                    datyp: 0,
                    temp: 0,
                    flags: 0,
                    name: "PUTS".into(),
                }),
                Gsd::Other {
                    gsdtyp: 2,
                    data: vec![1, 2, 3, 4],
                },
            ]),
            Record::Tir(vec![
                Tir::StaPq {
                    psect: 0,
                    offset: 0,
                },
                Tir::CtlSetrb {},
                Tir::StoImm {
                    data: vec![0x20, 0x00, 0x80, 0xd2],
                },
                Tir::StaGbl {
                    name: "PUTS".into(),
                },
                Tir::StoA64Jump26 { insn: 0x9400_0000 },
                Tir::StaLw { value: 3 },
                Tir::StoImmr {
                    data: vec![0x1f, 0x20, 0x03, 0xd5],
                },
                Tir::CtlAugrb { offset: 4 },
                Tir::Other {
                    code: 57,
                    args: vec![9, 9],
                },
            ]),
            Record::Dbg(vec![Tir::StaQw { value: 1 }, Tir::CtlDfloc {}]),
            Record::Eom(
                Eom {
                    total_lps: 0,
                    comcod: 0,
                },
                Some(Transfer {
                    tfrflg: 0,
                    temp: 0,
                    psindx: 0,
                    tfradr: 0,
                }),
            ),
        ]
    }

    #[test]
    fn round_trip() {
        let records = module();
        let bytes = write(&records);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed, records);
        assert_eq!(write(&parsed), bytes);
        let short_eom = [Record::Eom(
            Eom {
                total_lps: 0,
                comcod: 1,
            },
            None,
        )];
        assert_eq!(
            write(&short_eom).len(),
            10,
            "EEOM without a transfer address"
        );
    }

    #[test]
    fn alpha_layout() {
        let bytes = write(&module());
        // EMH: type 8, then MHD subtype 0, structure level 2, ARCH1 at offset 8.
        assert_eq!(&bytes[..2], &[8, 0]);
        assert_eq!(&bytes[4..7], &[0, 0, 2]);
        assert_eq!(&bytes[8..12], &ARCH_ARM64.to_le_bytes());
        // Each GSD subrecord is padded to a quadword.
        let gsd = bytes
            .windows(4)
            .position(|w| w == [10, 0, 0x60, 0])
            .unwrap();
        assert_eq!(
            &bytes[gsd + 8..gsd + 12],
            &[0, 0, 0x18, 0],
            "PSC is 24 bytes"
        );
    }

    #[test]
    fn rejects_other_architectures() {
        let mut bytes = write(&module());
        bytes[8] = 0;
        assert_eq!(parse(&bytes), Err(Error::Invalid("architecture")));
    }
}
