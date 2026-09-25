//! Directory files: sorted records of names, each with its versions and
//! file IDs, packed into blocks that each end with a -1 word.
//!
//! Writes keep a crash from leaving anything worse than a lost entry (its
//! file becomes an orphan, which is leaked space): a change that fits its
//! block is one block write; an append that spills out of the last block
//! goes to a spare block past the end of file first; anything else writes
//! the whole directory to a new place and then switches the header to it.

use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::index::pointers;
use crate::layout::{BLOCK, DirBlock, DirRecord, Header, MAX_RECORD, NameType, encode_map};
use crate::name::{self, Spec, Version};
use crate::volume::{Run, map_vbn};
use crate::{Alloc, BlockDevice, Error, Fid, Result, Volume, fch};

/// The versions of one name: what directory updates work on.
#[derive(Clone, Debug, Default)]
pub(crate) struct Group {
    /// The name as stored (on ODS-5, the case of the first version).
    pub name: Vec<u8>,
    pub flags: u8,
    pub verlimit: u16,
    /// Highest version first.
    pub entries: Vec<(u16, Fid)>,
}

/// Directory blocks are filled to this when rewritten, leaving room to
/// insert in place later.
const FILL: usize = BLOCK * 3 / 4;

/// File ID of the master file directory, 000000.DIR.
pub const MFD: Fid = Fid::new(4, 4);

/// One version of one name, as listed in a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    /// "NAME.TYPE" as stored.
    pub name: Vec<u8>,
    pub name_type: NameType,
    pub version: u16,
    pub fid: Fid,
    /// The name's version limit (0: none).
    pub verlimit: u16,
}

impl DirEntry {
    pub fn chars(&self) -> Vec<u16> {
        name::chars(&self.name, self.name_type)
    }

    /// Whether this is a subdirectory entry: NAME.DIR;1.
    pub fn is_dir_name(&self) -> bool {
        self.version == 1 && self.name.ends_with(b".DIR")
    }
}

/// A directory's headers (with their LBNs), map and parsed used blocks.
pub(crate) type DirContents = (Vec<(u64, Header)>, Vec<Run>, Vec<DirBlock>);

/// Blocks in use in a directory, from its end of file mark.
pub(crate) fn used_blocks(h: &Header, allocated: u64) -> u64 {
    let ra = h.record_attrs();
    let used = match ra.efblk {
        0 => 0,
        n => n as u64 - 1 + (ra.ffbyte != 0) as u64,
    };
    used.min(allocated)
}

impl<D: BlockDevice> Volume<D> {
    /// Reads and parses a directory's used blocks.
    pub(crate) fn read_dir(&mut self, dir: Fid) -> Result<DirContents, D::Error> {
        let hs = self.headers(dir)?;
        if hs[0].1.filechar() & fch::DIRECTORY == 0 {
            return Err(Error::NotDirectory);
        }
        let mut map = Vec::new();
        for (lbn, h) in &hs {
            map.extend(self.header_runs(h, *lbn)?);
        }
        let allocated = map.iter().map(|r| r.count).sum();
        let mut blocks = Vec::new();
        for vbn in 1..=used_blocks(&hs[0].1, allocated) {
            let lbn = map_vbn(&map, vbn).map(|(l, _)| l).unwrap_or(0);
            let b = self.read_block(lbn)?;
            blocks.push(DirBlock::parse(&b).map_err(|what| Error::Corrupt { what, lbn })?);
        }
        Ok((hs, map, blocks))
    }

    /// Every entry of a directory, in directory order: names ascending,
    /// versions descending.
    pub fn list(&mut self, dir: Fid) -> Result<Vec<DirEntry>, D::Error> {
        let (_, _, blocks) = self.read_dir(dir)?;
        let mut out = Vec::new();
        for r in blocks.iter().flat_map(|b| &b.records) {
            for &(version, fid) in &r.entries {
                out.push(DirEntry {
                    name: r.name.clone(),
                    name_type: r.name_type(),
                    version,
                    fid,
                    verlimit: r.verlimit,
                });
            }
        }
        Ok(out)
    }

    /// The versions of `name` in `dir`, highest first.
    pub fn versions(&mut self, dir: Fid, name: &[u8]) -> Result<Vec<DirEntry>, D::Error> {
        let want = name::chars(name, NameType::Isl1);
        let mut out = self.list(dir)?;
        out.retain(|e| name::cmp(&e.chars(), &want) == Ordering::Equal);
        Ok(out)
    }

    /// Finds one version of `name` in `dir`.
    pub fn lookup(&mut self, dir: Fid, name: &[u8], version: Version) -> Result<DirEntry, D::Error> {
        let vs = self.versions(dir, name)?;
        let found = match version {
            Version::Highest => vs.into_iter().next(),
            Version::Exact(v) => vs.into_iter().find(|e| e.version == v),
            Version::Relative(n) => vs.into_iter().nth(n as usize),
            Version::All => return Err(Error::Invalid("wildcard version")),
        };
        found.ok_or(Error::NotFound)
    }

