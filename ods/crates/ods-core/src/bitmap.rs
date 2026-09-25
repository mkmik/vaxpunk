//! BITMAP.SYS: the storage control block and one bit per cluster, set when
//! the cluster is free. Also the index file bitmap, one bit per file number,
//! set when the number is in use.

use alloc::vec::Vec;

use crate::layout::{BLOCK, Scb};
use crate::volume::{Run, map_vbn};
use crate::{Alloc, BlockDevice, Error, Fid, Result, Volume};

/// File ID of BITMAP.SYS.
pub(crate) const BITMAP: Fid = Fid::new(2, 2);

/// Bits per bitmap block.
pub(crate) const BITS: u64 = BLOCK as u64 * 8;

impl<D: BlockDevice> Volume<D> {
    pub fn scb(&mut self) -> Result<Scb, D::Error> {
        let map = self.file_map(BITMAP)?;
        let lbn = map_vbn(&map, 1).ok_or(Error::Corrupt { what: "empty storage bitmap file", lbn: 0 })?.0;
        let scb = Scb(self.read_block(lbn)?);
        if !scb.checksum_ok() {
            return Err(Error::Corrupt { what: "bad storage control block checksum", lbn });
        }
        Ok(scb)
    }

    /// Clusters the storage bitmap describes; a partial last cluster counts.
    pub(crate) fn clusters(&self) -> u64 {
        self.volume_size().div_ceil(self.cluster())
    }

    /// Volume size in blocks: the device size, or less if the volume was
    /// made for a smaller one.
    pub fn volume_size(&self) -> u64 {
        self.vol_blocks
    }

    /// The storage bitmap: its map (from VBN 2 on) and the bits.
    pub(crate) fn read_storage_bitmap(&mut self) -> Result<(Vec<Run>, Vec<u8>), D::Error> {
        let map = self.file_map(BITMAP)?;
        let blocks = self.clusters().div_ceil(BITS);
        let mut bits = Vec::with_capacity(blocks as usize * BLOCK);
        let mut runs = Vec::new();
        for vbn in 2..2 + blocks {
            let lbn =
                map_vbn(&map, vbn).ok_or(Error::Corrupt { what: "storage bitmap shorter than the volume", lbn: 0 })?.0;
            bits.extend_from_slice(&self.read_block(lbn)?);
            runs.push(Run { lbn, count: 1 });
        }
        Ok((runs, bits))
    }

    /// Free blocks, according to the storage bitmap.
    pub fn free_blocks(&mut self) -> Result<u64, D::Error> {
        let clusters = self.clusters();
        let (_, bits) = self.read_storage_bitmap()?;
        let free = (0..clusters).filter(|&c| bit(&bits, c)).count() as u64;
        Ok(free * self.cluster())
    }

    /// The index file bitmap and the LBN of each of its blocks.
    pub(crate) fn read_index_bitmap(&mut self) -> Result<(Vec<u64>, Vec<u8>), D::Error> {
        let (vbn0, size) = (self.home.ibmapvbn() as u64, self.home.ibmapsize() as u64);
        let mut lbns = Vec::new();
        let mut bits = Vec::new();
        for vbn in vbn0..vbn0 + size {
            let lbn =
                map_vbn(&self.index_map, vbn).ok_or(Error::Corrupt { what: "index file bitmap not mapped", lbn: 0 })?.0;
            bits.extend_from_slice(&self.read_block(lbn)?);
            lbns.push(lbn);
        }
        Ok((lbns, bits))
    }

    /// File numbers in use, according to the index file bitmap.
    pub fn files_in_use(&mut self) -> Result<u64, D::Error> {
        let max = self.home.maxfiles() as u64;
        let (_, bits) = self.read_index_bitmap()?;
        Ok((0..max).filter(|&i| bit(&bits, i)).count() as u64)
    }

