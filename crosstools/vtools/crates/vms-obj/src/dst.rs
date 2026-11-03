//! Debug symbol tables (DST): the records a compiler writes with an object
//! module's EDBG and ETBT records and the linker gathers into an image, for
//! DEBUG and traceback. The format is OpenVMS Alpha's, unchanged; see
//! `docs/debug-format.md`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::Error;
use crate::record::{Field, Reader, record};

/// Record types (`DST$K_*`) this module decodes.
pub const SOURCE: u16 = 155;
pub const PROLOG: u16 = 162;
pub const BLKBEG: u16 = 176;
pub const BLKEND: u16 = 177;
pub const LINE_NUM: u16 = 185;
pub const MODBEG: u16 = 188;
pub const MODEND: u16 = 189;
pub const RTNBEG: u16 = 190;
pub const RTNEND: u16 = 191;

/// Languages (`DST$K_MACRO`, ...), in `DST$L_MODBEG_LANGUAGE`.
pub mod language {
    pub const MACRO: u32 = 0;
    pub const BLISS: u32 = 2;
    pub const C: u32 = 7;
    /// What AMACRO, MACRO-32 for Alpha, writes.
    pub const AMACRO: u32 = 16;
}

/// One DST record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// A module's first record (`DST$K_MODBEG`).
    ModBeg(ModBeg),
    /// A module's last record (`DST$K_MODEND`).
    ModEnd,
    /// A routine's first record (`DST$K_RTNBEG`).
    RtnBeg(RtnBeg),
    /// A routine's last record (`DST$K_RTNEND`).
    RtnEnd(SizeEnd),
    /// A lexical block's first record (`DST$K_BLKBEG`).
    BlkBeg(BlkBeg),
    /// A lexical block's last record (`DST$K_BLKEND`).
    BlkEnd(SizeEnd),
    /// Where a routine's prologue ends, for breakpoints on it
    /// (`DST$K_PROLOG`).
    Prolog { address: u32 },
    /// Line numbers of the code: PC-correlation commands (`DST$K_LINE_NUM`).
    LineNum(Vec<Pc>),
    /// Source file correlation commands (`DST$K_SOURCE`).
    Source(Vec<Src>),
    /// Any other record (symbols, types, ...), its bytes after the type.
    Other { kind: u16, body: Vec<u8> },
}

record! {
    /// `DST$K_MODBEG`'s fields.
    pub struct ModBeg {
        pub flags: u8,
        pub unused: u8,
        /// A [`language`].
        pub language: u32,
        /// The DST format's version: 1 and 18 from the OpenVMS V8.4 compilers.
        pub major: u16,
        pub minor: u16,
        pub name: String,
        /// The compiler that wrote it, such as `AMAC V5.0-120-5`.
        pub ident: String,
    }
}

record! {
    /// `DST$K_RTNBEG`'s fields.
    pub struct RtnBeg {
        pub flags: u8,
        /// Its first instruction.
        pub address: u32,
        /// Its procedure descriptor (on Alpha) or 0.
        pub pd_address: u32,
        pub name: String,
    }
}

record! {
    /// `DST$K_BLKBEG`'s fields.
    pub struct BlkBeg {
        pub unused: u8,
        pub address: u32,
        pub name: String,
    }
}

record! {
    /// `DST$K_RTNEND`'s and `DST$K_BLKEND`'s fields.
    pub struct SizeEnd {
        pub unused: u8,
        /// The routine's or block's size in bytes.
        pub size: u32,
    }
}

