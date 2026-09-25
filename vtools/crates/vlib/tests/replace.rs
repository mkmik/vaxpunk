//! What vlib puts into a library, and its symbol index.

use vms_obj::olb::Library;

fn object(name: &str, source: &str) -> Vec<u8> {
    let opts = vasm::Options {
        name: name.into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    vms_obj::obj::write(&vasm::assemble(source, &opts).unwrap())
}

#[test]
fn symbol_index() {
    let mut lib = vlib::new(0);
    let a = object("A", ".WEAK W\n.PSECT $CODE$\nA:: W:: LOCAL: ret\n.END");
    assert_eq!(vlib::replace(&mut lib, "a.obj", &a, 0), Ok(vec![]));
    assert_eq!(
        lib.modules[0].symbols,
        ["A"],
        "strong global definitions only"
    );

    // The first module to define a symbol keeps it.
    let b = object("B", ".PSECT $CODE$\nA:: B:: ret\n.END");
    let warnings = vlib::replace(&mut lib, "b.obj", &b, 0).unwrap();
    assert_eq!(
        warnings,
        [
            "%VLIB-W-DUPGLOBAL, global symbol A from module B is already in the library, from module A"
        ]
    );
    assert_eq!(lib.modules[1].symbols, ["B"]);

    // A module of the same name replaces the old one and its symbols.
    let a2 = object("A", ".PSECT $CODE$\nC:: ret\n.END");
    assert_eq!(vlib::replace(&mut lib, "a2.obj", &a2, 1), Ok(vec![]));
    let lib = Library::parse(&lib.write()).unwrap();
    let modules: Vec<_> = lib
        .modules
        .iter()
        .map(|m| (m.name.as_str(), m.symbols.join(" "), m.inserted))
        .collect();
    assert_eq!(modules, [("A", "C".into(), 1), ("B", "B".into(), 0)]);
}

#[test]
fn bad_objects() {
    let mut lib = vlib::new(0);
    let err = vlib::replace(&mut lib, "x.obj", b"junk", 0).unwrap_err();
    assert!(
        err.starts_with("%VLIB-F-BADOBJ, x.obj: not an object file"),
        "{err}"
    );
    let long = format!("{}:: ret", "L".repeat(129));
    let err = vlib::replace(&mut lib, "l.obj", &object("L", &long), 0).unwrap_err();
    assert!(err.contains("is longer than 128 characters"), "{err}");
}