    /// Allocates `blocks` (rounded up to whole clusters) and marks them in
    /// the storage bitmap before anyone can reference them. `after` is the
    /// LBN just past the file's last run: allocation starts there when it
    /// can, so the new space merges with the old. Returns runs in LBN order
    /// of use.
    pub(crate) fn allocate(&mut self, blocks: u64, how: Alloc, after: Option<u64>) -> Result<Vec<Run>, D::Error> {
        let v = self.cluster();
        let want = blocks.div_ceil(v);
        if want == 0 {
            return Ok(Vec::new());
        }
        let (bmap, mut bits) = self.read_storage_bitmap()?;
        let clusters = self.clusters();
        // Free runs of clusters, in LBN order.
        let mut free = Vec::new();
        let mut c = 0;
        while c < clusters {
            if bit(&bits, c) {
                let start = c;
                while c < clusters && bit(&bits, c) {
                    c += 1;
                }
                free.push((start, c - start));
            } else {
                c += 1;
            }
        }
        let next = after.filter(|a| a % v == 0).map(|a| a / v);
        let adjacent = next.and_then(|n| free.iter().position(|&(s, _)| s == n));
        let mut take: Vec<(u64, u64)> = Vec::new();
        match how {
            Alloc::Contiguous => {
                let fits = |&(_, len): &(u64, u64)| len >= want;
                let i = match (next, adjacent) {
                    (Some(_), Some(i)) if fits(&free[i]) => i,
                    (Some(_), _) => return Err(Error::DeviceFull),
                    _ => free.iter().position(fits).ok_or(Error::DeviceFull)?,
                };
                take.push((free[i].0, want));
            }
            Alloc::BestTry | Alloc::Any => {
                let mut order: Vec<usize> = (0..free.len()).collect();
                if how == Alloc::BestTry {
                    order.sort_by_key(|&i| core::cmp::Reverse(free[i].1));
                }
                if let Some(i) = adjacent {
                    order.retain(|&j| j != i);
                    order.insert(0, i);
                }
                let mut left = want;
                for i in order {
                    if left == 0 {
                        break;
                    }
                    let n = free[i].1.min(left);
                    take.push((free[i].0, n));
                    left -= n;
                }
                if left > 0 {
                    return Err(Error::DeviceFull);
                }
            }
        }
        for &(start, n) in &take {
            for c in start..start + n {
                set_bit(&mut bits, c, false);
            }
        }
        self.write_bitmap_blocks(&bmap, &bits, &take)?;
        Ok(take.iter().map(|&(s, n)| Run { lbn: s * v, count: n * v }).collect())
    }

    /// Returns runs to the storage bitmap. Every cluster must be allocated.
    pub(crate) fn release(&mut self, runs: &[Run]) -> Result<(), D::Error> {
        if runs.is_empty() {
            return Ok(());
        }
        let v = self.cluster();
        let (bmap, mut bits) = self.read_storage_bitmap()?;
        let mut touched = Vec::new();
        for r in runs.iter().filter(|r| r.count > 0) {
            let (first, last) = (r.lbn / v, (r.lbn + r.count - 1) / v);
            for c in first..=last.min(self.clusters().saturating_sub(1)) {
                if bit(&bits, c) {
                    return Err(Error::Corrupt { what: "freeing a cluster that is already free", lbn: c * v });
                }
                set_bit(&mut bits, c, true);
            }
            touched.push((first, last + 1 - first));
        }
        self.write_bitmap_blocks(&bmap, &bits, &touched)
    }

    /// Writes the bitmap blocks covering the given cluster ranges.
    fn write_bitmap_blocks(&mut self, bmap: &[Run], bits: &[u8], ranges: &[(u64, u64)]) -> Result<(), D::Error> {
        let mut blocks: Vec<u64> = Vec::new();
        for &(start, n) in ranges {
            for b in start / BITS..=(start + n - 1) / BITS {
                if !blocks.contains(&b) {
                    blocks.push(b);
                }
            }
        }
        blocks.sort_unstable();
        for b in blocks {
            let Some(r) = bmap.get(b as usize) else { continue };
            let mut blk = [0u8; BLOCK];
            blk.copy_from_slice(&bits[b as usize * BLOCK..(b as usize + 1) * BLOCK]);
            self.write_block(r.lbn, &blk)?;
        }
        Ok(())
    }

    /// Sets or clears one bit of the index file bitmap.
    pub(crate) fn set_index_bit(&mut self, num: u32, on: bool) -> Result<(), D::Error> {
        let i = num as u64 - 1;
        let vbn = self.home.ibmapvbn() as u64 + i / BITS;
        let lbn =
            map_vbn(&self.index_map, vbn).ok_or(Error::Corrupt { what: "index file bitmap not mapped", lbn: 0 })?.0;
        let mut b = self.read_block(lbn)?;
        set_bit(&mut b, i % BITS, on);
        self.write_block(lbn, &b)
    }
}

pub(crate) fn bit(bits: &[u8], i: u64) -> bool {
    bits.get((i / 8) as usize).is_some_and(|b| b >> (i % 8) & 1 != 0)
}

pub(crate) fn set_bit(bits: &mut [u8], i: u64, on: bool) {
    if let Some(b) = bits.get_mut((i / 8) as usize) {
        let m = 1 << (i % 8);
        *b = if on { *b | m } else { *b & !m };
    }
}