/// Declares commands: a code byte and at most one argument.
macro_rules! commands {
    ($(#[$m:meta])* $name:ident, $what:literal { $($(#[$cm:meta])* $code:literal $v:ident $(($t:ty))?,)* }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum $name {
            $($(#[$cm])* $v $(($t))?,)*
        }

        impl $name {
            fn read_cmd(code: u8, r: &mut Reader<'_>) -> Result<$name, Error> {
                Ok(match code {
                    $($code => $name::$v $((<$t as Field>::read(r)?))?,)*
                    _ => return Err(Error::Invalid($what)),
                })
            }

            fn write_cmd(&self, out: &mut Vec<u8>) {
                match self {
                    $(commands!(@pat $name $v x $($t)?) => {
                        out.push($code);
                        commands!(@put x out $($t)?);
                    })*
                }
            }
        }
    };
    (@pat $name:ident $v:ident $x:ident $t:ty) => { $name::$v($x) };
    (@pat $name:ident $v:ident $x:ident) => { $name::$v };
    (@put $x:ident $out:ident $t:ty) => { Field::put($x, $out) };
    (@put $x:ident $out:ident) => {};
}

commands! {
    /// A PC-correlation command of a `LINE_NUM` record. See [`lines`] for
    /// what they do.
    Pc, "PC-correlation command" {
        /// `DST$K_DELTA_PC_W`: next line, this many bytes on.
        1 DeltaPcW(u16),
        /// `DST$K_INCR_LINUM`: add to the line number.
        2 IncrLinum(u8),
        3 IncrLinumW(u16),
        /// `DST$K_SET_LINUM_INCR`: what a Delta-PC adds to the line number.
        4 SetLinumIncr(u8),
        5 SetLinumIncrW(u16),
        /// `DST$K_RESET_LINUM_INCR`: back to 1.
        6 ResetLinumIncr,
        /// `DST$K_BEG_STMT_MODE`: Delta-PCs count statements in the line.
        7 BegStmtMode,
        8 EndStmtMode,
        /// `DST$K_SET_LINUM`.
        9 SetLinum(u16),
        /// `DST$K_SET_PC`: the module's lowest routine's address plus this.
        10 SetPc(u8),
        11 SetPcW(u16),
        12 SetPcL(u32),
        /// `DST$K_SET_STMTNUM`.
        13 SetStmtnum(u16),
        /// `DST$K_TERM`: the last line has this many bytes; no line is open.
        14 Term(u8),
        15 TermW(u16),
        /// `DST$K_SET_ABS_PC`.
        16 SetAbsPc(u32),
        17 DeltaPcL(u32),
        18 IncrLinumL(u32),
        19 SetLinumB(u8),
        20 SetLinumL(u32),
        21 TermL(u32),
        /// Delta-PC: next line, this many bytes on, from 0 to 128. Its
        /// command byte is minus that, not 0, which `parse` and `write` see
        /// to.
        0 DeltaPc(u8),
    }
}

commands! {
    /// A source file correlation command of a `SOURCE` record: which lines
    /// of which files the module's line numbers stand for.
    Src, "source correlation command" {
        /// `DST$K_SRC_DECLFILE`: a source file and its number.
        1 DeclFile(DeclFile),
        /// `DST$K_SRC_SETFILE`: the file the next lines come from.
        2 SetFile(u16),
        /// `DST$K_SRC_SETREC_L`: the file's record the next line is.
        3 SetRecL(u32),
        4 SetRecW(u16),
        /// `DST$K_SRC_SETLNUM_L`: the next line's number.
        5 SetLnumL(u32),
        6 SetLnumW(u16),
        /// `DST$K_SRC_INCRLNUM_B`: add to the line number.
        7 IncrLnumB(u8),
        /// `DST$K_SRC_DEFLINES_W`: that many lines are the next records.
        10 DefLinesW(u16),
        11 DefLinesB(u8),
        /// `DST$K_SRC_FORMFEED`: form feeds count as records.
        16 FormFeed,
    }
}

/// `DST$K_SRC_DECLFILE`'s fields, after its length byte (the bytes that
/// follow it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclFile {
    pub flags: u8,
    /// The number `SetFile` names it by.
    pub fileid: u16,
    /// The file's creation time, end of file block, first free byte and
    /// organization and record format, as RMS says them.
    pub rms_cdt: u64,
    pub rms_ebk: u32,
    pub rms_ffb: u16,
    pub rms_rfo: u8,
    pub filename: String,
    /// The module, if the file is one in a text library.
    pub libmodname: String,
}

impl Field for DeclFile {
    const SIZE: usize = 0;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u8::read(r)?;
        let r = &mut Reader(r.take(n.into())?);
        let d = DeclFile {
            flags: Field::read(r)?,
            fileid: Field::read(r)?,
            rms_cdt: Field::read(r)?,
            rms_ebk: Field::read(r)?,
            rms_ffb: Field::read(r)?,
            rms_rfo: Field::read(r)?,
            filename: Field::read(r)?,
            libmodname: Field::read(r)?,
        };
        if !r.0.is_empty() {
            return Err(Error::Invalid("source file declaration"));
        }
        Ok(d)
    }
    fn put(&self, out: &mut Vec<u8>) {
        let mut v = Vec::new();
        self.flags.put(&mut v);
        self.fileid.put(&mut v);
        self.rms_cdt.put(&mut v);
        self.rms_ebk.put(&mut v);
        self.rms_ffb.put(&mut v);
        self.rms_rfo.put(&mut v);
        self.filename.put(&mut v);
        self.libmodname.put(&mut v);
        let n = u8::try_from(v.len()).expect("source file declaration too long");
        out.push(n);
        out.extend(v);
    }
}

