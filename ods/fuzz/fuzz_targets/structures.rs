//! Parses one block as each on-disk structure, and the input as a file
//! specification. Nothing may panic, and whatever parses must serialize
//! back to the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ods_core::layout::{Block, DirBlock, Header, HomeBlock, RecordAttrs, Scb, decode_map, encode_map};
use ods_core::{Level, name};

fuzz_target!(|data: &[u8]| {
    let mut b: Block = [0; 512];
    let n = data.len().min(512);
    b[..n].copy_from_slice(&data[..n]);

    let _ = HomeBlock(b).invalid();
    let _ = Scb(b).checksum_ok();

    let h = Header(b);
    let _ = h.invalid();
    let _ = h.is_deleted();
    let _ = h.highwater_mark();
    let _ = h.acl_area();
    if let Some(id) = h.ident() {
        let mut c = h;
        c.set_ident(&id);
    }
    let ra = h.recattr();
    assert_eq!(RecordAttrs::from_bytes(&ra).to_bytes(), ra);
    if let Ok(ptrs) = decode_map(h.map_area()) {
        let mut enc = Vec::new();
        encode_map(&ptrs, &mut enc);
        assert_eq!(enc, h.map_area());
    }

    if let Ok(d) = DirBlock::parse(&b) {
        assert_eq!(d.to_block(), Some(b));
    }

    if let Ok(s) = std::str::from_utf8(data) {
        for level in [Level::Ods2, Level::Ods5] {
            if let Ok(spec) = name::parse(level, s)
                && let Some(f) = spec.file
            {
                let _ = name::validate(level, &f.name);
                let _ = name::display(level, &f.name, ods_core::layout::NameType::Isl1);
                let c = name::chars(&f.name, ods_core::layout::NameType::Isl1);
                let _ = name::matches(&c, &c);
            }
        }
    }
});
