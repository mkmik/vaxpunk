//! Reads arbitrary bytes as a relative or indexed file along every key and
//! checks it: bad data must come back as an error or a finding, never a
//! panic. Input that parses as FDL describing an indexed file is loaded
//! with a few records, and what the loader makes must read back and check
//! clean.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ods_core::RecordAttrs;
use ods_core::rms::{self, fdl};

fuzz_target!(|data: &[u8]| {
    // The first byte stands for what the file header would say.
    let Some((&sel, file)) = data.split_first() else { return };
    let attrs = RecordAttrs {
        rtype: [0x10, 0x20][sel as usize & 1] | [1, 2, 3][(sel >> 1) as usize % 3],
        maxrec: [0, 40, 100, 7][(sel >> 3) as usize & 3],
        bktsize: (sel >> 5) + 1,
        ..RecordAttrs::default()
    };
    if let Ok(f) = rms::open(file, &attrs) {
        for key in 0..f.keys().min(8) {
            let _ = f.records(key);
        }
    }
    let _ = rms::check(file, &attrs);

    let Ok(text) = std::str::from_utf8(data) else { return };
    let Ok(f) = fdl::parse(text) else { return };
    if f.org != 2 || f.spec.areas.iter().any(|a| a.alloc > 10_000) {
        return;
    }
    let size = if f.spec.mrs > 0 { f.spec.mrs as usize } else { 60 };
    let recs: Vec<Vec<u8>> =
        (0..40u8).map(|i| (0..size).map(|j| b'A' + (i.wrapping_mul(7) ^ j as u8) % 5).collect()).collect();
    let Ok(bytes) = rms::build(&f.spec, &recs) else { return };
    let attrs = f.spec.record_attrs((bytes.len() / 512) as u32);
    let file = rms::open(&bytes, &attrs).expect("the loader's file opens");
    for key in 0..file.keys() {
        let n = file.records(key).expect("the loader's file reads").len();
        assert!(n <= recs.len());
    }
    let r = file.check();
    assert!(r.findings.is_empty(), "{:?}", r.findings);
});