impl Record {
    pub fn kind(&self) -> u16 {
        match self {
            Record::ModBeg(_) => MODBEG,
            Record::ModEnd => MODEND,
            Record::RtnBeg(_) => RTNBEG,
            Record::RtnEnd(_) => RTNEND,
            Record::BlkBeg(_) => BLKBEG,
            Record::BlkEnd(_) => BLKEND,
            Record::Prolog { .. } => PROLOG,
            Record::LineNum(_) => LINE_NUM,
            Record::Source(_) => SOURCE,
            Record::Other { kind, .. } => *kind,
        }
    }
}

/// Parses DST records, up to the end of `dst` or a length of 0, which ends
/// an image's DST.
pub fn parse(dst: &[u8]) -> Result<Vec<Record>, Error> {
    let mut records = Vec::new();
    let mut r = Reader(&dst[..size(dst)?]);
    while !r.0.is_empty() {
        // DST$W_LENGTH counts the record's bytes but one, as VAX's
        // DST$B_LENGTH did: the bytes after a 1-byte length.
        let total = usize::from(u16::read(&mut r)?) + 1;
        if total < 4 {
            return Err(Error::Invalid("DST record length"));
        }
        let body = r.take(total - 2)?;
        let mut r = Reader(body);
        let kind = u16::read(&mut r)?;
        records.push(match kind {
            MODBEG => Record::ModBeg(ModBeg::read(&mut r)?),
            MODEND => Record::ModEnd,
            RTNBEG => Record::RtnBeg(RtnBeg::read(&mut r)?),
            RTNEND => Record::RtnEnd(SizeEnd::read(&mut r)?),
            BLKBEG => Record::BlkBeg(BlkBeg::read(&mut r)?),
            BLKEND => Record::BlkEnd(SizeEnd::read(&mut r)?),
            PROLOG => Record::Prolog {
                address: u32::read(&mut r)?,
            },
            LINE_NUM => {
                let mut cmds = Vec::new();
                while !r.0.is_empty() {
                    let code = u8::read(&mut r)?;
                    cmds.push(if code == 0 || code >= 0x80 {
                        Pc::DeltaPc(0u8.wrapping_sub(code))
                    } else {
                        Pc::read_cmd(code, &mut r)?
                    });
                }
                Record::LineNum(cmds)
            }
            SOURCE => {
                let mut cmds = Vec::new();
                while !r.0.is_empty() {
                    let code = u8::read(&mut r)?;
                    cmds.push(Src::read_cmd(code, &mut r)?);
                }
                Record::Source(cmds)
            }
            _ => Record::Other {
                kind,
                body: core::mem::take(&mut r.0).to_vec(),
            },
        });
        if !r.0.is_empty() {
            return Err(Error::Invalid("DST record length"));
        }
    }
    Ok(records)
}

/// The size of the DST records at the start of `dst`: up to its end or a
/// length of 0.
pub fn size(dst: &[u8]) -> Result<usize, Error> {
    let mut at = 0;
    while at < dst.len() {
        // A last 0 byte is the end, as a zero length would be.
        let len = match dst.get(at..at + 2) {
            Some(len) => [len[0], len[1]],
            None if dst[at] == 0 => break,
            None => return Err(Error::Truncated),
        };
        match usize::from(u16::from_le_bytes(len)) {
            0 => break,
            1..=2 => return Err(Error::Invalid("DST record length")),
            n if at + n + 1 > dst.len() => return Err(Error::Truncated),
            n => at += n + 1,
        }
    }
    Ok(at)
}

/// Serializes DST records. Panics if one is longer than 65536 bytes, or a
/// Delta-PC command's increment is over 128.
pub fn write(records: &[Record]) -> Vec<u8> {
    let mut out = Vec::new();
    for record in records {
        let start = out.len();
        out.extend([0, 0]);
        record.kind().put(&mut out);
        match record {
            Record::ModBeg(m) => m.write(&mut out),
            Record::ModEnd => {}
            Record::RtnBeg(r) => r.write(&mut out),
            Record::RtnEnd(e) | Record::BlkEnd(e) => e.write(&mut out),
            Record::BlkBeg(b) => b.write(&mut out),
            Record::Prolog { address } => address.put(&mut out),
            Record::LineNum(cmds) => {
                for c in cmds {
                    match c {
                        Pc::DeltaPc(n) => {
                            assert!(*n <= 128, "Delta-PC of {n}");
                            out.push(0u8.wrapping_sub(*n));
                        }
                        c => c.write_cmd(&mut out),
                    }
                }
            }
            Record::Source(cmds) => cmds.iter().for_each(|c| c.write_cmd(&mut out)),
            Record::Other { body, .. } => out.extend_from_slice(body),
        }
        let len = u16::try_from(out.len() - start - 1).expect("DST record too long");
        out[start..start + 2].copy_from_slice(&len.to_le_bytes());
    }
    out
}

