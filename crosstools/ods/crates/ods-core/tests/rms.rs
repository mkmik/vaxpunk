//! Relative and indexed files: the fixtures OpenVMS made (fixtures/rms)
//! must read by every key exactly as OpenVMS dumped them and check clean;
//! damaged copies must be caught; and files the loader makes must read
//! back and check clean.

use std::path::PathBuf;

use ods_core::RecordAttrs;
use ods_core::rms::{self, AreaSpec, KeySpec, Spec, dtype, fdl};
use ods_core::{rat, rfm};

const INDEXED: [&str; 7] = ["idx1", "idxc", "idxf", "idxb", "comp", "idxm", "uaf"];
const RELATIVE: [&str; 2] = ["rel", "relf"];

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rms").join(name)
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(path(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// What ANALYZE/RMS_FILE/FDL said of a fixture.
fn fixture_fdl(name: &str) -> fdl::Fdl {
    fdl::parse(&String::from_utf8(read(&format!("{name}.fdl"))).unwrap()).unwrap()
}

fn fixture(name: &str) -> (Vec<u8>, RecordAttrs) {
    let ext = if RELATIVE.contains(&name) { "rel" } else { "idx" };
    (read(&format!("{name}.{ext}")), fixture_fdl(name).record_attrs())
}

/// A record as fixtures/rms/make/dump.py prints it.
fn escape(rec: &[u8]) -> String {
    rec.iter()
        .map(|&b| if (32..127).contains(&b) && b != b'\\' { (b as char).to_string() } else { format!("\\x{b:02X}") })
        .collect()
}

fn unescape(line: &str) -> Vec<u8> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            out.push(u8::from_str_radix(&line[i + 2..i + 4], 16).unwrap());
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn dump(name: &str, key: usize) -> Vec<String> {
    String::from_utf8(read(&format!("{name}_key{key}.dump"))).unwrap().lines().map(String::from).collect()
}

fn lines(recs: &[Vec<u8>]) -> Vec<String> {
    recs.iter().map(|r| escape(r)).collect()
}

fn assert_clean(what: &str, r: &rms::Report) {
    assert!(r.findings.is_empty(), "{what}: {:#?}", r.findings);
}

#[test]
fn fixtures_read_as_openvms_dumped_them() {
    for name in INDEXED.iter().chain(&RELATIVE) {
        let (data, attrs) = fixture(name);
        let f = rms::open(&data, &attrs).unwrap();
        for key in 0..f.keys() {
            assert_eq!(lines(&f.records(key).unwrap()), dump(name, key), "{name} key {key}");
        }
    }
}

#[test]
fn relative_record_numbers() {
    let (data, attrs) = fixture("rel");
    let rms::RmsFile::Relative(r) = rms::open(&data, &attrs).unwrap() else { panic!("not relative") };
    let numbers: Vec<u32> = r.records().unwrap().iter().map(|r| r.0).collect();
    assert_eq!(numbers[..6], [1, 2, 3, 4, 6, 7], "record 5 was deleted");
    assert_eq!(numbers.len(), 199);
    assert_eq!(r.mrn, 500);
}

#[test]
fn fixtures_check_clean() {
    for name in INDEXED.iter().chain(&RELATIVE) {
        let (data, attrs) = fixture(name);
        let r = rms::check(&data, &attrs);
        assert_clean(name, &r);
        assert_eq!(r.records as usize, dump(name, 0).len(), "{name}");
    }
}

/// Damages a copy of a fixture and expects the checker to say so.
fn damaged(name: &str, damage: impl Fn(&mut Vec<u8>), expect: &str) {
    let (mut data, attrs) = fixture(name);
    damage(&mut data);
    let r = rms::check(&data, &attrs);
    assert!(r.findings.iter().any(|f| f.what.contains(expect)), "{name}: wanted {expect:?}, got {:#?}", r.findings);
    assert!(!r.is_sound());
}

fn at(vbn: usize) -> usize {
    (vbn - 1) * 512
}

#[test]
fn damage_is_reported() {
    // idx1: data buckets of key 0 from VBN 4, its SIDRs from VBN 24.
    damaged("idx1", |d| d[at(6)] ^= 1, "check characters differ");
    damaged("idx1", |d| d[at(8) + 8] = 10, "level 0");
    damaged("idx1", |d| d[at(2) + 40] ^= 1, "prologue checksum");
    damaged("relf", |d| d[100] ^= 1, "prologue checksum");
    // Swap the keys of the first two records of VBN 4.
    damaged(
        "idx1",
        |d| {
            let (a, b) = (at(4) + 14 + 11, at(4) + 14 + 26 + 11);
            for i in 0..8 {
                d.swap(a + i, b + i);
            }
        },
        "keys out of order",
    );
    // Drop the last pointer of the SIDR in VBN 24.
    damaged(
        "idx1",
        |d| {
            let b = at(24);
            let free = u16::from_le_bytes([d[b + 4], d[b + 5]]) - 7;
            d[b + 4..b + 6].copy_from_slice(&free.to_le_bytes());
            let size = u16::from_le_bytes([d[b + 14], d[b + 15]]) - 7;
            d[b + 14..b + 16].copy_from_slice(&size.to_le_bytes());
        },
        "missing from the key's SIDRs",
    );
    // A SIDR pointer to a record ID that isn't there.
    damaged("idx1", |d| d[at(24) + 14 + 2 + 5 + 1] = 99, "no record");
    // idxm's first RRV, at offset 185 of VBN 4, sent to another record ID.
    damaged("idxm", |d| d[at(4) + 185 + 3] = 9, "RRV 8 doesn't lead");
    // A bucket's free space offset past its end must be an error, not a panic.
    damaged("idxc", |d| d[at(4) + 4..at(4) + 6].copy_from_slice(&[0xff, 0xff]), "free space");
    // A FIX relative cell with an unknown control byte.
    damaged("relf", |d| d[512] = 0x40, "control byte");
}

/// Reads every key of a loaded file and checks it: the records along key 0
/// in primary order, every other key in its order with ties in primary
/// order, the ones with no key, a null one or too short left out.
fn round_trip(spec: &Spec, recs: &[Vec<u8>]) -> rms::Report {
    let data = rms::build(spec, recs).unwrap();
    assert_eq!(data.len() % 512, 0);
    let attrs = spec.record_attrs((data.len() / 512) as u32);
    let f = rms::open(&data, &attrs).unwrap();
    let key_of = |k: &KeySpec, r: &[u8]| -> Option<Vec<u8>> {
        let mut v = Vec::new();
        for &(p, s) in &k.segments {
            v.extend_from_slice(r.get(p as usize..p as usize + s as usize)?);
        }
        Some(v)
    };
    let rms::RmsFile::Indexed(x) = &f else { panic!("not indexed") };
    let mut primary: Vec<&Vec<u8>> = recs.iter().collect();
    primary.sort_by(|a, b| x.keys[0].compare(&key_of(&spec.keys[0], a).unwrap(), &key_of(&spec.keys[0], b).unwrap()));
    for (n, k) in spec.keys.iter().enumerate() {
        let mut want: Vec<(Vec<u8>, &Vec<u8>)> = primary
            .iter()
            .filter_map(|r| key_of(k, r).map(|kv| (kv, *r)))
            .filter(|(kv, _)| !(n > 0 && k.null_key && kv.iter().all(|&c| c == k.null_value)))
            .collect();
        want.sort_by(|a, b| x.keys[n].compare(&a.0, &b.0));
        let want: Vec<Vec<u8>> = want.into_iter().map(|w| w.1.clone()).collect();
        assert!(f.records(n).unwrap() == want, "key {n}");
    }
    let r = f.check();
    assert_clean("loaded file", &r);
    assert_eq!(r.records as usize, recs.len());
    r
}

/// A small deterministic generator, so failures repeat.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// SYSUAF.DAT's shape: make/uaf.fdl.
#[test]
fn load_sysuaf() {
    let f = fdl::parse(&String::from_utf8(read("make/uaf.fdl")).unwrap()).unwrap();
    assert_eq!(f.spec.keys.len(), 4);
    let mut g = Rng(7);
    let recs: Vec<Vec<u8>> = (0..400)
        .map(|i| {
            let mut r = vec![1, 0, 10, 1];
            r.extend(format!("{:<32}", format!("USER{i:04}")).bytes());
            let uic = (g.below(20) as u32 + 1) << 16 | g.below(8) as u32;
            r.extend(uic.to_le_bytes());
            r.extend([0; 4]);
            r.extend(g.next().to_le_bytes());
            r.extend(vec![b'x'; g.below(1300) as usize]);
            r
        })
        .collect();
    let r = round_trip(&f.spec, &recs);
    assert!(r.keys[0].levels.len() >= 2);
    // One record, and none: a file CREATE/FDL would make.
    round_trip(&f.spec, &recs[..1]);
    let empty = round_trip(&f.spec, &[]);
    assert!(empty.keys.iter().all(|k| k.levels.is_empty()));
}

/// Thousands of records with every compression: several index levels.
#[test]
fn load_compressed_multilevel() {
    let mut g = Rng(1);
    let mut key0 = KeySpec::string(2, 12);
    (key0.key_compr, key0.rec_compr, key0.idx_compr, key0.data_fill, key0.index_fill) = (true, true, true, 300, 300);
    let mut key1 = KeySpec::string(20, 6);
    (key1.key_compr, key1.idx_compr, key1.dups, key1.null_key, key1.null_value) = (true, true, true, true, b' ');
    let mut key2 = KeySpec::string(30, 4);
    (key2.datatype, key2.dups) = (dtype::INT4, true);
    let mut key3 = KeySpec::string(14, 3);
    key3.segments.push((0, 2));
    (key3.datatype, key3.dups) = (dtype::STRING | dtype::DESCENDING, true);
    let spec = Spec {
        rfm: rfm::VAR,
        mrs: 300,
        rat: rat::CR,
        areas: vec![AreaSpec { bktsz: 1, alloc: 0, deq: 0 }],
        keys: vec![key0, key1, key2, key3],
        cluster: 3,
    };
    let recs: Vec<Vec<u8>> = (0..6000)
        .map(|i| {
            let mut r = format!("{:02}K{:08}{:03}  ", i % 7, (i * 7919) % 100_003, i % 13).into_bytes();
            let dept = ["SALES ", "      ", "R&D   ", "ADMIN "][g.below(4) as usize];
            r.extend(dept.bytes());
            r.extend(b"    ");
            r.extend((g.below(2000) as i32 - 1000).to_le_bytes());
            r.extend(vec![b'z'; g.below(200) as usize]);
            r.extend(b"tail");
            r
        })
        .collect();
    let r = round_trip(&spec, &recs);
    assert!(r.keys[0].levels.len() >= 3, "{:?}", r.keys[0].levels);
}

/// FIX records in three areas, as make/idxf.fdl, with records too short
/// for nothing; and bucket overflow of one key's duplicates.
#[test]
fn load_fixed_areas() {
    let f = fdl::parse(&String::from_utf8(read("make/idxf.fdl")).unwrap()).unwrap();
    let recs: Vec<Vec<u8>> = String::from_utf8(read("make/fixd.txt"))
        .unwrap()
        .lines()
        .map(|l| {
            let mut r = l.as_bytes().to_vec();
            r.resize(40, b' ');
            r
        })
        .collect();
    round_trip(&f.spec, &recs);
    // Thousands of records with one alternate key value: SIDRs that go on
    // across buckets.
    let mut spec = f.spec.clone();
    spec.keys.truncate(2);
    let many: Vec<Vec<u8>> = (0..3000).map(|i| format!("{i:06} DEPT  SAME{:<23}", "").into_bytes()).collect();
    round_trip(&spec, &many);
}

#[test]
fn loader_refuses_bad_input() {
    let spec = Spec {
        rfm: rfm::VAR,
        mrs: 20,
        rat: rat::CR,
        areas: vec![AreaSpec { bktsz: 1, alloc: 0, deq: 0 }],
        keys: vec![KeySpec::string(0, 4)],
        cluster: 0,
    };
    assert!(rms::build(&spec, &[b"abcd".to_vec(), b"abcd".to_vec()]).is_err(), "duplicate key");
    assert!(rms::build(&spec, &[b"abc".to_vec()]).is_err(), "too short");
    assert!(rms::build(&spec, &[vec![b'a'; 21]]).is_err(), "too long");
    let mut s = spec.clone();
    s.keys[0].datatype = dtype::BIN4;
    s.keys[0].key_compr = true;
    assert!(rms::build(&s, &[b"abcd".to_vec()]).is_err(), "compressed integer key");
}

/// The fixtures' records, loaded with the fixtures' FDL, read back as
/// OpenVMS dumped them; and the data and SIDR buckets hold what CONVERT
/// put in each.
#[test]
fn load_fixture_records() {
    for name in ["idx1", "idxc", "idxf", "idxb", "comp", "uaf", "idxm"] {
        let f = fixture_fdl(name);
        let recs: Vec<Vec<u8>> = dump(name, 0).iter().map(|l| unescape(l)).collect();
        let data = rms::build(&f.spec, &recs).unwrap();
        let attrs = f.spec.record_attrs((data.len() / 512) as u32);
        let file = rms::open(&data, &attrs).unwrap();
        // idxm's duplicates of key 1 are in the order RMS $PUT them.
        let keys = if name == "idxm" { 1 } else { f.spec.keys.len() };
        for key in 0..keys {
            assert_eq!(lines(&file.records(key).unwrap()), dump(name, key), "{name} key {key}");
        }
        let r = file.check();
        assert_clean(name, &r);
        let (orig, oattrs) = fixture(name);
        let o = rms::check(&orig, &oattrs);
        if ["idx1", "idxc", "idxf", "idxb", "comp"].contains(&name) {
            for (k, (a, b)) in r.keys.iter().zip(&o.keys).enumerate() {
                assert_eq!(a.entries, b.entries, "{name} key {k}: entries per level 0 bucket");
            }
        }
    }
}
