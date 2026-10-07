//! Loading records into a new indexed file, the way `CONVERT` does: sorted
//! by the primary key into data buckets filled to the key's fill quantity,
//! an index built over them level by level up to one root, then each
//! alternate key's SIDRs and their index.

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::{
    AREA_LEN, AreaDesc, BKT_HDR, BucketHeader, KEY_LEN, KeyDesc, Result, RmsError, bktcb, compare, compress_key,
    compress_segments, dtype, irc, keyflag, set_checksum,
};
use crate::layout::{BLOCK, RecordAttrs};
use crate::rfm;

/// What a new indexed file is to be: what an FDL file says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// `rfm::FIX` or `rfm::VAR`.
    pub rfm: u8,
    /// The longest record: the record size for `FIX`, 0 for no limit.
    pub mrs: u16,
    /// Record attributes for the file header (`rat::CR` and friends).
    pub rat: u8,
    pub areas: Vec<AreaSpec>,
    /// Key 0, the primary key, first.
    pub keys: Vec<KeySpec>,
    /// The file is made a whole number of these blocks, the volume's
    /// cluster size, the spare ones going to the last area; 0 or 1 for any.
    pub cluster: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AreaSpec {
    /// Bucket size, 1 to 63 blocks.
    pub bktsz: u8,
    /// Blocks to give the area even if it needs fewer.
    pub alloc: u32,
    /// Default extension, blocks.
    pub deq: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeySpec {
    /// Each segment's position in the record and size: 1 to 8 of them.
    pub segments: Vec<(u16, u8)>,
    /// A [`dtype`] value.
    pub datatype: u8,
    pub dups: bool,
    pub changes: bool,
    /// Records whose key is all `null_value` aren't in an alternate key's
    /// index.
    pub null_key: bool,
    pub null_value: u8,
    pub key_compr: bool,
    /// Data record compression: the primary key only.
    pub rec_compr: bool,
    pub idx_compr: bool,
    /// Bytes of a data (or SIDR) and of an index bucket a load fills; 0 for
    /// all of it.
    pub data_fill: u16,
    pub index_fill: u16,
    pub data_area: u8,
    pub index_area: u8,
    pub level1_area: u8,
    pub name: Vec<u8>,
}

impl KeySpec {
    /// A string key of one segment, without duplicates or compression, in
    /// area 0.
    pub fn string(position: u16, size: u8) -> KeySpec {
        KeySpec {
            segments: vec![(position, size)],
            datatype: dtype::STRING,
            dups: false,
            changes: false,
            null_key: false,
            null_value: 0,
            key_compr: false,
            rec_compr: false,
            idx_compr: false,
            data_fill: 0,
            index_fill: 0,
            data_area: 0,
            index_area: 0,
            level1_area: 0,
            name: Vec::new(),
        }
    }

    fn size(&self) -> usize {
        self.segments.iter().map(|s| s.1 as usize).sum()
    }

    /// The shortest record that holds the key.
    fn min_record(&self) -> usize {
        self.segments.iter().map(|&(p, s)| p as usize + s as usize).max().unwrap_or(0)
    }

    fn key_of(&self, rec: &[u8]) -> Option<Vec<u8>> {
        let mut k = Vec::with_capacity(self.size());
        for &(p, s) in &self.segments {
            k.extend_from_slice(rec.get(p as usize..p as usize + s as usize)?);
        }
        Some(k)
    }

    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        compare(self.datatype, self.segments.len() as u8, a, b)
    }

    fn flags(&self, primary: bool) -> u8 {
        let mut f = 0;
        for (on, bit) in [
            (self.dups, keyflag::DUPKEYS),
            (self.changes && !primary, keyflag::CHGKEYS),
            (self.null_key && !primary, keyflag::NULKEYS),
            (self.idx_compr, keyflag::IDX_COMPR),
            (self.key_compr, keyflag::KEY_COMPR),
            (self.rec_compr && primary, keyflag::REC_COMPR),
        ] {
            if on {
                f |= bit;
            }
        }
        f
    }
}

