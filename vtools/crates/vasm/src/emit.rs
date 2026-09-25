//! Object records for an assembled module.

use vms_obj::ARCH_ARM64;
use vms_obj::obj::{self, Eom, Gsd, Mhd, Psc, Record, SymDef, SymRef, Tir, Transfer, psc, sym};

use crate::asm::{Chunk, Module};
use crate::encode::Fix;
use crate::expr::Value;

/// Payload room in one record, below the record size limit.
const ROOM: usize = obj::MAX_RECORD - 64;
/// Largest STO_IMM, so a command always fits in a record.
const MAX_IMM: usize = 4096;

/// The records of `m`. `name` is used if the source had no .TITLE.
pub fn records(m: &Module, name: &str, date: [u8; 17], tool: &str) -> Vec<Record> {
    let mut out = vec![
        Record::Mhd(Mhd {
            strlvl: obj::STRLVL,
            temp: 0,
            arch1: ARCH_ARM64,
            arch2: 0,
            recsiz: obj::MAX_RECORD as u32,
            name: m.name.clone().unwrap_or_else(|| name.to_string()),
            version: m.version.clone().unwrap_or_default(),
            date,
        }),
        Record::Text {
            subtype: obj::LNM,
            text: tool.as_bytes().to_vec(),
        },
    ];

    let mut gsd: Vec<Gsd> = m
        .psects
        .iter()
        .map(|p| {
            Gsd::Psc(Psc {
                align: p.align,
                temp: 0,
                flags: p.flags,
                alloc: p.size as u32,
                name: p.name.clone(),
            })
        })
        .collect();
    let mut abs = None;
    for (name, s) in &m.symbols {
        let weak = if s.weak { sym::WEAK } else { 0 };
        if s.external {
            gsd.push(Gsd::Ref(SymRef {
                datyp: 0,
                temp: 0,
                flags: weak,
                name: name.clone(),
            }));
            continue;
        }
        if !(s.global || s.weak) {
            continue;
        }
        let (flags, value, psindx) = match &s.value {
            Some(Value::Psect { psect, offset }) => {
                (sym::DEF | sym::REL | weak, *offset, *psect as u32)
            }
            Some(Value::Abs(n)) => {
                // Constants belong to the absolute psect.
                let psindx = *abs.get_or_insert_with(|| {
                    gsd.push(Gsd::Psc(Psc {
                        align: 0,
                        temp: 0,
                        flags: psc::SHR,
                        alloc: 0,
                        name: "$ABS$".into(),
                    }));
                    m.psects.len() as u32
                });
                (sym::DEF | weak, *n, psindx)
            }
            _ => continue,
        };
        let quad = if i32::try_from(value).is_err() {
            sym::QUAD_VAL
        } else {
            0
        };
        gsd.push(Gsd::Def(SymDef {
            datyp: 0,
            temp: 0,
            flags: flags | quad,
            value: value as u64,
            code_address: 0,
            ca_psindx: 0,
            psindx,
            name: name.clone(),
        }));
    }
    out.extend(split(gsd, gsd_size).into_iter().map(Record::Gsd));

    let mut cmds = Vec::new();
    for (i, p) in m.psects.iter().enumerate() {
        if p.chunks.is_empty() {
            continue;
        }
        cmds.push(Tir::StaPq {
            psect: i as u32,
            offset: 0,
        });
        cmds.push(Tir::CtlSetrb {});
        let mut loc = 0;
        for (offset, chunk) in &p.chunks {
            // Skip gaps; the linker zero-fills them.
            let mut gap = offset - loc;
            while gap > 0 {
                let step = gap.min(i32::MAX as u64);
                cmds.push(Tir::CtlAugrb {
                    offset: step as u32,
                });
                gap -= step;
            }
            loc = *offset;
            match chunk {
                Chunk::Bytes(b) => {
                    cmds.extend(b.chunks(MAX_IMM).map(|d| Tir::StoImm { data: d.to_vec() }));
                    loc += b.len() as u64;
                }
                Chunk::Insn { fix, word, target } => {
                    push(&mut cmds, target);
                    cmds.push(store(*fix, *word));
                    loc += 4;
                }
                Chunk::Data { size, value } => {
                    push(&mut cmds, value);
                    cmds.push(match (size, value) {
                        (1, _) => Tir::StoB {},
                        (2, _) => Tir::StoW {},
                        (4, _) => Tir::StoLw {},
                        (_, Value::Psect { .. }) => Tir::StoOff {},
                        _ => Tir::StoQw {},
                    });
                    loc += u64::from(*size);
                }
            }
        }
    }
    out.extend(split(cmds, Tir::size).into_iter().map(Record::Tir));

    let transfer = m.transfer.map(|(psect, offset)| Transfer {
        tfrflg: 0,
        temp: 0,
        psindx: psect as u32,
        tfradr: offset,
    });
    out.push(Record::Eom(
        Eom {
            total_lps: 0,
            comcod: 0,
        },
        transfer,
    ));
    out
}

/// Pushes a value on the linker's stack.
fn push(cmds: &mut Vec<Tir>, v: &Value) {
    match v {
        Value::Abs(n) => cmds.push(Tir::StaQw { value: *n as u64 }),
        Value::Psect { psect, offset } => cmds.push(Tir::StaPq {
            psect: *psect as u32,
            offset: *offset as u64,
        }),
        Value::Ext { name, offset } => {
            cmds.push(Tir::StaGbl { name: name.clone() });
            if *offset != 0 {
                cmds.push(Tir::StaQw {
                    value: *offset as u64,
                });
                cmds.push(Tir::OprAdd {});
            }
        }
    }
}

fn store(fix: Fix, insn: u32) -> Tir {
    match fix {
        Fix::Jump26 => Tir::StoA64Jump26 { insn },
        Fix::Branch19 => Tir::StoA64Branch19 { insn },
        Fix::Branch14 => Tir::StoA64Branch14 { insn },
        Fix::Adr => Tir::StoA64Adr { insn },
        Fix::Adrp => Tir::StoA64Adrp { insn },
        Fix::AddLo12 => Tir::StoA64AddLo12 { insn },
        Fix::Ldst(0) => Tir::StoA64Ldst8Lo12 { insn },
        Fix::Ldst(1) => Tir::StoA64Ldst16Lo12 { insn },
        Fix::Ldst(2) => Tir::StoA64Ldst32Lo12 { insn },
        Fix::Ldst(3) => Tir::StoA64Ldst64Lo12 { insn },
        Fix::Ldst(_) => Tir::StoA64Ldst128Lo12 { insn },
        Fix::Movw(0, true) => Tir::StoA64MovwG0 { insn },
        Fix::Movw(0, false) => Tir::StoA64MovwG0Nc { insn },
        Fix::Movw(1, true) => Tir::StoA64MovwG1 { insn },
        Fix::Movw(1, false) => Tir::StoA64MovwG1Nc { insn },
        Fix::Movw(2, true) => Tir::StoA64MovwG2 { insn },
        Fix::Movw(2, false) => Tir::StoA64MovwG2Nc { insn },
        Fix::Movw(_, _) => Tir::StoA64MovwG3 { insn },
    }
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