    /// Walks a directory path down from the MFD.
    pub fn find_dir(&mut self, dirs: &[Vec<u8>]) -> Result<Fid, D::Error> {
        let mut fid = MFD;
        for d in dirs {
            let mut n = d.clone();
            n.extend_from_slice(b".DIR");
            fid = match self.lookup(fid, &n, Version::Exact(1)) {
                Ok(e) => e.fid,
                Err(Error::NotFound) => return Err(Error::DirNotFound),
                Err(e) => return Err(e),
            };
        }
        Ok(fid)
    }

    /// Resolves a specification without wildcards to a file ID. A spec with
    /// no file name resolves to its directory.
    pub fn lookup_spec(&mut self, spec: &Spec) -> Result<Fid, D::Error> {
        if spec.is_wild() {
            return Err(Error::Invalid("wildcards in a single-file lookup"));
        }
        let dir = self.find_dir(&spec.dirs)?;
        match &spec.file {
            None => Ok(dir),
            Some(f) => Ok(self.lookup(dir, &f.name, f.version)?.fid),
        }
    }

    /// Parses and resolves a VMS file specification.
    pub fn lookup_path(&mut self, spec: &str) -> Result<Fid, D::Error> {
        let spec = name::parse(self.level, spec).map_err(Error::BadName)?;
        self.lookup_spec(&spec)
    }

