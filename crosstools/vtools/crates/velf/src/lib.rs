//! velf: turns the ELF relocatable object a C compiler writes for AArch64
//! into a vaxpunk object module (`docs/object-format.md`), so that vlib and
//! vlink take C code as they take MACRO-32 and BLISS-64. `docs/velf.md`
//! says what it maps to what, and what the compiler must be told.

use vms_obj::ARCH_ARM64;
use vms_obj::obj::{self, Eom, Gsd, Mhd, Psc, Record, SymDef, SymRef, Tir, psc, sym};

/// What gcc must be told so that its code keeps the vaxpunk calling
/// standard's rules and velf can take its object (`docs/velf.md`).
pub const GCC_FLAGS: &[&str] = &[
    "-ffreestanding",
    "-fno-pic",
    "-fno-pie",
    "-fno-common",
    "-ffixed-x18",
    "-mgeneral-regs-only",
    "-fno-omit-frame-pointer",
    "-fno-stack-protector",
    "-fno-asynchronous-unwind-tables",
    "-fno-unwind-tables",
    "-mbranch-protection=none",
    "-mno-outline-atomics",
];

/// Payload room in one record, below the record size limit.
const ROOM: usize = obj::MAX_RECORD - 64;
/// Largest STO_IMM, so a command always fits in a record.
const MAX_IMM: usize = 4096;

/// The psects velf fills, DEC C's names, in the order they are defined.
/// `$READONLY_ADDR$` holds read-only data with addresses in it, which an
/// image that moves must patch, so it isn't PIC.
const PSECTS: [(&str, u16); 5] = [
    ("$CODE$", psc::PIC | psc::REL | psc::SHR | psc::EXE),
    ("$READONLY$", psc::PIC | psc::REL | psc::SHR | psc::RD),
    ("$READONLY_ADDR$", psc::REL | psc::RD),
    ("$DATA$", psc::REL | psc::RD | psc::WRT),
    ("$BSS$", psc::REL | psc::RD | psc::WRT | psc::NOMOD),
];

// ELF constants (the ELF and AAELF64 specifications).
const SHT_SYMTAB: u32 = 2;
const SHT_RELA: u32 = 4;
const SHT_NOTE: u32 = 7;
const SHT_NOBITS: u32 = 8;
const SHT_REL: u32 = 9;
const SHF_ALLOC: u64 = 2;
const SHN_UNDEF: u16 = 0;
const SHN_LORESERVE: u16 = 0xff00;
const SHN_COMMON: u16 = 0xfff2;
const STB_LOCAL: u8 = 0;
const STB_WEAK: u8 = 2;
const STT_FILE: u8 = 4;

/// What a relocation stores, and so the TIR store it becomes.
#[derive(Clone, Copy)]
enum Kind {
    /// An instruction field: the store, and the field's bits, which the
    /// object gives as zeros.
    Insn(fn(u32) -> Tir, u32),
    /// A quadword or longword, S + A.
    Abs(usize),
    /// A longword or quadword, S + A - P.
    Rel(usize),
}

