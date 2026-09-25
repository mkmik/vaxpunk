//! Property and fault injection tests: random operations on a volume the
//! core initialized, checked against a model of what it should contain,
//! with the verifier run after every step.
//!
//! Failures print the seed; add it to SEEDS so it stays covered.

use std::collections::BTreeMap;

use ods_core::{Alloc, BLOCK, BlockDevice, Fid, InitParams, Level, NewFile, Severity, Version, Volume, rat, rfm};

/// Seeds that found bugs, kept.
const SEEDS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8];

/// An in-memory device that can stop accepting writes, like a machine
/// that lost power, and hands out one block per call when asked to.
#[derive(Clone)]
struct Mem {
    data: Vec<u8>,
    writes: usize,
    fail_after: Option<usize>,
}

#[derive(Debug)]
struct PowerLost;

impl Mem {
    fn new(blocks: usize) -> Mem {
        Mem { data: vec![0xa5; blocks * BLOCK], writes: 0, fail_after: None }
    }
}

impl BlockDevice for Mem {
    type Error = PowerLost;
    fn block_size(&self) -> usize {
        BLOCK
    }
    fn block_count(&self) -> u64 {
        (self.data.len() / BLOCK) as u64
    }
    fn read(&mut self, lbn: u64, buf: &mut [u8]) -> Result<(), PowerLost> {
        let at = lbn as usize * BLOCK;
        buf.copy_from_slice(&self.data[at..at + buf.len()]);
        Ok(())
    }
    fn write(&mut self, lbn: u64, buf: &[u8]) -> Result<(), PowerLost> {
        // One block at a time, each one atomic: a multi-block write can
        // stop part way, as a real one can.
        for (i, b) in buf.chunks(BLOCK).enumerate() {
            if self.fail_after.is_some_and(|n| self.writes >= n) {
                return Err(PowerLost);
            }
            self.writes += 1;
            let at = (lbn as usize + i) * BLOCK;
            self.data[at..at + BLOCK].copy_from_slice(b);
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), PowerLost> {
        Ok(())
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> Option<&'a T> {
        (!v.is_empty()).then(|| &v[self.below(v.len())])
    }
}

type Dir = Vec<Vec<u8>>;

/// A name's version limit and its versions' contents.
type Versions = (u16, BTreeMap<u16, Vec<u8>>);

/// What the volume should hold: directories, and for each name its
/// version limit and versions with their contents.
#[derive(Default, Clone)]
struct Model {
    names: Vec<Vec<u8>>,
    /// Operations `step` may pick (see its match arms).
    ops: Vec<usize>,
    /// Largest file content.
    max_len: usize,
    dirs: BTreeMap<Dir, u16>,
    files: BTreeMap<(Dir, Vec<u8>), Versions>,
}

impl Model {
    fn new(level: Level) -> Model {
        let mut names: Vec<&[u8]> =
            vec![b"A.TXT", b"B.DAT", b"LONGER_NAME_FOR_RECORDS.TEXT", b"C.", b"A-B.C", b"A$.TXT"];
        if level == Level::Ods5 {
            names.extend([&b"mixed.Case"[..], b"with space.txt", b"dots.in.name.txt", b"caf\xe9.txt"]);
        }
        Model::with(names.iter().map(|n| n.to_vec()).collect(), (0..12).collect(), 20000)
    }

