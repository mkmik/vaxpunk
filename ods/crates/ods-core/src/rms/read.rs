//! Reading relative and indexed files: the prologue, buckets, and records
//! along any key.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec::Vec;

use super::{
    AREA_LEN, Area, AreaDesc, BKT_HDR, BucketHeader, KEY_LEN, Key, KeyDesc, Result, RmsError, bktcb, corrupt,
    expand_key, expand_segments, irc, keyflag, uint,
};
use crate::layout::{BLOCK, Field, RecordAttrs};
use crate::rfm;

/// A record's file address: the bucket it was first put in and its ID
/// there.
pub type Rfa = (u32, u16);

/// A relative or indexed file.
pub enum RmsFile<'a> {
    Relative(Relative<'a>),
    Indexed(Indexed<'a>),
}

/// Opens a file's bytes as what its record attributes say it is.
pub fn open<'a>(data: &'a [u8], attrs: &RecordAttrs) -> Result<RmsFile<'a>> {
    match attrs.rtype >> 4 {
        1 => Ok(RmsFile::Relative(Relative::parse(data, attrs)?)),
        2 => Ok(RmsFile::Indexed(Indexed::parse(data, attrs)?)),
        _ => Err(RmsError::Unsupported("not a relative or indexed file")),
    }
}

impl RmsFile<'_> {
    /// The records along key `key`; a relative file has only key 0, its
    /// record numbers.
    pub fn records(&self, key: usize) -> Result<Vec<Vec<u8>>> {
        match self {
            RmsFile::Relative(r) if key == 0 => Ok(r.records()?.into_iter().map(|(_, d)| d).collect()),
            RmsFile::Relative(_) => Err(RmsError::Invalid(format!("a relative file has no key {key}"))),
            RmsFile::Indexed(x) => x.records(key),
        }
    }

    /// How many keys records can be read along.
    pub fn keys(&self) -> usize {
        match self {
            RmsFile::Relative(_) => 1,
            RmsFile::Indexed(x) => x.keys.len(),
        }
    }
}

fn block(data: &[u8], vbn: u32) -> Result<&[u8]> {
    let at = (vbn as usize).wrapping_sub(1).wrapping_mul(BLOCK);
    match data.get(at..at.wrapping_add(BLOCK)) {
        Some(b) if vbn > 0 => Ok(b),
        _ => corrupt("block past the end of file", vbn),
    }
}

/// A relative file: fixed cells, cell n holding record number n.
pub struct Relative<'a> {
    data: &'a [u8],
    fix: bool,
    /// The size word and VFC control area, before a cell's record.
    head: usize,
    mrs: usize,
    vfc: usize,
    bktsz: u32,
    pub dvbn: u32,
    /// Highest record number allowed.
    pub mrn: u32,
    /// The VBN after the last bucket in use.
    pub eof: u32,
}

/// `DLC$` cell control bits.
pub(crate) const DLC_DELETED: u8 = 1 << 2;
pub(crate) const DLC_REC: u8 = 1 << 3;

impl<'a> Relative<'a> {
    pub fn parse(data: &'a [u8], attrs: &RecordAttrs) -> Result<Relative<'a>> {
        let p = block(data, 1)?;
        if u16::get(&p[0x74..]) != 1 {
            return corrupt("relative file prologue version not 1", 1);
        }
        let format = attrs.rtype & 0xf;
        let vfc = match format {
            rfm::VFC if attrs.vfcsize == 0 => 2,
            rfm::VFC => attrs.vfcsize as usize,
            rfm::FIX | rfm::VAR => 0,
            _ => return Err(RmsError::Unsupported("relative file record format")),
        };
        let mrs = if attrs.maxrec > 0 { attrs.maxrec } else { attrs.rsize } as usize;
        let r = Relative {
            data,
            fix: format == rfm::FIX,
            head: if format == rfm::FIX { 0 } else { 2 },
            mrs,
            vfc,
            bktsz: attrs.bktsize.max(1) as u32,
            dvbn: u16::get(&p[0x68..]) as u32,
            mrn: u32::get(&p[0x6c..]),
            eof: u32::get(&p[0x70..]),
        };
        if mrs == 0 {
            return Err(RmsError::Unsupported("relative file without a record size"));
        }
        if r.dvbn < 2 {
            return corrupt("relative file data before VBN 2", 1);
        }
        if r.cells_per_bucket() == 0 {
            return corrupt("relative file cell larger than its bucket", 1);
        }
        Ok(r)
    }

    fn cell_size(&self) -> usize {
        1 + self.head + self.vfc + self.mrs
    }

    fn cells_per_bucket(&self) -> usize {
        self.bktsz as usize * BLOCK / self.cell_size()
    }

    /// The prologue block.
    pub fn prologue(&self) -> &'a [u8] {
        &self.data[..BLOCK]
    }

