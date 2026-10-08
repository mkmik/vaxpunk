//! The require files in crosstools/vtools/lib are what vdefs makes of the macro
//! libraries now: `cargo run -p vdefs` updates them.

use std::fs;
use std::path::Path;

use vdefs::Dialect;

#[test]
fn require_files_are_current() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    for lib in ["lib", "starlet"] {
        let text = fs::read_to_string(dir.join(format!("{lib}.mlb"))).unwrap();
        for (ext, dialect) in [("r64", Dialect::Bliss64), ("req", Dialect::Bliss32)] {
            let want = vdefs::generate(&format!("{lib}.mlb"), &text, dialect).unwrap();
            let have = fs::read_to_string(dir.join(format!("{lib}.{ext}"))).unwrap_or_default();
            assert!(
                have == want,
                "crosstools/vtools/lib/{lib}.{ext} is stale: run `cargo run -p vdefs`"
            );
        }
    }
    let text = fs::read_to_string(dir.join("starlet.mlb")).unwrap();
    let have = fs::read_to_string(dir.join("services.txt")).unwrap_or_default();
    assert!(
        have == vdefs::services("starlet.mlb", &text),
        "crosstools/vtools/lib/services.txt is stale: run `cargo run -p vdefs`"
    );
}
