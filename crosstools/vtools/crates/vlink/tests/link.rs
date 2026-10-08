//! Linking modules that vasm builds: relocation ranges just inside and just
//! outside each limit, and the errors the linker reports. Every link that
//! makes a movable image is also linked at another base, and the two must
//! differ exactly where the fixups say.

use vlink::{Linked, Options};
use vms_obj::exe::Image;
use vms_obj::obj::{self, Eom, Gsd, Mhd, Psc, Record, SymDef, Tir, psc, sym};

fn module(name: &str, source: &str) -> (String, Vec<u8>) {
    let opts = vasm::Options {
        name: name.into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    let records = vasm::assemble(source, &opts)
        .unwrap_or_else(|d| panic!("{d:?}"))
        .records;
    (format!("{name}.obj"), obj::write(&records))
}

/// A module made by hand: 16 bytes of $DATA$ with TABLE at the start, 4 of
/// $CODE$, and `tir`, which stores from TABLE on.
fn handmade(name: &str, tir: Vec<Tir>) -> (String, Vec<u8>) {
    let psect = |name: &str, flags| {
        Gsd::Psc(Psc {
            align: 3,
            temp: 0,
            flags: flags | psc::REL | psc::RD,
            alloc: 16,
            name: name.into(),
        })
    };
    let table = Gsd::Def(SymDef {
        datyp: 0,
        temp: 0,
        flags: sym::DEF | sym::REL,
        value: 0,
        code_address: 0,
        ca_psindx: 0,
        psindx: 0,
        name: "TABLE".into(),
    });
    let start = [
        Tir::StaPq {
            psect: 0,
            offset: 0,
        },
        Tir::CtlSetrb {},
    ];
    let records = [
        Record::Mhd(Mhd {
            strlvl: obj::STRLVL,
            temp: 0,
            arch1: vms_obj::ARCH_ARM64,
            arch2: 0,
            recsiz: obj::MAX_RECORD as u32,
            name: name.into(),
            version: String::new(),
            date: *b"25-SEP-2026 00:00",
        }),
        Record::Gsd(vec![
            psect("$DATA$", psc::WRT),
            psect("$CODE$", psc::PIC | psc::SHR | psc::EXE),
            table,
        ]),
        Record::Tir(start.into_iter().chain(tir).collect()),
        Record::Eom(
            Eom {
                total_lps: 0,
                comcod: 0,
            },
            None,
        ),
    ];
    (format!("{name}.obj"), obj::write(&records))
}

fn link_as(
    modules: &[(String, Vec<u8>)],
    base: u64,
    relocatable: bool,
) -> Result<Linked, Vec<String>> {
    vlink::link(
        modules,
        &Options {
            base,
            name: "TEST".into(),
            transfer: None,
            link_time: 0,
            relocatable,
        },
    )
}

/// Links without /RELOCATABLE. If the modules can also link /RELOCATABLE,
/// that image must be the same plus fixups, and must differ from itself
/// linked elsewhere exactly at them.
fn link(modules: &[(String, Vec<u8>)], base: u64) -> Result<Linked, Vec<String>> {
    let linked = link_as(modules, base, false)?;
    if let Ok(movable) = link_as(modules, base, true) {
        let image = &movable.image;
        let plain = Image {
            fixups: None,
            ..image.clone()
        };
        assert_eq!(plain, linked.image, "/RELOCATABLE changed the image");
        // Some bits flipped, above 4 GB unless longword addresses must stay
        // below 2 GB.
        let long = !image.fixups.as_ref().unwrap().long.is_empty();
        let other = base ^ if long { 0x0123_0000 } else { 0x1234_5678_0000 };
        let moved = link_as(modules, other, true).unwrap();
        vlink::check_fixups(image, &moved.image).unwrap_or_else(|e| panic!("{e}"));
    }
    Ok(linked)
}

/// Links `insn` in one module against FAR in another, `distance` bytes after
/// the instruction, and returns the patched instruction.
fn reach(insn: &str, distance: u64) -> Result<u32, String> {
    let main = module(
        "MAIN",
        &format!(".EXTERNAL FAR\n.PSECT $CODE$\nSTART:: {insn}\n.END START"),
    );
    // MAIN contributes 4 bytes to $CODE$; FAR's module starts 8 bytes in.
    let far = module(
        "FAR",
        &format!(".PSECT $CODE$\n.BLKB {}\nFAR:: ret\n.END", distance - 8),
    );
    let linked = link(&[main, far], vlink::DEFAULT_BASE).map_err(|e| e.join("\n"))?;
    Ok(u32::from_le_bytes(
        linked.image.sections[0].data[..4].try_into().unwrap(),
    ))
}

/// A signed field of `bits` bits at `shift`, in instructions (×4).
fn field(word: u32, shift: u32, bits: u32) -> i64 {
    let v = (word >> shift) & ((1 << bits) - 1);
    ((v << (32 - bits)) as i32 >> (32 - bits)) as i64 * 4
}

/// An instruction, its reach in bytes, and how to read its displacement.
type Case = (&'static str, u64, fn(u32) -> i64);

#[test]
fn branch_ranges() {
    let cases: [Case; 6] = [
        ("tbz x0, #0, FAR", 1 << 15, |w| field(w, 5, 14)),
        ("b.eq FAR", 1 << 20, |w| field(w, 5, 19)),
        ("cbz x0, FAR", 1 << 20, |w| field(w, 5, 19)),
        ("ldr x0, FAR", 1 << 20, |w| field(w, 5, 19)),
        ("adr x0, FAR", 1 << 20, |w| {
            field(w, 5, 19) + i64::from(w >> 29 & 3)
        }),
        ("bl FAR", 1 << 27, |w| field(w, 0, 26)),
    ];
    for (insn, limit, decode) in cases {
        let word = reach(insn, limit - 4).unwrap_or_else(|e| panic!("{insn} just in range: {e}"));
        assert_eq!(decode(word), (limit - 4) as i64, "{insn}");
        let err = reach(insn, limit).expect_err(insn);
        assert!(
            err.contains("out of range") && err.contains("module MAIN"),
            "{insn}: {err}"
        );
    }
}

/// A BL that can't reach a fixed address, as an image's call to the
/// executive, goes through a veneer at the end of the code: `ldr x16, 8;
/// br x16` and the address. Each address gets one veneer.
#[test]
fn veneers() {
    let main = module(
        "MAIN",
        ".EXTERNAL SYS\n.PSECT $CODE$\nSTART:: bl SYS\nbl SYS\nb SYS\nret\n.END START",
    );
    let sys = module("SYS", "SYS == 0x40037618\n.END");
    let linked = link(&[main, sys], vlink::DEFAULT_BASE).unwrap();
    let code = &linked.image.sections[0];
    let word = |i: usize| u32::from_le_bytes(code.data[4 * i..4 * i + 4].try_into().unwrap());
    let veneer = (field(word(0), 0, 26)) as usize;
    assert_eq!(field(word(1), 0, 26), veneer as i64 - 4);
    assert_eq!(field(word(2), 0, 26), veneer as i64 - 8);
    assert_eq!(code.data.len(), veneer + 16);
    assert_eq!(word(veneer / 4), 0x5800_0050);
    assert_eq!(word(veneer / 4 + 1), 0xd61f_0200);
    assert_eq!(code.data[veneer + 8..], 0x4003_7618u64.to_le_bytes());
    assert!(linked.map.contains("$VENEER$"), "{}", linked.map);
}

#[test]
fn value_checks() {
    let target = module(
        "T",
        ".PSECT $DATA$\n.BLKB 4\nODD:: .BLKB 4\nWORD:: .QUAD 0\n.END",
    );
    let main = |text: &str| {
        module(
            "MAIN",
            &format!(".EXTERNAL ODD, WORD\n.PSECT $CODE$\nSTART:: {text}\n.END START"),
        )
    };

    let err = link(
        &[main("ldr x0, [x0, #:lo12:ODD]"), target.clone()],
        vlink::DEFAULT_BASE,
    )
    .unwrap_err();
    assert!(err[0].contains("not aligned to the access size"), "{err:?}");
    assert!(
        link(
            &[main("ldr w0, [x0, #:lo12:ODD]"), target.clone()],
            vlink::DEFAULT_BASE
        )
        .is_ok()
    );

    // The image is above 0x10000, so the low chunk alone can't hold it.
    let err = link(
        &[main("movz x0, #:abs_g0:WORD"), target.clone()],
        vlink::DEFAULT_BASE,
    )
    .unwrap_err();
    assert!(
        err[0].contains("too large for this MOVZ/MOVK chunk"),
        "{err:?}"
    );
    assert!(
        link(
            &[main("movz x0, #:abs_g1:WORD"), target.clone()],
            vlink::DEFAULT_BASE
        )
        .is_ok()
    );

    // A 32-bit address works below 2 GB, where sign-extending it on a load
    // gives the same address, not above.
    let data = module("D", ".EXTERNAL WORD\n.PSECT $DATA$\n.LONG WORD\n.END");
    assert!(link(&[data.clone(), target.clone()], 0x7ffe_0000).is_ok());
    let err = link(&[data, target], 0x8000_0000).unwrap_err();
    assert_eq!(
        err,
        [
            "%VLINK-E-TRUNC, address %X80000010 doesn't fit in 4 bytes, signed, at \
             $DATA$ + %X0 in module D (D.obj)"
        ]
    );
}

/// The link-failure tests: each store the image can't move with, and how
/// the linker says so.
#[test]
fn unmovable() {
    let fails = |modules: &[(String, Vec<u8>)], base| link_as(modules, base, true).unwrap_err();

    let code = [module(
        "MAIN",
        ".PSECT $CODE$\nSTART:: ret\nTABLE:: .ADDRESS START\n.END START",
    )];
    let at = "$CODE$ + %X4 (TABLE) in module MAIN (MAIN.obj)";
    let warnings = link(&code, vlink::DEFAULT_BASE).unwrap().warnings;
    let msg = format!("address in a PIC psect needs a fixup, at {at}");
    assert_eq!(warnings, [format!("%VLINK-W-NOTPIC, {msg}")]);
    let err = fails(&code, vlink::DEFAULT_BASE);
    assert_eq!(err, [format!("%VLINK-E-NOTPIC, {msg}")]);

    let movz = [module(
        "MAIN",
        ".PSECT $CODE$\nSTART:: movz x0, #:abs_g1:START\nret\n.END START",
    )];
    assert!(link(&movz, vlink::DEFAULT_BASE).is_ok());
    assert_eq!(
        fails(&movz, vlink::DEFAULT_BASE),
        [
            "%VLINK-E-NORELOC, STO_A64_MOVW_G1 of an address can't move, at \
             $CODE$ + %X0 (START) in module MAIN (MAIN.obj)"
        ]
    );

    // Linked at 0, so that the address fits in a word.
    let word = [module(
        "MAIN",
        ".PSECT $DATA$\nWORDS:: .WORD 0, WORDS\n.END",
    )];
    assert!(link(&word, 0).is_ok());
    assert_eq!(
        fails(&word, 0),
        ["%VLINK-E-NORELOC, STO_W of an address can't move, at \
             $DATA$ + %X2 (WORDS+%X2) in module MAIN (MAIN.obj)"]
    );

    // TABLE >> 12: the count goes first.
    let shift = [handmade(
        "SHIFT",
        vec![
            Tir::StaLw {
                value: -12i32 as u32,
            },
            Tir::StaPq {
                psect: 0,
                offset: 0,
            },
            Tir::OprAsh {},
            Tir::StoQw {},
        ],
    )];
    assert!(link(&shift, vlink::DEFAULT_BASE).is_ok());
    assert_eq!(
        fails(&shift, vlink::DEFAULT_BASE),
        [
            "%VLINK-E-NORELOC, STO_QW of a value computed from an address can't move, at \
             $DATA$ + %X0 (TABLE) in module SHIFT (SHIFT.obj)"
        ]
    );
}

/// Stores the assembler can't write, with what the linker makes of them.
#[test]
fn stores_by_hand() {
    let fixups = |tir: Vec<Tir>| {
        let linked = link_as(&[handmade("M", tir)], vlink::DEFAULT_BASE, true)?;
        let f = linked.image.fixups.unwrap();
        Ok::<_, Vec<String>>((f.quad, f.long))
    };
    let here = || Tir::StaPq {
        psect: 0,
        offset: 0,
    };
    let code = || Tir::StaPq {
        psect: 1,
        offset: 0,
    };
    let back = || [here(), Tir::CtlSetrb {}];

    // CODE - . is the same wherever the image goes.
    let relative = vec![code(), here(), Tir::OprSub {}, Tir::StoLw {}];
    assert!(link(&[handmade("M", relative.clone())], vlink::DEFAULT_BASE).is_ok());
    assert_eq!(fixups(relative), Ok((vec![], vec![])));

    // An address that a constant replaces needs no fixup.
    let mut replaced = vec![code(), Tir::StoOff {}];
    replaced.extend(back());
    replaced.extend([Tir::StaQw { value: 5 }, Tir::StoQw {}]);
    assert!(link(&[handmade("M", replaced.clone())], vlink::DEFAULT_BASE).is_ok());
    assert_eq!(fixups(replaced), Ok((vec![], vec![])));

    // But half an address is neither, until the other half goes too.
    let mut halved = vec![code(), Tir::StoOff {}];
    halved.extend(back());
    halved.extend([Tir::StaLw { value: 0 }, Tir::StoLw {}]);
    let mut both = halved.clone();
    both.extend([Tir::StaLw { value: 0 }, Tir::StoLw {}]);
    assert_eq!(
        fixups(halved),
        Err(vec![
            "%VLINK-E-NORELOC, part of an address is overwritten, at \
             $DATA$ + %X0 (TABLE) in module M (M.obj)"
                .to_string()
        ])
    );
    assert_eq!(fixups(both), Ok((vec![], vec![])));

    // Neither an address in PIC code nor a value that can't move is a
    // problem once a constant replaces it.
    let in_code = [code(), Tir::CtlSetrb {}];
    let mut replaced = in_code.to_vec();
    replaced.extend([code(), Tir::StoOff {}]);
    replaced.extend(in_code.clone());
    replaced.extend([Tir::StaQw { value: 5 }, Tir::StoQw {}]);
    assert_eq!(fixups(replaced), Ok((vec![], vec![])));
    let count = Tir::StaLw {
        value: -12i32 as u32,
    };
    let mut shifted = vec![count, here(), Tir::OprAsh {}, Tir::StoQw {}];
    shifted.extend(back());
    shifted.extend([Tir::StaQw { value: 5 }, Tir::StoQw {}]);
    assert_eq!(fixups(shifted), Ok((vec![], vec![])));

    // Offsets count from the first image section, here the code.
    let two = vec![code(), Tir::StoOff {}, code(), Tir::StoLw {}];
    assert_eq!(fixups(two), Ok((vec![0x10000], vec![0x10008])));
}

/// Every module's labels in an absolute psect are the offsets it assembled,
/// to itself and to other modules alike.
#[test]
fn absolute_psects() {
    let a = module("A", ".PSECT OFFS, ABS\n.BLKQ 2\n.END");
    let b = module(
        "B",
        ".PSECT OFFS, ABS\n.BLKQ 1\nFIELD:: .BLKQ 1\n.PSECT $DATA$\n.QUAD FIELD\n.END",
    );
    let c = module("C", ".EXTERNAL FIELD\n.PSECT $DATA$\n.QUAD FIELD\n.END");
    let linked = link(&[a, b, c], vlink::DEFAULT_BASE).unwrap();
    let data = &linked.image.sections[0].data;
    assert_eq!(data[..16], [8, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn map_lists_fixups() {
    let main = module(
        "MAIN",
        ".PSECT $DATA$\nPOINTERS:: .ADDRESS START\n.LONG START\n\
         .PSECT $CODE$\nSTART:: ret\n.END START",
    );
    let linked = link_as(&[main], vlink::DEFAULT_BASE, true).unwrap();
    assert_eq!(
        linked.warnings,
        ["%VLINK-I-FIXUPS, 1 quadword and 1 longword fixups"]
    );
    let map = &linked.map;
    for line in [
        "  0000000000020000  quadword  MAIN                             $DATA$ + %X0 (POINTERS)",
        "  0000000000020008  longword  MAIN                             $DATA$ + %X8 (POINTERS+%X8)",
    ] {
        assert!(map.contains(line), "{map}");
    }
}

#[test]
fn symbol_errors() {
    let a = module(
        "A",
        ".EXTERNAL MISSING\n.PSECT $CODE$\nSTART:: bl MISSING\nDUP:: ret\n.END START",
    );
    let b = module("B", ".PSECT $CODE$\nDUP:: ret\n.END");
    let err = link(&[a, b], vlink::DEFAULT_BASE).unwrap_err();
    assert!(
        err.iter()
            .any(|e| e.contains("symbol DUP is defined in modules A and B")),
        "{err:?}"
    );
    assert!(
        err.iter()
            .any(|e| e.contains("undefined symbol MISSING, referenced by A")),
        "{err:?}"
    );

    // A weak definition gives way to a strong one.
    let weak = module("W", ".WEAK SYM\n.PSECT $CODE$\nSYM:: ret\n.END");
    let strong = module("S", ".PSECT $CODE$\nSYM:: nop\nret\n.END");
    let linked = link(&[weak, strong], vlink::DEFAULT_BASE).unwrap();
    assert!(
        linked
            .map
            .contains("SYM                              0000000000010008  S"),
        "{}",
        linked.map
    );
}

/// An object library holding `modules`.
fn library(modules: &[(String, Vec<u8>)]) -> (String, Vec<u8>) {
    let mut lib = vlib::new(0);
    for (file, bytes) in modules {
        assert_eq!(vlib::replace(&mut lib, file, bytes, 0), Ok(vec![]));
    }
    ("LIB.OLB".into(), lib.write())
}

/// The modules in a map's object module synopsis.
fn linked_modules(map: &str) -> Vec<&str> {
    map.lines()
        .skip(3)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .collect()
}

#[test]
fn library_search() {
    let lib = library(&[
        // NUM needs CH, from the same library.
        module("NUM", ".EXTERNAL CH\n.PSECT $CODE$\nNUM:: bl CH\nret\n.END"),
        module("CH", ".PSECT $CODE$\nCH:: ret\n.END"),
        // Nothing needs UNUSED; taking it would define START twice.
        module("UNUSED", ".PSECT $CODE$\nSTART:: ret\n.END"),
        // A weak reference alone doesn't take a module.
        module("OPTION", ".PSECT $CODE$\nOPTION:: ret\n.END"),
    ]);
    let main = module(
        "MAIN",
        ".EXTERNAL NUM\n.WEAK OPTION\n.PSECT $CODE$\nSTART:: bl NUM\n.QUAD OPTION\n.END START",
    );
    let linked = link(&[main.clone(), lib.clone()], vlink::DEFAULT_BASE).unwrap();
    assert_eq!(linked_modules(&linked.map), ["MAIN", "NUM", "CH"]);
    let option = &linked.image.sections[0].data[4..12];
    assert_eq!(option, [0; 8], "OPTION stays undefined, so 0");

    // A library only serves the modules before it.
    let err = link(&[lib, main], vlink::DEFAULT_BASE).unwrap_err();
    assert!(
        err.iter()
            .any(|e| e.contains("undefined symbol NUM, referenced by MAIN")),
        "{err:?}"
    );
}

#[test]
fn bases_and_layout() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/vasm/hello.mar"
    ))
    .unwrap();
    for base in [0x10000, 0x4000_0000_0000] {
        let linked = link(&[module("HELLO", &source)], base).unwrap();
        let image = &linked.image;
        assert_eq!(image.transfer, base, "transfer address at {base:#x}");
        assert_eq!(image.sections.len(), 2, "code and read-only data");
        assert_eq!(image.sections[0].vaddr, base);
        assert_eq!(
            image.sections[1].vaddr,
            base + 0x10000,
            "sections are 64 KB aligned"
        );
        // adr x0, message: 64 KB ahead, the same at every base.
        let adr = u32::from_le_bytes(image.sections[0].data[..4].try_into().unwrap());
        assert_eq!(field(adr, 5, 19) + i64::from(adr >> 29 & 3), 0x10000);
    }
}
