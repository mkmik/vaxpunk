//! vdump of what vasm makes of the programs in tests/run, compared with the
//! checked-in dumps next to this file. `UPDATE_GOLDEN=1 cargo test` rewrites
//! them.

use std::path::Path;
use std::{env, fs};

fn check(program: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        fs::read_to_string(root.join("../../tests/run").join(format!("{program}.mar"))).unwrap();
    let records = vasm::assemble(&source, "TEST", *b"25-SEP-2026 00:00").expect("assembles");
    let dump = vdump::dump(&vms_obj::obj::write(&records)).unwrap();
    let path = root.join("tests").join(format!("{program}.obj.txt"));
    if env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &dump).unwrap();
    }
    let golden = fs::read_to_string(&path).unwrap_or_default();
    assert!(dump == golden, "vdump of {program}.obj changed:\n{dump}");
}

#[test]
fn hello() {
    check("hello");
}