    fn with(names: Vec<Vec<u8>>, ops: Vec<usize>, max_len: usize) -> Model {
        let mut m = Model { names, ops, max_len, ..Model::default() };
        m.dirs.insert(Vec::new(), 0);
        m
    }
}

fn clock() -> u64 {
    0x00a0_0000_0000_0000
}

fn write_content<D: BlockDevice>(v: &mut Volume<D>, fid: Fid, data: &[u8]) -> ods_core::Result<(), D::Error> {
    let blocks = data.len().div_ceil(BLOCK) as u64;
    let have = v.stat(fid)?.allocated;
    if blocks > have {
        v.extend(fid, blocks - have, Alloc::Any)?;
    }
    let mut buf = data.to_vec();
    buf.resize(blocks as usize * BLOCK, 0);
    if !buf.is_empty() {
        v.write_blocks(fid, 1, &buf)?;
    }
    let mut a = v.attributes(fid)?;
    a.record.efblk = (data.len() / BLOCK) as u32 + 1;
    a.record.ffbyte = (data.len() % BLOCK) as u16;
    v.set_attributes(fid, &a)
}

fn read_content<D: BlockDevice>(v: &mut Volume<D>, fid: Fid) -> Vec<u8> {
    let i = v.stat(fid).unwrap();
    let n = i.attrs.record.eof_bytes() as usize;
    let mut buf = vec![0u8; n.div_ceil(BLOCK) * BLOCK];
    v.read_blocks(fid, 1, &mut buf).unwrap();
    buf.truncate(n);
    buf
}

fn fresh(rng: &mut Rng, level: Level, blocks: usize) -> Volume<Mem> {
    let blocks = blocks + rng.below(blocks);
    let p = InitParams {
        label: b"MODEL".to_vec(),
        level,
        cluster: 1 + rng.below(4) as u16,
        now: clock(),
        ..InitParams::default()
    };
    let mut v = ods_core::initialize(Mem::new(blocks), &p).unwrap();
    v.set_clock(clock);
    v
}

/// One random operation, applied to both the volume and the model. The
/// volume's answer decides expected failures (a full disk, a name that is
/// taken); the model is only updated on success.
fn step<D: BlockDevice>(v: &mut Volume<D>, m: &mut Model, rng: &mut Rng) -> ods_core::Result<String, D::Error>
where
    D::Error: std::fmt::Debug,
{
    let dirs: Vec<Dir> = m.dirs.keys().cloned().collect();
    let dir = rng.pick(&dirs).unwrap().clone();
    let dfid = v.find_dir(&dir)?;
    let keys: Vec<(Dir, Vec<u8>)> = m.files.keys().cloned().collect();
    let op = *rng.pick(&m.ops).unwrap();
    let desc = match op {
        0..=3 => {
            let name = rng.pick(&m.names).unwrap().to_vec();
            let len = [0, 1, 100, 511, 512, 513, 3000, 20000][rng.below(8)].min(m.max_len);
            let data: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            let new = NewFile {
                record: ods_core::RecordAttrs { rtype: rfm::VAR, rattrib: rat::CR, ..Default::default() },
                blocks: rng.below(3) as u64,
                alloc: [Alloc::Any, Alloc::BestTry, Alloc::Contiguous][rng.below(3)],
                ..Default::default()
            };
            let r = v.create(dfid, &name, None, &new);
            let (fid, ver) = match r {
                Err(ods_core::Error::DeviceFull | ods_core::Error::HeaderFull) => return Ok("create: full".into()),
                other => other?,
            };
            let limit = m.dirs[&dir];
            let entry = m
                .files
                .entry((dir.clone(), name.clone()))
                .or_insert((if limit == 0 { 32767 } else { limit }, BTreeMap::new()));
            entry.1.insert(ver, Vec::new());
            while entry.1.len() > entry.0 as usize {
                let lowest = *entry.1.keys().next().unwrap();
                entry.1.remove(&lowest);
            }
            match write_content(v, fid, &data) {
                Ok(()) => {
                    m.files.get_mut(&(dir.clone(), name.clone())).unwrap().1.insert(ver, data);
                }
                Err(ods_core::Error::DeviceFull | ods_core::Error::HeaderFull) => {
                    // The file exists, possibly with part of the data; drop it.
                    v.delete(dfid, &name, ver)?;
                    let e = m.files.get_mut(&(dir.clone(), name.clone())).unwrap();
                    e.1.remove(&ver);
                    if e.1.is_empty() {
                        m.files.remove(&(dir.clone(), name.clone()));
                    }
                }
                Err(e) => return Err(e),
            }
            format!("create {dir:?} {} {ver} {len}", String::from_utf8_lossy(&name))
        }
        4 => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let ver = *rng.pick(&m.files[&k].1.keys().copied().collect::<Vec<_>>()).unwrap();
            v.delete(fdir, &k.1, ver)?;
            let e = m.files.get_mut(&k).unwrap();
            e.1.remove(&ver);
            if e.1.is_empty() {
                m.files.remove(&k);
            }
            format!("delete {k:?};{ver}")
        }
        5 => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let keep = 1 + rng.below(2);
            v.purge(fdir, &k.1, keep)?;
            let e = m.files.get_mut(&k).unwrap();
            while e.1.len() > keep {
                let lowest = *e.1.keys().next().unwrap();
                e.1.remove(&lowest);
            }
            format!("purge {k:?} keep {keep}")
        }
        6 => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let ver = *rng.pick(&m.files[&k].1.keys().copied().collect::<Vec<_>>()).unwrap();
            let to_dir = rng.pick(&dirs).unwrap().clone();
            let tfid = v.find_dir(&to_dir)?;
            let to = rng.pick(&m.names).unwrap().to_vec();
            let r = v.rename(fdir, &k.1, Version::Exact(ver), tfid, &to, None);
            let nv = match r {
                Err(ods_core::Error::Exists | ods_core::Error::DeviceFull) => return Ok("rename: refused".into()),
                other => other?,
            };
            let data = m.files.get_mut(&k).unwrap().1.remove(&ver).unwrap();
            if m.files[&k].1.is_empty() {
                m.files.remove(&k);
            }
            let limit = m.dirs[&to_dir];
            let t = m
                .files
                .entry((to_dir.clone(), to.clone()))
                .or_insert((if limit == 0 { 32767 } else { limit }, BTreeMap::new()));
            t.1.insert(nv, data);
            format!("rename {k:?};{ver} -> {to_dir:?} {};{nv}", String::from_utf8_lossy(&to))
        }
        7 => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let ver = *rng.pick(&m.files[&k].1.keys().copied().collect::<Vec<_>>()).unwrap();
            let fid = v.lookup(fdir, &k.1, Version::Exact(ver))?.fid;
            let data = m.files[&k].1[&ver].clone();
            let keep = rng.below(data.len().div_ceil(BLOCK) + 2) as u64;
            v.truncate(fid, keep)?;
            let now = v.stat(fid)?.allocated;
            let mut d = data;
            d.truncate((now as usize * BLOCK).min(d.len()));
            m.files.get_mut(&k).unwrap().1.insert(ver, d);
            format!("truncate {k:?};{ver} to {keep}")
        }
        8 => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let ver = *rng.pick(&m.files[&k].1.keys().copied().collect::<Vec<_>>()).unwrap();
            let fid = v.lookup(fdir, &k.1, Version::Exact(ver))?.fid;
            match v.extend(fid, 1 + rng.below(40) as u64, Alloc::Any) {
                Err(ods_core::Error::DeviceFull) => return Ok("extend: full".into()),
                other => other?,
            }
            format!("extend {k:?};{ver}")
        }
        9 => {
            if dir.len() >= 3 {
                return Ok("mkdir: deep enough".into());
            }
            let name = format!("D{}", rng.below(4)).into_bytes();
            let mut sub = dir.clone();
            sub.push(name.clone());
            match v.create_dir(dfid, &name) {
                Err(ods_core::Error::Exists) => return Ok("mkdir: exists".into()),
                Err(ods_core::Error::DeviceFull | ods_core::Error::HeaderFull) => return Ok("mkdir: full".into()),
                other => other?,
            };
            m.dirs.insert(sub.clone(), m.dirs[&dir]);
            format!("mkdir {sub:?}")
        }
        10 => {
            let limit = [0, 1, 2, 3][rng.below(4)];
            let mut a = v.attributes(dfid)?;
            a.record.versions = limit;
            v.set_attributes(dfid, &a)?;
            m.dirs.insert(dir.clone(), limit);
            format!("version limit {dir:?} {limit}")
        }
        _ => {
            let Some(k) = rng.pick(&keys).cloned() else { return Ok("none".into()) };
            let fdir = v.find_dir(&k.0)?;
            let ver = *rng.pick(&m.files[&k].1.keys().copied().collect::<Vec<_>>()).unwrap();
            let fid = v.lookup(fdir, &k.1, Version::Exact(ver))?.fid;
            let len = rng.below(5000).min(m.max_len);
            let data: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            match write_content(v, fid, &data) {
                Err(ods_core::Error::DeviceFull) => return Ok("rewrite: full".into()),
                other => other?,
            }
            m.files.get_mut(&k).unwrap().1.insert(ver, data);
            format!("rewrite {k:?};{ver} {len}")
        }
    };
    Ok(desc)
}

