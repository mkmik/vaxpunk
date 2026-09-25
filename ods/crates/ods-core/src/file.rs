//! Open files: virtual to logical block mapping, reads and writes of
//! virtual blocks, extend, truncate and attributes.
//!
//! Map changes happen only at the end of a file's map, so each one is safe
//! to interrupt: extending appends pointers to the last header (a new
//! extension header is written before it is linked), truncating unlinks and
//! shortens before it frees.

use alloc::vec::Vec;

use crate::index::pointers;
use crate::layout::{BLOCK, Header, NameType, Pointer, decode_map, encode_map};
use crate::volume::{Run, map_vbn, merge_runs};
use crate::{BlockDevice, Error, Fid, RecordAttrs, Result, Volume, fch};

/// How to place new blocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alloc {
    /// Wherever there is room, in as many pieces as it takes.
    #[default]
    Any,
    /// Largest free pieces first, so as few as possible (FCH$V_CONTIGB).
    BestTry,
    /// One piece, or fail (FCH$V_CONTIG).
    Contiguous,
}

/// The attributes a file's owner can change: record attributes, file
/// characteristics, ownership, protection and dates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attributes {
    pub record: RecordAttrs,
    pub filechar: u32,
    /// Owner UIC: group in the high word, member in the low word.
    pub owner: u32,
    /// SOGW protection, 4 bits per category, a set bit denies.
    pub protection: u16,
    pub revision: u16,
    pub created: u64,
    pub revised: u64,
    pub expires: u64,
    pub backup: u64,
    /// ODS-5 only.
    pub accessed: u64,
    pub attr_changed: u64,
}

/// What a file's headers say about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    pub fid: Fid,
    /// Primary name from the ident area, "NAME.TYPE;VERSION" (empty if the
    /// header has no ident area).
    pub name: Vec<u8>,
    pub name_type: NameType,
    pub attrs: Attributes,
    pub backlink: Fid,
    /// Blocks mapped by all headers.
    pub allocated: u64,
    /// Number of headers (1 plus extension headers).
    pub headers: usize,
    /// Number of map runs.
    pub extents: usize,
    pub highwater: Option<u32>,
    /// Raw access control list, concatenated over all headers.
    pub acl: Vec<u8>,
}

/// File characteristics the owner may set; the file system keeps the rest.
const USER_FCH: u32 = fch::NOBACKUP
    | fch::WRITEBACK
    | fch::READCHECK
    | fch::WRITCHECK
    | fch::CONTIGB
    | fch::LOCKED
    | fch::NOCHARGE
    | fch::ERASE;

pub(crate) fn attributes(h: &Header) -> Attributes {
    let id = h.ident().unwrap_or_default();
    Attributes {
        record: h.record_attrs(),
        filechar: h.filechar(),
        owner: h.fileowner(),
        protection: h.fileprot(),
        revision: id.revision,
        created: id.credate,
        revised: id.revdate,
        expires: id.expdate,
        backup: id.bakdate,
        accessed: id.accdate,
        attr_changed: id.attdate,
    }
}

impl<D: BlockDevice> Volume<D> {
    pub fn stat(&mut self, fid: Fid) -> Result<FileInfo, D::Error> {
        let hs = self.headers(fid)?;
        let mut map = Vec::new();
        let mut acl = Vec::new();
        for (lbn, h) in &hs {
            map.extend(self.header_runs(h, *lbn)?);
            acl.extend_from_slice(h.acl_area());
        }
        let h = &hs[0].1;
        let id = h.ident().unwrap_or_default();
        Ok(FileInfo {
            fid: h.fid(),
            name: id.name,
            name_type: id.name_type,
            attrs: attributes(h),
            backlink: h.backlink(),
            allocated: map.iter().map(|r| r.count).sum(),
            headers: hs.len(),
            extents: map.len(),
            highwater: h.highwater_mark(),
            acl,
        })
    }

    /// The file's map as (LBN, block count) runs, in VBN order.
    pub fn extents(&mut self, fid: Fid) -> Result<Vec<(u64, u64)>, D::Error> {
        Ok(self.file_map(fid)?.iter().map(|r| (r.lbn, r.count)).collect())
    }

