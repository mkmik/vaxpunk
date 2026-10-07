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
        let source = path.with_extension("b64");
        let text = fs::read_to_string(&source).unwrap();
        let opts = vbliss::Options {
            include: vec![dir.join("../../lib")],
            ..vbliss::Options::default()
        };
        let out = vbliss::translate(&source, &text, &opts);
        assert_eq!(
            out.ir.as_deref(),
            Some(fs::read_to_string(&path).unwrap().as_str()),
            "{}: {:?}",
            path.display(),
            out.error()
        );
        checked += 1;
    }
    assert!(checked > 0);
}
