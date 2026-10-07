//! The IR vbliss prints for the programs in tests/bliss that have a
//! NAME.ir next to them: the contract stage 1 must print too (PRD-0004).

use std::fs;
use std::path::Path;

#[test]
fn ir() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/bliss");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "ir") {
            continue;
        }
        let source = fs::read_to_string(path.with_extension("b64")).unwrap();
        let out = vbliss::translate(&source, &dir).unwrap();
        assert_eq!(
            out.ir,
            fs::read_to_string(&path).unwrap(),
            "{}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0);
}
