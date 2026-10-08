//! vlink: links object modules into an executable image at a fixed base,
//! and writes a VMS-style map.
//!
//! Object libraries give the modules that define what the others need.
//! Psects with the same name are merged across modules; CON contributions are
//! concatenated, OVR ones overlaid. Psects go into image sections by
//! protection: code, read-only data, writable data, demand-zero. Then each
//! module's TIR commands run against the final addresses, keeping track of
//! which stored values are addresses, so that a loader can move the image.

mod map;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use vms_obj::exe::{Eisd, Fixups, Image, SECTION_ALIGN, Section};
use vms_obj::obj::{self, Gsd, Record, Tir, psc, sym};
use vms_obj::olb::{self, Library};
use vms_obj::reloc::{self, Need, Weight};

pub struct Options {
    /// Address of the first image section.
    pub base: u64,
    /// Image name, up to 39 characters.
    pub name: String,
    /// Symbol to start at, instead of the first transfer address found.
    pub transfer: Option<String>,
    /// Link time, in VMS format.
    pub link_time: u64,
    /// Make an image a loader may move: anything that can't move is an
    /// error, and the image gets a fixup section.
    pub relocatable: bool,
}

#[derive(Debug)]
pub struct Linked {
    pub image: Image,
    pub map: String,
    /// Warnings and information, as VMS messages.
    pub warnings: Vec<String>,
}

/// The image base VMS uses by default: the first page above 64 KB.
pub const DEFAULT_BASE: u64 = 0x10000;

/// A veneer's bytes.
const VENEER: u64 = 16;

