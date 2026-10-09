//! Decoded dumps of vaxpunk object modules, object libraries and images, the
//! equivalent of ANALYZE/OBJECT and ANALYZE/IMAGE, with code disassembled.

use std::collections::HashMap;
use std::fmt::Write;

use vms_obj::Error;
use vms_obj::exe::{Eiaf, Eihd, Eisd, Image, write_eiaf};
use vms_obj::obj::{self, Gsd, Record, Tir, psc, sym};
use vms_obj::olb::{self, Library};
use vms_obj::reloc::{self, Need, Weight};
use yaxpeax_arch::{Arch, Decoder, U8Reader};
use yaxpeax_arm::armv8::a64::ARMv8;

/// Indent of data and code under a TIR command.
const DATA_INDENT: &str = "                        ";

#[derive(Default)]
pub struct Options {
    /// For objects: the weight of each stored value, and what the store
    /// needs for the image to move (docs/linker.md).
    pub weights: bool,
    /// For images: the image's link map, which names the psects.
    pub map: Option<String>,
}

/// Dumps an image, an object module or an object library, whichever `file` is.
pub fn dump(file: &[u8], opts: &Options) -> Result<String, Error> {
    // An image starts with EIHD majorid 3, minorid 0; a module with EMH (8).
    let text = if olb::is_library(file) {
        library(&Library::parse(file)?, opts)?
    } else if file.starts_with(&[3, 0, 0, 0, 0, 0, 0, 0]) {
        let iafva = Eihd::parse(file)?.iafva;
        image(&Image::parse(file)?, iafva, opts.map.as_deref())
    } else {
        object(&obj::parse(file)?, opts.weights)
    };
    Ok(text.lines().flat_map(|l| [l.trim_end(), "\n"]).collect())
}

fn library(lib: &Library, opts: &Options) -> Result<String, Error> {
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
        o.push_str(&object(&obj::parse(&m.object)?, opts.weights));
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

/// Dumps an image; its fixup section, if any, is at `iafva`.
fn image(image: &Image, iafva: u64, map: Option<&str>) -> String {
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
    if let Some(v) = &image.vector {
        let _ = writeln!(
            o,
            "Symbol vector: {:016X}, {} entries",
            v.addr,
            v.entries.len()
        );
        for (i, (name, addr)) in v.entries.iter().enumerate() {
            let _ = writeln!(o, "  {:4}  {addr:016X}  {name}", 8 * i);
        }
    }
    if image.fixups.is_none() && image.shareables.is_empty() {
        return out;
    }

    // The section as vms-obj writes it, which is how it parsed.
    let raw = write_eiaf(image.fixups.as_ref(), &image.shareables);
    let h = Eiaf::parse(&raw).expect("a fixup section header");
    let _ = writeln!(
        o,
        "Fixup section: {iafva:016X}-{:016X}, {} bytes, FIXUPVEC",
        iafva + raw.len() as u64 - 1,
        raw.len()
    );
    let _ = writeln!(
        o,
        "  EIAF {}.{}, header {} bytes, flags {:08X}",
        h.majorid, h.minorid, h.size, h.flags
    );
    let _ = writeln!(
        o,
        "  quadword relocation fixups at {}, longword relocation fixups at {}",
        h.qrelfixoff, h.lrelfixoff
    );
    let start = image.sections.iter().map(|s| s.vaddr).min().unwrap_or(0);
    for (i, shl) in image.shareables.iter().enumerate() {
        let _ = writeln!(o, "  shareable image {i}: {}", shl.name);
        for &(off, entry) in &shl.quad {
            let at = start + u64::from(off);
            let _ = writeln!(o, "    quadword at {at:016X}: symbol vector + {entry}");
        }
    }
    let Some(f) = &image.fixups else {
        return out;
    };
    if !f.long.is_empty() {
        let _ = writeln!(
            o,
            "  longword addresses {:08X} to {:08X}",
            h.lw_min, h.lw_max
        );
    }
    let psects = map.map_or(Vec::new(), map_psects);
    for (list, size, kind) in [(&f.quad, 8, "quadword"), (&f.long, 4, "longword")] {
        for &off in list {
            let at = start + u64::from(off);
            // Image::parse checked that a section holds it.
            let (i, s) = image
                .sections
                .iter()
                .enumerate()
                .find(|(_, s)| {
                    at.checked_sub(s.vaddr)
                        .is_some_and(|i| i < u64::from(s.size))
                })
                .expect("a section");
            let i0 = (at - s.vaddr) as usize;
            let mut v = [0; 8];
            v[..size].copy_from_slice(&s.data[i0..i0 + size]);
            let value = u64::from_le_bytes(v);
            let place = match psects.iter().find(|(_, b, n)| (*b..b + n).contains(&at)) {
                Some((name, b, _)) => format!("{name} + %X{:X}", at - b),
                None => format!("image section {} + %X{i0:X}", i + 1),
            };
            let value = format!("{value:0width$X}", width = 2 * size);
            let _ = writeln!(o, "  {kind} at {at:016X}: {value:<16}  {place}");
        }
    }
    out
}

/// The relocatable psects in a vlink map: name, base and length.
fn map_psects(map: &str) -> Vec<(String, u64, u64)> {
    map.lines()
        .skip_while(|l| l.trim() != "Program Section Synopsis")
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with(' '))
        .filter_map(|l| {
            let &[name, base, _, len, _, attrs] = &l.split_whitespace().collect::<Vec<_>>()[..]
            else {
                return None;
            };
            attrs.split(',').any(|a| a == "REL").then_some(())?;
            let hex = |s| u64::from_str_radix(s, 16).ok();
            Some((name.to_string(), hex(base)?, hex(len)?))
        })
        .collect()
}

