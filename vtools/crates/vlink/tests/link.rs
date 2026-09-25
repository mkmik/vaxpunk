//! Linking modules that vasm builds: relocation ranges just inside and just
//! outside each limit, and the errors the linker reports.

use vlink::{Linked, Options};
use vms_obj::obj;

fn module(name: &str, source: &str) -> (String, Vec<u8>) {
    let opts = vasm::Options {
        name: name.into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    let records = vasm::assemble(source, &opts).unwrap_or_else(|d| panic!("{d:?}"));
    (format!("{name}.obj"), obj::write(&records))
}

fn link(modules: &[(String, Vec<u8>)], base: u64) -> Result<Linked, Vec<String>> {
    vlink::link(
        modules,
        &Options {
            base,
            name: "TEST".into(),
            transfer: None,
            link_time: 0,
        },
    )
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

    // A 32-bit address works below 4 GB, not above.
    let data = module("D", ".EXTERNAL WORD\n.PSECT $DATA$\n.LONG WORD\n.END");
    assert!(link(&[data.clone(), target.clone()], vlink::DEFAULT_BASE).is_ok());
    let err = link(&[data, target], 0x1_0000_0000).unwrap_err();
    assert!(err[0].contains("doesn't fit in 4 bytes"), "{err:?}");
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
        "/../../tests/run/hello.mar"
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
