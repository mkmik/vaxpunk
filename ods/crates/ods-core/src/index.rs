//! INDEXF.SYS: file numbers, sequence numbers and header slots.

use crate::bitmap::bit;
use crate::layout::{Header, Pointer};
use crate::volume::{INDEXF, map_vbn};
use crate::{Alloc, BlockDevice, Error, Fid, Result, Volume, fch};

impl<D: BlockDevice> Volume<D> {
    /// Writes a header after fixing its checksum. The index file's own
    /// header also goes to its backup copy.
    pub(crate) fn write_header(&mut self, lbn: u64, h: &mut Header) -> Result<(), D::Error> {
        h.update_checksum();
        self.write_block(lbn, &h.0)?;
        if h.fid() == INDEXF && h.seg_num() == 0 {
            let alt = self.home.altidxlbn() as u64;
            self.write_block(alt, &h.0)?;
        }
        Ok(())
    }

    /// Takes a file number: the first one past the reserved files that is
    /// clear in the index file bitmap and whose slot holds no valid header.
    /// The sequence number goes one past the slot's previous one, so file
    /// IDs of deleted files never come back to life. The index file's end
    /// of file moves past the slot and the bitmap bit is set before the
    /// caller writes the header.
    pub(crate) fn alloc_header(&mut self) -> Result<(Fid, u64), D::Error> {
        let max = self.home.maxfiles();
        let (_, bits) = self.read_index_bitmap()?;
        let mut num = self.home.resfiles() as u32;
        loop {
            num += 1;
            if num > max {
                return Err(Error::HeaderFull);
            }
            if num & 0xffff == 0 || bit(&bits, num as u64 - 1) {
                continue;
            }
            let vbn = self.home.header_vbn0() as u64 + num as u64 - 1;
            if map_vbn(&self.index_map, vbn).is_none() {
                self.extend_index(vbn)?;
            }
            let lbn = self.header_lbn(num).ok_or(Error::Corrupt { what: "index file extension failed", lbn: 0 })?;
            let (ihs, ih) = self.index_header()?;
            let eof = ih.record_attrs().efblk as u64;
            let old = Header(self.read_block(lbn)?);
            if old.invalid().is_none() {
                continue; // a live header the bitmap forgot: leave it alone
            }
            // Past the end of file a slot was never used, unless it holds a
            // deleted header anyway; otherwise the old sequence word is
            // still there, even in a damaged header.
            let seq = if vbn >= eof && !old.is_deleted() {
                1
            } else {
                match old.fid().seq.wrapping_add(1) {
                    0 => 1,
                    s => s,
                }
            };
            if vbn >= eof {
                let mut ih = ih;
                let mut ra = ih.record_attrs();
                ra.efblk = vbn as u32 + 1;
                ra.ffbyte = 0;
                ih.set_record_attrs(&ra);
                self.write_header(ihs, &mut ih)?;
            }
            self.set_index_bit(num, true)?;
            return Ok((Fid::new(num, seq), lbn));
        }
    }

    fn index_header(&mut self) -> Result<(u64, Header), D::Error> {
        let lbn = self.header_lbn(1).unwrap_or(0);
        Ok((lbn, self.read_header_at(lbn, Some(INDEXF))?))
    }

    /// Grows INDEXF.SYS so that it maps `vbn`, by at least half again its
    /// header space so growth stays rare.
    fn extend_index(&mut self, vbn: u64) -> Result<(), D::Error> {
        let have: u64 = self.index_map.iter().map(|r| r.count).sum();
        let headers = have.saturating_sub(self.home.header_vbn0() as u64 - 1);
        let grow = (vbn - have).max(headers / 2).max(16);
        let after = self.index_map.last().map(|r| r.lbn + r.count);
        let runs = self.allocate(grow, Alloc::Any, after)?;
        self.append_runs(INDEXF, &runs)?;
        self.index_map.extend(runs);
        crate::volume::merge_runs(&mut self.index_map);
        Ok(())
    }

    /// Turns a header into a deleted one (it keeps its sequence number for
    /// the next user of the slot) and frees its file number.
    pub(crate) fn delete_header(&mut self, lbn: u64, mut h: Header) -> Result<(), D::Error> {
        let num = h.fid().num;
        h.set_filechar(h.filechar() | fch::MARKDEL);
        let seq = h.fid().seq;
        h.set_fid(Fid { num: 0, seq, rvn: 0 });
        h.set_checksum(0);
        self.write_block(lbn, &h.0)?;
        self.set_index_bit(num, false)
    }

    /// A fresh header for `fid`: offsets for an ident area sized to `name`
    /// (none for extension headers), no map, no ACL.
    pub(crate) fn new_header(&self, fid: Fid, name_len: Option<usize>) -> Header {
        let level = self.level.number();
        let mut h = Header::default();
        let id = 40;
        let mp = match name_len {
            Some(n) => id + Header::ident_words(level, n),
            None => id,
        };
        h.set_idoffset(id);
        h.set_mpoffset(mp);
        h.set_acoffset(0xff);
        h.set_rsoffset(0xff);
        h.set_struclev((level as u16) << 8 | 1);
        h.set_fid(fid);
        h
    }
}

/// Encodes runs as the smallest pointers that hold them, splitting runs too
/// long for one pointer.
pub(crate) fn pointers(runs: &[crate::volume::Run]) -> alloc::vec::Vec<Pointer> {
    let mut out = alloc::vec::Vec::new();
    for r in runs {
        let (mut lbn, mut left) = (r.lbn, r.count);
        while left > 0 {
            let n = left.min(0x4000_0000);
            if let Some(p) = Pointer::extent(n as u32, lbn as u32) {
                out.push(p);
            }
            lbn += n;
            left -= n;
        }
    }
    out
}
