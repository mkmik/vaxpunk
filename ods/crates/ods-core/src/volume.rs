//! Mounting and volume-level state: the home block, the index file map,
//! block I/O and reading file headers.

use alloc::vec::Vec;

use crate::layout::{BLOCK, Block, Header, HomeBlock, Pointer, decode_map};
use crate::{BlockDevice, Error, Fid, Result};

/// Structure level of a volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Ods2,
    Ods5,
}

impl Level {
    pub fn number(self) -> u8 {
        match self {
            Level::Ods2 => 2,
            Level::Ods5 => 5,
        }
    }
}

/// A run of consecutive logical blocks mapped by a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub lbn: u64,
    pub count: u64,
}

/// Joins runs that continue one another.
pub(crate) fn merge_runs(runs: &mut Vec<Run>) {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for r in runs.drain(..) {
        match out.last_mut() {
            Some(l) if l.lbn + l.count == r.lbn => l.count += r.count,
            _ => out.push(r),
        }
    }
    *runs = out;
}

/// Finds `vbn` (1-based) in a map: the LBN and how many blocks follow it in
/// the same run.
pub(crate) fn map_vbn(map: &[Run], vbn: u64) -> Option<(u64, u64)> {
    let mut first = 1;
    for r in map {
        if vbn < first + r.count {
            let off = vbn.checked_sub(first)?;
            return Some((r.lbn + off, r.count - off));
        }
        first += r.count;
    }
    None
}

/// A mounted volume. All operations go through the device on every call:
/// nothing is cached except the home block and the index file's map.
pub struct Volume<D: BlockDevice> {
    pub(crate) dev: D,
    pub(crate) home: HomeBlock,
    pub(crate) level: Level,
    pub(crate) writable: bool,
    pub(crate) index_map: Vec<Run>,
    /// Volume size from the SCB, at most the device size.
    pub(crate) vol_blocks: u64,
    pub(crate) clock: fn() -> u64,
}

fn no_clock() -> u64 {
    0
}

impl<D: BlockDevice> Volume<D> {
    /// Mounts the volume on `dev`. Writes need `writable`.
    pub fn mount(mut dev: D, writable: bool) -> Result<Self, D::Error> {
        if dev.block_size() != BLOCK {
            return Err(Error::Unsupported("block size other than 512"));
        }
        let home = find_home(&mut dev)?;
        let level = if home.level() == 5 { Level::Ods5 } else { Level::Ods2 };
        let vol_blocks = dev.block_count();
        let mut vol = Volume { dev, home, level, writable, index_map: Vec::new(), vol_blocks, clock: no_clock };
        if home.rvn() > 1 || home.setcount() > 1 {
            return Err(Error::Unsupported("volume sets"));
        }
        // The first 16 headers sit right after the index file bitmap, so the
        // index file's own header needs no map to be found.
        let lbn = home.ibmaplbn() as u64 + home.ibmapsize() as u64;
        vol.index_map = Vec::from([Run { lbn, count: 1 }]);
        let hdr = vol.read_header_at(lbn, Some(INDEXF))?;
        vol.index_map = vol.header_runs(&hdr, lbn)?;
        // Extension headers of the index file are found through the part of
        // its map read so far.
        let mut h = hdr;
        let mut guard = 0u32;
        while !h.ext_fid().is_zero() {
            guard += 1;
            let lbn = vol
                .header_lbn(h.ext_fid().num)
                .ok_or(Error::Corrupt { what: "index file extension outside the index file", lbn })?;
            let ext = vol.read_header_at(lbn, Some(h.ext_fid()))?;
            if ext.seg_num() as u32 != guard {
                return Err(Error::Corrupt { what: "extension header out of sequence", lbn });
            }
            let runs = vol.header_runs(&ext, lbn)?;
            vol.index_map.extend(runs);
            h = ext;
        }
        let scb = vol.scb()?;
        if scb.volsize() == 0 || scb.cluster() != home.cluster() {
            return Err(Error::Corrupt { what: "storage control block disagrees with the home block", lbn: 0 });
        }
        vol.vol_blocks = vol.vol_blocks.min(scb.volsize() as u64);
        Ok(vol)
    }

    /// Supplies the clock used to date new files: VMS time, 100 ns units
    /// since 17-Nov-1858. Without one, dates are zero.
    pub fn set_clock(&mut self, clock: fn() -> u64) {
        self.clock = clock;
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn home(&self) -> &HomeBlock {
        &self.home
    }

    pub fn cluster(&self) -> u64 {
        self.home.cluster() as u64
    }

    pub fn is_writable(&self) -> bool {
        self.writable
    }

    pub fn device(&mut self) -> &mut D {
        &mut self.dev
    }

    /// Flushes the device and gives it back.
    pub fn dismount(mut self) -> Result<D, D::Error> {
        self.dev.flush()?;
        Ok(self.dev)
    }

    pub fn read_block(&mut self, lbn: u64) -> Result<Block, D::Error> {
        let mut b = [0u8; BLOCK];
        if lbn >= self.dev.block_count() {
            return Err(Error::Corrupt { what: "block beyond the end of the volume", lbn });
        }
        self.dev.read(lbn, &mut b)?;
        Ok(b)
    }

    pub(crate) fn write_block(&mut self, lbn: u64, b: &Block) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        if lbn >= self.dev.block_count() {
            return Err(Error::Corrupt { what: "write beyond the end of the volume", lbn });
        }
        Ok(self.dev.write(lbn, b)?)
    }

