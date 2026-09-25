//! vlink: links object modules into an executable image at a fixed base,
//! and writes a VMS-style map.
//!
//! Psects with the same name are merged across modules; CON contributions are
//! concatenated, OVR ones overlaid. Psects go into image sections by
//! protection: code, read-only data, writable data, demand-zero. Then each
//! module's TIR commands run against the final addresses.

mod map;

use std::collections::HashMap;

use vms_obj::exe::{Eisd, Image, SECTION_ALIGN, Section};
use vms_obj::obj::{self, Gsd, Record, Tir, psc, sym};
use vms_obj::reloc;

pub struct Options {
    /// Address of the first image section.
    pub base: u64,
    /// Image name, up to 39 characters.
    pub name: String,
    /// Symbol to start at, instead of the first transfer address found.
    pub transfer: Option<String>,
    /// Link time, in VMS format.
    pub link_time: u64,
}

#[derive(Debug)]
pub struct Linked {
    pub image: Image,
    pub map: String,
    /// Warnings, as VMS messages.
    pub warnings: Vec<String>,
}

/// The image base VMS uses by default: the first page above 64 KB.
pub const DEFAULT_BASE: u64 = 0x10000;

/// Links object files, given as (file name, contents). Errors come back as
/// VMS messages.
pub fn link(inputs: &[(String, Vec<u8>)], opts: &Options) -> Result<Linked, Vec<String>> {
    let mut l = Linker::default();
    for (file, bytes) in inputs {
        let records = obj::parse(bytes).map_err(|e| {
            vec![format!(
                "%VLINK-F-BADOBJ, {file} is not an object file: {e}"
            )]
        })?;
        l.add(file, records)?;
    }
    l.resolve()?;
    l.layout(opts.base)?;
    let data = l.execute()?;

    let transfer = match &opts.transfer {
        Some(name) => match l.value(name) {
            Some(v) => v,
            None => {
                return Err(vec![format!(
                    "%VLINK-F-NOTFR, transfer symbol {name} is not defined"
                )]);
            }
        },
        None => match l
            .modules
            .iter()
            .enumerate()
            .find_map(|(i, m)| m.transfer.map(|t| (i, t)))
        {
            Some((m, (psect, offset))) => l.part_base(m, psect).map_err(|e| vec![e])? + offset,
            None => {
                l.warnings
                    .push("%VLINK-W-USRTFR, the image has no transfer address".into());
                0
            }
        },
    };
    let ident = l
        .modules
        .iter()
        .find(|m| m.transfer.is_some())
        .or(l.modules.first());
    let image = Image {
        name: opts.name.chars().take(39).collect(),
        ident: ident.map_or(String::new(), |m| m.version.chars().take(15).collect()),
        link_time: opts.link_time,
        transfer,
        sections: l
            .sections
            .iter()
            .zip(data)
            .map(|(s, data)| Section {
                vaddr: s.start,
                size: (s.end - s.start) as u32,
                flags: s.class.flags(),
                data,
            })
            .collect(),
    };
    let map = map::map(&l, &image);
    Ok(Linked {
        image,
        map,
        warnings: l.warnings,
    })
}

struct Module {
    name: String,
    version: String,
    file: String,
    /// For each of the module's psects: the image psect and its part.
    psects: Vec<(usize, usize)>,
    /// TIR records, run after layout.
    records: Vec<Record>,
    transfer: Option<(u32, u64)>,
}

struct Psect {
    name: String,
    flags: u16,
    align: u8,
    parts: Vec<Part>,
    base: u64,
    size: u64,
}

/// One module's contribution to a psect.
struct Part {
    module: usize,
    size: u64,
    align: u8,
    offset: u64,
}

struct Def {
    module: usize,
    /// The module's psect index, or None for an absolute value.
    psect: Option<u32>,
    value: u64,
    weak: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Class {
    Code,
    ReadOnly,
    Data,
    Zero,
}

impl Class {
    fn of(flags: u16) -> Class {
        if flags & psc::EXE != 0 {
            Class::Code
        } else if flags & psc::WRT == 0 {
            Class::ReadOnly
        } else if flags & psc::NOMOD != 0 {
            Class::Zero
        } else {
            Class::Data
        }
    }