impl Spec {
    /// The file header's record attributes for the file `build` made, of
    /// `blocks` blocks: its end of file is the end of its allocation.
    pub fn record_attrs(&self, blocks: u32) -> RecordAttrs {
        RecordAttrs {
            rtype: 0x20 | self.rfm,
            rattrib: self.rat,
            rsize: if self.rfm == rfm::FIX { self.mrs } else { 0 },
            hiblk: blocks,
            efblk: blocks + 1,
            ffbyte: 0,
            bktsize: self.areas.iter().map(|a| a.bktsz).max().unwrap_or(0),
            maxrec: self.mrs,
            ..RecordAttrs::default()
        }
    }

    fn validate(&self) -> Result<()> {
        let bad = |m: &str| Err(RmsError::Invalid(m.into()));
        if self.rfm != rfm::FIX && self.rfm != rfm::VAR {
            return bad("an indexed file's records are FIX or VAR");
        }
        if self.rfm == rfm::FIX && self.mrs == 0 {
            return bad("FIX records need a size");
        }
        if self.areas.is_empty() || self.areas.len() > 255 {
            return bad("an indexed file has 1 to 255 areas");
        }
        if self.areas.iter().any(|a| !(1..=63).contains(&a.bktsz)) {
            return bad("bucket sizes are 1 to 63 blocks");
        }
        if self.keys.is_empty() || self.keys.len() > 255 {
            return bad("an indexed file has 1 to 255 keys");
        }
        for (n, k) in self.keys.iter().enumerate() {
            let bad = |m: &str| Err(RmsError::Invalid(format!("key {n}: {m}")));
            if !(1..=8).contains(&k.segments.len()) || k.segments.iter().any(|s| s.1 == 0) {
                return bad("1 to 8 segments, none empty");
            }
            if k.size() > 255 {
                return bad("longer than 255 bytes");
            }
            if self.mrs > 0 && k.min_record() > self.mrs as usize {
                return bad("past the end of the longest record");
            }
            let width = match k.datatype & !dtype::DESCENDING {
                dtype::STRING => 0,
                dtype::INT2 | dtype::BIN2 => 2,
                dtype::INT4 | dtype::BIN4 => 4,
                dtype::INT8 | dtype::BIN8 => 8,
                _ => return bad("data type not supported: string, int2/4/8, bin2/4/8 and their descending forms"),
            };
            if width > 0 && (k.segments.len() != 1 || k.size() != width) {
                return bad("an integer key is one segment of its size");
            }
            if width > 0 && (k.key_compr || k.idx_compr || k.rec_compr) {
                return bad("only string keys are compressed");
            }
            for a in [k.data_area, k.index_area, k.level1_area] {
                if a as usize >= self.areas.len() {
                    return bad("names an area the file doesn't have");
                }
            }
            if self.areas[k.index_area as usize].bktsz != self.areas[k.level1_area as usize].bktsz {
                return bad("index and level 1 index areas need the same bucket size");
            }
            if n == 0 && (k.key_compr || k.rec_compr) {
                let mut s = k.segments.clone();
                s.sort();
                if s.windows(2).any(|w| w[0].0 as usize + w[0].1 as usize > w[1].0 as usize) {
                    return bad("a compressed primary key's segments may not overlap");
                }
            }
        }
        Ok(())
    }
}

/// A bucket being built. VBNs inside it are filled in once every bucket
/// has one.
struct Bucket {
    area: u8,
    level: u8,
    keyref: u8,
    buf: Vec<u8>,
    free: usize,
    /// Records (data buckets) or entries (index buckets) in it.
    count: usize,
    next: usize,
    last: bool,
    root: bool,
    /// Index pointer size; 0 elsewhere.
    ps: usize,
    /// (offset, width, what the VBN there is of).
    patches: Vec<(usize, usize, Target)>,
    high: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Target {
    Bucket(usize),
    /// The bucket of a record, by its number in the input.
    Record(usize),
}

/// A key's index, as planned: its first level 0 bucket, root and depth.
struct Tree {
    first: usize,
    root: usize,
    levels: u8,
}

struct Plan<'s> {
    spec: &'s Spec,
    ps: usize,
    buckets: Vec<Bucket>,
    /// Each input record's bucket and ID.
    placed: Vec<(usize, u16)>,
}