    /// Every cell up to the end of file: record number, VBN, bytes.
    /// Stops with an error at a bucket past the end of the data.
    pub(crate) fn cells(&self) -> impl Iterator<Item = Result<(u32, u32, &'a [u8])>> + '_ {
        let buckets = self.eof.saturating_sub(self.dvbn) / self.bktsz;
        let per = self.cells_per_bucket();
        (0..buckets).flat_map(move |b| {
            let vbn = self.dvbn + b * self.bktsz;
            let at = (vbn as usize - 1) * BLOCK;
            let bucket = self.data.get(at..at + self.bktsz as usize * BLOCK);
            (0..per).map(move |c| {
                let Some(bucket) = bucket else { return corrupt("bucket past the end of file", vbn) };
                let n = (b as u64 * per as u64 + c as u64 + 1).min(u32::MAX as u64) as u32;
                let cell = &bucket[c * self.cell_size()..(c + 1) * self.cell_size()];
                Ok((n, vbn + (c * self.cell_size() / BLOCK) as u32, cell))
            })
        })
    }

    /// The record in a cell, if it holds one.
    pub(crate) fn cell_record(&self, cell: &'a [u8], vbn: u32) -> Result<Option<&'a [u8]>> {
        if cell[0] & DLC_REC == 0 || cell[0] & DLC_DELETED != 0 {
            return Ok(None);
        }
        let size = if self.fix { self.mrs } else { u16::get(&cell[1..]) as usize };
        if size > self.mrs + self.vfc {
            return corrupt("record longer than the cell", vbn);
        }
        Ok(Some(&cell[1 + self.head..1 + self.head + size]))
    }

    /// The records, with their numbers, in order.
    pub fn records(&self) -> Result<Vec<(u32, Vec<u8>)>> {
        let mut out = Vec::new();
        for c in self.cells() {
            let (n, vbn, cell) = c?;
            if let Some(r) = self.cell_record(cell, vbn)? {
                out.push((n, r.to_vec()));
            }
        }
        Ok(out)
    }
}

/// A prologue 3 indexed file.
pub struct Indexed<'a> {
    data: &'a [u8],
    /// The record size of a FIX file, whose records have no size word
    /// unless compressed.
    pub(crate) fix: Option<usize>,
    pub keys: Vec<Key>,
    /// Where each key descriptor is: VBN and offset.
    pub key_at: Vec<(u32, usize)>,
    pub areas: Vec<Area>,
    /// VBN of the first area descriptor.
    pub avbn: u32,
}

/// A primary data record, decoded.
pub(crate) struct DataRec {
    pub ctrl: u8,
    pub id: u16,
    pub rfa: Rfa,
    pub key: Vec<u8>,
    pub rec: Vec<u8>,
}

pub(crate) enum Item {
    Rec(DataRec),
    /// An RRV: its ID, and where the record is now.
    Rrv {
        ctrl: u8,
        id: u16,
        to: Rfa,
    },
}

/// A SIDR: a key value and pointers to the records that have it.
pub(crate) struct Sidr {
    pub key: Vec<u8>,
    /// Control byte and RFA; a one-byte stub has RFA (0, 0).
    pub ptrs: Vec<(u8, Rfa)>,
}

impl<'a> Indexed<'a> {
    pub fn parse(data: &'a [u8], attrs: &RecordAttrs) -> Result<Indexed<'a>> {
        let p = block(data, 1)?;
        if u16::get(&p[0x74..]) != 3 {
            return Err(RmsError::Unsupported("indexed file prologue other than 3"));
        }
        let fix = (attrs.rtype & 0xf == rfm::FIX)
            .then_some(if attrs.maxrec > 0 { attrs.maxrec } else { attrs.rsize } as usize);
        let (mut keys, mut key_at) = (Vec::new(), Vec::new());
        let (mut vbn, mut off) = (1u32, 0usize);
        loop {
            if off + KEY_LEN > BLOCK - 2 || key_at.contains(&(vbn, off)) || keys.len() == 255 {
                return corrupt("key descriptor chain", vbn);
            }
            let b = block(data, vbn)?;
            let mut k = [0u8; KEY_LEN];
            k.copy_from_slice(&b[off..off + KEY_LEN]);
            let k = KeyDesc(k);
            let size: usize = k.segs().map(|s| s.1).sum();
            if !(1..=8).contains(&k.segments()) || size != k.keysz() as usize || size == 0 {
                return corrupt("key descriptor segments", vbn);
            }
            if !k.has(keyflag::INITIDX) && (k.datbktsz() == 0 || k.idxbktsz() == 0 || k.rootlev() == 0) {
                return corrupt("key descriptor bucket sizes", vbn);
            }
            keys.push(k);
            key_at.push((vbn, off));
            if k.idxfl() == 0 {
                break;
            }
            (vbn, off) = (k.idxfl(), k.noff() as usize);
        }
        let (avbn, amax) = (p[0x66] as u32, p[0x67] as usize);
        let mut areas = Vec::new();
        for a in 0..amax {
            let b = block(data, avbn + (a / 8) as u32)?;
            let mut d = [0u8; AREA_LEN];
            d.copy_from_slice(&b[a % 8 * AREA_LEN..(a % 8 + 1) * AREA_LEN]);
            areas.push(AreaDesc(d));
        }
        Ok(Indexed { data, fix, keys, key_at, areas, avbn })
    }

    /// The file's size in blocks.
    pub fn blocks(&self) -> u32 {
        (self.data.len() / BLOCK).min(u32::MAX as usize) as u32
    }

    pub(crate) fn block(&self, vbn: u32) -> Result<&'a [u8]> {
        block(self.data, vbn)
    }