/// Checks that the volume holds exactly what the model says.
fn compare(v: &mut Volume<Mem>, m: &Model, contents: bool) {
    for dir in m.dirs.keys() {
        let dfid = v.find_dir(dir).unwrap();
        let mut got: BTreeMap<Vec<u8>, Vec<u16>> = BTreeMap::new();
        for e in v.list(dfid).unwrap() {
            if dir.is_empty() && e.fid.num <= 9 {
                continue;
            }
            if e.is_dir_name()
                && m.dirs.contains_key(&[dir.clone(), vec![e.name[..e.name.len() - 4].to_vec()]].concat())
            {
                continue;
            }
            got.entry(e.name.clone()).or_default().push(e.version);
        }
        let mut want: BTreeMap<Vec<u8>, Vec<u16>> = BTreeMap::new();
        for ((d, n), (_, vs)) in &m.files {
            if d == dir {
                want.insert(n.clone(), vs.keys().rev().copied().collect());
            }
        }
        assert_eq!(got, want, "directory {dir:?}");
        if contents {
            for ((d, n), (_, vs)) in &m.files {
                if d != dir {
                    continue;
                }
                for (&ver, data) in vs {
                    let fid = v.lookup(dfid, n, Version::Exact(ver)).unwrap().fid;
                    assert_eq!(&read_content(v, fid), data, "{dir:?} {};{ver}", String::from_utf8_lossy(n));
                }
            }
        }
    }
}

