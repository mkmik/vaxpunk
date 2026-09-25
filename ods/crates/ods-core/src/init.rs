//! INITIALIZE: a new, empty volume on a blank device.
//!
//! The result copies what OpenVMS INITIALIZE writes (checked against a
//! volume VMS 7.1 initialized): the same reserved files with the same
//! attributes, the home block copies filling the index file's first
//! clusters, and the index file, storage bitmap and MFD in the middle of the
//! volume. See docs/initialize.md.

use alloc::vec;
use alloc::vec::Vec;

use crate::index::pointers;
use crate::layout::{BLOCK, Block, DirBlock, DirRecord, Header, HomeBlock, Ident, NameType, Scb, encode_map};
use crate::volume::Run;
use crate::{BlockDevice, Error, Fid, Level, RecordAttrs, Result, Volume, fch, rat, rfm};

/// Parameters for [`initialize`]. Zero means "the default".
#[derive(Clone, Debug)]
pub struct InitParams {
    /// Volume label, 1 to 12 printable ASCII characters.
    pub label: Vec<u8>,
    pub level: Level,
    /// Blocks per cluster. Default: 1 up to 50,000 blocks, else 3, and more
    /// if needed to keep the storage bitmap within 255 blocks.
    pub cluster: u16,
    /// Default: volume size / ((cluster + 1) * 2), as VMS computes it.
    pub max_files: u32,
    /// File headers to preallocate. Default and minimum: 16.
    pub headers: u32,
    /// Volume owner UIC.
    pub owner: u32,
    pub owner_name: Vec<u8>,
    /// Volume protection (a set bit denies, as for files).
    pub protection: u16,
    /// Default protection of new files.
    pub file_protection: u16,
    /// Creation time, VMS format.
    pub now: u64,
}

impl Default for InitParams {
    fn default() -> Self {
        InitParams {
            label: Vec::new(),
            level: Level::Ods2,
            cluster: 0,
            max_files: 0,
            headers: 0,
            owner: 0x0001_0004,
            owner_name: Vec::new(),
            protection: 0,
            file_protection: 0xfa00,
            now: 0,
        }
    }
}

/// The reserved files, in file number order: name and record size. All
/// are fixed-length record files except the MFD.
const RESERVED: [(&[u8], u16); 9] = [
    (b"INDEXF.SYS", 512),
    (b"BITMAP.SYS", 512),
    (b"BADBLK.SYS", 512),
    (b"000000.DIR", 512),
    (b"CORIMG.SYS", 512),
    (b"VOLSET.SYS", 64),
    (b"CONTIN.SYS", 512),
    (b"BACKUP.SYS", 64),
    (b"BADLOG.SYS", 16),
];

fn padded(s: &[u8]) -> [u8; 12] {
    let mut a = [b' '; 12];
    a[..s.len().min(12)].copy_from_slice(&s[..s.len().min(12)]);
    a
}

/// Hands out clusters, first fit from a starting point.
struct Clusters(Vec<bool>);

impl Clusters {
    fn take(&mut self, from: u64, n: u64) -> Option<u64> {
        let len = self.0.len() as u64;
        let fits = |s: u64| s + n <= len && (s..s + n).all(|c| !self.0[c as usize]);
        let start = (from..len).chain(0..from).find(|&s| fits(s))?;
        for c in start..start + n {
            self.0[c as usize] = true;
        }
        Some(start)
    }
}

