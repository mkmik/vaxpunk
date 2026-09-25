//! Decoded dumps of vaxpunk object modules, object libraries and images, the
//! equivalent of ANALYZE/OBJECT and ANALYZE/IMAGE, with code disassembled.

use std::fmt::Write;

use vms_obj::Error;
use vms_obj::exe::{Eisd, Image};
use vms_obj::obj::{self, Gsd, Record, Tir, psc, sym};
use vms_obj::olb::{self, Library};
use yaxpeax_arch::{Arch, Decoder, U8Reader};
use yaxpeax_arm::armv8::a64::ARMv8;

/// Indent of data and code under a TIR command.
const DATA_INDENT: &str = "                        ";

/// Dumps an image, an object module or an object library, whichever `file` is.
pub fn dump(file: &[u8]) -> Result<String, Error> {
    // An image starts with EIHD majorid 3, minorid 0; a module with EMH (8).
    let text = if olb::is_library(file) {
        library(&Library::parse(file)?)?
    } else if file.starts_with(&[3, 0, 0, 0, 0, 0, 0, 0]) {
        image(&Image::parse(file)?)
    } else {
        object(&obj::parse(file)?)
    };
    Ok(text.lines().flat_map(|l| [l.trim_end(), "\n"]).collect())
}

fn library(lib: &Library) -> Result<String, Error> {
    let mut out = String::new();
    let o = &mut out;
    let _ = writeln!(o, "Object library, created by {:?}", lib.creator);
    let _ = writeln!(o, "  created {:016X}", lib.created);
    let _ = writeln!(o, "  updated {:016X}", lib.updated);
    for m in &lib.modules {
        let _ = writeln!(o, "\nModule {}, ident {:?}", m.name, m.ident);
        let _ = writeln!(o, "  inserted {:016X}", m.inserted);
        for s in &m.symbols {
            let _ = writeln!(o, "  symbol {s}");
        }
        o.push_str(&object(&obj::parse(&m.object)?));
    }
    Ok(out)
}

const EISD_FLAGS: [&str; 15] = [
    "GBL",
    "CRF",
    "DZRO",
    "WRT",
    "INITALCODE",
    "BASED",
    "FIXUPVEC",
    "RESIDENT",
    "VECTOR",
    "PROTECT",
    "LASTCLU",
    "EXE",
    "NONSHRADR",
    "QUAD_LENGTH",
    "ALLOC_64BIT",
];

fn image(image: &Image) -> String {
    let mut out = String::new();
    let o = &mut out;
    let _ = writeln!(o, "Image {}, ident {:?}", image.name, image.ident);
    let _ = writeln!(o, "  transfer address {:016X}", image.transfer);
    let _ = writeln!(o, "  link time {:016X}", image.link_time);
    for (i, s) in image.sections.iter().enumerate() {
        let _ = writeln!(
            o,
            "Section {}: {:016X}-{:016X}, {} bytes, {}",
            i + 1,
            s.vaddr,
            s.vaddr + u64::from(s.size).max(1) - 1,
            s.size,
            flags(s.flags, &EISD_FLAGS)
        );
        if s.flags & Eisd::M_EXE != 0 {
            code(o, "  ", s.vaddr, &s.data);
        } else {
            hex(o, "  ", s.vaddr, &s.data);
        }
    }
    out
}

