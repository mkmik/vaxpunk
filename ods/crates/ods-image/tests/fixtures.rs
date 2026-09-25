//! Real VMS disks (see fixtures/fetch.sh): every on-disk structure must
//! parse and serialize back to the same bytes, every volume but one must
//! verify clean, and each must list exactly as the reviewed listing in
//! fixtures/expected says. Missing images are skipped.
//!
//! `ODS_BLESS=1 cargo test` rewrites the listings; review the diff.

use std::fmt::Write;
use std::path::PathBuf;

use ods_image::layout::{DirBlock, RecordAttrs, decode_map, encode_map};
use ods_image::{Fid, Found, Header, Image, Mode, Severity, attrs, fch, time};

fn fixture(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("skipping {name}: not fetched (fixtures/fetch.sh)");
        None
    }
}

/// FNV-1a, 64 bits: enough to notice a changed byte.
fn fnv(data: &[u8], mut h: u64) -> u64 {
    for &b in data {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Parses and re-serializes every structure of every file.
fn round_trips(img: &mut Image, found: &[Found]) {
    let h = *img.home();
    let mut again = h;
    again.update_checksums();
    assert!(again == h, "home block checksums");
    let mut n = 0;
    for f in found {
        for (lbn, hdr) in img.headers(f.entry.fid).unwrap() {
            n += 1;
            let mut c: Header = hdr;
            c.update_checksum();
            assert!(c == hdr, "header checksum at LBN {lbn}");
            let ra = hdr.recattr();
            assert_eq!(RecordAttrs::from_bytes(&ra).to_bytes(), ra, "record attributes at LBN {lbn}");
            let map = hdr.map_area();
            let mut enc = Vec::new();
            encode_map(&decode_map(map).unwrap(), &mut enc);
            assert_eq!(enc, map, "map area at LBN {lbn}");
            if let Some(id) = hdr.ident() {
                let mut c = hdr;
                let area = c.idoffset() as usize * 2..(c.mpoffset() as usize * 2).min(510);
                c.0[area].fill(0xee);
                c.set_ident(&id);
                assert_eq!(
                    c.ident_area(),
                    hdr.ident_area(),
                    "ident area at LBN {lbn}: {:?}",
                    String::from_utf8_lossy(&id.name)
                );
            }
        }
        if img.stat(f.entry.fid).unwrap().attrs.filechar & fch::DIRECTORY != 0 {
            let used = img.stat(f.entry.fid).unwrap().attrs.record.efblk.saturating_sub(1) as u64;
            let mut vbn = 0;
            for (lbn, count) in img.extents(f.entry.fid).unwrap() {
                for i in 0..count {
                    vbn += 1;
                    if vbn > used {
                        break;
                    }
                    let b = img.read_block(lbn + i).unwrap();
                    let d = DirBlock::parse(&b).unwrap();
                    assert_eq!(d.to_block().as_ref(), Some(&b), "directory block at LBN {}", lbn + i);
                }
            }
        }
    }
    assert!(n > 0);
}

/// One line per entry: what the listing in fixtures/expected records.
fn listing(img: &mut Image, found: &[Found]) -> String {
    let mut out = String::new();
    for f in found {
        let i = img.stat(f.entry.fid).unwrap();
        let a = &i.attrs;
        // Some files map blocks the device does not have (BADBLK.SYS on
        // an RK07 rounds the last track up to a cluster): unreadable.
        let mut sum = 0xcbf2_9ce4_8422_2325;
        let mut r = img.reader(f.entry.fid).unwrap();
        let mut buf = vec![0u8; 1 << 16];
        let sum = loop {
            match std::io::Read::read(&mut r, &mut buf) {
                Ok(0) => break format!("{sum:016x}"),
                Ok(n) => sum = fnv(&buf[..n], sum),
                Err(_) => break "unreadable".into(),
            }
        };
        writeln!(
            out,
            "{} {} {}/{} {}/{}/{} {} {} {} {} {sum}",
            img.file_spec(f),
            i.fid,
            a.record.eof_bytes(),
            i.allocated,
            attrs::rfm_name(a.record.rtype),
            attrs::rat_names(a.record.rattrib),
            a.record.rsize,
            attrs::uic(a.owner),
            attrs::protection(a.protection),
            time::format(a.created).replace(' ', "_"),
            attrs::fch_names(a.filechar),
        )
        .unwrap();
    }
    out
}

/// BACKUP save sets check themselves: their blocks are numbered, and after
/// each group of data blocks comes one holding the XOR of their contents.
/// Reading them through the file system and finding both intact shows it
/// found every block of the file, in order. Returns the blocks checked.
fn save_sets(img: &mut Image, found: &[Found]) -> u64 {
    let mut checked = 0;
    for f in found {
        let r = img.stat(f.entry.fid).unwrap().attrs.record;
        let size = r.rsize as usize;
        if r.rtype & 0xf != 1 || size < 2048 || r.eof_bytes() == 0 || r.eof_bytes() % size as u64 != 0 {
            continue;
        }
        let mut rd = img.reader(f.entry.fid).unwrap();
        let mut blk = vec![0u8; size];
        let mut xor = vec![0u8; size - 256];
        let mut n = 0u32;
        while std::io::Read::read_exact(&mut rd, &mut blk).is_ok() {
            let word = |o: usize| u16::from_le_bytes([blk[o], blk[o + 1]]);
            if word(0) != 256 {
                break; // not a save set
            }
            n += 1;
            let number = u32::from_le_bytes(blk[8..12].try_into().unwrap());
            assert_eq!(number, n, "{}: block {n} numbered {number}", img.file_spec(f));
            xor.iter_mut().zip(&blk[256..]).for_each(|(x, b)| *x ^= b);
            if word(6) == 2 {
                assert!(xor.iter().all(|&x| x == 0), "{}: XOR group ending at block {n} is wrong", img.file_spec(f));
            }
            if word(6) == 2 {
                xor.fill(0);
            }
        }
        checked += n as u64;
    }
    checked
}

/// Round trips, verification and the listing, for one fixture. `defects`
/// are the errors the image is known to have.
fn check(name: &str, defects: &[Fid]) {
    let Some(path) = fixture(name) else { return };
    let mut img = Image::open(&path, Mode::ReadOnly).unwrap();
    let found = img.search("[...]*.*;*").unwrap();
    round_trips(&mut img, &found);
    let blocks = save_sets(&mut img, &found);
    eprintln!("{name}: {} entries, {blocks} save set blocks checked", found.len());
    let r = img.verify().unwrap();
    let errors: Vec<Fid> = r.findings.iter().filter(|f| f.severity == Severity::Error).filter_map(|f| f.fid).collect();
    assert_eq!(errors, defects, "{name}: {:#?}", r.findings);
    let got = listing(&mut img, &found);
    let expected =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected").join(format!("{name}.txt"));
    if std::env::var_os("ODS_BLESS").is_some() {
        std::fs::create_dir_all(expected.parent().unwrap()).unwrap();
        std::fs::write(&expected, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&expected).unwrap_or_default();
    if got != want {
        let diff: Vec<String> = got
            .lines()
            .zip(want.lines())
            .filter(|(g, w)| g != w)
            .take(5)
            .map(|(g, w)| format!("got  {g}\nwant {w}"))
            .collect();
        panic!(
            "{name}: listing differs ({} vs {} lines)\n{}",
            got.lines().count(),
            want.lines().count(),
            diff.join("\n")
        );
    }
}

#[test]
fn vaxvms_v1_0() {
    check("vaxvms-v1.0.rk07", &[]);
}

#[test]
fn dungeon() {
    // Five files added to the disk by some tool that left the index file's
    // end of file behind their headers.
    let past_eof = [Fid::new(11, 1), Fid::new(12, 1), Fid::new(13, 1), Fid::new(14, 2), Fid::new(15, 2)];
    check("dungeon.rk07", &past_eof);
}

#[test]
fn vms_7_1_init() {
    check("vms-7.1-init.dsk", &[]);
}

#[test]
fn vms_5_5_2() {
    check("vms-5.5-2.iso", &[]);
}

#[test]
fn vms_6_0() {
    check("vms-6.0.iso", &[]);
}

#[test]
fn alpha_8_4_2l1() {
    check("alpha-8.4-2l1.iso", &[]);
}