fn run(seed: u64, steps: usize, level: Level) {
    run_model(seed, steps, level, 3000, Model::new(level));
}

/// Directory blocks in use and the most records one name has.
fn dir_shape(v: &mut Volume<Mem>) -> (u32, usize) {
    let blocks = v.stat(ods_core::MFD).unwrap().attrs.record.efblk - 1;
    let mut per: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    for e in v.list(ods_core::MFD).unwrap() {
        *per.entry(e.name).or_default() += 1;
    }
    (blocks, per.values().copied().max().unwrap_or(0))
}

fn run_model(seed: u64, steps: usize, level: Level, blocks: usize, model: Model) -> Volume<Mem> {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut v = fresh(&mut rng, level, blocks);
    let mut m = model;
    for i in 0..steps {
        let what = step(&mut v, &mut m, &mut rng).unwrap_or_else(|e| panic!("seed {seed} step {i}: {e}"));
        if std::env::var_os("MODEL_TRACE").is_some() {
            eprintln!("{seed} {i}: {what}");
        }
        let r = v.verify().unwrap();
        assert!(r.findings.is_empty(), "seed {seed} step {i} ({what}): {:#?}", r.findings);
        compare(&mut v, &m, i % 10 == 9 || i == steps - 1);
    }
    v
}

#[test]
fn random_operations_ods2() {
    for &seed in SEEDS {
        run(seed, 150, Level::Ods2);
    }
}

#[test]
fn random_operations_ods5() {
    for &seed in SEEDS {
        run(seed + 1000, 100, Level::Ods5);
    }
}

/// Replays a sequence of operations, cutting the power after each
/// possible number of writes. Every state left behind must mount and
/// verify with nothing worse than leaked space.
fn crash_everywhere(seed: u64, steps: usize, blocks: usize, model: &Model) {
    let mut rng = Rng(seed | 1);
    let base = fresh(&mut rng, Level::Ods2, blocks);
    let start = base.dismount().unwrap();
    let ops_seed = rng.next();
    let total = {
        let mut v = Volume::mount(start.clone(), true).unwrap();
        v.set_clock(clock);
        let (mut m, mut r) = (model.clone(), Rng(ops_seed));
        for _ in 0..steps {
            step(&mut v, &mut m, &mut r).unwrap();
        }
        v.dismount().unwrap().writes - start.writes
    };
    for n in 0..total {
        let mut dev = start.clone();
        dev.fail_after = Some(start.writes + n);
        let mut v = Volume::mount(dev, true).unwrap();
        v.set_clock(clock);
        let (mut m, mut r) = (model.clone(), Rng(ops_seed));
        for _ in 0..steps {
            if step(&mut v, &mut m, &mut r).is_err() {
                break;
            }
        }
        let mut dev = v.dismount().unwrap();
        dev.fail_after = None;
        let mut v =
            Volume::mount(dev, false).unwrap_or_else(|e| panic!("seed {seed} crash after {n} writes: mount: {e}"));
        let rep = v.verify().unwrap();
        let errors: Vec<_> = rep.findings.iter().filter(|f| f.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "seed {seed} crash after {n} of {total} writes: {errors:#?}");
    }
}

#[test]
fn crashes_leave_only_leaks() {
    for &seed in &SEEDS[..4] {
        crash_everywhere(seed, 25, 3000, &Model::new(Level::Ods2));
    }
}

/// Names enough to spread a directory over many blocks.
fn many_names(n: usize) -> Vec<Vec<u8>> {
    let mut r = Rng(42);
    (0..n).map(|i| format!("F{}_{:04}{}.DAT", r.below(10), i, "X".repeat(r.below(25))).into_bytes()).collect()
}

/// Creates, deletes and renames in one directory, so it grows.
const DIR_OPS: [usize; 6] = [0, 1, 2, 3, 4, 6];

#[test]
fn big_directories() {
    for &seed in &SEEDS[..3] {
        let mut v = run_model(seed, 600, Level::Ods2, 20000, Model::with(many_names(400), DIR_OPS.to_vec(), 512));
        let (blocks, _) = dir_shape(&mut v);
        assert!(blocks >= 10, "MFD only {blocks} blocks");
    }
}