    fn flags(self) -> u32 {
        match self {
            Class::Code => Eisd::M_EXE,
            Class::ReadOnly => 0,
            Class::Data => Eisd::M_WRT | Eisd::M_CRF,
            Class::Zero => Eisd::M_WRT | Eisd::M_DZRO,
        }
    }
}

struct ImageSection {
    class: Class,
    start: u64,
    end: u64,
}

#[derive(Default)]
struct Linker {
    modules: Vec<Module>,
    psects: Vec<Psect>,
    sections: Vec<ImageSection>,
    defs: HashMap<String, Def>,
    /// Symbol references: name, module, weak.
    refs: Vec<(String, usize, bool)>,
    /// Definitions in the order they appeared: name, module, psect, value, flags.
    raw_defs: Vec<(String, usize, u32, u64, u16)>,
    warnings: Vec<String>,
}

impl Linker {
    fn add(&mut self, file: &str, records: Vec<Record>) -> Result<(), Vec<String>> {
        let mut cur: Option<Module> = None;
        let bad = |msg: &str| vec![format!("%VLINK-F-BADOBJ, {file}: {msg}")];
        for record in records {
            if let Record::Mhd(h) = &record {
                if cur.is_some() {
                    return Err(bad("a module has no end-of-module record"));
                }
                cur = Some(Module {
                    name: h.name.clone(),
                    version: h.version.clone(),
                    file: file.to_string(),
                    psects: Vec::new(),
                    records: Vec::new(),
                    transfer: None,
                });
                continue;
            }
            let Some(m) = cur.as_mut() else {
                return Err(bad("records before the module header"));
            };
            match record {
                Record::Eom(eom, transfer) => {
                    let mut m = cur.take().unwrap();
                    if eom.comcod >= 2 {
                        return Err(vec![format!(
                            "%VLINK-F-COMPERR, module {} has compilation errors",
                            m.name
                        )]);
                    }
                    m.transfer = transfer.map(|t| (t.psindx, t.tfradr));
                    self.modules.push(m);
                }
                Record::Gsd(subrecords) => {
                    let module = self.modules.len();
                    for sub in subrecords {
                        match sub {
                            Gsd::Psc(p) => {
                                let psect =
                                    self.psect(&p.name, p.flags, p.align).map_err(|e| vec![e])?;
                                let parts = &mut self.psects[psect].parts;
                                parts.push(Part {
                                    module,
                                    size: u64::from(p.alloc),
                                    align: p.align,
                                    offset: 0,
                                });
                                m.psects.push((psect, parts.len() - 1));
                            }
                            Gsd::Def(d) => self
                                .raw_defs
                                .push((d.name, module, d.psindx, d.value, d.flags)),
                            Gsd::Ref(r) => {
                                self.refs.push((r.name, module, r.flags & sym::WEAK != 0))
                            }
                            Gsd::Other { gsdtyp, .. } => {
                                return Err(bad(&format!(
                                    "unsupported GSD subrecord type {gsdtyp}"
                                )));
                            }
                        }
                    }
                }
                Record::Tir(_) | Record::Dbg(_) | Record::Tbt(_) => m.records.push(record),
                Record::Text { .. } | Record::Mhd(_) => {}
            }
        }
        if cur.is_some() {
            return Err(bad("the last module has no end-of-module record"));
        }
        Ok(())
    }

    /// The image psect called `name`, created on first use.
    fn psect(&mut self, name: &str, flags: u16, align: u8) -> Result<usize, String> {
        if flags & psc::EXE != 0 && flags & psc::WRT != 0 {
            return Err(format!(
                "%VLINK-F-PSECTATTR, psect {name} is both executable and writable"
            ));
        }
        if let Some(i) = self.psects.iter().position(|p| p.name == name) {
            let p = &mut self.psects[i];
            if p.flags != flags {
                return Err(format!(
                    "%VLINK-F-PSECTATTR, psect {name} has conflicting attributes in different modules"
                ));
            }
            p.align = p.align.max(align);
            return Ok(i);
        }
        self.psects.push(Psect {
            name: name.to_string(),
            flags,
            align,
            parts: Vec::new(),
            base: 0,
            size: 0,
        });
        Ok(self.psects.len() - 1)
    }