/// Links object files and object libraries, given as (file name, contents), in
/// order. Errors come back as VMS messages.
pub fn link(inputs: &[(String, Vec<u8>)], opts: &Options) -> Result<Linked, Vec<String>> {
    let mut l = Linker::default();
    for (file, bytes) in inputs {
        if olb::is_library(bytes) {
            let lib = Library::parse(bytes).map_err(|e| {
                vec![format!(
                    "%VLINK-F-BADLIB, {file} is not an object library: {e}"
                )]
            })?;
            l.search(file, &lib)?;
            continue;
        }
        let records = obj::parse(bytes).map_err(|e| {
            vec![format!(
                "%VLINK-F-BADOBJ, {file} is not an object file: {e}"
            )]
        })?;
        l.add(file, records)?;
    }
    l.resolve()?;
    l.layout(opts.base)?;
    let (mut data, mut errors) = l.execute();
    // A B or BL that can't reach a fixed address, the executive's from an
    // image in P0, goes through a veneer: then everything again.
    if !l.far.is_empty() {
        l.add_veneers();
        l.layout(opts.base)?;
        (data, errors) = l.execute();
    }
    // A fixup in a PIC psect is always worth a warning; a store that can't
    // move matters only if the image must.
    for (code, msg) in l.problems() {
        if opts.relocatable {
            errors.push(format!("%VLINK-E-{code}, {msg}"));
        } else if code == "NOTPIC" {
            l.warnings.push(format!("%VLINK-W-{code}, {msg}"));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let fixups = if opts.relocatable {
        let f = l.fixup_list()?;
        l.warnings.push(format!(
            "%VLINK-I-FIXUPS, {} quadword and {} longword fixups",
            f.quad.len(),
            f.long.len()
        ));
        Some(f)
    } else {
        None
    };

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
        fixups,
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

/// What a store left in the image that matters if the image moves: an
/// address, which the loader fixes up, or something nothing can fix.
struct Stored {
    /// Bytes: 8 for a quadword address, 4 for a longword one.
    size: u64,
    module: usize,
    value: u64,
    /// Why it can't move; None for an address.
    stuck: Option<String>,
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
    /// What the image holds that matters if it moves, by address.
    stored: BTreeMap<u64, Stored>,
    /// Fixed addresses a B or BL couldn't reach, and the psect of their
    /// veneers once there is one, in this order.
    far: BTreeSet<u64>,
    veneers: Option<usize>,
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

    /// Takes from `lib` each module that defines a symbol strongly referenced
    /// and still undefined, until there are none. As on VMS, a library only
    /// serves the modules before it, and the modules it gives.
    // ponytail: rescans every definition and reference per module taken;
    // index them if libraries grow to thousands of modules.
    fn search(&mut self, file: &str, lib: &Library) -> Result<(), Vec<String>> {
        let index: HashMap<&str, &olb::Module> = lib
            .modules
            .iter()
            .flat_map(|m| m.symbols.iter().map(move |s| (s.as_str(), m)))
            .collect();
        let mut taken = HashSet::new();
        loop {
            let defined: HashSet<&str> = self.raw_defs.iter().map(|d| d.0.as_str()).collect();
            let next = self
                .refs
                .iter()
                .filter(|(name, _, weak)| !weak && !defined.contains(name.as_str()))
                .find_map(|(name, ..)| {
                    index
                        .get(name.as_str())
                        .filter(|m| !taken.contains(&m.name))
                });
            let Some(m) = next else {
                return Ok(());
            };
            taken.insert(&m.name);
            let records = obj::parse(&m.object).map_err(|e| {
                vec![format!(
                    "%VLINK-F-BADOBJ, module {} in {file} is not an object module: {e}",
                    m.name
                )]
            })?;
            self.add(file, records)?;
        }
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
        self.sections.clear();
        for p in &mut self.psects {
            let mut size = 0;
            for part in &mut p.parts {
                // In an absolute psect, as in an overlaid one, every module's
                // part is at 0: its labels are the constants it assembled.
                if p.flags & psc::OVR != 0 || p.flags & psc::REL == 0 {
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

    /// Adds $VENEER$, a code psect of a module of its own, with a veneer
    /// for each address in `far`: `ldr x16, 8; br x16` and the address,
    /// which DESIGN-0004 lets a linker put between a call and its target.
    fn add_veneers(&mut self) {
        let flags = psc::PIC | psc::REL | psc::SHR | psc::EXE | psc::RD;
        let p = self.psect("$VENEER$", flags, 3).unwrap();
        let module = self.modules.len();
        self.psects[p].parts.push(Part {
            module,
            size: VENEER * self.far.len() as u64,
            align: 3,
            offset: 0,
        });
        self.modules.push(Module {
            name: "$VENEERS".into(),
            version: String::new(),
            file: String::new(),
            psects: vec![(p, 0)],
            records: Vec::new(),
            transfer: None,
        });
        self.veneers = Some(p);
    }

    /// The veneer that jumps to `target`, if there is one.
    fn veneer(&self, target: u64) -> Option<u64> {
        let p = &self.psects[self.veneers?];
        let i = self.far.iter().position(|&t| t == target)?;
        Some(p.base + VENEER * i as u64)
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

    /// Where an address is, for messages: the psect and offset in the module
    /// contribution that holds it, the nearest global label at or before it
    /// there, and the module.
    fn place(&self, addr: u64) -> String {
        match self.locate(addr) {
            Some((m, at)) => {
                let m = &self.modules[m];
                format!("{at} in module {} ({})", m.name, m.file)
            }
            None => format!("%X{addr:X}"),
        }
    }

    /// The module whose contribution holds `addr`, and where in it: psect +
    /// offset (label+offset).
    fn locate(&self, addr: u64) -> Option<(usize, String)> {
        for p in self.psects.iter().filter(|p| p.flags & psc::REL != 0) {
            for part in &p.parts {
                let start = p.base + part.offset;
                if !(start..start + part.size.max(1)).contains(&addr) {
                    continue;
                }
                let label = self
                    .defs
                    .iter()
                    .filter(|(_, d)| d.module == part.module && d.psect.is_some())
                    .filter_map(|(name, _)| Some((self.value(name)?, name)))
                    .filter(|&(v, _)| (start..=addr).contains(&v))
                    .max();
                let label = match label {
                    Some((v, name)) if v == addr => format!(" ({name})"),
                    Some((v, name)) => format!(" ({name}+%X{:X})", addr - v),
                    None => String::new(),
                };
                return Some((
                    part.module,
                    format!("{} + %X{:X}{label}", p.name, addr - start),
                ));
            }
        }
        None
    }

    /// The relocatable psect holding `addr`.
    fn psect_at(&self, addr: u64) -> Option<&Psect> {
        self.psects
            .iter()
            .find(|p| p.flags & psc::REL != 0 && (p.base..p.base + p.size).contains(&addr))
    }

    /// The addresses the image holds, which the loader fixes up.
    fn fixups(&self) -> impl Iterator<Item = (u64, &Stored)> {
        self.stored
            .iter()
            .filter(|(_, s)| s.stuck.is_none())
            .map(|(&at, s)| (at, s))
    }

    /// What keeps the image from moving, from what it holds in the end:
    /// message code (NOTPIC for a fixup in a PIC psect, NORELOC for what
    /// nothing can fix) and text, each once.
    fn problems(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for (&at, s) in &self.stored {
            let problem = match &s.stuck {
                Some(why) => ("NORELOC", why.clone()),
                None if self.psect_at(at).is_some_and(|p| p.flags & psc::PIC != 0) => (
                    "NOTPIC",
                    format!(
                        "address in a PIC psect needs a fixup, at {}",
                        self.place(at)
                    ),
                ),
                None => continue,
            };
            // The parts of one store on both sides of a later one.
            if !out.contains(&problem) {
                out.push(problem);
            }
        }
        out
    }

    /// The fixups for the image's fixup section.
    fn fixup_list(&self) -> Result<Fixups, Vec<String>> {
        let start = self.sections.first().map_or(0, |s| s.start);
        let mut f = Fixups::default();
        let mut values = Vec::new();
        for (at, fix) in self.fixups() {
            let Ok(off) = u32::try_from(at - start) else {
                return Err(vec![format!(
                    "%VLINK-F-TOOBIG, the address at {} is 4 GB or more into the image",
                    self.place(at)
                )]);
            };
            if fix.size == 8 {
                f.quad.push(off);
            } else {
                f.long.push(off);
                // Checked when it was stored.
                values.push(fix.value as i32);
            }
        }
        f.long_min = values.iter().copied().min().unwrap_or(0);
        f.long_max = values.iter().copied().max().unwrap_or(0);
        Ok(f)
    }

    /// Runs every module's TIR commands; returns each image section's
    /// contents, and the errors.
    fn execute(&mut self) -> (Vec<Vec<u8>>, Vec<String>) {
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
        let mut stored = BTreeMap::new();
        let mut far = BTreeSet::new();
        let (mut errors, mut warnings) = (Vec::new(), Vec::new());
        for (i, m) in self.modules.iter().enumerate() {
            let mut run = Run {
                l: self,
                module: i,
                stack: Vec::new(),
                loc: None,
                data: &mut data,
                stored: &mut stored,
                far: &mut far,
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
        self.stored = stored;
        if self.veneers.is_some() {
            for &target in &self.far {
                let at = self.veneer(target).unwrap();
                let s = self
                    .sections
                    .iter()
                    .position(|s| s.start <= at && at < s.end);
                let off = (at - self.sections[s.unwrap()].start) as usize;
                let code = [0x5800_0050u32.to_le_bytes(), 0xd61f_0200u32.to_le_bytes()];
                data[s.unwrap()][off..off + 8].copy_from_slice(&code.concat());
                data[s.unwrap()][off + 8..off + 16].copy_from_slice(&target.to_le_bytes());
            }
        } else {
            self.far = far;
        }
        (data, errors)
    }
}

/// TIR execution for one module.
struct Run<'a> {
    l: &'a Linker,
    module: usize,
    /// Values, each with its weight.
    stack: Vec<(u64, Weight)>,
    /// The location counter, once set.
    loc: Option<u64>,
    data: &'a mut Vec<Vec<u8>>,
    stored: &'a mut BTreeMap<u64, Stored>,
    /// The fixed addresses B and BL couldn't reach.
    far: &'a mut BTreeSet<u64>,
}

impl Run<'_> {
    fn name(&self) -> &str {
        &self.l.modules[self.module].name
    }

    fn underflow(&self) -> String {
        format!(
            "%VLINK-F-STACK, linker stack underflow in module {}",
            self.name()
        )
    }

    fn pop(&mut self) -> Result<(u64, Weight), String> {
        self.stack.pop().ok_or_else(|| self.underflow())
    }

    /// A symbol's value, and its weight: 1 for an address, 0 for a constant.
    fn symbol(&self, name: &str) -> Result<(u64, Weight), String> {
        let Some(def) = self.l.defs.get(name) else {
            // A weak reference to a missing symbol is 0; others were reported.
            return Ok((0, Some(0)));
        };
        match self.l.value(name) {
            Some(v) => Ok((v, Some(def.psect.is_some().into()))),
            None => Err(format!("%VLINK-F-BADOBJ, symbol {name} has a bad psect")),
        }
    }

    /// Notes what the `size` bytes that `cmd` stored at `at`, of `v` with
    /// weight `k`, need for the image to move.
    fn fix(&mut self, at: u64, size: u64, cmd: &Tir, v: u64, k: Weight) {
        let stuck = match reloc::need(cmd, k) {
            Need::Nothing => return,
            Need::Fixup(_) => None,
            Need::Impossible => {
                let what = match k {
                    // Only a PC-relative field needs an address.
                    Some(0) => "a fixed address".to_string(),
                    Some(1) => "an address".into(),
                    Some(n) => format!("an address times {n}"),
                    None => "a value computed from an address".into(),
                };
                let at = self.l.place(at);
                Some(format!("{} of {what} can't move, at {at}", cmd.name()))
            }
        };
        let module = self.module;
        self.stored.insert(
            at,
            Stored {
                size,
                module,
                value: v,
                stuck,
            },
        );
    }

    /// Stores bytes at the location counter and advances it; returns where
    /// they went.
    fn store(&mut self, bytes: &[u8]) -> Result<u64, String> {
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
        // The bytes replace what was stored there. What an earlier store
        // left on either side stays, but part of an address isn't one.
        let left = (self.stored.range(..at).next_back())
            .and_then(|(&a, s)| (a + s.size > at).then_some(a));
        let hit: Vec<u64> = (left.into_iter())
            .chain(self.stored.range(at..end).map(|(&a, _)| a))
            .collect();
        for a in hit {
            let old = self.stored.remove(&a).unwrap();
            let stuck = old.stuck.clone().unwrap_or_else(|| {
                format!("part of an address is overwritten, at {}", self.l.place(a))
            });
            for (from, to) in [(a, at), (end, a + old.size)] {
                if from < to {
                    let rest = Stored {
                        size: to - from,
                        module: old.module,
                        value: old.value,
                        stuck: Some(stuck.clone()),
                    };
                    self.stored.insert(from, rest);
                }
            }
        }
        self.loc = Some(end);
        Ok(at)
    }

    fn step(&mut self, cmd: &Tir) -> Result<(), String> {
        use Tir::*;
        match cmd {
            StaGbl { name } => {
                let v = self.symbol(name)?;
                self.stack.push(v);
            }
            StaLw { value } => self.stack.push((*value as i32 as u64, Some(0))),
            StaQw { value } => self.stack.push((*value, Some(0))),
            StaPq { psect, offset } => {
                let base = self.l.part_base(self.module, *psect)?;
                // part_base checked the index.
                let (p, _) = self.l.modules[self.module].psects[*psect as usize];
                let rel = self.l.psects[p].flags & psc::REL != 0;
                self.stack
                    .push((base.wrapping_add(*offset), Some(rel.into())));
            }
            StoB {} | StoW {} | StoLw {} => {
                let bits = match cmd {
                    StoB {} => 8,
                    StoW {} => 16,
                    _ => 32,
                };
                let (v, k) = self.pop()?;
                let s = v as i64;
                // An address in a longword is sign-extended when it's loaded,
                // as on VMS.
                let address = reloc::need(cmd, k) == Need::Fixup(4);
                let fits = if address {
                    i32::try_from(s).is_ok()
                } else {
                    s >= -(1 << (bits - 1)) && s < 1 << bits
                };
                if !fits {
                    let at = self.loc.map_or(String::new(), |a| self.l.place(a));
                    let (what, how) = if address {
                        ("address ", ", signed,")
                    } else {
                        ("", "")
                    };
                    return Err(format!(
                        "%VLINK-E-TRUNC, {what}%X{v:X} doesn't fit in {} bytes{how} at {at}",
                        bits / 8
                    ));
                }
                let at = self.store(&v.to_le_bytes()[..bits / 8])?;
                self.fix(at, bits as u64 / 8, cmd, v, k);
            }
            StoQw {} | StoOff {} => {
                let (v, k) = self.pop()?;
                let at = self.store(&v.to_le_bytes())?;
                self.fix(at, 8, cmd, v, k);
            }
            StoImmr { data } => {
                let (n, k) = self.pop()?;
                let at = self.loc.unwrap_or(0);
                for _ in 0..n {
                    self.store(data)?;
                }
                self.fix(at, n.saturating_mul(data.len() as u64), cmd, n, k);
            }
            StoGbl { name } | StoCa { name } => {
                let (v, k) = self.symbol(name)?;
                let at = self.store(&v.to_le_bytes())?;
                self.fix(at, 8, cmd, v, k);
            }
            StoImm { data } => {
                self.store(data)?;
            }
            OprNop {} => {}
            CtlSetrb {} => self.loc = Some(self.pop()?.0),
            CtlAugrb { offset } => {
                let Some(at) = self.loc else {
                    return Err(format!(
                        "%VLINK-F-NOLOC, module {} moves the location before setting it",
                        self.name()
                    ));
                };
                self.loc = Some(at.wrapping_add(*offset as i32 as u64));
            }
            _ => {
                if let Some(n) = reloc::operands(cmd) {
                    let Some(first) = self.stack.len().checked_sub(n) else {
                        return Err(self.underflow());
                    };
                    let args = self.stack.split_off(first);
                    self.stack.push(reloc::operate(cmd, &args));
                    return Ok(());
                }
                let Some((kind, insn)) = reloc::Kind::of(cmd) else {
                    return Err(format!(
                        "%VLINK-F-UNSUPPORTED, TIR command {} ({}) in module {}",
                        cmd.name(),
                        cmd.code(),
                        self.name()
                    ));
                };
                let (s, k) = self.pop()?;
                let p = self.loc.unwrap_or(0);
                let mut word = reloc::apply(kind, insn, s, p);
                if word.is_err() && kind == reloc::Kind::Jump26 && k == Some(0) {
                    // The first pass notes the address and goes on; the
                    // second has the veneer.
                    word = match self.l.veneers {
                        None => {
                            self.far.insert(s);
                            Ok(insn)
                        }
                        Some(_) => reloc::apply(kind, insn, self.l.veneer(s).unwrap(), p),
                    };
                }
                let word = word.map_err(|e| {
                    format!(
                        "%VLINK-E-RELOC, {e}: target %X{s:X}, instruction at {}",
                        self.l.place(p)
                    )
                })?;
                let at = self.store(&word.to_le_bytes())?;
                self.fix(at, 4, cmd, s, k);
            }
        }
        Ok(())
    }
}

/// Checks that `moved` is `image` linked again at another base, a test of
/// the fixup list: the two differ exactly where `image`'s fixups are, each
/// by the distance between the bases.
pub fn check_fixups(image: &Image, moved: &Image) -> Result<(), String> {
    let fixups = image.fixups.as_ref().ok_or("the image has no fixups")?;
    let start = |i: &Image| i.sections.first().map_or(0, |s| s.vaddr);
    let d = start(moved).wrapping_sub(start(image));
    // A transfer address of 0 means none.
    let transfer = match image.transfer {
        0 => 0,
        t => t.wrapping_add(d),
    };
    let same =
        image.sections.len() == moved.sections.len()
            && transfer == moved.transfer
            && image.sections.iter().zip(&moved.sections).all(|(a, b)| {
                (a.vaddr.wrapping_add(d), a.size, a.flags) == (b.vaddr, b.size, b.flags)
            });
    if !same {
        return Err("the images have different layouts".into());
    }
    let mut want: BTreeMap<u64, usize> = (fixups.quad.iter().map(|&o| (o, 8)))
        .chain(fixups.long.iter().map(|&o| (o, 4)))
        .map(|(o, size)| (u64::from(o), size))
        .collect();
    for (a, b) in image.sections.iter().zip(&moved.sections) {
        let off = a.vaddr - start(image);
        let mut i = 0;
        while i < a.data.len() {
            let at = off + i as u64;
            let Some(n) = want.remove(&at) else {
                if a.data[i] != b.data[i] {
                    return Err(format!(
                        "the byte at %X{at:X} changed, but no fixup covers it"
                    ));
                }
                i += 1;
                continue;
            };
            let get = |data: &[u8]| {
                let mut v = [0; 8];
                v[..n].copy_from_slice(&data[i..i + n]);
                u64::from_le_bytes(v)
            };
            let (x, y) = (get(&a.data), get(&b.data));
            let by = if n == 8 {
                y.wrapping_sub(x)
            } else {
                (i64::from(y as u32 as i32) - i64::from(x as u32 as i32)) as u64
            };
            if by != d {
                return Err(format!(
                    "the address at %X{at:X} moved by %X{by:X}, not %X{d:X}"
                ));
            }
            i += n;
        }
    }
    match want.first_key_value() {
        Some((at, _)) => Err(format!("the fixup at %X{at:X} is outside the contents")),
        None => Ok(()),
    }
}