/// A line of source and the code it became.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub line: u32,
    /// The statement in the line: 1 but in statement mode.
    pub stmt: u32,
    /// The address of its first byte, and its size.
    pub pc: u32,
    pub size: u32,
}

/// The lines that a module's PC-correlation commands, all of its
/// `LINE_NUM` records' in order, define, as TRACE reads them. `start_pc` is
/// the address of the module's lowest routine, which `SetPc` counts from.
///
/// A Delta-PC adds the increment (1 at first) to the line number, or 1 to
/// the statement number in statement mode, moves the PC on and starts that
/// line there; the next Delta-PC or a `Term` ends it. The line number and PC
/// may be set only between a `Term` and the next Delta-PC.
pub fn lines<'a>(
    cmds: impl IntoIterator<Item = &'a Pc>,
    start_pc: u32,
) -> Result<Vec<Line>, Error> {
    const BAD: Error = Error::Invalid("PC-correlation commands");
    let (mut line, mut stmt, mut incr, mut stmt_mode) = (0u32, 1u32, 1u32, false);
    let mut pc = start_pc;
    let mut out: Vec<Line> = Vec::new();
    // Whether the last line in `out` is open: the next Delta-PC or Term
    // ends it.
    let mut open = false;
    for cmd in cmds {
        // A Delta-PC or a Term moves the PC on.
        let term = match *cmd {
            Pc::DeltaPc(_) | Pc::DeltaPcW(_) | Pc::DeltaPcL(_) => false,
            Pc::Term(_) | Pc::TermW(_) | Pc::TermL(_) => true,
            Pc::BegStmtMode if !open => return Err(BAD),
            Pc::BegStmtMode | Pc::EndStmtMode => {
                (stmt, stmt_mode) = (1, *cmd == Pc::BegStmtMode);
                continue;
            }
            Pc::SetStmtnum(n) => {
                stmt = n.into();
                continue;
            }
            Pc::IncrLinum(_) | Pc::IncrLinumW(_) | Pc::IncrLinumL(_) => {
                line = line.checked_add(arg(cmd)).ok_or(BAD)?;
                if stmt_mode {
                    stmt = 1;
                }
                continue;
            }
            Pc::SetLinumIncr(_) | Pc::SetLinumIncrW(_) | Pc::ResetLinumIncr => {
                incr = if *cmd == Pc::ResetLinumIncr {
                    1
                } else {
                    arg(cmd)
                };
                if stmt_mode {
                    stmt = 1;
                }
                continue;
            }
            // The rest come only between lines.
            _ if open => return Err(BAD),
            // Unlike INCR_LINUM, this keeps the statement number, as TRACE
            // does.
            Pc::SetLinumB(_) | Pc::SetLinum(_) | Pc::SetLinumL(_) => {
                line = arg(cmd);
                continue;
            }
            Pc::SetPc(_) | Pc::SetPcW(_) | Pc::SetPcL(_) => {
                pc = start_pc.checked_add(arg(cmd)).ok_or(BAD)?;
                continue;
            }
            Pc::SetAbsPc(n) => {
                pc = n;
                continue;
            }
        };
        pc = pc.checked_add(arg(cmd)).ok_or(BAD)?;
        if open {
            let last = out.last_mut().unwrap();
            last.size = pc - last.pc;
        }
        open = !term;
        if open {
            if stmt_mode {
                stmt += 1;
            } else {
                line = line.checked_add(incr).ok_or(BAD)?;
            }
            out.push(Line {
                line,
                stmt,
                pc,
                size: 0,
            });
        }
    }
    if open {
        return Err(BAD);
    }
    Ok(out)
}