/// What each AArch64 relocation velf takes becomes. They are those a
/// small-model, non-PIC, non-TLS object has.
fn kind(r_type: u32) -> Option<Kind> {
    const LO12: u32 = 0xfff << 10;
    const IMM19: u32 = 0x7ffff << 5;
    Some(match r_type {
        257 => Kind::Abs(8),                                                // ABS64
        258 => Kind::Abs(4),                                                // ABS32
        260 => Kind::Rel(8),                                                // PREL64
        261 => Kind::Rel(4),                                                // PREL32
        273 => Kind::Insn(|insn| Tir::StoA64Branch19 { insn }, IMM19),      // LD_PREL_LO19
        274 => Kind::Insn(|insn| Tir::StoA64Adr { insn }, 3 << 29 | IMM19), // ADR_PREL_LO21
        275 | 276 => Kind::Insn(|insn| Tir::StoA64Adrp { insn }, 3 << 29 | IMM19), // ADR_PREL_PG_HI21, _NC
        277 => Kind::Insn(|insn| Tir::StoA64AddLo12 { insn }, LO12), // ADD_ABS_LO12_NC
        278 => Kind::Insn(|insn| Tir::StoA64Ldst8Lo12 { insn }, LO12), // LDST8_ABS_LO12_NC
        279 => Kind::Insn(|insn| Tir::StoA64Branch14 { insn }, 0x3fff << 5), // TSTBR14
        280 => Kind::Insn(|insn| Tir::StoA64Branch19 { insn }, IMM19), // CONDBR19
        282 | 283 => Kind::Insn(|insn| Tir::StoA64Jump26 { insn }, 0x3ff_ffff), // JUMP26, CALL26
        284 => Kind::Insn(|insn| Tir::StoA64Ldst16Lo12 { insn }, LO12), // LDST16_ABS_LO12_NC
        285 => Kind::Insn(|insn| Tir::StoA64Ldst32Lo12 { insn }, LO12), // LDST32_ABS_LO12_NC
        286 => Kind::Insn(|insn| Tir::StoA64Ldst64Lo12 { insn }, LO12), // LDST64_ABS_LO12_NC
        299 => Kind::Insn(|insn| Tir::StoA64Ldst128Lo12 { insn }, LO12), // LDST128_ABS_LO12_NC
        _ => return None,
    })
}

/// A section of the ELF file.
struct Section<'a> {
    name: &'a str,
    kind: u32,
    flags: u64,
    data: &'a [u8],
    size: u64,
    link: u32,
    info: u32,
    align: u64,
}

/// A symbol of the ELF file.
struct Symbol<'a> {
    name: &'a str,
    bind: u8,
    typ: u8,
    shndx: u16,
    value: u64,
}

/// A relocation: where in its section, what kind, which symbol, the addend.
struct Rela {
    offset: u64,
    r_type: u32,
    sym: usize,
    addend: i64,
}

/// Where a kept section went: its psect, by index in PSECTS, and offset.
#[derive(Clone, Copy)]
struct Place {
    psect: usize,
    offset: u64,
}