    /// A bucket's bytes.
    pub(crate) fn bucket(&self, vbn: u32, blocks: u8) -> Result<&'a [u8]> {
        let at = (vbn as usize).wrapping_sub(1).wrapping_mul(BLOCK);
        match self.data.get(at..at.wrapping_add(blocks as usize * BLOCK)) {
            Some(b) if vbn > 0 && blocks > 0 => Ok(b),
            _ => corrupt("bucket past the end of file", vbn),
        }
    }

    /// The buckets of one level, along their chain from `first` to the one
    /// marked last.
    pub(crate) fn chain(&self, first: u32, blocks: u8) -> Result<Vec<u32>> {
        let mut out = Vec::new();
        let mut vbn = first;
        loop {
            let b = self.bucket(vbn, blocks)?;
            out.push(vbn);
            let h = BucketHeader(&b[..BKT_HDR]);
            if h.bktcb() & bktcb::LASTBKT != 0 {
                return Ok(out);
            }
            if out.len() > self.blocks() as usize {
                return corrupt("bucket chain without an end", first);
            }
            vbn = h.nxtbkt();
        }
    }

    /// Where a data or SIDR bucket's entries end.
    fn freespace(b: &[u8], vbn: u32) -> Result<usize> {
        let free = u16::get(&b[4..]) as usize;
        if free < BKT_HDR || free > b.len() - 1 {
            return corrupt("bucket free space offset", vbn);
        }
        Ok(free)
    }