impl Plan<'_> {
    fn bucket_bytes(&self, area: u8) -> usize {
        self.spec.areas[area as usize].bktsz as usize * BLOCK
    }

    fn new_bucket(&mut self, area: u8, level: u8, keyref: u8) -> usize {
        let size = self.bucket_bytes(area);
        let ps = if level > 0 { self.ps } else { 0 };
        self.buckets.push(Bucket {
            area,
            level,
            keyref,
            buf: vec![0; size],
            free: BKT_HDR,
            count: 0,
            next: 0,
            last: false,
            root: false,
            ps,
            patches: Vec::new(),
            high: Vec::new(),
        });
        self.buckets.len() - 1
    }

    /// Chains a level's buckets in order, the last back to the first.
    fn chain(&mut self, level: &[usize]) {
        for (i, &b) in level.iter().enumerate() {
            self.buckets[b].next = level[(i + 1) % level.len()];
        }
        if let Some(&l) = level.last() {
            self.buckets[l].last = true;
        }
    }

    /// Puts the records in primary key order into data buckets.
    fn primary<R: AsRef<[u8]>>(&mut self, recs: &[R], keys: &[Vec<u8>], order: &[usize]) -> Result<Vec<usize>> {
        let spec = self.spec;
        let k = &spec.keys[0];
        let size = self.bucket_bytes(k.data_area);
        let fill = fill(k.data_fill, size);
        let raw_fix = self.spec.rfm == rfm::FIX && !k.key_compr && !k.rec_compr;
        let mut level = Vec::new();
        let mut enc = Vec::new();
        for &i in order {
            let (rec, key) = (recs[i].as_ref(), &keys[i]);
            let head = 1 + 2 + 2 + 4 + if raw_fix { 0 } else { 2 };
            let cur = level.last().copied();
            enc.clear();
            encode_record(k, rec, key, cur.map(|c: usize| &self.buckets[c].high[..]), &mut enc);
            // CONVERT judges whether a record fits by its size first in a
            // bucket, compressed against nothing, and leaves a byte spare.
            let mut alone = Vec::new();
            encode_record(k, rec, key, None, &mut alone);
            let fits = |b: &Bucket| b.free < fill && b.free + head + alone.len() < size - 1;
            let b = match cur {
                Some(c) if fits(&self.buckets[c]) => c,
                _ => {
                    let b = self.new_bucket(k.data_area, 0, 0);
                    level.push(b);
                    enc = alone;
                    if BKT_HDR + head + enc.len() >= size {
                        return Err(RmsError::Invalid(format!("record {} doesn't fit in a bucket", i + 1)));
                    }
                    b
                }
            };
            let bk = &mut self.buckets[b];
            bk.count += 1;
            let id = bk.count as u16;
            // Pointer size 2: a 4-byte VBN, as CONVERT writes.
            let mut e = vec![2u8];
            e.extend_from_slice(&id.to_le_bytes());
            e.extend_from_slice(&id.to_le_bytes());
            bk.patches.push((bk.free + e.len(), 4, Target::Bucket(b)));
            e.extend_from_slice(&[0; 4]);
            if !raw_fix {
                e.extend_from_slice(&(enc.len() as u16).to_le_bytes());
            }
            e.extend_from_slice(&enc);
            bk.buf[bk.free..bk.free + e.len()].copy_from_slice(&e);
            bk.free += e.len();
            bk.high.clone_from(key);
            self.placed[i] = (b, id);
        }
        Ok(level)
    }

    /// Puts an alternate key's SIDRs, (key, records with it) in key order,
    /// into SIDR buckets. A SIDR too big for one bucket goes on in the next
    /// with the same key.
    fn sidrs(&mut self, n: usize, groups: &[(Vec<u8>, Vec<usize>)]) -> Result<Vec<usize>> {
        let spec = self.spec;
        let k = &spec.keys[n];
        let size = self.bucket_bytes(k.data_area);
        let fill = fill(k.data_fill, size);
        let mut level: Vec<usize> = Vec::new();
        let mut kenc = Vec::new();
        for (key, recs) in groups {
            let mut rest = &recs[..];
            while !rest.is_empty() {
                let cur = level.last().copied();
                kenc.clear();
                encode_key(k.key_compr, key, None, &mut kenc);
                let len = |m: usize, kl: usize| 2 + kl + 7 * m;
                let (b, m) = match cur {
                    Some(c)
                        if self.buckets[c].free < fill
                            && self.buckets[c].free + len(rest.len(), kenc.len()) < size - 1 =>
                    {
                        kenc.clear();
                        encode_key(k.key_compr, key, Some(&self.buckets[c].high), &mut kenc);
                        (c, rest.len())
                    }
                    _ => {
                        let b = self.new_bucket(k.data_area, 0, n as u8);
                        level.push(b);
                        let room = (size - 1 - BKT_HDR).saturating_sub(len(0, kenc.len())) / 7;
                        if room == 0 {
                            return Err(RmsError::Invalid(format!("key {n} too long for its bucket size")));
                        }
                        (b, rest.len().min(room))
                    }
                };
                let mut e = Vec::new();
                e.extend_from_slice(&((kenc.len() + 7 * m) as u16).to_le_bytes());
                e.extend_from_slice(&kenc);
                let bk = &mut self.buckets[b];
                for (j, &r) in rest[..m].iter().enumerate() {
                    e.push(2 | if j == 0 { irc::FIRST } else { 0 });
                    e.extend_from_slice(&[0; 6]);
                    bk.patches.push((bk.free + e.len() - 4, 4, Target::Record(r)));
                }
                bk.buf[bk.free..bk.free + e.len()].copy_from_slice(&e);
                bk.free += e.len();
                bk.count += 1;
                bk.high.clone_from(key);
                rest = &rest[m..];
            }
        }
        Ok(level)
    }

    /// Builds index levels over a key's level 0 buckets until one bucket,
    /// the root, holds them all.
    fn index(&mut self, n: usize, level0: Vec<usize>) -> Result<Tree> {
        let spec = self.spec;
        let k = &spec.keys[n];
        let size = self.bucket_bytes(k.index_area);
        let fill = fill(k.index_fill, size);
        let ksz = k.size();
        self.chain(&level0);
        let first = level0[0];
        let mut below = level0;
        let mut depth = 0u8;
        loop {
            depth += 1;
            let area = if depth == 1 { k.level1_area } else { k.index_area };
            let mut level: Vec<usize> = Vec::new();
            let mut kenc = Vec::new();
            for (j, &child) in below.iter().enumerate() {
                let high = if j + 1 == below.len() { high_key(k, ksz) } else { self.buckets[child].high.clone() };
                let cur = level.last().copied();
                kenc.clear();
                encode_key(k.idx_compr, &high, cur.map(|c| &self.buckets[c].high[..]), &mut kenc);
                let ps = self.ps;
                let fits = |b: &Bucket, len: usize| {
                    b.free + len + (b.count + 1) * ps <= size - 4 && (b.count < 2 || b.free + b.count * ps < fill)
                };
                let b = match cur {
                    Some(c) if fits(&self.buckets[c], kenc.len()) => c,
                    _ => {
                        let b = self.new_bucket(area, depth, n as u8);
                        level.push(b);
                        kenc.clear();
                        encode_key(k.idx_compr, &high, None, &mut kenc);
                        b
                    }
                };
                let bk = &mut self.buckets[b];
                bk.buf[bk.free..bk.free + kenc.len()].copy_from_slice(&kenc);
                bk.free += kenc.len();
                bk.count += 1;
                bk.patches.push((size - 4 - bk.count * ps, ps, Target::Bucket(child)));
                bk.high = high;
            }
            self.chain(&level);
            if level.len() == 1 {
                self.buckets[level[0]].root = true;
                return Ok(Tree { first, root: level[0], levels: depth });
            }
            if level.len() >= below.len() || depth == 255 {
                return Err(RmsError::Invalid(format!("key {n} too long for its index bucket size")));
            }
            below = level;
        }
    }
}