    fn resolve(&mut self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for (name, module, psect, value, flags) in std::mem::take(&mut self.raw_defs) {
            if self.modules[module].psects.get(psect as usize).is_none() {
                errors.push(format!(
                    "%VLINK-F-BADOBJ, symbol {name} refers to a psect module {} doesn't define",
                    self.modules[module].name
                ));
                continue;
            }
            let absolute = flags & sym::REL == 0;
            let def = Def {
                module,
                psect: (!absolute).then_some(psect),
                value,
                weak: flags & sym::WEAK != 0,
            };
            match self.defs.get(&name) {
                Some(old) if old.weak && !def.weak => {
                    self.defs.insert(name, def);
                }
                Some(old) if !old.weak && !def.weak => errors.push(format!(
                    "%VLINK-E-MULDEF, symbol {name} is defined in modules {} and {}",
                    self.modules[old.module].name, self.modules[module].name
                )),
                Some(_) => {}
                None => {
                    self.defs.insert(name, def);
                }
            }
        }
        let mut undefined: Vec<(String, Vec<String>)> = Vec::new();
        for (name, module, weak) in &self.refs {
            if *weak || self.defs.contains_key(name) {
                continue;
            }
            let by = self.modules[*module].name.clone();
            match undefined.iter_mut().find(|u| u.0 == *name) {
                Some(u) if !u.1.contains(&by) => u.1.push(by),
                Some(_) => {}
                None => undefined.push((name.clone(), vec![by])),
            }
        }
        for (name, by) in undefined {
            errors.push(format!(
                "%VLINK-E-UDFSYM, undefined symbol {name}, referenced by {}",
                by.join(", ")
            ));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn layout(&mut self, base: u64) -> Result<(), Vec<String>> {
        for p in &mut self.psects {
            let mut size = 0;
            for part in &mut p.parts {
                if p.flags & psc::OVR != 0 {
                    size = size.max(part.size);
                } else {
                    part.offset = size.next_multiple_of(1 << part.align);
                    size = part.offset + part.size;
                }
            }
            p.size = size;
        }
        let mut at = base;
        for class in [Class::Code, Class::ReadOnly, Class::Data, Class::Zero] {
            let members: Vec<usize> = (0..self.psects.len())
                .filter(|&i| {
                    self.psects[i].flags & psc::REL != 0 && Class::of(self.psects[i].flags) == class
                })
                .collect();
            if members.iter().all(|&i| self.psects[i].size == 0) {
                continue;
            }
            let start = at.next_multiple_of(SECTION_ALIGN);
            let mut pos = start;
            for i in members {
                let p = &mut self.psects[i];
                p.base = pos.next_multiple_of(1 << p.align);
                pos = p.base + p.size;
            }
            if pos - start > u64::from(u32::MAX) {
                return Err(vec![
                    "%VLINK-F-TOOBIG, an image section is larger than 4 GB".to_string(),
                ]);
            }
            self.sections.push(ImageSection {
                class,
                start,
                end: pos,
            });
            at = pos;
        }
        Ok(())
    }

    /// Where a module's psect contribution starts.
    fn part_base(&self, module: usize, psect: u32) -> Result<u64, String> {
        let m = &self.modules[module];
        let Some(&(p, part)) = m.psects.get(psect as usize) else {
            return Err(format!(
                "%VLINK-F-BADOBJ, module {} refers to psect {psect}, which it doesn't define",
                m.name
            ));
        };
        Ok(self.psects[p].base + self.psects[p].parts[part].offset)
    }

    /// A global symbol's final value.
    fn value(&self, name: &str) -> Option<u64> {
        let d = self.defs.get(name)?;
        match d.psect {
            Some(p) => self.part_base(d.module, p).ok().map(|b| b + d.value),
            None => Some(d.value),
        }
    }

    /// Psect, offset and module of an address, for messages.
    fn place(&self, addr: u64) -> String {
        for p in &self.psects {
            for part in &p.parts {
                let start = p.base + part.offset;
                if (start..start + part.size.max(1)).contains(&addr) {
                    let m = &self.modules[part.module];
                    return format!(
                        "{} + %X{:X} in module {} ({})",
                        p.name,
                        addr - start,
                        m.name,
                        m.file
                    );
                }
            }
        }
        format!("%X{addr:X}")
    }

    /// Runs every module's TIR commands; returns each image section's
    /// contents.
    fn execute(&mut self) -> Result<Vec<Vec<u8>>, Vec<String>> {
        let mut data: Vec<Vec<u8>> = self
            .sections
            .iter()
            .map(|s| {
                if s.class == Class::Zero {
                    Vec::new()
                } else {
                    vec![0; (s.end - s.start) as usize]
                }
            })
            .collect();
        let (mut errors, mut warnings) = (Vec::new(), Vec::new());
        for (i, m) in self.modules.iter().enumerate() {
            let mut run = Run {
                l: self,
                module: i,
                stack: Vec::new(),
                loc: None,
                data: &mut data,
            };
            let cmds = m.records.iter().filter_map(|r| match r {
                Record::Tir(c) => Some(c),
                _ => None, // no debugger yet: DBG and TBT records are ignored
            });
            for cmd in cmds.flatten() {
                if let Err(e) = run.step(cmd) {
                    errors.push(e);
                    break;
                }
            }
            if !run.stack.is_empty() {
                warnings.push(format!(
                    "%VLINK-W-EOMSTCK, module {} left values on the linker's stack",
                    m.name
                ));
            }
        }
        self.warnings.extend(warnings);
        if errors.is_empty() {
            Ok(data)
        } else {
            Err(errors)
        }
    }
}

/// TIR execution for one module.
struct Run<'a> {
    l: &'a Linker,
    module: usize,
    stack: Vec<u64>,
    /// The location counter, once set.
    loc: Option<u64>,
    data: &'a mut Vec<Vec<u8>>,
}

impl Run<'_> {
    fn name(&self) -> &str {
        &self.l.modules[self.module].name
    }