/// A command's argument; 0 if it has none.
fn arg(cmd: &Pc) -> u32 {
    match *cmd {
        Pc::DeltaPc(n)
        | Pc::IncrLinum(n)
        | Pc::SetLinumIncr(n)
        | Pc::SetPc(n)
        | Pc::Term(n)
        | Pc::SetLinumB(n) => n.into(),
        Pc::DeltaPcW(n)
        | Pc::IncrLinumW(n)
        | Pc::SetLinumIncrW(n)
        | Pc::SetLinum(n)
        | Pc::SetPcW(n)
        | Pc::SetStmtnum(n)
        | Pc::TermW(n) => n.into(),
        Pc::SetPcL(n)
        | Pc::SetAbsPc(n)
        | Pc::DeltaPcL(n)
        | Pc::IncrLinumL(n)
        | Pc::SetLinumL(n)
        | Pc::TermL(n) => n,
        Pc::ResetLinumIncr | Pc::BegStmtMode | Pc::EndStmtMode => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Records of the DST that LINK/DEBUG made on OpenVMS Alpha V8.4 of
    /// this, assembled with MACRO/MIGRATION/DEBUG:
    ///
    /// ```text
    ///  1         .TITLE  DBGM  debug records probe
    ///  2         .IDENT  /V1.2/
    ///  3         .PSECT  $DATA$,WRT,NOEXE,QUAD
    ///  4 COUNT:  .LONG   0
    ///  5         .PSECT  $CODE$,EXE,NOWRT,QUAD
    ///  6         .ENTRY  DBGM_TWO,^M<>
    ///  7         INCL    COUNT
    ///  8         ADDL3   #2, COUNT, R0
    ///  9         RET
    /// 10         .ENTRY  DBGM_MAIN,^M<>
    /// 11         CALLS   #0, DBGM_TWO
    /// 12         CALLS   #0, DBGM_TWO
    /// 13         MOVL    #1, R0
    /// 14         RET
    /// 15         .END    DBGM_MAIN
    /// ```
    const AMACRO: &str = concat!(
        "2200bc0002001000000001001200044442474d0f414d41432056352e302d3132302d35",
        "39009b0001340001008e3c22bd084fbc0001000000ac010220535953245359534445",
        "564943453a5b4f5241434c455d4442474d2e4d41523b3100",
        "13009b00050100000002010003010000000a0f00",
        "0e000800010000020005434f554e54",
        "1600be00804000030028000100094442474d5f4d41494e",
        "0800b9001040000300",
        "0c00b900130900e4f4f4fc0e1c",
        "2000c0000b40000300140b5c00030014090910090909100909140b900003001000",
        "0800bf000054000000",
        "0300bd00",
    );

    #[test]
    fn alpha_records() {
        let bytes = hex(AMACRO);
        let records = parse(&bytes).unwrap();
        assert_eq!(write(&records), bytes);
        assert_eq!(size(&bytes), Ok(bytes.len()));
        assert_eq!(
            records[0],
            Record::ModBeg(ModBeg {
                flags: 2,
                unused: 0,
                language: language::AMACRO,
                major: 1,
                minor: 18,
                name: "DBGM".into(),
                ident: "AMAC V5.0-120-5".into(),
            })
        );
        let Record::Source(decl) = &records[1] else {
            panic!("{:?}", records[1])
        };
        let [Src::DeclFile(d)] = &decl[..] else {
            panic!("{decl:?}")
        };
        assert_eq!(d.filename, "SYS$SYSDEVICE:[ORACLE]DBGM.MAR;1");
        assert_eq!(
            (d.fileid, d.rms_ebk, d.rms_ffb, d.rms_rfo),
            (1, 1, 0x1ac, 2)
        );
        assert_eq!(
            records[2],
            Record::Source(vec![
                Src::SetLnumL(1),
                Src::SetFile(1),
                Src::SetRecL(1),
                Src::DefLinesW(15)
            ])
        );
        // COUNT, a longword at 0x20000.
        assert_eq!(
            records[3],
            Record::Other {
                kind: 8,
                body: hex("010000020005434f554e54")
            }
        );
        assert_eq!(
            records[4],
            Record::RtnBeg(RtnBeg {
                flags: 0x80,
                address: 0x30040,
                pd_address: 0x10028,
                name: "DBGM_MAIN".into(),
            })
        );
        let pcs: Vec<&Pc> = records
            .iter()
            .filter_map(|r| match r {
                Record::LineNum(cmds) => Some(cmds),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(
            pcs[..3],
            [&Pc::SetAbsPc(0x30040), &Pc::SetLinumB(9), &Pc::DeltaPc(0)]
        );
        let line = |line, pc, size| Line {
            line,
            stmt: 1,
            pc,
            size,
        };
        assert_eq!(
            lines(pcs, 0x30000),
            Ok(vec![
                line(10, 0x30040, 0x1c),
                line(11, 0x3005c, 0xc),
                line(12, 0x30068, 0xc),
                line(13, 0x30074, 4),
                line(14, 0x30078, 0x1c),
            ])
        );
        assert_eq!(
            records[8],
            Record::RtnEnd(SizeEnd {
                unused: 0,
                size: 0x54
            })
        );
        assert_eq!(records[9], Record::ModEnd);
    }

    #[test]
    fn round_trip() {
        let cmds = vec![
            Pc::SetPcW(0x100),
            Pc::SetLinum(1000),
            Pc::DeltaPc(128),
            Pc::BegStmtMode,
            Pc::DeltaPcW(4),
            Pc::SetStmtnum(7),
            Pc::DeltaPcL(4),
            Pc::EndStmtMode,
            Pc::SetLinumIncr(10),
            Pc::IncrLinumW(2),
            Pc::DeltaPc(0),
            Pc::TermL(8),
            Pc::SetLinumL(1 << 20),
            Pc::SetAbsPc(0x5000),
            Pc::ResetLinumIncr,
            Pc::IncrLinumL(1),
            Pc::IncrLinum(1),
            Pc::SetLinumIncrW(1),
            Pc::DeltaPc(4),
            Pc::TermW(4),
            Pc::SetPc(0),
            Pc::SetPcL(0),
            Pc::SetLinumB(1),
            Pc::DeltaPc(1),
            Pc::Term(3),
        ];
        let records = vec![
            Record::BlkBeg(BlkBeg {
                unused: 0,
                address: 0x1000,
                name: "".into(),
            }),
            Record::Prolog { address: 0x1008 },
            Record::LineNum(cmds.clone()),
            Record::Source(vec![
                Src::SetRecW(2),
                Src::SetLnumW(3),
                Src::IncrLnumB(4),
                Src::DefLinesB(5),
                Src::FormFeed,
            ]),
            Record::BlkEnd(SizeEnd { unused: 0, size: 4 }),
        ];
        let bytes = write(&records);
        assert_eq!(parse(&bytes).unwrap(), records);
        // An image's DST ends with a zero length, then the block's fill.
        let mut padded = bytes.clone();
        padded.resize(bytes.len() + 30, 0);
        assert_eq!(parse(&padded).unwrap(), records);
        assert_eq!(size(&padded), Ok(bytes.len()));

        let line = |line, stmt, pc, size| Line {
            line,
            stmt,
            pc,
            size,
        };
        assert_eq!(
            lines(&cmds, 0x1000),
            Ok(vec![
                line(1001, 1, 0x1180, 4),
                line(1001, 2, 0x1184, 4),
                line(1001, 8, 0x1188, 0),
                line(1013, 1, 0x1188, 8),
                line(1048579, 1, 0x5004, 4),
                line(2, 1, 0x1001, 3),
            ])
        );
    }

    #[test]
    fn rejects_bad_commands() {
        let bad = Err(Error::Invalid("PC-correlation commands"));
        let open = [Pc::DeltaPc(0)];
        assert_eq!(lines(&open, 0), bad, "a line never ended");
        assert_eq!(
            lines(&[Pc::BegStmtMode], 0),
            bad,
            "statements outside a line"
        );
        for set in [Pc::SetLinum(1), Pc::SetPc(1), Pc::SetAbsPc(1)] {
            assert_eq!(lines(&[Pc::DeltaPc(0), set, Pc::Term(0)], 0), bad);
        }
        assert_eq!(
            lines(&[Pc::SetPcL(1), Pc::Term(0)], u32::MAX),
            bad,
            "past 4 GB"
        );

        // A LINE_NUM record with command 22, and one with a truncated SET_LINUM.
        assert_eq!(
            parse(&hex("0400b90016")),
            Err(Error::Invalid("PC-correlation command"))
        );
        assert_eq!(parse(&hex("0500b90009ff")), Err(Error::Truncated));
        assert_eq!(
            parse(&hex("0400bd0000")),
            Err(Error::Invalid("DST record length")),
            "MODEND with a byte"
        );
        assert_eq!(size(&hex("0100")), Err(Error::Invalid("DST record length")));
        assert_eq!(
            parse(&hex("0200bd")),
            Err(Error::Invalid("DST record length"))
        );
        assert_eq!(size(&hex("0800bf00")), Err(Error::Truncated));
    }
}