/// The object module for the ELF relocatable object `elf`, named `name`
/// (upper case, at most 31 characters) and made at `date`.
pub fn convert(elf: &[u8], name: &str, date: [u8; 17]) -> Result<Vec<Record>, String> {
    let sections = sections(elf)?;
    let symbols = symbols(&sections)?;
    let relas = relocations(&sections)?;

    // Each kept section's psect and offset in it, in section order.
    let mut places = vec![None; sections.len()];
    let mut size = [0u64; PSECTS.len()];
    let mut align = [0u8; PSECTS.len()];
    for (i, s) in sections.iter().enumerate() {
        let addresses = relas[i]
            .iter()
            .any(|r| matches!(kind(r.r_type), Some(Kind::Abs(_))));
        let Some(psect) = psect(s, addresses)? else {
            continue;
        };
        if s.size == 0 {
            continue;
        }
        let a = s.align.max(1);
        if !a.is_power_of_two() || a > 1 << 16 {
            return Err(format!("section {} is aligned to {a}", s.name));
        }
        let offset = size[psect].next_multiple_of(a);
        size[psect] = offset + s.size;
        align[psect] = align[psect].max(a.trailing_zeros() as u8);
        places[i] = Some(Place { psect, offset });
    }
    // Psect numbers in the module: the used ones, in PSECTS' order.
    let mut index = [u32::MAX; PSECTS.len()];
    let mut gsd = Vec::new();
    for (p, &(pname, flags)) in PSECTS.iter().enumerate() {
        if places.iter().flatten().any(|pl| pl.psect == p) {
            index[p] = gsd.len() as u32;
            gsd.push(Gsd::Psc(Psc {
                align: align[p],
                temp: 0,
                flags,
                alloc: u32::try_from(size[p]).map_err(|_| format!("{pname} is too big"))?,
                name: pname.into(),
            }));
        }
    }

    // Global symbols, by VMS name: C's names in upper case, as DEC C
    // makes them by default.
    let mut names: Vec<(String, &str)> = Vec::new();
    for s in symbols
        .iter()
        .filter(|s| s.bind != STB_LOCAL && s.typ != STT_FILE)
    {
        let vms = vms_name(s.name)?;
        match names.iter().find(|(n, _)| *n == vms) {
            Some((_, c)) if *c != s.name => {
                return Err(format!("{} and {c} are both {vms} in upper case", s.name));
            }
            Some(_) => continue,
            None => names.push((vms.clone(), s.name)),
        }
        let weak = if s.bind == STB_WEAK { sym::WEAK } else { 0 };
        match s.shndx {
            SHN_UNDEF => gsd.push(Gsd::Ref(SymRef {
                datyp: 0,
                temp: 0,
                flags: weak,
                name: vms,
            })),
            SHN_COMMON => return Err(format!("{} is common: compile with -fno-common", s.name)),
            n if n >= SHN_LORESERVE => {
                return Err(format!("{} is absolute, which velf doesn't take", s.name));
            }
            n => {
                let pl = places[usize::from(n)]
                    .ok_or_else(|| format!("{} is in a section velf drops", s.name))?;
                let value = pl.offset + s.value;
                let quad = if i32::try_from(value).is_err() {
                    sym::QUAD_VAL
                } else {
                    0
                };
                gsd.push(Gsd::Def(SymDef {
                    datyp: 0,
                    temp: 0,
                    flags: sym::DEF | sym::REL | weak | quad,
                    value,
                    code_address: 0,
                    ca_psindx: 0,
                    psindx: index[pl.psect],
                    name: vms,
                }));
            }
        }
    }

    // The text: each kept section's bytes, with its relocations as stores.
    let mut cmds = Vec::new();
    for (i, s) in sections.iter().enumerate() {
        let Some(pl) = places[i] else { continue };
        if s.kind == SHT_NOBITS || s.size == 0 {
            continue;
        }
        let psect = index[pl.psect];
        cmds.push(Tir::StaPq {
            psect,
            offset: pl.offset,
        });
        cmds.push(Tir::CtlSetrb {});
        let mut at = 0usize;
        let mut rels: Vec<&Rela> = relas[i].iter().collect();
        rels.sort_by_key(|r| r.offset);
        for r in rels {
            let k = kind(r.r_type).ok_or_else(|| {
                format!(
                    "relocation type {} at {}+{:#x} isn't one velf takes",
                    r.r_type, s.name, r.offset
                )
            })?;
            let start = r.offset as usize;
            let len = match k {
                Kind::Insn(..) => 4,
                Kind::Abs(n) | Kind::Rel(n) => n,
            };
            if start < at || start + len > s.data.len() {
                return Err(format!("bad relocation at {}+{:#x}", s.name, r.offset));
            }
            imm(&mut cmds, &s.data[at..start]);
            push(&mut cmds, &symbols, &places, &index, r)?;
            match k {
                Kind::Insn(store, field) => {
                    let word = u32::from_le_bytes(s.data[start..start + 4].try_into().unwrap());
                    cmds.push(store(word & !field));
                }
                Kind::Abs(8) => cmds.push(Tir::StoQw {}),
                Kind::Abs(_) => cmds.push(Tir::StoLw {}),
                Kind::Rel(n) => {
                    cmds.push(Tir::StaPq {
                        psect,
                        offset: pl.offset + r.offset,
                    });
                    cmds.push(Tir::OprSub {});
                    cmds.push(if n == 8 { Tir::StoQw {} } else { Tir::StoLw {} });
                }
            }
            at = start + len;
        }
        imm(&mut cmds, &s.data[at..]);
    }

    let mut out = vec![
        Record::Mhd(Mhd {
            strlvl: obj::STRLVL,
            temp: 0,
            arch1: ARCH_ARM64,
            arch2: 0,
            recsiz: obj::MAX_RECORD as u32,
            name: name.into(),
            version: String::new(),
            date,
        }),
        Record::Text {
            subtype: obj::LNM,
            text: concat!("velf ", env!("CARGO_PKG_VERSION")).into(),
        },
    ];
    out.extend(split(gsd, gsd_size).into_iter().map(Record::Gsd));
    out.extend(split(cmds, Tir::size).into_iter().map(Record::Tir));
    out.push(Record::Eom(
        Eom {
            total_lps: 0,
            comcod: 0,
        },
        None,
    ));
    Ok(out)
}

