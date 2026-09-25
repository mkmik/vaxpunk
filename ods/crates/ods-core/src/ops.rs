//! Operations that change several structures, in the order VMS uses:
//! space is taken in the bitmaps before anything points at it, a header is
//! written before the directory entry that names it, and an entry goes away
//! before its file does. A crash between two writes leaves at worst leaked
//! space (an allocated block or header nothing uses, a file no directory
//! lists), never a reference to something that is not there.

use alloc::vec::Vec;

use crate::dir::Group;
use crate::index::pointers;
use crate::layout::{BLOCK, Ident, NameType, encode_map};
use crate::name::{self, MAX_VERSION, Version};
use crate::volume::Level;
use crate::{Alloc, BlockDevice, Error, Fid, RecordAttrs, Result, Volume, fch, rat, rfm};

/// What a new file starts with. Its end of file is VBN 1: nothing written.
#[derive(Clone, Debug, Default)]
pub struct NewFile {
    /// Record attributes; the allocation and end of file fields are ignored.
    pub record: RecordAttrs,
    /// Blocks to allocate now.
    pub blocks: u64,
    pub alloc: Alloc,
    /// Default: the directory's owner.
    pub owner: Option<u32>,
    /// Default: the volume's default file protection.
    pub protection: Option<u16>,
}

/// Directory record flags for a stored name: the name type in bits 3-5.
/// On ODS-5, as VMS does, a name that ODS-2 would accept once uppercased
/// is of the ODS-2 type, whatever its case; others are ISO Latin-1.
fn name_flags(level: Level, name: &[u8]) -> (NameType, u8) {
    let t = match level {
        Level::Ods5 if name::validate(Level::Ods2, &name.to_ascii_uppercase()).is_err() => NameType::Isl1,
        _ => NameType::Ods2,
    };
    (t, t.code() << 3)
}

/// "NAME.TYPE;VERSION", as the ident area stores it.
fn ident_name(name: &[u8], version: u16) -> Vec<u8> {
    let mut n = name.to_vec();
    n.push(b';');
    let mut digits = [0u8; 5];
    let mut v = version;
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    n.extend_from_slice(&digits[i..]);
    n
}

impl<D: BlockDevice> Volume<D> {
    fn check_dir(&mut self, dir: Fid) -> Result<crate::layout::Header, D::Error> {
        let h = self.read_header(dir)?;
        if h.filechar() & fch::DIRECTORY == 0 {
            return Err(Error::NotDirectory);
        }
        Ok(h)
    }