fn object(records: &[Record], weights: bool) -> String {
    let mut out = String::new();
    let o = &mut out;
    // Psects in definition order, for names and to know which hold code.
    let mut psects: Vec<(String, u16)> = Vec::new();
    // Where STO commands store: psect and offset, while it is known.
    let mut loc: Option<(u32, u64)> = None;
    let mut pushed: Option<(u32, u64)> = None;
    // For weights: symbols this module defines, and the linker's stack.
    let mut defs: HashMap<String, (u64, Weight)> = HashMap::new();
    let mut stack: Vec<(u64, Weight)> = Vec::new();
    if weights {
        let _ = writeln!(o, "Weights take external symbols to be addresses.");
    }

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
                            psects.push((p.name.clone(), p.flags));
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
                            let rel = d.flags & sym::REL != 0;
                            defs.insert(d.name.clone(), (d.value, Some(rel.into())));
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
                    let note = match record {
                        Record::Tir(_) if weights => weigh(cmd, &mut stack, &defs, &psects, loc),
                        _ => String::new(),
                    };
                    let _ = write!(o, "  {:<20}", cmd.name());
                    match cmd {
                        Tir::StaGbl { name } | Tir::StoGbl { name } | Tir::StoCa { name } => {
                            let _ = writeln!(o, "{name}{note}");
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
                            let _ = writeln!(o, "{} bytes{note}", data.len());
                            match loc {
                                Some((p, off))
                                    if psects
                                        .get(p as usize)
                                        .is_some_and(|p| p.1 & psc::EXE != 0) =>
                                {
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
                        Tir::StoB {} => stored(o, &mut loc, 1, &note),
                        Tir::StoW {} => stored(o, &mut loc, 2, &note),
                        Tir::StoLw {} => stored(o, &mut loc, 4, &note),
                        Tir::StoQw {} | Tir::StoOff {} => stored(o, &mut loc, 8, &note),
                        Tir::Other { code, args } => {
                            let _ = writeln!(o, "command {code}, {} argument bytes", args.len());
                        }
                        _ => match instruction(cmd) {
                            Some(insn) => {
                                let _ = writeln!(o, "{insn:08x}  {}{note}", disassemble(insn));
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
fn stored(o: &mut String, loc: &mut Option<(u32, u64)>, n: u64, note: &str) {
    let _ = writeln!(o, "{note}");
    advance(loc, n);
}

/// Runs `cmd` on the linker's stack of values and weights, as far as one
/// module tells, and says what a store at `loc` needs for the image to move.
/// External symbols count as addresses, and psects as based at 0: only a
/// product's weight depends on values.
fn weigh(
    cmd: &Tir,
    stack: &mut Vec<(u64, Weight)>,
    defs: &HashMap<String, (u64, Weight)>,
    psects: &[(String, u16)],
    loc: Option<(u32, u64)>,
) -> String {
    use Tir::*;
    let symbol = |name: &String| defs.get(name).copied().unwrap_or((0, Some(1)));
    let has = |p: u32, flag: u16| psects.get(p as usize).is_some_and(|p| p.1 & flag != 0);
    let k = match cmd {
        StoGbl { name } | StoCa { name } => symbol(name).1,
        StoB {} | StoW {} | StoLw {} | StoQw {} | StoOff {} | StoImmr { .. } => {
            stack.pop().and_then(|v| v.1)
        }
        _ if reloc::Kind::of(cmd).is_some() => stack.pop().and_then(|v| v.1),
        _ => {
            match cmd {
                StaGbl { name } => stack.push(symbol(name)),
                StaLw { value } => stack.push((*value as i32 as u64, Some(0))),
                StaQw { value } => stack.push((*value, Some(0))),
                StaPq { psect, offset } => {
                    stack.push((*offset, Some(has(*psect, psc::REL).into())))
                }
                CtlSetrb {} => drop(stack.pop()),
                _ => {
                    if let Some(n) = reloc::operands(cmd) {
                        let args = stack.split_off(stack.len().saturating_sub(n));
                        let result = if args.len() == n {
                            reloc::operate(cmd, &args)
                        } else {
                            (0, None)
                        };
                        stack.push(result);
                    }
                }
            }
            return String::new();
        }
    };
    let pic = loc.is_some_and(|(p, _)| has(p, psc::PIC));
    let need = match reloc::need(cmd, k) {
        Need::Nothing => "no fixup",
        Need::Fixup(8) if pic => "quadword fixup in a PIC psect: NOTPIC",
        Need::Fixup(8) => "quadword fixup",
        Need::Fixup(_) if pic => "longword fixup in a PIC psect: NOTPIC",
        Need::Fixup(_) => "longword fixup",
        Need::Impossible => "can't move: NORELOC",
    };
    let k = k.map_or("?".to_string(), |k| k.to_string());
    format!("  [weight {k}: {need}]")
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