/// The psect a section goes in, by index in PSECTS, or None if velf drops
/// it: what isn't loaded, notes, and unwind tables, which VMS doesn't use.
fn psect(s: &Section, addresses: bool) -> Result<Option<usize>, String> {
    if s.flags & SHF_ALLOC == 0 || s.kind == SHT_NOTE || s.name.starts_with(".eh_frame") {
        return Ok(None);
    }
    let is = |prefix: &str| s.name == prefix || s.name.starts_with(&format!("{prefix}."));
    Ok(Some(if is(".text") {
        0
    } else if is(".rodata") {
        if addresses { 2 } else { 1 }
    } else if is(".data") {
        3
    } else if is(".bss") {
        4
    } else {
        return Err(format!("section {} isn't one velf takes", s.name));
    }))
}

/// Pushes the relocation's target, S + A, on the linker's stack.
fn push(
    cmds: &mut Vec<Tir>,
    symbols: &[Symbol],
    places: &[Option<Place>],
    index: &[u32],
    r: &Rela,
) -> Result<(), String> {
    let s = symbols
        .get(r.sym)
        .ok_or_else(|| format!("relocation against symbol {}", r.sym))?;
    if s.bind != STB_LOCAL {
        cmds.push(Tir::StaGbl {
            name: vms_name(s.name)?,
        });
        if r.addend != 0 {
            cmds.push(Tir::StaQw {
                value: r.addend as u64,
            });
            cmds.push(Tir::OprAdd {});
        }
        return Ok(());
    }
    // A local symbol, a section's or a static's: its psect and offset.
    let pl = places
        .get(usize::from(s.shndx))
        .copied()
        .flatten()
        .ok_or_else(|| format!("relocation against {}, in a section velf drops", s.name))?;
    cmds.push(Tir::StaPq {
        psect: index[pl.psect],
        offset: pl
            .offset
            .wrapping_add(s.value)
            .wrapping_add(r.addend as u64),
    });
    Ok(())
}

/// Stores bytes as they are.
fn imm(cmds: &mut Vec<Tir>, bytes: &[u8]) {
    cmds.extend(
        bytes
            .chunks(MAX_IMM)
            .map(|d| Tir::StoImm { data: d.to_vec() }),
    );
}

/// A C name as VMS sees it: upper case.
fn vms_name(name: &str) -> Result<String, String> {
    if name.is_empty() || name.len() > 255 || !name.is_ascii() {
        return Err(format!("{name:?} can't be a VMS name"));
    }
    Ok(name.to_ascii_uppercase())
}

fn gsd_size(g: &Gsd) -> usize {
    let body = match g {
        Gsd::Psc(p) => 9 + p.name.len(),
        Gsd::Def(d) => 29 + d.name.len(),
        Gsd::Ref(r) => 5 + r.name.len(),
        Gsd::Other { data, .. } => data.len(),
    };
    (4 + body).next_multiple_of(8)
}

/// Groups items into records that stay under the size limit.
fn split<T>(items: Vec<T>, size: impl Fn(&T) -> usize) -> Vec<Vec<T>> {
    let mut groups: Vec<Vec<T>> = Vec::new();
    let mut used = ROOM;
    for item in items {
        let n = size(&item);
        if used + n > ROOM {
            groups.push(Vec::new());
            used = 0;
        }
        used += n;
        groups.last_mut().unwrap().push(item);
    }
    groups
}

// The ELF reader: a little-endian ELF64 relocatable object for AArch64.

