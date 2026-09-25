//! The link map, laid out like VMS LINK/MAP. Values are 16 hex digits, so
//! tools such as `vrun --map` can read the symbol lists back.

use std::fmt::Write;

use vms_obj::exe::Image;
use vms_obj::obj::psc;

use crate::{Class, Linker};

pub(crate) fn map(l: &Linker, image: &Image) -> String {
    let mut o = String::new();
    let w = &mut o;

    let _ = writeln!(w, "Object Module Synopsis\n");
    let _ = writeln!(w, "  {:<31}  {:<15}  File", "Module", "Ident");
    for m in &l.modules {
        let _ = writeln!(w, "  {:<31}  {:<15}  {}", m.name, m.version, m.file);
    }

    let _ = writeln!(w, "\nImage Section Synopsis\n");
    let _ = writeln!(
        w,
        "  {:<16}  {:<16}  {:<8}  Contents",
        "Base", "End", "Length"
    );
    for s in &l.sections {
        let kind = match s.class {
            Class::Code => "code: read, execute",
            Class::ReadOnly => "data: read-only",
            Class::Data => "data: read, write",
            Class::Zero => "demand-zero: read, write",
        };
        let _ = writeln!(
            w,
            "  {:016X}  {:016X}  {:08X}  {kind}",
            s.start,
            last(s.start, s.end - s.start),
            s.end - s.start
        );
    }

    let _ = writeln!(w, "\nProgram Section Synopsis\n");
    let _ = writeln!(
        w,
        "  {:<31}  {:<16}  {:<16}  {:<8}  Align  Attributes",
        "Psect / Module", "Base", "End", "Length"
    );
    let mut psects: Vec<_> = l.psects.iter().collect();
    psects.sort_by_key(|p| p.base);
    for p in psects {
        let _ = writeln!(
            w,
            "  {:<31}  {:016X}  {:016X}  {:08X}  2**{:<3}  {}",
            p.name,
            p.base,
            last(p.base, p.size),
            p.size,
            p.align,
            attributes(p.flags)
        );
        for part in &p.parts {
            let base = p.base + part.offset;
            let _ = writeln!(
                w,
                "    {:<29}  {:016X}  {:016X}  {:08X}  2**{}",
                l.modules[part.module].name,
                base,
                last(base, part.size),
                part.size,
                part.align
            );
        }
    }

    let mut symbols: Vec<(&str, u64, &str)> = l
        .defs
        .iter()
        .filter_map(|(name, d)| {
            Some((
                name.as_str(),
                l.value(name)?,
                l.modules[d.module].name.as_str(),
            ))
        })
        .collect();
    symbols.sort();
    let _ = writeln!(w, "\nSymbols By Name\n");
    let _ = writeln!(w, "  {:<31}  {:<16}  Module", "Symbol", "Value");
    for (name, value, module) in &symbols {
        let _ = writeln!(w, "  {name:<31}  {value:016X}  {module}");
    }
    symbols.sort_by_key(|s| (s.1, s.0));
    let _ = writeln!(w, "\nSymbols By Value\n");
    let _ = writeln!(w, "  {:<16}  Symbol", "Value");
    for (name, value, _) in &symbols {
        let _ = writeln!(w, "  {value:016X}  {name}");
    }

    let _ = writeln!(w, "\nImage Synopsis\n");
    let _ = writeln!(w, "  Image name        {}", image.name);
    let _ = writeln!(w, "  Image ident       {}", image.ident);
    let at = symbols
        .iter()
        .find(|s| s.1 == image.transfer)
        .map_or(String::new(), |s| format!(" ({})", s.0));
    let _ = writeln!(w, "  Transfer address  {:016X}{at}", image.transfer);
    o
}

fn last(base: u64, size: u64) -> u64 {
    base + size.max(1) - 1
}

/// Attributes as LINK/MAP lists them.
fn attributes(flags: u16) -> String {
    let pairs = [
        (psc::PIC, "PIC", "NOPIC"),
        (psc::OVR, "OVR", "CON"),
        (psc::REL, "REL", "ABS"),
        (psc::GBL, "GBL", "LCL"),
        (psc::SHR, "SHR", "NOSHR"),
        (psc::EXE, "EXE", "NOEXE"),
        (psc::RD, "RD", "NORD"),
        (psc::WRT, "WRT", "NOWRT"),
    ];
    let mut out: Vec<&str> = pairs
        .iter()
        .map(|&(bit, on, off)| if flags & bit != 0 { on } else { off })
        .collect();
    if flags & psc::NOMOD != 0 {
        out.push("NOMOD");
    }
    out.join(",")
}