    /// Reads whole virtual blocks starting at `vbn` (1-based). Fails past
    /// the last allocated block.
    pub fn read_blocks(&mut self, fid: Fid, vbn: u64, buf: &mut [u8]) -> Result<(), D::Error> {
        let map = self.file_map(fid)?;
        for (lbn, range) in pieces(&map, vbn, buf.len())? {
            self.dev.read(lbn, &mut buf[range])?;
        }
        Ok(())
    }

    /// Writes whole virtual blocks at `vbn` within the file's allocation,
    /// and raises the highwater mark past them.
    pub fn write_blocks(&mut self, fid: Fid, vbn: u64, buf: &[u8]) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let map = self.file_map(fid)?;
        for (lbn, range) in pieces(&map, vbn, buf.len())? {
            self.dev.write(lbn, &buf[range])?;
        }
        let end = vbn + (buf.len() / BLOCK) as u64;
        let lbn = self.header_lbn(fid.num).unwrap_or(0);
        let mut h = self.read_header(fid)?;
        if h.highwater_mark().is_some_and(|hw| (hw as u64) < end + 1) {
            h.set_highwater((end + 1) as u32);
            self.write_header(lbn, &mut h)?;
        }
        Ok(())
    }

    /// Adds at least `blocks` blocks (whole clusters) to the end of a file.
    /// A contiguous file stays contiguous or the call fails, unless `how`
    /// is `Any`, which clears the contiguous bit.
    pub fn extend(&mut self, fid: Fid, blocks: u64, how: Alloc) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let map = self.file_map(fid)?;
        let h = self.read_header(fid)?;
        let contig = h.filechar() & fch::CONTIG != 0;
        let after = map.last().map(|r| r.lbn + r.count);
        let how = if contig && how != Alloc::Any { Alloc::Contiguous } else { how };
        let runs = self.allocate(blocks, how, after.filter(|_| how == Alloc::Contiguous || !map.is_empty()))?;
        if contig && how == Alloc::Any && (runs.len() > 1 || runs.first().map(|r| r.lbn) != after) {
            let lbn = self.header_lbn(fid.num).unwrap_or(0);
            let mut h = self.read_header(fid)?;
            h.set_filechar(h.filechar() & !fch::CONTIG);
            self.write_header(lbn, &mut h)?;
        }
        self.append_runs(fid, &runs)
    }

    /// Appends allocated runs to a file's map and raises its allocated size.
    pub(crate) fn append_runs(&mut self, fid: Fid, runs: &[Run]) -> Result<(), D::Error> {
        let mut runs = runs.to_vec();
        merge_runs(&mut runs);
        if runs.is_empty() {
            return Ok(());
        }
        let hs = self.headers(fid)?;
        let (last_lbn, mut last) = hs[hs.len() - 1];
        let old = decode_map(last.map_area()).map_err(|what| Error::Corrupt { what, lbn: last_lbn })?;
        let cap = last.map_capacity();
        // Merge with the last pointer when the new space continues it, as
        // long as the old pointers still fit afterwards.
        let mut ptrs = old.clone();
        let mut rest = runs.clone();
        if let Some(&Pointer::Extent { count, lbn, .. }) = old.last() {
            let total = count as u64 + rest[0].count;
            if lbn as u64 + count as u64 == rest[0].lbn && total <= 0x4000_0000 {
                let mut merged = old.clone();
                let n = merged.len();
                merged[n - 1] = Pointer::extent(total as u32, lbn).unwrap_or(merged[n - 1]);
                if encoded(&merged) <= cap {
                    ptrs = merged;
                    rest.remove(0);
                }
            }
        }
        ptrs.extend(pointers(&rest));
        let keep = fit(&ptrs, cap).max(old.len());
        let overflow = ptrs.split_off(keep);
        // Chain new extension headers for what does not fit, written last
        // to first so each one links to a header already on disk.
        let mut link = last.ext_fid();
        let ext_cap = BLOCK - 2 - 80;
        let mut chunks = Vec::new();
        let mut o = &overflow[..];
        while !o.is_empty() {
            let n = fit(o, ext_cap).max(1);
            chunks.push(o[..n].to_vec());
            o = &o[n..];
        }
        let primary = hs[0].1;
        let mut ids = Vec::new();
        for _ in &chunks {
            ids.push(self.alloc_header()?);
        }
        for (i, chunk) in chunks.iter().enumerate().rev() {
            let (efid, elbn) = ids[i];
            let mut e = self.new_header(efid, None);
            e.set_seg_num(last.seg_num() + 1 + i as u16);
            e.set_recattr(primary.recattr());
            e.set_filechar(primary.filechar());
            e.set_fileowner(primary.fileowner());
            e.set_fileprot(primary.fileprot());
            e.set_backlink(fid);
            e.set_ext_fid(link);
            let mut m = Vec::new();
            encode_map(chunk, &mut m);
            if !e.set_map(&m) {
                return Err(Error::Invalid("map pointers overflow the header"));
            }
            self.write_header(elbn, &mut e)?;
            link = efid;
        }
        let mut m = Vec::new();
        encode_map(&ptrs, &mut m);
        if !last.set_map(&m) {
            return Err(Error::Invalid("map pointers overflow the header"));
        }
        last.set_ext_fid(link);
        let added: u64 = runs.iter().map(|r| r.count).sum();
        if hs.len() == 1 {
            add_hiblk(&mut last, added);
            self.write_header(last_lbn, &mut last)?;
        } else {
            self.write_header(last_lbn, &mut last)?;
            let (plbn, mut p) = self.headers(fid)?[0];
            add_hiblk(&mut p, added);
            self.write_header(plbn, &mut p)?;
        }
        Ok(())
    }

    /// Cuts a file down to `blocks` blocks (rounded up to whole clusters)
    /// and frees the rest, moving the end of file back if it was past.
    pub fn truncate(&mut self, fid: Fid, blocks: u64) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let keep = blocks.next_multiple_of(self.cluster());
        let hs = self.headers(fid)?;
        let mut vbn = 0u64; // blocks mapped by the headers before this one
        let mut cut = None;
        for (i, (lbn, h)) in hs.iter().enumerate() {
            let ptrs = decode_map(h.map_area()).map_err(|what| Error::Corrupt { what, lbn: *lbn })?;
            let n: u64 = ptrs.iter().map(extent_blocks).sum();
            if vbn + n > keep {
                cut = Some((i, ptrs, keep - vbn));
                break;
            }
            vbn += n;
        }
        let Some((k, ptrs, mut left)) = cut else {
            return Ok(());
        };
        let mut kept = Vec::new();
        let mut freed = Vec::new();
        for p in ptrs {
            match p {
                Pointer::Extent { count, lbn, .. } if left < count as u64 => {
                    if left > 0 {
                        kept.push(Pointer::extent(left as u32, lbn).unwrap_or(p));
                    }
                    freed.push(Run { lbn: lbn as u64 + left, count: count as u64 - left });
                    left = 0;
                }
                Pointer::Extent { count, .. } => {
                    left -= count as u64;
                    kept.push(p);
                }
                Pointer::Placement(_) => kept.push(p),
            }
        }
        let (klbn, mut kh) = hs[k];
        let mut m = Vec::new();
        encode_map(&kept, &mut m);
        if !kh.set_map(&m) {
            return Err(Error::Invalid("map pointers overflow the header"));
        }
        kh.set_ext_fid(Fid::default());
        let dropped = &hs[k + 1..];
        for (lbn, h) in dropped {
            freed.extend(self.header_runs(h, *lbn)?);
        }
        if k == 0 {
            shrink_attrs(&mut kh, keep);
            self.write_header(klbn, &mut kh)?;
        } else {
            self.write_header(klbn, &mut kh)?;
            let (plbn, mut p) = hs[0];
            shrink_attrs(&mut p, keep);
            self.write_header(plbn, &mut p)?;
        }
        for (lbn, h) in dropped {
            self.delete_header(*lbn, *h)?;
        }
        self.release(&freed)
    }

    /// Current attributes of a file.
    pub fn attributes(&mut self, fid: Fid) -> Result<Attributes, D::Error> {
        Ok(attributes(&self.read_header(fid)?))
    }

    /// Replaces a file's attributes. The allocated size (`hiblk`) always
    /// reflects the map, and file characteristics only change where the
    /// owner may change them (and CONTIG may only be cleared).
    pub fn set_attributes(&mut self, fid: Fid, a: &Attributes) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let allocated: u64 = self.file_map(fid)?.iter().map(|r| r.count).sum();
        let lbn = self.header_lbn(fid.num).unwrap_or(0);
        let mut h = self.read_header(fid)?;
        let mut r = a.record;
        r.hiblk = allocated as u32;
        if r.efblk as u64 > allocated + 1 || r.efblk as u64 == allocated + 1 && r.ffbyte != 0 {
            return Err(Error::Invalid("end of file past the allocation"));
        }
        if r.ffbyte as usize > BLOCK {
            return Err(Error::Invalid("first free byte past the block"));
        }
        h.set_record_attrs(&r);
        let keep_contig = h.filechar() & a.filechar & fch::CONTIG;
        h.set_filechar(h.filechar() & !(USER_FCH | fch::CONTIG) | a.filechar & USER_FCH | keep_contig);
        h.set_fileowner(a.owner);
        h.set_fileprot(a.protection);
        if let Some(mut id) = h.ident() {
            id.revision = a.revision;
            id.credate = a.created;
            id.revdate = a.revised;
            id.expdate = a.expires;
            id.bakdate = a.backup;
            id.accdate = a.accessed;
            id.attdate = a.attr_changed;
            h.set_ident(&id);
        }
        let used = r.eof_bytes().div_ceil(BLOCK as u64);
        if h.highwater_mark().is_some_and(|hw| (hw as u64) < used + 1) {
            h.set_highwater((used + 1) as u32);
        }
        self.write_header(lbn, &mut h)
    }
}