    /// LBN of the header for file number `num`, if the index file maps it.
    pub fn header_lbn(&self, num: u32) -> Option<u64> {
        if num == 0 || num > self.home.maxfiles() {
            return None;
        }
        map_vbn(&self.index_map, self.home.header_vbn0() as u64 + num as u64 - 1).map(|(lbn, _)| lbn)
    }

    /// Reads and validates the header at `lbn`. With `want`, it must be that
    /// file's header (a zero sequence number matches any).
    pub(crate) fn read_header_at(&mut self, lbn: u64, want: Option<Fid>) -> Result<Header, D::Error> {
        let h = Header(self.read_block(lbn)?);
        if let Some(what) = h.invalid() {
            return Err(match want {
                Some(fid) if h.is_deleted() || h.fid().num == 0 => Error::Stale(fid),
                _ => Error::Corrupt { what, lbn },
            });
        }
        if let Some(fid) = want
            && (h.fid().num != fid.num || fid.seq != 0 && h.fid().seq != fid.seq)
        {
            return Err(Error::Stale(fid));
        }
        Ok(h)
    }

    /// Reads the primary header of a file.
    pub fn read_header(&mut self, fid: Fid) -> Result<Header, D::Error> {
        if fid.rvn > 1 {
            return Err(Error::Unsupported("relative volume numbers"));
        }
        let lbn = self.header_lbn(fid.num).ok_or(Error::Stale(fid))?;
        let h = self.read_header_at(lbn, Some(fid))?;
        if h.seg_num() != 0 {
            return Err(Error::Invalid("extension header, not a file"));
        }
        Ok(h)
    }

    /// All headers of a file, primary first, following the extension chain.
    pub fn headers(&mut self, fid: Fid) -> Result<Vec<(u64, Header)>, D::Error> {
        let first = self.read_header(fid)?;
        let mut hs = Vec::from([(self.header_lbn(fid.num).unwrap_or(0), first)]);
        loop {
            let (lbn, h) = hs[hs.len() - 1];
            let next = h.ext_fid();
            if next.is_zero() {
                return Ok(hs);
            }
            let at = self
                .header_lbn(next.num)
                .ok_or(Error::Corrupt { what: "extension header outside the index file", lbn })?;
            let ext = self.read_header_at(at, Some(next))?;
            if ext.seg_num() != h.seg_num().wrapping_add(1) || ext.seg_num() == 0 {
                return Err(Error::Corrupt { what: "extension header out of sequence", lbn: at });
            }
            hs.push((at, ext));
        }
    }

    /// Runs mapped by one header, checked against the volume size. The last
    /// cluster may stick out: INITIALIZE gives a partial one to BADBLK.SYS.
    pub(crate) fn header_runs(&self, h: &Header, lbn: u64) -> Result<Vec<Run>, D::Error> {
        let bad = |what| Error::Corrupt { what, lbn };
        let size = self.vol_blocks.next_multiple_of(self.cluster());
        let mut runs = Vec::new();
        for p in decode_map(h.map_area()).map_err(bad)? {
            if let Pointer::Extent { count, lbn: start, .. } = p {
                if start == u32::MAX {
                    return Err(Error::Unsupported("sparse files"));
                }
                let (start, count) = (start as u64, count as u64);
                if start + count > size {
                    return Err(bad("map pointer beyond the end of the volume"));
                }
                runs.push(Run { lbn: start, count });
            }
        }
        Ok(runs)
    }

    /// The whole map of a file, in VBN order.
    pub(crate) fn file_map(&mut self, fid: Fid) -> Result<Vec<Run>, D::Error> {
        let mut map = Vec::new();
        for (lbn, h) in self.headers(fid)? {
            map.extend(self.header_runs(&h, lbn)?);
        }
        Ok(map)
    }
}

/// File ID of INDEXF.SYS.
pub(crate) const INDEXF: Fid = Fid::new(1, 1);

/// Reads the primary home block, falling back to the first valid copy along
/// the search sequence. The geometry that fixes the search delta is unknown
/// here, so every block is a candidate.
fn find_home<D: BlockDevice>(dev: &mut D) -> Result<HomeBlock, D::Error> {
    let mut b = [0u8; BLOCK];
    for lbn in 1..dev.block_count().min(1 << 16) {
        dev.read(lbn, &mut b)?;
        let h = HomeBlock(b);
        if h.invalid().is_none() && h.homelbn() as u64 == lbn {
            return Ok(h);
        }
    }
    Err(Error::NoHomeBlock)
}