/// Builds a new volume on `dev`, which is entirely overwritten where the
/// structures go, and mounts it for writing.
pub fn initialize<D: BlockDevice>(mut dev: D, p: &InitParams) -> Result<Volume<D>, D::Error> {
    if dev.block_size() != BLOCK {
        return Err(Error::Unsupported("block size other than 512"));
    }
    let size = dev.block_count();
    if size < 100 {
        return Err(Error::Invalid("a volume needs at least 100 blocks"));
    }
    if size > u32::MAX as u64 {
        return Err(Error::Unsupported("volumes of 2^32 blocks or more"));
    }
    if p.label.is_empty() || p.label.len() > 12 || !p.label.iter().all(|c| (0x20..0x7f).contains(c)) {
        return Err(Error::Invalid("volume label must be 1 to 12 printable characters"));
    }
    let v = match p.cluster {
        0 => (if size > 50_000 { 3 } else { 1 }).max(size.div_ceil(255 * 4096)),
        c => c as u64,
    };
    if v > size / 50 || v > 0x4000 {
        return Err(Error::Invalid("cluster factor too large for the volume"));
    }
    let clusters = size.div_ceil(v);
    let bitmap_blocks = clusters.div_ceil(4096);
    let resfiles = RESERVED.len() as u32;
    let max_files = match p.max_files {
        0 => (size / ((v + 1) * 2)) as u32,
        n => n,
    }
    .clamp(resfiles + 16, (1 << 24) - 1);
    let ibmap_blocks = max_files.div_ceil(4096) as u64;
    let headers = p.headers.max(16) as u64;

    // Synthetic geometry, sectors x 1 x cylinders: the home block search
    // delta is sectors + 1, which puts the backup home block past the first
    // two clusters.
    let sectors = (2 * v).max(32);
    let alhome = 1 + sectors + 1;
    if alhome >= size / 2 {
        return Err(Error::Invalid("volume too small for this cluster factor"));
    }

    let mut used = Clusters(vec![false; clusters as usize]);
    let none = || Error::DeviceFull;
    used.take(0, 2).ok_or_else(none)?; // boot block, home block and copies
    used.take(alhome / v, 1).ok_or_else(none)?;
    let partial = !size.is_multiple_of(v);
    if partial {
        used.take(clusters - 1, 1).ok_or_else(none)?;
    }
    let mid = clusters / 2;
    let mfd = used.take(mid, 1).ok_or_else(none)? * v;
    let bitmap = used.take(mid, (1 + bitmap_blocks).div_ceil(v)).ok_or_else(none)? * v;
    let index_blocks = (ibmap_blocks + headers).next_multiple_of(v);
    let ibmap = used.take(mid, index_blocks / v).ok_or_else(none)? * v;
    let altidx = used.take(mid, 1).ok_or_else(none)? * v;
    let first_header = ibmap + ibmap_blocks;

    let level = p.level.number() as u16;
    let mut home = HomeBlock::default();
    home.set_alhomelbn(alhome as u32);
    home.set_altidxlbn(altidx as u32);
    home.set_struclev(level << 8 | 1);
    home.set_cluster(v as u16);
    home.set_alhomevbn((2 * v + 1 + alhome % v) as u16);
    home.set_altidxvbn((3 * v + 1) as u16);
    home.set_ibmapvbn((4 * v + 1) as u16);
    home.set_ibmaplbn(ibmap as u32);
    home.set_maxfiles(max_files);
    home.set_ibmapsize(ibmap_blocks as u16);
    home.set_resfiles(resfiles as u16);
    home.set_volowner(p.owner);
    home.set_protect(p.protection);
    home.set_fileprot(p.file_protection);
    home.set_recprot(0xfe00);
    home.set_credate(p.now);
    home.set_window(7);
    home.set_lru_lim(3);
    home.set_extend(5);
    home.set_strucname([b' '; 12]);
    home.set_volname(padded(&p.label));
    home.set_ownername(padded(&p.owner_name));
    home.set_format(padded(b"DECFILE11B"));

    // The index file: boot and home clusters, backup home cluster, backup
    // header cluster, then bitmap and headers.
    let index_runs = [
        Run { lbn: 0, count: 2 * v },
        Run { lbn: alhome / v * v, count: v },
        Run { lbn: altidx, count: v },
        Run { lbn: ibmap, count: index_blocks },
    ];
    let bitmap_run = Run { lbn: bitmap, count: (1 + bitmap_blocks).next_multiple_of(v) };
    let mfd_run = Run { lbn: mfd, count: v };
    let bad_runs: Vec<Run> = if partial { vec![Run { lbn: (clusters - 1) * v, count: v }] } else { vec![] };

    let mut hdrs = Vec::new();
    for (i, &(name, rsize)) in RESERVED.iter().enumerate() {
        let num = i as u32 + 1;
        let (runs, efblk, fc): (&[Run], u64, u32) = match num {
            1 => (&index_runs, 4 * v + ibmap_blocks + resfiles as u64 + 1, 0),
            2 => (core::slice::from_ref(&bitmap_run), bitmap_blocks + 2, fch::CONTIG),
            3 => (&bad_runs, if partial { v + 1 } else { 1 }, 0),
            4 => (core::slice::from_ref(&mfd_run), 2, fch::CONTIG | fch::DIRECTORY),
            _ => (&[], 1, 0),
        };
        let hiblk: u64 = runs.iter().map(|r| r.count).sum();
        let mut name_v = name.to_vec();
        name_v.extend_from_slice(b";1");
        // Reserved files keep level 2 headers even on ODS-5, as VMS does.
        let mut h = Header::default();
        h.set_idoffset(40);
        h.set_mpoffset(40 + Header::ident_words(2, name_v.len()));
        h.set_acoffset(0xff);
        h.set_rsoffset(0xff);
        h.set_struclev(0x0201);
        h.set_fid(Fid::new(num, num as u16));
        h.set_filechar(fc);
        h.set_recprot(0xfe00);
        h.set_fileowner(p.owner);
        h.set_fileprot(if num == 4 { p.file_protection & !0x4000 } else { p.file_protection });
        h.set_backlink(Fid::new(4, 4));
        h.set_record_attrs(&RecordAttrs {
            rtype: if num == 4 { rfm::VAR } else { rfm::FIX },
            rattrib: if num == 4 { rat::BLK } else { 0 },
            rsize,
            maxrec: rsize,
            hiblk: hiblk as u32,
            efblk: efblk as u32,
            ..RecordAttrs::default()
        });
        h.set_highwater(if num == 4 { 2 } else { hiblk as u32 + 1 });
        h.set_ident(&Ident {
            name: name_v,
            name_type: NameType::Ods2,
            revision: 1,
            credate: p.now,
            revdate: p.now,
            ..Ident::default()
        });
        let mut m = Vec::new();
        encode_map(&pointers(runs), &mut m);
        if !h.set_map(&m) {
            return Err(Error::Invalid("map pointers overflow the header"));
        }
        h.update_checksum();
        hdrs.push(h);
    }

    let zero = [0u8; BLOCK];
    let mut put = |lbn: u64, b: &Block| dev.write(lbn, b);

    // Index file bitmap and headers; unused preallocated slots are zeroed.
    let mut ibits = vec![0u8; ibmap_blocks as usize * BLOCK];
    for n in 0..resfiles as usize {
        ibits[n / 8] |= 1 << (n % 8);
    }
    for (i, chunk) in ibits.chunks(BLOCK).enumerate() {
        put(ibmap + i as u64, chunk.try_into().unwrap_or(&zero))?;
    }
    for n in 0..index_blocks - ibmap_blocks {
        put(first_header + n, hdrs.get(n as usize).map_or(&zero, |h| &h.0))?;
    }
    put(altidx, &hdrs[0].0)?;
    for n in 1..v {
        put(altidx + n, &zero)?;
    }

    // Storage bitmap: a set bit is a free cluster.
    let mut scb = Scb::default();
    scb.set_struclev(level << 8 | 1);
    scb.set_cluster(v as u16);
    scb.set_volsize(size as u32);
    scb.set_blksize(1);
    scb.set_sectors(sectors as u32);
    scb.set_tracks(1);
    scb.set_cylinders((size / sectors) as u32);
    scb.set_volockname(padded(&p.label));
    scb.update_checksum();
    put(bitmap, &scb.0)?;
    let mut bits = vec![0u8; bitmap_blocks as usize * BLOCK];
    for (c, &u) in used.0.iter().enumerate() {
        if !u {
            bits[c / 8] |= 1 << (c % 8);
        }
    }
    for (i, chunk) in bits.chunks(BLOCK).enumerate() {
        put(bitmap + 1 + i as u64, chunk.try_into().unwrap_or(&zero))?;
    }
    for n in 1 + bitmap_blocks..bitmap_run.count {
        put(bitmap + n, &zero)?;
    }

    // The MFD lists the reserved files, in name order.
    let mut recs: Vec<DirRecord> = RESERVED
        .iter()
        .enumerate()
        .map(|(i, &(name, _))| DirRecord {
            name: name.to_vec(),
            verlimit: 1,
            flags: 0,
            entries: vec![(1, Fid::new(i as u32 + 1, i as u16 + 1))],
            pad: 0,
        })
        .collect();
    recs.sort_by(|a, b| a.name.cmp(&b.name));
    let mfd_block = DirBlock { records: recs, tail: Vec::new() }.to_block().ok_or(Error::Invalid("MFD overflow"))?;
    put(mfd, &mfd_block)?;
    for n in 1..v {
        put(mfd + n, &zero)?;
    }

    // Boot block, then the home block copies: the rest of the first two
    // clusters and all of the backup cluster. Each names its own LBN and
    // VBN. The primary goes last, so an interrupted INITIALIZE leaves no
    // volume rather than half of one.
    put(0, &zero)?;
    let copy = |lbn: u64, vbn: u64| {
        let mut h = home;
        h.set_homelbn(lbn as u32);
        h.set_homevbn(vbn as u16);
        h.update_checksums();
        h.0
    };
    for lbn in 2..2 * v {
        put(lbn, &copy(lbn, lbn + 1))?;
    }
    let bh = alhome / v * v;
    for n in 0..v {
        put(bh + n, &copy(bh + n, 2 * v + 1 + n))?;
    }
    put(1, &copy(1, 2))?;
    dev.flush()?;
    Volume::mount(dev, true)
}
