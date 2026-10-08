//! Relative and indexed files on a volume: the OpenVMS fixtures copied in
//! read by every key as OpenVMS dumped them, and an indexed file loaded
//! from text has the attributes RMS wants and reads back.

use std::path::{Path, PathBuf};

use ods_image::rms::{self, fdl};
use ods_image::{Conversion, Image, InitParams};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rms")
}

fn text(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}

fn image(name: &str) -> Image {
    let d = std::env::temp_dir().join(format!("ods-image-rms-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let p = InitParams { label: b"RMS".to_vec(), ..InitParams::default() };
    Image::create(d.join("t.img"), 20_000, &p).unwrap()
}

fn escape(rec: &[u8]) -> String {
    rec.iter()
        .map(|&b| if (32..127).contains(&b) && b != b'\\' { (b as char).to_string() } else { format!("\\x{b:02X}") })
        .collect()
}

#[test]
fn fixtures_copied_in_read_by_every_key() {
    let mut img = image("fixtures");
    for (name, ext, keys) in [
        ("idx1", "idx", 2),
        ("idxf", "idx", 3),
        ("idxm", "idx", 2),
        ("uaf", "idx", 4),
        ("rel", "rel", 1),
        ("relf", "rel", 1),
    ] {
        let data = std::fs::read(fixtures().join(format!("{name}.{ext}"))).unwrap();
        let f = fdl::parse(&text(&fixtures().join(format!("{name}.fdl")))).unwrap();
        let mut attrs = f.record_attrs();
        let blocks = (data.len() / 512) as u32;
        (attrs.hiblk, attrs.efblk) = (blocks, blocks + 1);
        let spec = format!("[000000]{}.{}", name.to_uppercase(), ext.to_uppercase());
        let (fid, _) = img.copy_in(&mut &data[..], &spec, Conversion::Binary, None, Some(attrs)).unwrap();
        for key in 0..keys {
            let got: Vec<String> = img.read_records(fid, key).unwrap().iter().map(|r| escape(r)).collect();
            let want = text(&fixtures().join(format!("{name}_key{key}.dump")));
            assert_eq!(got, want.lines().collect::<Vec<_>>(), "{name} key {key}");
        }
        assert!(img.read_records(fid, keys).is_err());
        let r = img.check_file(fid).unwrap();
        assert!(r.findings.is_empty(), "{name}: {:?}", r.findings);
    }
}

#[test]
fn load_text_as_indexed_file() {
    let mut img = image("load");
    let f = fdl::parse(&text(&fixtures().join("make/idxf.fdl"))).unwrap();
    let recs = rms::lines_to_records(&f.spec, text(&fixtures().join("make/fixd.txt")).as_bytes()).unwrap();
    assert!(recs.iter().all(|r| r.len() == 40));
    let (fid, name) = img.load_indexed(&f.spec, &recs, "[000000]IDXF.IDX").unwrap();
    assert_eq!(name, "[000000]IDXF.IDX;1");
    let a = img.stat(fid).unwrap().attrs.record;
    assert_eq!((a.rtype, a.maxrec, a.rsize, a.bktsize, a.efblk, a.ffbyte), (0x21, 40, 40, 3, a.hiblk + 1, 0));
    for key in 0..3 {
        let got: Vec<String> = img.read_records(fid, key).unwrap().iter().map(|r| escape(r)).collect();
        let want = text(&fixtures().join(format!("idxf_key{key}.dump")));
        assert_eq!(got, want.lines().collect::<Vec<_>>(), "key {key}");
    }
    assert!(img.check_file(fid).unwrap().is_sound());
    assert!(img.verify().unwrap().is_sound());
    // A sequential file reads in order, and has nothing to check.
    let (seq, _) =
        img.copy_in(&mut &b"one\ntwo\n"[..], "[000000]S.TXT", Conversion::LinesToRecords, None, None).unwrap();
    assert_eq!(img.read_records(seq, 0).unwrap(), [b"one".to_vec(), b"two".to_vec()]);
    assert!(img.check_file(seq).is_err());
    // Lines too long for FIX records are refused.
    assert!(rms::lines_to_records(&f.spec, &[b'x'; 41]).is_err());
}