    fn pop(&mut self) -> Result<u64, String> {
        self.stack.pop().ok_or_else(|| {
            format!(
                "%VLINK-F-STACK, linker stack underflow in module {}",
                self.name()
            )
        })
    }

    fn symbol(&self, name: &str) -> Result<u64, String> {
        match self.l.value(name) {
            Some(v) => Ok(v),
            // A weak reference to a missing symbol is 0; others were reported.
            None if !self.l.defs.contains_key(name) => Ok(0),
            None => Err(format!("%VLINK-F-BADOBJ, symbol {name} has a bad psect")),
        }
    }

    /// Stores bytes at the location counter and advances it.
    fn store(&mut self, bytes: &[u8]) -> Result<(), String> {
        let Some(at) = self.loc else {
            return Err(format!(
                "%VLINK-F-NOLOC, module {} stores data before setting a location",
                self.name()
            ));
        };
        let end = at + bytes.len() as u64;
        let found = self
            .l
            .sections
            .iter()
            .position(|s| s.start <= at && end <= s.end);
        match found {
            Some(i) if self.l.sections[i].class == Class::Zero => {
                return Err(format!(
                    "%VLINK-F-DZRO, initialized data in a demand-zero psect at {}",
                    self.l.place(at)
                ));
            }
            Some(i) => {
                let off = (at - self.l.sections[i].start) as usize;
                self.data[i][off..off + bytes.len()].copy_from_slice(bytes);
            }
            None => {
                return Err(format!(
                    "%VLINK-F-OUTSIDE, module {} stores outside the image at %X{at:X}",
                    self.name()
                ));
            }
        }
        self.loc = Some(end);
        Ok(())
    }

