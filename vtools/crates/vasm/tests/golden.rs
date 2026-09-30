//! vdump of what vasm makes of the programs in tests/run, and of
//! expressions.mar here, compared with the checked-in dumps next to this
//! file. `UPDATE_GOLDEN=1 cargo test` rewrites them.

use std::path::Path;
use std::{env, fs};

/// Checks `source`, relative to vtools, against `NAME.obj.txt`.
fn check(source: &str, weights: bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("../..").join(source);
    let opts = vasm::Options {
        name: "TEST".into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    let text = fs::read_to_string(&source).unwrap();
    let records = vasm::assemble(&text, &opts).expect("assembles").records;
    let opts = vdump::Options {
        weights,
        ..Default::default()
    };
    let dump = vdump::dump(&vms_obj::obj::write(&records), &opts).unwrap();
    let name = source.file_stem().unwrap().to_string_lossy();
    let path = root.join("tests").join(format!("{name}.obj.txt"));
    if env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &dump).unwrap();
    }
    let golden = fs::read_to_string(&path).unwrap_or_default();
    assert!(dump == golden, "vdump of {name}.obj changed:\n{dump}");
}

#[test]
fn hello() {
    check("examples/vasm/hello.mar", false);
}

/// Every form an address takes, with the weight the linker will give each
/// stored value, so that no later change can quietly fold an address into
/// a constant. Also: which symbols are addresses (`REL`).
#[test]
fn expressions() {
    check("crates/vasm/tests/expressions.mar", true);
}