    /// Changes the versions of `name` in `dir` through `f`, then writes the
    /// directory back. `f` sees an empty group for a name not yet there.
    pub(crate) fn dir_update<T>(
        &mut self,
        dir: Fid,
        name: &[u8],
        f: impl FnOnce(&mut Group) -> Result<T, D::Error>,
    ) -> Result<T, D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let (hs, map, mut blocks) = self.read_dir(dir)?;
        let want = name::chars(name, NameType::Isl1);
        let key = |r: &DirRecord| name::cmp(&name::chars(&r.name, r.name_type()), &want);
        // Where the name's records are, or where they would go.
        let mut at: Vec<(usize, usize)> = Vec::new();
        let mut insert = None;
        for (b, blk) in blocks.iter().enumerate() {
            for (i, r) in blk.records.iter().enumerate() {
                match key(r) {
                    Ordering::Equal => at.push((b, i)),
                    Ordering::Greater if insert.is_none() => insert = Some((b, i)),
                    _ => {}
                }
            }
        }
        let mut g = Group::default();
        for &(b, i) in &at {
            let r = &blocks[b].records[i];
            if g.entries.is_empty() {
                (g.name, g.flags, g.verlimit) = (r.name.clone(), r.flags, r.verlimit);
            }
            g.entries.extend_from_slice(&r.entries);
        }
        let out = f(&mut g)?;
        g.entries.sort_by_key(|e| core::cmp::Reverse(e.0));
        let recs = records(&g);
        let last = blocks.len().saturating_sub(1);
        let same_block = at.iter().all(|&(b, _)| b == at[0].0);
        let (b, i) = match (at.first(), insert) {
            (Some(&(b, i)), _) => (b, i),
            (None, Some(p)) => p,
            (None, None) => (last, blocks.get(last).map_or(0, |x| x.records.len())),
        };
        if blocks.is_empty() || !same_block {
            return self.dir_rewrite(&hs, &map, blocks, &recs).map(|_| out);
        }
        let appending = at.is_empty() && b == last && i == blocks[last].records.len();
        let mut blk = blocks[b].clone();
        blk.records.retain(|r| key(r) != Ordering::Equal);
        blk.records.splice(i..i, recs.iter().cloned());
        let lbn = |vbn: u64| map_vbn(&map, vbn).map(|(l, _)| l).unwrap_or(0);
        if let Some(raw) = blk.to_block() {
            if !blk.records.is_empty() || blocks.len() == 1 {
                return self.write_block(lbn(b as u64 + 1), &raw).map(|_| out);
            }
            if b == last {
                // The last block emptied: just end the file before it.
                self.set_dir_eof(&hs, last as u64 + 1)?;
                return Ok(out);
            }
        } else if appending && self.dir_append_block(dir, &hs, &map, &recs)? {
            return Ok(out);
        }
        blocks[b] = blk;
        self.dir_rewrite(&hs, &map, blocks, &[]).map(|_| out)
    }

    /// Moves the directory's end of file to `efblk`.
    fn set_dir_eof(&mut self, hs: &[(u64, Header)], efblk: u64) -> Result<(), D::Error> {
        let (lbn, mut h) = hs[0];
        let mut r = h.record_attrs();
        r.efblk = efblk as u32;
        r.ffbyte = 0;
        h.set_record_attrs(&r);
        if h.highwater_mark().is_some_and(|hw| (hw as u64) < efblk) {
            h.set_highwater(efblk as u32);
        }
        self.write_header(lbn, &mut h)
    }

    /// Puts records for a name that sorts after every other into a new
    /// block after the end of file: already allocated, or taken from free
    /// space right after the directory. Writes the block, then moves the end
    /// of file over it, which is the one write that makes the entry appear.
    /// `false` if there is no room.
    fn dir_append_block(
        &mut self,
        dir: Fid,
        hs: &[(u64, Header)],
        map: &[Run],
        recs: &[DirRecord],
    ) -> Result<bool, D::Error> {
        let Some(raw) = (DirBlock { records: recs.to_vec(), tail: Vec::new() }).to_block() else {
            return Ok(false);
        };
        let allocated: u64 = map.iter().map(|r| r.count).sum();
        let used = used_blocks(&hs[0].1, allocated);
        let mut map = map.to_vec();
        if used >= allocated {
            let after = map.last().map(|r| r.lbn + r.count);
            match self.allocate(self.cluster(), Alloc::Contiguous, after) {
                Ok(runs) => {
                    self.append_runs(dir, &runs)?;
                    map.extend(runs);
                }
                Err(Error::DeviceFull) => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        let hs = self.headers(dir)?;
        let lbn = map_vbn(&map, used + 1).map(|(l, _)| l).unwrap_or(0);
        self.write_block(lbn, &raw)?;
        self.set_dir_eof(&hs, used + 2)?;
        Ok(true)
    }

    /// Writes the whole directory to newly allocated contiguous space and
    /// points the header at it, then frees the old space. `recs`, if not
    /// empty, is inserted in name order (for a group spread over blocks).
    fn dir_rewrite(
        &mut self,
        hs: &[(u64, Header)],
        old: &[Run],
        blocks: Vec<DirBlock>,
        recs: &[DirRecord],
    ) -> Result<(), D::Error> {
        let mut all: Vec<DirRecord> = blocks.into_iter().flat_map(|b| b.records).collect();
        if let Some(first) = recs.first() {
            let want = name::chars(&first.name, NameType::Isl1);
            all.retain(|r| name::cmp(&name::chars(&r.name, r.name_type()), &want) != Ordering::Equal);
            let pos = all
                .iter()
                .position(|r| name::cmp(&name::chars(&r.name, r.name_type()), &want) == Ordering::Greater)
                .unwrap_or(all.len());
            all.splice(pos..pos, recs.iter().cloned());
        }
        let packed = pack(all);
        let n = packed.len() as u64;
        let alloc = (n + n / 4).max(1);
        let runs = self.allocate(alloc, Alloc::Contiguous, None)?;
        let start = runs[0].lbn;
        for (i, blk) in packed.iter().enumerate() {
            let raw = blk.to_block().ok_or(Error::Invalid("directory record too large"))?;
            self.write_block(start + i as u64, &raw)?;
        }
        let (plbn, mut h) = hs[0];
        let mut m = Vec::new();
        encode_map(&pointers(&runs), &mut m);
        h.set_map(&m);
        h.set_ext_fid(Fid::default());
        let mut r = h.record_attrs();
        r.hiblk = runs.iter().map(|r| r.count).sum::<u64>() as u32;
        r.efblk = n as u32 + 1;
        r.ffbyte = 0;
        h.set_record_attrs(&r);
        if h.highwater_mark().is_some() {
            h.set_highwater(n as u32 + 1);
        }
        self.write_header(plbn, &mut h)?;
        for (lbn, e) in &hs[1..] {
            self.delete_header(*lbn, *e)?;
        }
        self.release(old)
    }
}

/// A group's records: the name repeated as often as its versions need,
/// each record as full as a block allows.
fn records(g: &Group) -> Vec<DirRecord> {
    let per = (MAX_RECORD - 6 - g.name.len().next_multiple_of(2)) / 8;
    g.entries
        .chunks(per.max(1))
        .map(|c| DirRecord { name: g.name.clone(), verlimit: g.verlimit, flags: g.flags, entries: c.to_vec(), pad: 0 })
        .collect()
}

/// Packs records into blocks, each filled to about three quarters.
fn pack(all: Vec<DirRecord>) -> Vec<DirBlock> {
    let mut out = Vec::new();
    let mut cur = DirBlock::default();
    for r in all {
        let used = cur.used();
        if !cur.records.is_empty() && (used + r.size() > FILL || used + r.size() > MAX_RECORD) {
            out.push(core::mem::take(&mut cur));
        }
        cur.records.push(r);
    }
    out.push(cur);
    out
}