    /// Splits a primary record's body into its key and the record.
    fn decode(&self, k: &Key, body: &[u8], prev: &[u8], vbn: u32) -> Result<(Vec<u8>, Vec<u8>)> {
        let ksz = k.keysz() as usize;
        if !k.has(keyflag::KEY_COMPR | keyflag::REC_COMPR) {
            let Some(key) = k.key_of(body) else { return corrupt("record shorter than its primary key", vbn) };
            return Ok((key, body.to_vec()));
        }
        let (key, q) = if k.has(keyflag::KEY_COMPR) {
            expand_key(body, prev, ksz).ok_or(RmsError::Corrupt { what: "compressed key", vbn })?
        } else {
            (body.get(..ksz).ok_or(RmsError::Corrupt { what: "record shorter than its key", vbn })?.to_vec(), ksz)
        };
        let mut rec = Vec::new();
        if k.has(keyflag::REC_COMPR) {
            expand_segments(&body[q..], &mut rec).ok_or(RmsError::Corrupt { what: "compressed record", vbn })?;
        } else {
            rec.extend_from_slice(&body[q..]);
        }
        // The key's segments go back where they were, lowest first.
        let mut segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut from = 0;
        for (pos, size) in k.segs() {
            segs.push((pos, from, size));
            from += size;
        }
        segs.sort();
        for (pos, from, size) in segs {
            if pos > rec.len() {
                return corrupt("compressed record shorter than its key position", vbn);
            }
            rec.splice(pos..pos, key[from..from + size].iter().copied());
        }
        Ok((key, rec))
    }

    /// The records and RRVs of a primary data bucket.
    pub(crate) fn data_bucket(&self, k: &Key, vbn: u32, b: &[u8]) -> Result<Vec<Item>> {
        let free = Self::freespace(b, vbn)?;
        let raw_fix = self.fix.filter(|_| !k.has(keyflag::KEY_COMPR | keyflag::REC_COMPR));
        let mut out = Vec::new();
        let mut prev = Vec::new();
        let mut p = BKT_HDR;
        let short = RmsError::Corrupt { what: "record past the bucket's free space", vbn };
        while p < free {
            let ctrl = b[p];
            let ps = (ctrl & irc::PTRSZ) as usize + 2;
            let head = b.get(p..p + 5 + ps).filter(|_| p + 5 + ps <= free).ok_or(short.clone())?;
            let id = u16::get(&head[1..]);
            let (id2, vbn2) = (u16::get(&head[3..]), uint(&head[5..]) as u32);
            p += 5 + ps;
            if ctrl & irc::RRV != 0 {
                out.push(Item::Rrv { ctrl, id, to: (vbn2, id2) });
                continue;
            }
            let size = match raw_fix {
                Some(n) => n,
                None => {
                    let s = b.get(p..p + 2).filter(|_| p + 2 <= free).ok_or(short.clone())?;
                    p += 2;
                    u16::get(s) as usize
                }
            };
            let body = b.get(p..p + size).filter(|_| p + size <= free).ok_or(short.clone())?;
            p += size;
            let (key, rec) = self.decode(k, body, &prev, vbn)?;
            prev.clone_from(&key);
            out.push(Item::Rec(DataRec { ctrl, id, rfa: (vbn2, id2), key, rec }));
        }
        Ok(out)
    }

    /// The SIDRs of an alternate key's level 0 bucket.
    pub(crate) fn sidr_bucket(&self, k: &Key, vbn: u32, b: &[u8]) -> Result<Vec<Sidr>> {
        let free = Self::freespace(b, vbn)?;
        let ksz = k.keysz() as usize;
        let mut out: Vec<Sidr> = Vec::new();
        let mut p = BKT_HDR;
        let short = RmsError::Corrupt { what: "SIDR past the bucket's free space", vbn };
        while p < free {
            let size = b.get(p..p + 2).filter(|_| p + 2 <= free).ok_or(short.clone())?;
            let size = u16::get(size) as usize;
            let body = b.get(p + 2..p + 2 + size).filter(|_| p + 2 + size <= free).ok_or(short.clone())?;
            p += 2 + size;
            let (key, mut q) = if k.has(keyflag::KEY_COMPR) {
                let prev = out.last().map_or(&[][..], |s| &s.key[..]);
                expand_key(body, prev, ksz).ok_or(RmsError::Corrupt { what: "compressed SIDR key", vbn })?
            } else {
                (body.get(..ksz).ok_or(short.clone())?.to_vec(), ksz)
            };
            let mut ptrs = Vec::new();
            while q < body.len() {
                let c = body[q];
                if c & irc::NOPTRSZ != 0 {
                    ptrs.push((c, (0, 0)));
                    q += 1;
                    continue;
                }
                let ps = (c & irc::PTRSZ) as usize + 2;
                let ptr = body.get(q..q + 3 + ps).ok_or(RmsError::Corrupt { what: "SIDR pointer", vbn })?;
                ptrs.push((c, (uint(&ptr[3..]) as u32, u16::get(&ptr[1..]))));
                q += 3 + ps;
            }
            out.push(Sidr { key, ptrs });
        }
        Ok(out)
    }

