//! The image layer's own logic: copies, text views, trees with manifests.

use std::path::{Path, PathBuf};

use ods_image::{Conversion, Image, InitParams, Level, Mode, attrs};

/// A fresh directory for one test.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ods-image-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn image(dir: &Path, level: Level) -> Image {
    let p = InitParams { label: b"TEST".to_vec(), level, ..InitParams::default() };
    Image::create(dir.join("t.img"), 20_000, &p).unwrap()
}

/// Lines of varied length: some odd, some empty, some long.
fn text(lines: usize) -> Vec<u8> {
    let mut t = Vec::new();
    for i in 0..lines {
        t.extend(format!("line {i} {}\n", "x".repeat(i * 7 % 300)).bytes());
        if i % 17 == 0 {
            t.push(b'\n');
        }
    }
    t
}

#[test]
fn text_view_reads_anywhere() {
    let d = scratch("text");
    let mut img = image(&d, Level::Ods2);
    let t = text(3000);
    let (fid, _) = img.copy_in(&mut &t[..], "[000000]T.TXT", Conversion::LinesToRecords, None, None).unwrap();
    let mut whole = Vec::new();
    img.copy_out(fid, &mut whole, Conversion::RecordsToLines).unwrap();
    assert_eq!(whole, t);
    let view = img.text_view(fid).unwrap();
    assert_eq!(view.size, t.len() as u64);
    let mut seed = 7u64;
    for _ in 0..200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let off = (seed >> 33) % (t.len() as u64 + 10);
        let len = ((seed >> 13) % 70_000) as usize;
        let mut buf = vec![0u8; len];
        let n = img.read_text(fid, &view, off, &mut buf).unwrap();
        let want = &t[(off as usize).min(t.len())..(off as usize + len).min(t.len())];
        assert_eq!(&buf[..n], want, "offset {off} length {len}");
    }
}

#[test]
fn export_import_keeps_everything() {
    let d = scratch("tree");
    let mut img = image(&d, Level::Ods5);
    img.mkdir("[Src]").unwrap();
    img.mkdir("[Src.Sub]").unwrap();
    let t = text(50);
    img.copy_in(&mut &t[..], "[Src]notes.txt", Conversion::LinesToRecords, None, None).unwrap();
    img.copy_in(&mut &t[..10], "[Src]notes.txt", Conversion::LinesToRecords, None, None).unwrap();
    img.copy_in(&mut &b"\x00\x01binary"[..], "[Src.Sub]blob.bin", Conversion::Binary, None, None).unwrap();
    let fid = img.lookup("[Src]notes.txt;1").unwrap();
    let mut a = img.attributes(fid).unwrap();
    a.protection = attrs::parse_protection("S:RWED,O:RWED,G:R,W:").unwrap();
    a.owner = attrs::parse_uic("[200,3]").unwrap();
    a.expires = 0x00b0_0000_0000_0000;
    img.set_attributes(fid, &a).unwrap();

    let out = d.join("out");
    let m = img.export("[Src]", &out).unwrap();
    assert_eq!(m.entries.iter().filter(|e| !e.directory).count(), 3);
    assert!(out.join("notes.txt;1").exists() && out.join("Sub").join("blob.bin;1").exists());

    img.mkdir("[Copy]").unwrap();
    let got = img.import(&out, "[Copy]").unwrap();
    assert_eq!(got.len(), 3);
    for (from, to) in [
        ("[Src]notes.txt;1", "[Copy]notes.txt;1"),
        ("[Src]notes.txt;2", "[Copy]notes.txt;2"),
        ("[Src.Sub]blob.bin;1", "[Copy.Sub]blob.bin;1"),
    ] {
        let (f, t) = (img.lookup(from).unwrap(), img.lookup(to).unwrap());
        let (mut x, mut y) = (Vec::new(), Vec::new());
        img.copy_out(f, &mut x, Conversion::Binary).unwrap();
        img.copy_out(t, &mut y, Conversion::Binary).unwrap();
        assert_eq!(x, y, "{from}");
        let (mut a, mut b) = (img.attributes(f).unwrap(), img.attributes(t).unwrap());
        // Allocation depends on placement; the rest must match.
        (a.record.hiblk, b.record.hiblk) = (0, 0);
        assert_eq!(a, b, "{from}");
    }
    assert!(img.verify().unwrap().findings.is_empty());
    drop(img);
    let mut img = Image::open(d.join("t.img"), Mode::ReadOnly).unwrap();
    assert!(img.verify().unwrap().findings.is_empty());
}