/// The key above every other, which the last bucket of each index level
/// ends in: all ones for strings and unsigned integers, as `CONVERT`
/// writes; for the other types the highest of their order.
fn high_key(k: &KeySpec, ksz: usize) -> Vec<u8> {
    let signed = matches!(k.datatype & !dtype::DESCENDING, dtype::INT2 | dtype::INT4 | dtype::INT8);
    let descending = k.datatype & dtype::DESCENDING != 0;
    let mut v = vec![if descending { 0 } else { 0xff }; ksz];
    if signed && let Some(top) = v.last_mut() {
        *top = if descending { 0x80 } else { 0x7f };
    }
    v
}

/// Fill quantity in bytes: 0 means the whole bucket.
fn fill(bytes: u16, size: usize) -> usize {
    if bytes == 0 { size } else { (bytes as usize).min(size) }
}

fn encode_key(compressed: bool, key: &[u8], prev: Option<&[u8]>, out: &mut Vec<u8>) {
    if compressed {
        compress_key(key, prev.filter(|p| !p.is_empty()), out);
    } else {
        out.extend_from_slice(key);
    }
}

/// A primary record as stored: as it is, or its key first and the rest
/// compressed.
fn encode_record(k: &KeySpec, rec: &[u8], key: &[u8], prev: Option<&[u8]>, out: &mut Vec<u8>) {
    if !k.key_compr && !k.rec_compr {
        out.extend_from_slice(rec);
        return;
    }
    encode_key(k.key_compr, key, prev, out);
    let mut rest = rec.to_vec();
    let mut segs = k.segments.clone();
    segs.sort();
    for &(p, s) in segs.iter().rev() {
        rest.drain(p as usize..p as usize + s as usize);
    }
    if k.rec_compr {
        compress_segments(&rest, out);
    } else {
        out.extend_from_slice(&rest);
    }
}