    /// An index bucket's keys and the buckets they lead to.
    pub(crate) fn index_bucket(&self, k: &Key, vbn: u32, b: &[u8]) -> Result<(Vec<Vec<u8>>, Vec<u32>)> {
        let ps = (b[13] >> 3 & 3) as usize + 2;
        let free = u16::get(&b[4..]) as usize;
        if free < BKT_HDR || free > b.len() - 4 {
            return corrupt("index bucket free space offset", vbn);
        }
        let ksz = k.keysz() as usize;
        let (mut keys, mut ptrs) = (Vec::<Vec<u8>>::new(), Vec::new());
        let mut p = BKT_HDR;
        while p < free {
            let (key, n) = if k.has(keyflag::IDX_COMPR) {
                let prev = keys.last().map_or(&[][..], |k| &k[..]);
                expand_key(&b[p..free], prev, ksz).ok_or(RmsError::Corrupt { what: "compressed index key", vbn })?
            } else {
                (
                    b.get(p..p + ksz)
                        .filter(|_| p + ksz <= free)
                        .ok_or(RmsError::Corrupt { what: "index key", vbn })?
                        .to_vec(),
                    ksz,
                )
            };
            p += n;
            let q = (b.len() - 4).checked_sub((keys.len() + 1) * ps).filter(|&q| q >= free);
            let Some(q) = q else { return corrupt("index keys run into their pointers", vbn) };
            keys.push(key);
            ptrs.push(uint(&b[q..q + ps]) as u32);
        }
        Ok((keys, ptrs))
    }

    /// The primary records in key order, deleted ones too, with the
    /// bucket each is in.
    pub(crate) fn primary(&self) -> Result<Vec<(u32, DataRec)>> {
        let k = &self.keys[0];
        let mut out = Vec::new();
        if k.has(keyflag::INITIDX) {
            return Ok(out);
        }
        for vbn in self.chain(k.ldvbn(), k.datbktsz())? {
            let b = self.bucket(vbn, k.datbktsz())?;
            for item in self.data_bucket(k, vbn, b)? {
                if let Item::Rec(r) = item {
                    out.push((vbn, r));
                }
            }
        }
        Ok(out)
    }

    /// An alternate key's SIDRs in key order.
    pub(crate) fn sidrs(&self, key: usize) -> Result<Vec<(u32, Sidr)>> {
        let k = &self.keys[key];
        let mut out = Vec::new();
        if k.has(keyflag::INITIDX) {
            return Ok(out);
        }
        for vbn in self.chain(k.ldvbn(), k.datbktsz())? {
            let b = self.bucket(vbn, k.datbktsz())?;
            out.extend(self.sidr_bucket(k, vbn, b)?.into_iter().map(|s| (vbn, s)));
        }
        Ok(out)
    }

    /// The records along key `key`, as a sequential `$GET` loop would read
    /// them: deleted records left out, duplicates of an alternate key in
    /// the order they were put.
    pub fn records(&self, key: usize) -> Result<Vec<Vec<u8>>> {
        if key >= self.keys.len() {
            return Err(RmsError::Invalid(format!("the file has no key {key}")));
        }
        let live = |c: u8| c & irc::DELETED == 0;
        let prim = self.primary()?;
        if key == 0 {
            return Ok(prim.into_iter().filter(|(_, r)| live(r.ctrl)).map(|(_, r)| r.rec).collect());
        }
        let by_rfa: BTreeMap<Rfa, usize> =
            prim.iter().enumerate().filter(|(_, (_, r))| live(r.ctrl)).map(|(i, (_, r))| (r.rfa, i)).collect();
        let mut out = Vec::new();
        for (vbn, s) in self.sidrs(key)? {
            for (c, rfa) in s.ptrs {
                if c & (irc::DELETED | irc::NOPTRSZ) != 0 {
                    continue;
                }
                let Some(&i) = by_rfa.get(&rfa) else { return corrupt("SIDR pointer to no record", vbn) };
                out.push(prim[i].1.rec.clone());
            }
        }
        Ok(out)
    }
}