#[test]
fn many_versions() {
    let names = vec![b"V.TXT".to_vec(), b"A_RATHER_LONG_NAME_WITH_MANY_VERSIONS.DATA".to_vec()];
    for &seed in &SEEDS[..3] {
        // Creates and deletes only: a few hundred versions per name.
        let mut v = run_model(seed, 500, Level::Ods2, 20000, Model::with(names.clone(), vec![0, 1, 2, 3, 0, 1, 4], 0));
        let (_, versions) = dir_shape(&mut v);
        assert!(versions > 120, "only {versions} versions of a name");
    }
}

#[test]
fn big_directories_survive_crashes() {
    for &seed in &SEEDS[..2] {
        crash_everywhere(seed, 120, 20000, &Model::with(many_names(200), DIR_OPS.to_vec(), 512));
    }
}

/// Fills a volume with one-cluster files and deletes every other one, so
/// free space is all holes.
fn fragment(v: &mut Volume<Mem>) {
    let name = |i: usize| format!("S{i:04}.TMP").into_bytes();
    for i in 0..600 {
        let new = NewFile { blocks: 1, ..NewFile::default() };
        v.create(ods_core::MFD, &name(i), None, &new).unwrap();
    }
    for i in (0..600).step_by(2) {
        v.delete(ods_core::MFD, &name(i), 1).unwrap();
    }
}

fn fragmented(blocks: usize) -> Volume<Mem> {
    let p = InitParams { label: b"FRAG".to_vec(), cluster: 1, now: clock(), ..InitParams::default() };
    let mut v = ods_core::initialize(Mem::new(blocks), &p).unwrap();
    v.set_clock(clock);
    fragment(&mut v);
    v
}

/// A file in hundreds of pieces needs extension headers: grow it across
/// the holes, read it back, shrink it, and check the volume each time.
#[test]
fn extension_headers() {
    let mut rng = Rng(99);
    let mut v = fragmented(4000);
    let (fid, _) = v.create(ods_core::MFD, b"BIG.DAT", None, &NewFile::default()).unwrap();
    let data: Vec<u8> = (0..250 * BLOCK).map(|_| rng.next() as u8).collect();
    write_content(&mut v, fid, &data).unwrap();
    let info = v.stat(fid).unwrap();
    assert!(info.headers >= 3, "{} headers, {} extents", info.headers, info.extents);
    assert_eq!(read_content(&mut v, fid), data);
    let r = v.verify().unwrap();
    assert!(r.findings.is_empty(), "{:#?}", r.findings);
    for keep in [180u64, 100, 3, 0] {
        v.truncate(fid, keep).unwrap();
        let r = v.verify().unwrap();
        assert!(r.findings.is_empty(), "truncated to {keep}: {:#?}", r.findings);
        assert_eq!(read_content(&mut v, fid), data[..(keep as usize * BLOCK).min(data.len())]);
    }
    assert_eq!(v.stat(fid).unwrap().headers, 1);
    write_content(&mut v, fid, &data[..200 * BLOCK]).unwrap();
    v.delete(ods_core::MFD, b"BIG.DAT", 1).unwrap();
    assert!(v.verify().unwrap().findings.is_empty());
}

/// The same, cut short after every write.
#[test]
fn extension_headers_survive_crashes() {
    let start = fragmented(3000).dismount().unwrap();
    let data = vec![0x5a; 180 * BLOCK];
    let run = |dev: Mem| -> (Mem, bool) {
        let mut v = Volume::mount(dev, true).unwrap();
        v.set_clock(clock);
        let ok = (|| {
            let (fid, _) = v.create(ods_core::MFD, b"BIG.DAT", None, &NewFile::default())?;
            write_content(&mut v, fid, &data)?;
            v.truncate(fid, 20)?;
            v.delete(ods_core::MFD, b"BIG.DAT", 1)
        })()
        .is_ok();
        (v.dismount().unwrap(), ok)
    };
    let (end, ok) = run(start.clone());
    assert!(ok);
    let total = end.writes - start.writes;
    for n in 0..total {
        let mut dev = start.clone();
        dev.fail_after = Some(start.writes + n);
        let (mut dev, _) = run(dev);
        dev.fail_after = None;
        let mut v = Volume::mount(dev, false).unwrap();
        let errors: Vec<_> =
            v.verify().unwrap().findings.into_iter().filter(|f| f.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "crash after {n} of {total} writes: {errors:#?}");
    }
}
