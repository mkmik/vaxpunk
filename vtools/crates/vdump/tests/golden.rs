//! vdump output for hand-built files, compared with the checked-in dumps
//! next to this file. `UPDATE_GOLDEN=1 cargo test` rewrites them.

use std::path::Path;
use std::{env, fs};

use vms_obj::exe::{Eisd, Image, Section};
use vms_obj::obj::{self, Eom, Gsd, Mhd, Psc, Record, SymDef, Tir, Transfer, psc, sym};
use vms_obj::olb::{Library, Module};

fn check(name: &str, file: &[u8]) {
    let dump = vdump::dump(file).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(format!("{name}.txt"));
    if env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &dump).unwrap();
    }
    let golden = fs::read_to_string(&path).unwrap_or_default();
    assert!(dump == golden, "vdump output for {name} changed:\n{dump}");
}

/// mov x1, #6; svc #2; mov x0, #1; ret
const CODE: [u32; 4] = [0xd28000c1, 0xd4000041, 0xd2800020, 0xd65f03c0];

/// The hello program as an assembler would write it: `adr x0, msg` as a
/// relocation to offset 0x14 of $CODE$.
fn hello() -> Vec<Record> {
    let code: Vec<u8> = CODE.iter().flat_map(|i| i.to_le_bytes()).collect();
    vec![
        Record::Mhd(Mhd {
            strlvl: obj::STRLVL,
            temp: 0,
            arch1: vms_obj::ARCH_ARM64,
            arch2: 0,
            recsiz: obj::MAX_RECORD as u32,
            name: "HELLO".into(),
            version: "V1.0".into(),
            date: *b"25-SEP-2026 17:00",
        }),
        Record::Text {
            subtype: obj::LNM,
            text: b"hand-built".to_vec(),
        },
        Record::Gsd(vec![
            Gsd::Psc(Psc {
                align: 2,
                temp: 0,
                flags: psc::PIC | psc::REL | psc::SHR | psc::EXE | psc::RD,
                alloc: 26,
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
        ]),
        Record::Tir(vec![
            Tir::StaPq {
                psect: 0,
                offset: 0,
            },
            Tir::CtlSetrb {},
            Tir::StaPq {
                psect: 0,
                offset: 0x14,
            },
            Tir::StoA64Adr { insn: 0x1000_0000 },
            Tir::StoImm { data: code },
            Tir::StoImm {
                data: b"hello\n".to_vec(),
            },
        ]),
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
fn hello_obj() {
    check("hello.obj", &obj::write(&hello()));
}

/// A library holding the hello module.
#[test]
fn hello_olb() {
    let lib = Library {
        creator: "hand-built".into(),
        created: 0x00a1_b2c3_d4e5_f607,
        updated: 0x00a1_b2c3_d4e5_f608,
        modules: vec![Module {
            name: "HELLO".into(),
            ident: "V1.0".into(),
            inserted: 0x00a1_b2c3_d4e5_f609,
            symbols: vec!["HELLO".into()],
            object: obj::write(&hello()),
        }],
    };
    check("hello.olb", &lib.write());
}

/// The same program linked at 0x10000.
#[test]
fn hello_exe() {
    let mut data: Vec<u8> = [0x100000a0]
        .iter()
        .chain(&CODE)
        .flat_map(|i: &u32| i.to_le_bytes())
        .collect();
    data.extend(b"hello\n");
    let image = Image {
        name: "HELLO".into(),
        ident: "V1.0".into(),
        link_time: 0,
        transfer: 0x10000,
        sections: vec![
            Section {
                vaddr: 0x10000,
                size: data.len() as u32,
                flags: Eisd::M_EXE,
                data,
            },
            Section {
                vaddr: 0x20000,
                size: 0x100,
                flags: Eisd::M_WRT | Eisd::M_DZRO,
                data: vec![],
            },
        ],
    };
    check("hello.exe", &image.write());
}
