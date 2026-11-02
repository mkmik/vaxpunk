//! The listing `/LIST` writes, laid out as AMACRO's `/LIST/MACHINE_CODE`
//! is. First the source: each line with the location counter before it.
//! Then the machine code, a psect at a time: its labels, and its
//! instructions and data in hex, each with the statement it came from and,
//! the first time, the number of the line in the source that made it.

use std::fmt::Write;

use vms_obj::obj::psc;

use crate::Options;
use crate::asm::{Chunk, Module};
use crate::expr::{self, Value};

/// The column the line numbers go in, after the machine code.
const LINE_COL: usize = 124;

pub fn text(source: &str, m: &Module, opts: &Options, tool: &str) -> String {
    // VMS dates pad the day with a space.
    let date = crate::date_time();
    let date = match date.strip_prefix('0') {
        Some(d) => format!(" {d}"),
        None => date,
    };
    let name = m.name.as_deref().unwrap_or(&opts.name);
    let ident = m.version.as_deref().unwrap_or("");
    let file = opts
        .path
        .as_ref()
        .map_or("<source>".into(), |p| p.display().to_string());
    let mut out = String::new();
    let mut page = 0;
    // ponytail: a page per part, not per 66 lines; break pages if a
    // printer ever reads these.
    let mut header = |out: &mut String, title: &str| {
        if page > 0 {
            out.push('\n');
        }
        page += 1;
        let _ = write!(
            out,
            "{name:<32}{title:<32}{date}  {tool:<34}Page {page}\n{ident:<32}{:<32}{date}  {file}\n\n",
            ""
        );
    };

    header(&mut out, &m.title);
    let mut at = 0;
    let mut offsets = m.offsets.iter().peekable();
    for (i, line) in source.lines().enumerate() {
        // A line not read on its own, as a macro definition's, is where
        // the one before it left the location counter.
        if let Some((_, o)) = offsets.next_if(|(n, _)| *n == i + 1) {
            at = *o;
        }
        let _ = writeln!(out, "{:22}{at:08X}\t{:>7} {line}", "", i + 1);
    }

    header(&mut out, "Machine Code Listing");
    for (i, p) in m.psects.iter().enumerate() {
        let mut labels: Vec<(u64, String)> = m
            .symbols
            .iter()
            .filter_map(|(name, s)| match &s.value {
                Some(Value::Psect { psect, offset }) if *psect == i && s.label => Some((
                    *offset as u64,
                    format!(
                        "{}:{}",
                        expr::display(name),
                        if s.global { ":" } else { "" }
                    ),
                )),
                _ => None,
            })
            .collect();
        let mut rows: Vec<_> = m.rows.iter().filter(|r| r.psect == i).collect();
        if labels.is_empty() && rows.is_empty() {
            continue;
        }
        labels.sort_by_key(|l| l.0);
        rows.sort_by_key(|r| r.offset);
        let _ = writeln!(
            out,
            "\n\t\t\t\t.PSECT\t{}, {}",
            p.name,
            attributes(p.flags, p.align)
        );
        // The psect's contents, with zeros where the linker stores a value.
        let mut image = vec![0u8; p.size as usize];
        for (at, chunk) in &p.chunks {
            let at = *at as usize;
            match chunk {
                Chunk::Bytes(b) => image[at..at + b.len()].copy_from_slice(b),
                Chunk::Insn { word, .. } => image[at..at + 4].copy_from_slice(&word.to_le_bytes()),
                Chunk::Data { .. } => {}
            }
        }
        let mut labels = labels.into_iter().peekable();
        let mut last = 0;
        for r in rows {
            while let Some((at, label)) = labels.next_if(|l| l.0 <= r.offset) {
                let _ = writeln!(out, "\t     {at:04X}\t{label}");
            }
            let end = r.offset + r.size;
            for at in (r.offset..end).step_by(usize::from(r.unit)) {
                let bytes = &image[at as usize..end.min(at + u64::from(r.unit)) as usize];
                let hex: String = bytes.iter().rev().map(|b| format!("{b:02X}")).collect();
                let mut line = format!("{hex:>8}     {at:04X}");
                if at == r.offset {
                    line = format!("{line}\t\t{}", r.text);
                    if r.line != last {
                        last = r.line;
                        // Tabs up to the column, then spaces.
                        while width(&line) + 8 <= LINE_COL {
                            line.push('\t');
                        }
                        let pad = LINE_COL.saturating_sub(width(&line)).max(1);
                        line = format!("{line}{:pad$}; {:06}", "", r.line);
                    }
                }
                let _ = writeln!(out, "{}", line.trim_end());
            }
        }
        for (at, label) in labels {
            let _ = writeln!(out, "\t     {at:04X}\t{label}");
        }
    }
    out
}

/// A psect's alignment and attributes, as `.PSECT` takes them.
fn attributes(flags: u16, align: u8) -> String {
    let align = match align {
        0 => "BYTE".into(),
        1 => "WORD".into(),
        2 => "LONG".into(),
        3 => "QUAD".into(),
        4 => "OCTA".into(),
        16 => "PAGE".into(),
        n => n.to_string(),
    };
    let pick =
        |bit: u16, on: &'static str, off: &'static str| if flags & bit != 0 { on } else { off };
    [
        align.as_str(),
        pick(psc::PIC, "PIC", "NOPIC"),
        pick(psc::OVR, "OVR", "CON"),
        pick(psc::REL, "REL", "ABS"),
        pick(psc::GBL, "GBL", "LCL"),
        pick(psc::SHR, "SHR", "NOSHR"),
        pick(psc::EXE, "EXE", "NOEXE"),
        pick(psc::RD, "RD", "NORD"),
        pick(psc::WRT, "WRT", "NOWRT"),
    ]
    .join(", ")
}

/// How wide `s` prints, with tab stops every 8 columns.
fn width(s: &str) -> usize {
    s.chars()
        .fold(0, |w, c| if c == '\t' { (w / 8 + 1) * 8 } else { w + 1 })
}