fn bytes(elf: &[u8], at: u64, len: u64) -> Result<&[u8], String> {
    usize::try_from(at)
        .ok()
        .zip(usize::try_from(len).ok())
        .and_then(|(a, l)| elf.get(a..a.checked_add(l)?))
        .ok_or_else(|| "the ELF file is cut short".into())
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

/// The NUL-terminated string at `at` in a string table.
fn string(table: &[u8], at: u32) -> Result<&str, String> {
    let s = table
        .get(at as usize..)
        .ok_or("a name outside its string table")?;
    let end = s
        .iter()
        .position(|&c| c == 0)
        .ok_or("a name without its end")?;
    std::str::from_utf8(&s[..end]).map_err(|_| "a name that isn't UTF-8".into())
}

fn sections(elf: &[u8]) -> Result<Vec<Section<'_>>, String> {
    let h = bytes(elf, 0, 64)?;
    if &h[..4] != b"\x7fELF" || h[4] != 2 || h[5] != 1 {
        return Err("not a little-endian ELF64 file".into());
    }
    if u16_at(h, 16) != 1 || u16_at(h, 18) != 183 {
        return Err("not a relocatable object for AArch64".into());
    }
    let (shoff, shentsize, shnum, shstrndx) = (
        u64_at(h, 40),
        u64::from(u16_at(h, 58)),
        u64::from(u16_at(h, 60)),
        usize::from(u16_at(h, 62)),
    );
    if shentsize != 64 {
        return Err("section headers of an unknown size".into());
    }
    let headers: Vec<&[u8]> = (0..shnum)
        .map(|i| bytes(elf, shoff + i * 64, 64))
        .collect::<Result<_, _>>()?;
    let names = headers
        .get(shstrndx)
        .ok_or("no section name table")
        .and_then(|sh| bytes(elf, u64_at(sh, 24), u64_at(sh, 32)).map_err(|_| "bad name table"))?;
    headers
        .iter()
        .map(|sh| {
            let kind = u32_at(sh, 4);
            let size = u64_at(sh, 32);
            Ok(Section {
                name: string(names, u32_at(sh, 0))?,
                kind,
                flags: u64_at(sh, 8),
                data: if kind == SHT_NOBITS {
                    &[]
                } else {
                    bytes(elf, u64_at(sh, 24), size)?
                },
                size,
                link: u32_at(sh, 40),
                info: u32_at(sh, 44),
                align: u64_at(sh, 48),
            })
        })
        .collect()
}

fn symbols<'a>(sections: &[Section<'a>]) -> Result<Vec<Symbol<'a>>, String> {
    let Some(tab) = sections.iter().find(|s| s.kind == SHT_SYMTAB) else {
        return Ok(Vec::new());
    };
    let strings = sections
        .get(tab.link as usize)
        .ok_or("the symbol table has no string table")?
        .data;
    tab.data
        .as_chunks::<24>()
        .0
        .iter()
        .map(|e| {
            Ok(Symbol {
                name: string(strings, u32_at(e, 0))?,
                bind: e[4] >> 4,
                typ: e[4] & 15,
                shndx: u16_at(e, 6),
                value: u64_at(e, 8),
            })
        })
        .collect()
}

/// Each section's relocations, by section index.
fn relocations(sections: &[Section]) -> Result<Vec<Vec<Rela>>, String> {
    let mut relas: Vec<Vec<Rela>> = sections.iter().map(|_| Vec::new()).collect();
    for s in sections {
        if s.kind == SHT_REL {
            return Err(format!("{} has relocations without addends", s.name));
        }
        if s.kind != SHT_RELA {
            continue;
        }
        let target = relas
            .get_mut(s.info as usize)
            .ok_or_else(|| format!("{} relocates no section", s.name))?;
        for e in s.data.as_chunks::<24>().0 {
            let info = u64_at(e, 8);
            target.push(Rela {
                offset: u64_at(e, 0),
                r_type: info as u32,
                sym: (info >> 32) as usize,
                addend: u64_at(e, 16) as i64,
            });
        }
    }
    Ok(relas)
}