    /// Creates `name` ("NAME.TYPE") in `dir`, as `version` or one above the
    /// highest. Versions beyond the name's version limit are purged.
    /// Returns the new file and its version.
    pub fn create(&mut self, dir: Fid, name: &[u8], version: Option<u16>, f: &NewFile) -> Result<(Fid, u16), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        name::validate(self.level, name).map_err(Error::BadName)?;
        let dh = self.check_dir(dir)?;
        let existing = self.versions(dir, name)?;
        let version = match version {
            Some(v) if v == 0 || v > MAX_VERSION => return Err(Error::Invalid("version out of range")),
            Some(v) if existing.iter().any(|e| e.version == v) => return Err(Error::Exists),
            Some(v) => v,
            None => match existing.first() {
                Some(e) if e.version >= MAX_VERSION => return Err(Error::VersionOverflow),
                Some(e) => e.version + 1,
                None => 1,
            },
        };
        // All versions share the case of the first one (ODS-5).
        let stored = existing.first().map_or(name.to_vec(), |e| e.name.clone());
        let (fid, lbn) = self.alloc_header()?;
        let runs = match self.allocate(f.blocks, f.alloc, None) {
            Ok(r) => r,
            Err(e) => {
                let old = self.read_block(lbn)?;
                self.delete_header(lbn, crate::layout::Header(old))?;
                return Err(e);
            }
        };
        let (name_type, flags) = name_flags(self.level, &stored);
        let full = ident_name(&stored, version);
        let mut h = self.new_header(fid, Some(full.len()));
        let mut r = f.record;
        r.hiblk = 0;
        r.efblk = 1;
        r.ffbyte = 0;
        h.set_record_attrs(&r);
        let mut fc = 0;
        if f.alloc == Alloc::Contiguous && !runs.is_empty() {
            fc |= fch::CONTIG;
        }
        if f.alloc == Alloc::BestTry {
            fc |= fch::CONTIGB;
        }
        h.set_filechar(fc);
        h.set_fileowner(f.owner.unwrap_or(dh.fileowner()));
        h.set_fileprot(f.protection.unwrap_or(self.home.fileprot()));
        h.set_backlink(dir);
        h.set_highwater(1);
        let now = (self.clock)();
        h.set_ident(&Ident {
            name: full,
            name_type,
            revision: 1,
            credate: now,
            revdate: now,
            accdate: now,
            attdate: now,
            ..Ident::default()
        });
        self.write_header(lbn, &mut h)?;
        // The space goes on like any extension: it may need more headers.
        if let Err(e) = self.append_runs(fid, &runs) {
            if !matches!(e, Error::Device(_)) {
                self.delete_file(fid)?;
                self.release(&runs)?;
            }
            return Err(e);
        }
        let default_limit = match dh.record_attrs().versions {
            0 => MAX_VERSION,
            n => n,
        };
        let entered = self.dir_update(dir, &stored, |g: &mut Group| {
            if g.entries.is_empty() {
                (g.name, g.flags, g.verlimit) = (stored.clone(), flags, default_limit);
            }
            if g.entries.iter().any(|e| e.0 == version) {
                return Err(Error::Exists);
            }
            g.entries.push((version, fid));
            g.entries.sort_by_key(|e| core::cmp::Reverse(e.0));
            Ok(g.entries.iter().skip(g.verlimit.max(1) as usize).copied().collect::<Vec<_>>())
        });
        let excess = match entered {
            Ok(x) => x,
            Err(e) => {
                // Undo, so a failed create leaks nothing.
                self.delete_file(fid)?;
                return Err(e);
            }
        };
        for (v, _) in excess {
            self.delete(dir, &stored, v)?;
        }
        Ok((fid, version))
    }

    /// Creates subdirectory `name` (without ".DIR") in `parent`. It gets
    /// the parent's owner, default version limit and protection, less
    /// delete access.
    pub fn create_dir(&mut self, parent: Fid, name: &[u8]) -> Result<Fid, D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let mut full = name.to_vec();
        full.extend_from_slice(b".DIR");
        name::validate(self.level, &full).map_err(Error::BadName)?;
        let ph = self.check_dir(parent)?;
        if !self.versions(parent, &full)?.is_empty() {
            return Err(Error::Exists);
        }
        let v = self.cluster();
        let (fid, lbn) = self.alloc_header()?;
        let runs = match self.allocate(v, Alloc::Contiguous, None) {
            Ok(r) => r,
            Err(e) => {
                let old = self.read_block(lbn)?;
                self.delete_header(lbn, crate::layout::Header(old))?;
                return Err(e);
            }
        };
        let mut empty = [0u8; BLOCK];
        empty[..2].copy_from_slice(&0xffffu16.to_le_bytes());
        self.write_block(runs[0].lbn, &empty)?;
        let (name_type, flags) = name_flags(self.level, &full);
        let id_name = ident_name(&full, 1);
        let mut h = self.new_header(fid, Some(id_name.len()));
        h.set_record_attrs(&RecordAttrs {
            rtype: rfm::VAR,
            rattrib: rat::BLK,
            rsize: BLOCK as u16,
            maxrec: BLOCK as u16,
            hiblk: v as u32,
            efblk: 2,
            versions: ph.record_attrs().versions,
            ..RecordAttrs::default()
        });
        h.set_filechar(fch::DIRECTORY | fch::CONTIG);
        h.set_fileowner(ph.fileowner());
        h.set_fileprot(ph.fileprot() | 0x8888);
        h.set_backlink(parent);
        h.set_highwater(2);
        let now = (self.clock)();
        h.set_ident(&Ident {
            name: id_name,
            name_type,
            revision: 1,
            credate: now,
            revdate: now,
            accdate: now,
            attdate: now,
            ..Ident::default()
        });
        let mut m = Vec::new();
        encode_map(&pointers(&runs), &mut m);
        if !h.set_map(&m) {
            return Err(Error::Invalid("map pointers overflow the header"));
        }
        self.write_header(lbn, &mut h)?;
        let r = self.dir_update(parent, &full, |g: &mut Group| {
            if !g.entries.is_empty() {
                return Err(Error::Exists);
            }
            (g.name, g.flags, g.verlimit) = (full.clone(), flags, MAX_VERSION);
            g.entries.push((1, fid));
            Ok(())
        });
        if let Err(e) = r {
            self.delete_file(fid)?;
            return Err(e);
        }
        Ok(fid)
    }

    /// Deletes one version. The file goes with its entry when the entry is
    /// its primary one (the directory its back link names); an alias entry
    /// is just removed. A directory must be empty.
    pub fn delete(&mut self, dir: Fid, name: &[u8], version: u16) -> Result<(), D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        let e = self.lookup(dir, name, Version::Exact(version))?;
        if e.fid.num <= self.home.resfiles() as u32 {
            return Err(Error::Reserved);
        }
        let h = self.read_header(e.fid)?;
        if h.filechar() & fch::DIRECTORY != 0 && !self.list(e.fid)?.is_empty() {
            return Err(Error::DirNotEmpty);
        }
        self.remove_entry(dir, name, version)?;
        if h.backlink() == dir || h.backlink().num == dir.num && h.backlink().seq == 0 {
            self.delete_file(e.fid)?;
        }
        Ok(())
    }

    fn remove_entry(&mut self, dir: Fid, name: &[u8], version: u16) -> Result<Fid, D::Error> {
        self.dir_update(dir, name, |g: &mut Group| {
            let i = g.entries.iter().position(|e| e.0 == version).ok_or(Error::NotFound)?;
            Ok(g.entries.remove(i).1)
        })
    }

    /// Frees a file's headers and blocks, erasing the blocks first if the
    /// file or volume asks for it.
    pub(crate) fn delete_file(&mut self, fid: Fid) -> Result<(), D::Error> {
        let hs = self.headers(fid)?;
        let mut runs = Vec::new();
        for (lbn, h) in &hs {
            runs.extend(self.header_runs(h, *lbn)?);
        }
        if hs[0].1.filechar() & fch::ERASE != 0 || self.home.volchar() & 4 != 0 {
            let zero = [0u8; BLOCK];
            for r in &runs {
                for lbn in r.lbn..r.lbn + r.count {
                    self.write_block(lbn, &zero)?;
                }
            }
        }
        for (lbn, h) in &hs {
            self.delete_header(*lbn, *h)?;
        }
        self.release(&runs)
    }

    /// Deletes all but the `keep` highest versions of `name`. Returns the
    /// versions deleted.
    pub fn purge(&mut self, dir: Fid, name: &[u8], keep: usize) -> Result<Vec<u16>, D::Error> {
        let old: Vec<u16> = self.versions(dir, name)?.iter().skip(keep.max(1)).map(|e| e.version).collect();
        for &v in &old {
            self.delete(dir, name, v)?;
        }
        Ok(old)
    }

    /// Renames or moves a directory entry: the new entry is made, the
    /// header's name and back link updated, then the old entry removed.
    /// Without `to_version` the version stays if free, else goes above the
    /// highest. Returns the new version.
    pub fn rename(
        &mut self,
        from_dir: Fid,
        from: &[u8],
        from_version: Version,
        to_dir: Fid,
        to: &[u8],
        to_version: Option<u16>,
    ) -> Result<u16, D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        name::validate(self.level, to).map_err(Error::BadName)?;
        self.check_dir(to_dir)?;
        let e = self.lookup(from_dir, from, from_version)?;
        if e.fid.num <= self.home.resfiles() as u32 {
            return Err(Error::Reserved);
        }
        let hlbn = self.header_lbn(e.fid.num).unwrap_or(0);
        let mut h = self.read_header(e.fid)?;
        let is_dir = h.filechar() & fch::DIRECTORY != 0;
        if is_dir {
            let (_, t) = name::split(to);
            if !t.eq_ignore_ascii_case(b"DIR") || to_version.is_some_and(|v| v != 1) {
                return Err(Error::Invalid("a directory must be renamed to NAME.DIR;1"));
            }
            if self.contains_dir(e.fid, to_dir)? {
                return Err(Error::Invalid("cannot move a directory into itself"));
            }
        }
        let existing = self.versions(to_dir, to)?;
        let same =
            to_dir == from_dir && name::cmp(&name::chars(to, NameType::Isl1), &e.chars()) == core::cmp::Ordering::Equal;
        let version = match to_version {
            _ if is_dir => 1,
            Some(v) if v == 0 || v > MAX_VERSION => return Err(Error::Invalid("version out of range")),
            Some(v) => v,
            None if !existing.iter().any(|x| x.version == e.version) => e.version,
            None => match existing.first() {
                Some(x) if x.version >= MAX_VERSION => return Err(Error::VersionOverflow),
                Some(x) => x.version + 1,
                None => 1,
            },
        };
        if same && version == e.version {
            return Ok(version);
        }
        let stored = existing.first().map_or(to.to_vec(), |x| x.name.clone());
        let (name_type, flags) = name_flags(self.level, &stored);
        let limit = match self.read_header(to_dir)?.record_attrs().versions {
            0 => MAX_VERSION,
            n => n,
        };
        self.dir_update(to_dir, &stored, |g: &mut Group| {
            if g.entries.iter().any(|x| x.0 == version) {
                return Err(Error::Exists);
            }
            if g.entries.is_empty() {
                (g.name, g.flags, g.verlimit) = (stored.clone(), flags, limit);
            }
            g.entries.push((version, e.fid));
            Ok(())
        })?;
        if let Some(mut id) = h.ident() {
            let full = ident_name(&stored, version);
            let words = crate::layout::Header::ident_words(self.level.number(), full.len());
            if h.struclev() >> 8 != 5 || words <= h.mpoffset() - h.idoffset() {
                id.name = full;
                id.name_type = name_type;
                h.set_ident(&id);
            }
        }
        if h.backlink() == from_dir {
            h.set_backlink(to_dir);
        }
        self.write_header(hlbn, &mut h)?;
        self.remove_entry(from_dir, &e.name, e.version)?;
        Ok(version)
    }

    /// Whether `inner` is `dir` or below it.
    fn contains_dir(&mut self, dir: Fid, inner: Fid) -> Result<bool, D::Error> {
        let mut todo = Vec::from([dir]);
        let mut seen = Vec::new();
        while let Some(d) = todo.pop() {
            if d == inner {
                return Ok(true);
            }
            if seen.contains(&d) {
                continue;
            }
            seen.push(d);
            for e in self.list(d)? {
                if e.is_dir_name()
                    && e.fid != d
                    && self.read_header(e.fid).is_ok_and(|h| h.filechar() & fch::DIRECTORY != 0)
                {
                    todo.push(e.fid);
                }
            }
        }
        Ok(false)
    }
}