/// Builds an indexed file holding `records`, in any order. Returns its
/// bytes, VBN 1 first: a whole number of blocks, all of them allocated
/// to the prologue or an area.
pub fn build<R: AsRef<[u8]>>(spec: &Spec, records: &[R]) -> Result<Vec<u8>> {
    spec.validate()?;
    let k0 = &spec.keys[0];
    let mut keys = Vec::with_capacity(records.len());
    for (i, r) in records.iter().enumerate() {
        let r = r.as_ref();
        let bad = |m: &str| Err(RmsError::Invalid(format!("record {}: {m}", i + 1)));
        if spec.rfm == rfm::FIX && r.len() != spec.mrs as usize {
            return bad("not the file's record size");
        }
        if (spec.mrs > 0 && r.len() > spec.mrs as usize) || r.len() > 32767 {
            return bad("longer than the longest record");
        }
        match k0.key_of(r) {
            Some(k) => keys.push(k),
            None => return bad("too short for the primary key"),
        }
    }
    let mut order: Vec<usize> = (0..records.len()).collect();
    order.sort_by(|&a, &b| k0.compare(&keys[a], &keys[b]));
    if !k0.dups
        && let Some(w) = order.windows(2).find(|w| keys[w[0]] == keys[w[1]])
    {
        return Err(RmsError::Invalid(format!("records {} and {}: duplicate primary key", w[0] + 1, w[1] + 1)));
    }

    // Each alternate key's SIDRs: (key, records) in key order, records in
    // primary key order.
    let mut groups: Vec<Vec<(Vec<u8>, Vec<usize>)>> = vec![Vec::new()];
    for (n, k) in spec.keys.iter().enumerate().skip(1) {
        let mut entries: Vec<(Vec<u8>, usize)> = order
            .iter()
            .filter_map(|&i| k.key_of(records[i].as_ref()).map(|kv| (kv, i)))
            .filter(|(kv, _)| !(k.null_key && kv.iter().all(|&c| c == k.null_value)))
            .collect();
        entries.sort_by(|a, b| k.compare(&a.0, &b.0));
        let mut g: Vec<(Vec<u8>, Vec<usize>)> = Vec::new();
        for (kv, i) in entries {
            match g.last_mut() {
                Some(last) if last.0 == kv => {
                    if !k.dups {
                        return Err(RmsError::Invalid(format!(
                            "records {} and {}: duplicate key {n}",
                            last.1[0] + 1,
                            i + 1
                        )));
                    }
                    last.1.push(i);
                }
                _ => g.push((kv, vec![i])),
            }
        }
        groups.push(g);
    }

    // Index pointers are as wide as the file's VBNs need: plan with two
    // bytes, and again wider if the file came out bigger than that.
    let mut ps = 2;
    loop {
        let (plan, trees) = plan(spec, records, &keys, &order, &groups, ps)?;
        let (bytes, highest) = lay_out(&plan, &trees);
        let need = if highest > 0xff_ffff {
            4
        } else if highest > 0xffff {
            3
        } else {
            2
        };
        if need <= ps {
            return Ok(bytes);
        }
        ps = need;
    }
}