fn extent_blocks(p: &Pointer) -> u64 {
    match p {
        Pointer::Extent { count, .. } => *count as u64,
        Pointer::Placement(_) => 0,
    }
}

fn encoded(ptrs: &[Pointer]) -> usize {
    ptrs.iter().map(Pointer::size).sum()
}

/// How many leading pointers fit in `cap` bytes.
fn fit(ptrs: &[Pointer], cap: usize) -> usize {
    let mut used = 0;
    ptrs.iter()
        .take_while(|p| {
            used += p.size();
            used <= cap
        })
        .count()
}

fn add_hiblk(h: &mut Header, blocks: u64) {
    let mut r = h.record_attrs();
    r.hiblk = (r.hiblk as u64 + blocks) as u32;
    h.set_record_attrs(&r);
}

/// Record attributes and highwater after truncating to `keep` blocks.
fn shrink_attrs(h: &mut Header, keep: u64) {
    let mut r = h.record_attrs();
    r.hiblk = keep as u32;
    if r.efblk as u64 > keep + 1 || r.efblk as u64 == keep + 1 && r.ffbyte != 0 {
        r.efblk = keep as u32 + 1;
        r.ffbyte = 0;
    }
    h.set_record_attrs(&r);
    if h.highwater_mark().is_some_and(|hw| hw as u64 > keep + 1) {
        h.set_highwater(keep as u32 + 1);
    }
}

/// Splits a transfer of `len` bytes starting at `vbn` into one device
/// transfer per run: (LBN, range of the buffer).
pub(crate) fn pieces<E>(map: &[Run], vbn: u64, len: usize) -> Result<Vec<(u64, core::ops::Range<usize>)>, E> {
    if !len.is_multiple_of(BLOCK) || vbn == 0 {
        return Err(Error::Invalid("transfer not in whole blocks from VBN 1"));
    }
    let mut out = Vec::new();
    let mut done = 0;
    while done < len {
        let (lbn, run) = map_vbn(map, vbn + (done / BLOCK) as u64).ok_or(Error::BeyondEof)?;
        let n = (run.saturating_mul(BLOCK as u64)).min((len - done) as u64) as usize;
        out.push((lbn, done..done + n));
        done += n;
    }
    Ok(out)
}