fn object(records: &[Record]) -> String {
    let mut out = String::new();
    let o = &mut out;
    // Psects in definition order, for names and to know which hold code.
    let mut psects: Vec<(String, bool)> = Vec::new();
    // Where STO commands store: psect and offset, while it is known.
    let mut loc: Option<(u32, u64)> = None;
    let mut pushed: Option<(u32, u64)> = None;

    for record in records {
        match record {
            Record::Mhd(m) => {
                let _ = writeln!(
                    o,
                    "EMH MHD  module {} {:?}, created {}, structure level {}, architecture {}, longest record {}",
                    m.name,
                    m.version,
                    String::from_utf8_lossy(&m.date),
                    m.strlvl,
                    m.arch1,
                    m.recsiz
                );
            }
            Record::Text { subtype, text } => {
                let names = ["MHD", "LNM", "SRC", "TTL", "CPR", "MTC", "GTX"];
                let name = names.get(*subtype as usize).unwrap_or(&"?");
                let _ = writeln!(o, "EMH {name}  {:?}", String::from_utf8_lossy(text));
            }
            Record::Gsd(subrecords) => {
                let _ = writeln!(o, "EGSD");
                for sub in subrecords {
                    match sub {
                        Gsd::Psc(p) => {
                            let _ = writeln!(
                                o,
                                "  PSC  {} {}: {} bytes, align 2**{}, {}",
                                psects.len(),
                                p.name,
                                p.alloc,
                                p.align,
                                flags(p.flags.into(), &psc::NAMES)
                            );
                            psects.push((p.name.clone(), p.flags & psc::EXE != 0));
                        }
                        Gsd::Def(d) => {
                            let _ = write!(
                                o,
                                "  SYM  definition {}: psect {} value {:016X}",
                                d.name, d.psindx, d.value
                            );
                            if d.flags & sym::NORM != 0 {
                                let _ = write!(
                                    o,
                                    ", entry psect {} offset {:016X}",
                                    d.ca_psindx, d.code_address
                                );
                            }
                            let _ = writeln!(o, ", {}", flags(d.flags.into(), &sym::NAMES));
                        }
                        Gsd::Ref(r) => {
                            let _ = writeln!(
                                o,
                                "  SYM  reference {}, {}",
                                r.name,
                                flags(r.flags.into(), &sym::NAMES)
                            );
                        }
                        Gsd::Other { gsdtyp, data } => {
                            let _ = writeln!(o, "  GSD type {gsdtyp}, {} bytes", data.len());
                        }
                    }
                }
            }
            Record::Tir(cmds) | Record::Dbg(cmds) | Record::Tbt(cmds) => {
                let kind = match record {
                    Record::Tir(_) => "ETIR",
                    Record::Dbg(_) => "EDBG",
                    _ => "ETBT",
                };
                let _ = writeln!(o, "{kind}");
                for cmd in cmds {
                    let _ = write!(o, "  {:<20}", cmd.name());
                    match cmd {
                        Tir::StaGbl { name } | Tir::StoGbl { name } | Tir::StoCa { name } => {
                            let _ = writeln!(o, "{name}");
                            if !matches!(cmd, Tir::StaGbl { .. }) {
                                advance(&mut loc, 8);
                            }
                        }
                        Tir::StaLw { value } => {
                            let _ = writeln!(o, "{:08X}", value);
                        }
                        Tir::StaQw { value } => {
                            let _ = writeln!(o, "{:016X}", value);
                        }
                        Tir::StaPq { psect, offset } => {
                            let name = psects.get(*psect as usize).map_or("?", |p| &p.0);
                            let _ = writeln!(o, "psect {psect} ({name}) offset {offset:016X}");
                            pushed = Some((*psect, *offset));
                        }
                        Tir::CtlSetrb {} => {
                            let _ = writeln!(o);
                            loc = pushed;
                        }
                        Tir::CtlAugrb { offset } => {
                            let _ = writeln!(o, "{:+}", *offset as i32);
                            advance(&mut loc, i64::from(*offset as i32) as u64);
                        }
                        Tir::StoImm { data } | Tir::StoImmr { data } => {
                            let _ = writeln!(o, "{} bytes", data.len());
                            match loc {
                                Some((p, off)) if psects.get(p as usize).is_some_and(|p| p.1) => {
                                    code(o, DATA_INDENT, off, data)
                                }
                                _ => hex(o, DATA_INDENT, loc.map_or(0, |l| l.1), data),
                            }
                            if matches!(cmd, Tir::StoImmr { .. }) {
                                loc = None; // the repeat count was on the stack
                            } else {
                                advance(&mut loc, data.len() as u64);
                            }
                        }
                        Tir::StoB {} => stored(o, &mut loc, 1),
                        Tir::StoW {} => stored(o, &mut loc, 2),
                        Tir::StoLw {} => stored(o, &mut loc, 4),
                        Tir::StoQw {} | Tir::StoOff {} => stored(o, &mut loc, 8),
                        Tir::Other { code, args } => {
                            let _ = writeln!(o, "command {code}, {} argument bytes", args.len());
                        }
                        _ => match instruction(cmd) {
                            Some(insn) => {
                                let _ = writeln!(o, "{insn:08x}  {}", disassemble(insn));
                                advance(&mut loc, 4);
                            }
                            None => {
                                let _ = writeln!(o);
                            }
                        },
                    }
                }
            }
            Record::Eom(eom, transfer) => {
                let status = ["success", "warning", "error", "abort"];
                let _ = write!(
                    o,
                    "EEOM {}",
                    status.get(eom.comcod as usize).unwrap_or(&"?")
                );
                if let Some(t) = transfer {
                    let _ = write!(o, ", transfer psect {} offset {:016X}", t.psindx, t.tfradr);
                }
                let _ = writeln!(o);
            }
        }
    }
    out
}