fn plan<'s, R: AsRef<[u8]>>(
    spec: &'s Spec,
    records: &[R],
    keys: &[Vec<u8>],
    order: &[usize],
    groups: &[Vec<(Vec<u8>, Vec<usize>)>],
    ps: usize,
) -> Result<(Plan<'s>, Vec<Option<Tree>>)> {
    let mut p = Plan { spec, ps, buckets: Vec::new(), placed: vec![(0, 0); records.len()] };
    let mut trees = Vec::new();
    for (n, g) in groups.iter().enumerate() {
        let level0 = if n == 0 { p.primary(records, keys, order)? } else { p.sidrs(n, g)? };
        trees.push(if level0.is_empty() { None } else { Some(p.index(n, level0)?) });
    }
    Ok((p, trees))
}

/// Gives every bucket its VBN, each area's in one extent after the
/// prologue, and writes the file. Returns it and its highest VBN.
fn lay_out(p: &Plan, trees: &[Option<Tree>]) -> (Vec<u8>, u32) {
    let spec = p.spec;
    let key_blocks = (spec.keys.len() - 1).div_ceil(5) as u32;
    let area_blocks = spec.areas.len().div_ceil(8) as u32;
    let avbn = 2 + key_blocks;
    let mut next = avbn + area_blocks;
    let mut start = Vec::new();
    let mut used = vec![0u32; spec.areas.len()];
    for b in &p.buckets {
        used[b.area as usize] += spec.areas[b.area as usize].bktsz as u32;
    }
    let mut extent = Vec::new();
    for (a, area) in spec.areas.iter().enumerate() {
        start.push(next);
        extent.push(used[a].max(area.alloc));
        next += extent[a];
    }
    let total = next - 1;
    let cluster = spec.cluster.max(1);
    if let Some(last) = extent.last_mut() {
        *last += total.next_multiple_of(cluster) - total;
    }
    let total = total.next_multiple_of(cluster);

    let mut at = start.clone();
    let vbns: Vec<u32> = p
        .buckets
        .iter()
        .map(|b| {
            let v = at[b.area as usize];
            at[b.area as usize] += spec.areas[b.area as usize].bktsz as u32;
            v
        })
        .collect();
    let mut file = vec![0u8; total as usize * BLOCK];
    for (i, b) in p.buckets.iter().enumerate() {
        let vbn = vbns[i];
        let mut buf = b.buf.clone();
        let mut h = BucketHeader([0u8; BKT_HDR]);
        h.set_indexno(b.keyref);
        h.set_adrsample(vbn as u16);
        h.set_freespace(b.free as u16);
        h.set_nxtrecid(if b.level == 0 && b.keyref == 0 { b.count as u16 + 1 } else { 1 });
        h.set_nxtbkt(vbns[b.next]);
        h.set_level(b.level);
        let mut cb = if b.last { bktcb::LASTBKT } else { 0 };
        if b.root {
            cb |= bktcb::ROOTBKT;
        }
        if b.level > 0 {
            cb |= ((b.ps - 2) as u8) << 3;
            let size = buf.len();
            let vfree = (size - 4 - b.count * b.ps - 1) as u16;
            buf[size - 4..size - 2].copy_from_slice(&vfree.to_le_bytes());
        }
        h.set_bktcb(cb);
        buf[..BKT_HDR].copy_from_slice(&h.0);
        for &(off, width, t) in &b.patches {
            let v = match t {
                Target::Bucket(x) => vbns[x],
                Target::Record(r) => vbns[p.placed[r].0],
            };
            buf[off..off + width].copy_from_slice(&v.to_le_bytes()[..width]);
            if let Target::Record(r) = t {
                buf[off - 2..off].copy_from_slice(&p.placed[r].1.to_le_bytes());
            }
        }
        let at = (vbn as usize - 1) * BLOCK;
        file[at..at + buf.len()].copy_from_slice(&buf);
    }

    // The prologue: key descriptors, chained, then the areas.
    let place = |n: usize| if n == 0 { (1u32, 0usize) } else { (2 + (n as u32 - 1) / 5, (n - 1) % 5 * KEY_LEN) };
    for (n, k) in spec.keys.iter().enumerate() {
        let mut d = KeyDesc([0u8; KEY_LEN]);
        if n + 1 < spec.keys.len() {
            let (v, o) = place(n + 1);
            d.set_idxfl(v);
            d.set_noff(o as u16);
        }
        d.set_ianum(k.index_area);
        d.set_lanum(k.level1_area);
        d.set_danum(k.data_area);
        let isize = spec.areas[k.index_area as usize].bktsz;
        let dsize = spec.areas[k.data_area as usize].bktsz;
        d.set_idxbktsz(isize);
        d.set_datbktsz(dsize);
        let mut flags = k.flags(n == 0);
        match &trees[n] {
            Some(t) => {
                d.set_rootlev(t.levels);
                d.set_rootvbn(vbns[t.root]);
                d.set_ldvbn(vbns[t.first]);
            }
            None => flags |= keyflag::INITIDX,
        }
        d.set_flags(flags);
        d.set_datatype(k.datatype);
        d.set_segments(k.segments.len() as u8);
        d.set_nullchar(k.null_value);
        d.set_keysz(k.size() as u8);
        d.set_keyref(n as u8);
        d.set_minrecsz(k.min_record() as u16);
        d.set_idxfill(fill(k.index_fill, isize as usize * BLOCK) as u16);
        d.set_datfill(fill(k.data_fill, dsize as usize * BLOCK) as u16);
        let (mut sizes, mut types) = ([0u8; 8], [0u8; 8]);
        for (i, &(pos, size)) in k.segments.iter().enumerate() {
            d.0[28 + 2 * i..30 + 2 * i].copy_from_slice(&pos.to_le_bytes());
            sizes[i] = size;
            types[i] = if k.segments.len() == 1 { k.datatype } else { 0 };
        }
        d.set_sizes(sizes);
        d.set_types(types);
        if !k.name.is_empty() {
            let mut name = [b' '; 32];
            let len = k.name.len().min(32);
            name[..len].copy_from_slice(&k.name[..len]);
            d.set_keynam(name);
        }
        let (v, o) = place(n);
        let at = (v as usize - 1) * BLOCK + o;
        file[at..at + KEY_LEN].copy_from_slice(&d.0);
    }
    file[0x66] = avbn as u8;
    file[0x67] = spec.areas.len() as u8;
    file[0x74] = 3;
    for (a, area) in spec.areas.iter().enumerate() {
        let mut d = AreaDesc([0u8; AREA_LEN]);
        d.set_areaid(a as u8);
        d.set_arbktsz(area.bktsz);
        d.set_cvbn(start[a]);
        d.set_cnblk(extent[a]);
        d.set_used(used[a]);
        d.set_nxtvbn(start[a] + used[a]);
        d.set_deq(area.deq);
        d.set_total_alloc(extent[a]);
        let at = (avbn as usize + a / 8 - 1) * BLOCK + a % 8 * AREA_LEN;
        file[at..at + AREA_LEN].copy_from_slice(&d.0);
    }
    for vbn in 1..avbn + area_blocks {
        let at = (vbn as usize - 1) * BLOCK;
        set_checksum(&mut file[at..at + BLOCK]);
    }
    (file, total)
}