    fn step(&mut self, cmd: &Tir) -> Result<(), String> {
        use Tir::*;
        match cmd {
            StaGbl { name } => {
                let v = self.symbol(name)?;
                self.stack.push(v);
            }
            StaLw { value } => self.stack.push(*value as i32 as u64),
            StaQw { value } => self.stack.push(*value),
            StaPq { psect, offset } => {
                let base = self.l.part_base(self.module, *psect)?;
                self.stack.push(base.wrapping_add(*offset));
            }
            StoB {} | StoW {} | StoLw {} => {
                let bits = match cmd {
                    StoB {} => 8,
                    StoW {} => 16,
                    _ => 32,
                };
                let v = self.pop()?;
                let s = v as i64;
                if s < -(1 << (bits - 1)) || s >= 1 << bits {
                    let at = self.loc.map_or(String::new(), |a| self.l.place(a));
                    return Err(format!(
                        "%VLINK-E-TRUNC, %X{v:X} doesn't fit in {} bytes at {at}",
                        bits / 8
                    ));
                }
                self.store(&v.to_le_bytes()[..bits / 8])?;
            }
            StoQw {} | StoOff {} => {
                let v = self.pop()?;
                self.store(&v.to_le_bytes())?;
            }
            StoImmr { data } => {
                let n = self.pop()?;
                for _ in 0..n {
                    self.store(data)?;
                }
            }
            StoGbl { name } | StoCa { name } => {
                let v = self.symbol(name)?;
                self.store(&v.to_le_bytes())?;
            }
            StoImm { data } => self.store(data)?,
            OprNop {} => {}
            OprNeg {} | OprCom {} => {
                let v = self.pop()?;
                self.stack.push(if matches!(cmd, OprNeg {}) {
                    v.wrapping_neg()
                } else {
                    !v
                });
            }
            OprAdd {}
            | OprSub {}
            | OprMul {}
            | OprDiv {}
            | OprAnd {}
            | OprIor {}
            | OprEor {}
            | OprAsh {}
            | OprRot {} => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(match cmd {
                    OprAdd {} => a.wrapping_add(b),
                    OprSub {} => a.wrapping_sub(b),
                    OprMul {} => a.wrapping_mul(b),
                    OprDiv {} if b == 0 => 0,
                    OprDiv {} => (a as i64).wrapping_div(b as i64) as u64,
                    OprAnd {} => a & b,
                    OprIor {} => a | b,
                    OprEor {} => a ^ b,
                    // The second value popped is the count: positive left.
                    OprAsh {} => match a as i64 {
                        n if n >= 0 => b.wrapping_shl(n as u32),
                        n => ((b as i64).wrapping_shr(n.unsigned_abs() as u32)) as u64,
                    },
                    _ => match a as i64 {
                        n if n >= 0 => b.rotate_left(n as u32),
                        n => b.rotate_right(n.unsigned_abs() as u32),
                    },
                });
            }
            OprSel {} => {
                let cond = self.pop()?;
                let second = self.pop()?;
                let third = self.pop()?;
                self.stack.push(if cond & 1 != 0 { second } else { third });
            }
            CtlSetrb {} => self.loc = Some(self.pop()?),
            CtlAugrb { offset } => {
                let Some(at) = self.loc else {
                    return Err(format!(
                        "%VLINK-F-NOLOC, module {} moves the location before setting it",
                        self.name()
                    ));
                };
                self.loc = Some(at.wrapping_add(*offset as i32 as u64));
            }
            _ => match reloc::Kind::of(cmd) {
                Some((kind, insn)) => {
                    let s = self.pop()?;
                    let p = self.loc.unwrap_or(0);
                    let word = reloc::apply(kind, insn, s, p).map_err(|e| {
                        format!(
                            "%VLINK-E-RELOC, {e}: target %X{s:X}, instruction at {}",
                            self.l.place(p)
                        )
                    })?;
                    self.store(&word.to_le_bytes())?;
                }
                None => {
                    return Err(format!(
                        "%VLINK-F-UNSUPPORTED, TIR command {} ({}) in module {}",
                        cmd.name(),
                        cmd.code(),
                        self.name()
                    ));
                }
            },
        }
        Ok(())
    }
}