/// The instruction template of an ARM64 store command.
fn instruction(cmd: &Tir) -> Option<u32> {
    use Tir::*;
    match *cmd {
        StoA64Jump26 { insn }
        | StoA64Branch19 { insn }
        | StoA64Branch14 { insn }
        | StoA64Adr { insn }
        | StoA64Adrp { insn }
        | StoA64AddLo12 { insn }
        | StoA64Ldst8Lo12 { insn }
        | StoA64Ldst16Lo12 { insn }
        | StoA64Ldst32Lo12 { insn }
        | StoA64Ldst64Lo12 { insn }
        | StoA64Ldst128Lo12 { insn }
        | StoA64MovwG0 { insn }
        | StoA64MovwG0Nc { insn }
        | StoA64MovwG1 { insn }
        | StoA64MovwG1Nc { insn }
        | StoA64MovwG2 { insn }
        | StoA64MovwG2Nc { insn }
        | StoA64MovwG3 { insn } => Some(insn),
        _ => None,
    }
}

/// Moves the location counter, if it is known.
fn advance(loc: &mut Option<(u32, u64)>, n: u64) {
    if let Some((_, off)) = loc {
        *off = off.wrapping_add(n);
    }
}

/// Ends the line of a store command that pops its data.
fn stored(o: &mut String, loc: &mut Option<(u32, u64)>, n: u64) {
    let _ = writeln!(o);
    advance(loc, n);
}

/// Names of the set bits, or "none".
fn flags(bits: u32, names: &[&str]) -> String {
    let set: Vec<&str> = (0..32)
        .filter(|b| bits & (1 << b) != 0)
        .map(|b| names.get(b).copied().unwrap_or("?"))
        .collect();
    if set.is_empty() {
        "none".into()
    } else {
        set.join(" ")
    }
}

fn disassemble(word: u32) -> String {
    let bytes = word.to_le_bytes();
    match <ARMv8 as Arch>::Decoder::default().decode(&mut U8Reader::new(&bytes)) {
        Ok(insn) => insn.to_string(),
        Err(_) => format!(".long 0x{word:08x}"),
    }
}

/// One instruction per line; a trailing partial word as hex.
fn code(o: &mut String, indent: &str, addr: u64, data: &[u8]) {
    let (words, tail) = data.as_chunks::<4>();
    for (i, &w) in words.iter().enumerate() {
        let word = u32::from_le_bytes(w);
        let at = addr + 4 * i as u64;
        let _ = writeln!(o, "{indent}{at:016X}  {word:08x}  {}", disassemble(word));
    }
    if !tail.is_empty() {
        hex(o, indent, addr + (data.len() - tail.len()) as u64, tail);
    }
}

/// 16 bytes per line, with the printable ones as text.
fn hex(o: &mut String, indent: &str, addr: u64, data: &[u8]) {
    for (i, line) in data.chunks(16).enumerate() {
        let _ = write!(o, "{indent}{:016X} ", addr + 16 * i as u64);
        for b in line {
            let _ = write!(o, " {b:02x}");
        }
        let text: String = line
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        let _ = writeln!(o, "{:pad$}  {text}", "", pad = 3 * (16 - line.len()));
    }
}
