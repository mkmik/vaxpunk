//! Files-11 volumes in image files, for programs on macOS and Linux.
//!
//! [`Image`] binds `ods-core` to an image file and adds what a host program
//! wants on top of virtual blocks: paths in VMS or Unix form, byte streams,
//! records, text conversion, whole-tree copies, and errors that say what
//! they were about.

pub mod attrs;
mod copy;
mod device;
mod error;
pub mod path;
pub mod records;
pub mod time;
mod tree;

use std::fs::{File, OpenOptions};
use std::path::Path;

pub use copy::{Conversion, FileReader, TextView};
pub use device::{Container, FileDevice};
pub use error::{Context, Error, Kind, OdsError, Result};
pub use ods_core::layout::{self, Header, HomeBlock, NameType};
pub use ods_core::name::{self, Spec, Version};
pub use ods_core::{
    Alloc, Attributes, DirEntry, Fid, FileInfo, Finding, InitParams, Level, MFD, NewFile, RecordAttrs, Report,
    Severity, fch, rat, rfm,
};
pub use tree::{Manifest, ManifestEntry, safe_host_name};

use ods_core::{BLOCK, Volume};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    ReadOnly,
    ReadWrite,
}

/// A mounted image file.
pub struct Image {
    vol: Volume<FileDevice>,
    container: Container,
}

/// A directory entry found by a search, with the directory it is in.
#[derive(Clone, Debug)]
pub struct Found {
    /// Directory path below the MFD.
    pub dir: Vec<Vec<u8>>,
    pub entry: DirEntry,
}

/// Summary of a volume, from the home block and the bitmaps.
#[derive(Clone, Debug)]
pub struct Info {
    pub label: String,
    pub owner_name: String,
    pub format: String,
    pub level: Level,
    pub struclev: u16,
    pub cluster: u64,
    pub volume_blocks: u64,
    pub free_blocks: u64,
    pub max_files: u32,
    pub files: u64,
    pub reserved_files: u16,
    pub owner_uic: u32,
    pub protection: u16,
    pub file_protection: u16,
    pub created: u64,
    pub container: Container,
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim_end().to_string()
}

impl Image {
    /// Opens and mounts an image. Read-write takes an exclusive lock and
    /// read-only a shared one, so no reader sees a half-written volume.
    pub fn open(path: impl AsRef<Path>, mode: Mode) -> Result<Image> {
        let path = path.as_ref();
        let file = OpenOptions::new().read(true).write(mode == Mode::ReadWrite).open(path).at(path.display())?;
        lock(&file, mode).at(path.display())?;
        let (dev, container) = FileDevice::new(file).at(path.display())?;
        let mut vol = Volume::mount(dev, mode == Mode::ReadWrite).at(path.display())?;
        vol.set_clock(time::now);
        Ok(Image { vol, container })
    }

    /// Creates a new image file of `blocks` blocks and initializes a volume
    /// on it. Fails if the file exists.
    pub fn create(path: impl AsRef<Path>, blocks: u64, params: &InitParams) -> Result<Image> {
        let path = path.as_ref();
        let file = OpenOptions::new().read(true).write(true).create_new(true).open(path).at(path.display())?;
        lock(&file, Mode::ReadWrite).at(path.display())?;
        file.set_len(blocks * BLOCK as u64).at(path.display())?;
        let mut p = params.clone();
        if p.now == 0 {
            p.now = time::now();
        }
        let mut vol = ods_core::initialize(FileDevice::raw(file, blocks), &p).at(path.display())?;
        vol.set_clock(time::now);
        Ok(Image { vol, container: Container::Raw })
    }

    /// Writes everything out to the image file.
    pub fn flush(&mut self) -> Result<()> {
        ods_core::BlockDevice::flush(self.vol.device()).map_err(|e| Error::from(OdsError::Device(e)))
    }

    pub fn level(&self) -> Level {
        self.vol.level()
    }

    pub fn home(&self) -> &HomeBlock {
        self.vol.home()
    }

    pub fn container(&self) -> &Container {
        &self.container
    }

    pub fn is_writable(&self) -> bool {
        self.vol.is_writable()
    }

    pub fn info(&mut self) -> Result<Info> {
        let h = *self.vol.home();
        Ok(Info {
            label: text(&h.volname()),
            owner_name: text(&h.ownername()),
            format: text(&h.format()),
            level: self.vol.level(),
            struclev: h.struclev(),
            cluster: self.vol.cluster(),
            volume_blocks: self.vol.volume_size(),
            free_blocks: self.vol.free_blocks()?,
            max_files: h.maxfiles(),
            files: self.vol.files_in_use()?,
            reserved_files: h.resfiles(),
            owner_uic: h.volowner(),
            protection: h.protect(),
            file_protection: h.fileprot(),
            created: h.credate(),
            container: self.container.clone(),
        })
    }

    /// Parses a file specification, VMS or Unix form.
    pub fn parse(&self, spec: &str) -> Result<Spec> {
        let vms = path::to_vms(spec);
        name::parse(self.level(), &vms).map_err(|e| Error::from(OdsError::BadName(e)).at(spec))
    }

    /// Parses a specification that must name one file, and finds its
    /// directory.
    fn parse_file(&mut self, spec: &str) -> Result<(Fid, name::FileName)> {
        let s = self.parse(spec)?;
        let file = s.file.ok_or_else(|| Error::usage("expected a file name, not a directory").at(spec))?;
        if s.dirs.iter().chain([&file.name]).any(|n| n.contains(&name::ANY) || n.contains(&name::ONE)) {
            return Err(Error::usage("wildcards are not allowed here").at(spec));
        }
        let dir = self.vol.find_dir(&s.dirs).at(spec)?;
        Ok((dir, file))
    }

    /// Resolves a specification without wildcards to a file ID. A spec with
    /// only a directory resolves to that directory.
    pub fn lookup(&mut self, spec: &str) -> Result<Fid> {
        let s = self.parse(spec)?;
        self.vol.lookup_spec(&s).at(spec)
    }

    pub fn stat(&mut self, fid: Fid) -> Result<FileInfo> {
        self.vol.stat(fid).at(fid)
    }

    pub fn list(&mut self, dir: Fid) -> Result<Vec<DirEntry>> {
        self.vol.list(dir).at(dir)
    }

    /// Prints a name the way VMS would, with ODS-5 escapes.
    pub fn display(&self, name: &[u8], t: NameType) -> String {
        name::display(self.level(), name, t)
    }

    /// "[A.B]" for a directory path.
    pub fn dir_spec(&self, dir: &[Vec<u8>]) -> String {
        if dir.is_empty() {
            return "[000000]".into();
        }
        // Every dot in a directory name is literal: display it as a name
        // whose type is empty, then drop that type's dot.
        let parts: Vec<String> = dir
            .iter()
            .map(|d| {
                let mut s = self.display(&[&d[..], b"."].concat(), NameType::Isl1);
                s.pop();
                s
            })
            .collect();
        format!("[{}]", parts.join("."))
    }

    /// "[A.B]NAME.TYPE;1" for a file in a directory, with its name as the
    /// header stores it.
    pub fn spec_of(&mut self, dir: &[Vec<u8>], fid: Fid) -> Result<String> {
        let i = self.stat(fid)?;
        Ok(format!("{}{}", self.dir_spec(dir), self.display(&i.name, i.name_type)))
    }

    /// "[A.B]NAME.TYPE;1" for a found entry.
    pub fn file_spec(&self, f: &Found) -> String {
        format!("{}{};{}", self.dir_spec(&f.dir), self.display(&f.entry.name, f.entry.name_type), f.entry.version)
    }

    /// Finds every entry matching a specification with wildcards, `...`
    /// included. A missing file part means `*.*`; a missing version means
    /// the highest.
    pub fn search(&mut self, spec: &str) -> Result<Vec<Found>> {
        let s = self.parse(spec)?;
        let file = s
            .file
            .clone()
            .unwrap_or(name::FileName { name: vec![name::ANY, b'.', name::ANY], version: Version::Highest });
        let mut out = Vec::new();
        let mut seen = vec![MFD];
        self.walk(MFD, &mut Vec::new(), &s.dirs, &file, &mut seen, &mut out).at(spec)?;
        Ok(out)
    }

    fn walk(
        &mut self,
        dir: Fid,
        path: &mut Vec<Vec<u8>>,
        pats: &[Vec<u8>],
        file: &name::FileName,
        seen: &mut Vec<Fid>,
        out: &mut Vec<Found>,
    ) -> std::result::Result<(), OdsError> {
        let entries = self.vol.list(dir)?;
        let Some((pat, rest)) = pats.split_first() else {
            let want = name::chars(&file.name, NameType::Isl1);
            let mut rank = 0;
            for (i, e) in entries.iter().enumerate() {
                rank = if i > 0 && entries[i - 1].name == e.name { rank + 1 } else { 0 };
                let version_ok = match file.version {
                    Version::All => true,
                    Version::Highest => rank == 0,
                    Version::Exact(v) => e.version == v,
                    Version::Relative(n) => rank == n as usize,
                };
                if version_ok && name::matches(&want, &e.chars()) {
                    out.push(Found { dir: path.clone(), entry: e.clone() });
                }
            }
            return Ok(());
        };
        let ellipsis = pat == name::ELLIPSIS;
        if ellipsis {
            self.walk(dir, path, rest, file, seen, out)?;
        }
        let want = name::chars(pat, NameType::Isl1);
        for e in &entries {
            let (stem, ty) = name::split(&e.name);
            if e.version != 1 || !ty.eq_ignore_ascii_case(b"DIR") || seen.contains(&e.fid) {
                continue;
            }
            if !ellipsis && !name::matches(&want, &name::chars(stem, e.name_type)) {
                continue;
            }
            match self.vol.stat(e.fid) {
                Ok(i) if i.attrs.filechar & fch::DIRECTORY != 0 => {}
                _ => continue,
            }
            seen.push(e.fid);
            path.push(stem.to_vec());
            self.walk(e.fid, path, if ellipsis { pats } else { rest }, file, seen, out)?;
            path.pop();
        }
        Ok(())
    }

    /// Reads bytes at `offset`, up to the end of file. Returns how many.
    pub fn read_at(&mut self, fid: Fid, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let eof = self.stat(fid)?.attrs.record.eof_bytes();
        let end = eof.min(offset + buf.len() as u64);
        if offset >= end {
            return Ok(0);
        }
        let first = offset / BLOCK as u64;
        let last = (end - 1) / BLOCK as u64;
        let mut blocks = vec![0u8; ((last - first + 1) as usize) * BLOCK];
        self.vol.read_blocks(fid, first + 1, &mut blocks).at(fid)?;
        let skip = (offset % BLOCK as u64) as usize;
        let n = (end - offset) as usize;
        buf[..n].copy_from_slice(&blocks[skip..skip + n]);
        Ok(n)
    }

    /// Reads a logical block, for dumps.
    pub fn read_block(&mut self, lbn: u64) -> Result<[u8; BLOCK]> {
        self.vol.read_block(lbn).at(format!("LBN {lbn}"))
    }

    /// All headers of a file, with their LBNs, for dumps.
    pub fn headers(&mut self, fid: Fid) -> Result<Vec<(u64, Header)>> {
        self.vol.headers(fid).at(fid)
    }

    /// LBN of a file number's header.
    pub fn header_lbn(&self, num: u32) -> Option<u64> {
        self.vol.header_lbn(num)
    }

    /// The file's map, as (LBN, count) runs.
    pub fn extents(&mut self, fid: Fid) -> Result<Vec<(u64, u64)>> {
        self.vol.extents(fid).at(fid)
    }

    pub fn attributes(&mut self, fid: Fid) -> Result<Attributes> {
        self.vol.attributes(fid).at(fid)
    }

    pub fn set_attributes(&mut self, fid: Fid, a: &Attributes) -> Result<()> {
        self.vol.set_attributes(fid, a).at(fid)
    }

    /// Default version limit of a directory (0: none).
    pub fn set_version_limit(&mut self, dir: Fid, limit: u16) -> Result<()> {
        let mut a = self.attributes(dir)?;
        a.record.versions = limit;
        self.set_attributes(dir, &a)
    }

    /// Creates a directory; its parent must exist.
    pub fn mkdir(&mut self, spec: &str) -> Result<Fid> {
        let s = self.parse(spec)?;
        let mut dirs = s.dirs;
        if let Some(f) = s.file {
            let (stem, ty) = name::split(&f.name);
            if !ty.is_empty() && !ty.eq_ignore_ascii_case(b"DIR") {
                return Err(Error::usage("a directory's type is DIR").at(spec));
            }
            dirs.push(stem.to_vec());
        }
        let (last, parents) = dirs.split_last().ok_or_else(|| Error::usage("the MFD already exists").at(spec))?;
        let parent = self.vol.find_dir(parents).at(spec)?;
        self.vol.create_dir(parent, last).at(spec)
    }

    /// Deletes the files a specification matches. As on VMS it must say
    /// which versions: `;n`, `;0`, `;-n` or `;*`. Returns what was deleted.
    pub fn delete(&mut self, spec: &str) -> Result<Vec<String>> {
        if !spec.contains(';') {
            return Err(Error::from(OdsError::NoVersion).at(spec));
        }
        let found = self.search(spec)?;
        if found.is_empty() {
            return Err(Error::from(OdsError::NotFound).at(spec));
        }
        let mut out = Vec::new();
        for f in found {
            let what = self.file_spec(&f);
            let dir = self.vol.find_dir(&f.dir).at(&what)?;
            self.vol.delete(dir, &f.entry.name, f.entry.version).at(&what)?;
            out.push(what);
        }
        Ok(out)
    }

    /// Keeps the `keep` highest versions of every file matching `spec`.
    /// Returns what was deleted.
    pub fn purge(&mut self, spec: &str, keep: usize) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut done: Vec<(Vec<Vec<u8>>, Vec<u8>)> = Vec::new();
        for f in self.search(spec)? {
            let key = (f.dir.clone(), f.entry.name.clone());
            if done.contains(&key) {
                continue;
            }
            done.push(key);
            let dir = self.vol.find_dir(&f.dir).at(spec)?;
            for v in self.vol.purge(dir, &f.entry.name, keep).at(spec)? {
                let mut g = f.clone();
                g.entry.version = v;
                out.push(self.file_spec(&g));
            }
        }
        Ok(out)
    }

    /// Renames or moves one file. Returns its new specification.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<String> {
        let (fdir, f) = self.parse_file(from)?;
        let t = self.parse(to)?;
        let tdir = self.vol.find_dir(&t.dirs).at(to)?;
        let (tname, tver) = match &t.file {
            Some(tf) => (
                tf.name.clone(),
                match tf.version {
                    Version::Exact(v) => Some(v),
                    _ => None,
                },
            ),
            None => (f.name.clone(), None),
        };
        let v = self.vol.rename(fdir, &f.name, f.version, tdir, &tname, tver).at(from)?;
        let found = Found {
            dir: t.dirs.clone(),
            entry: DirEntry { name: tname, name_type: NameType::Isl1, version: v, fid: Fid::default(), verlimit: 0 },
        };
        Ok(self.file_spec(&found))
    }

    pub fn verify(&mut self) -> Result<Report> {
        Ok(self.vol.verify()?)
    }

    pub fn repair_bitmap(&mut self) -> Result<Report> {
        Ok(self.vol.repair_bitmap()?)
    }
}

/// Reads a logical block without mounting, for looking at volumes that do
/// not mount.
pub fn raw_block(path: impl AsRef<Path>, lbn: u64) -> Result<[u8; BLOCK]> {
    let path = path.as_ref();
    let file = File::open(path).at(path.display())?;
    let (mut dev, _) = FileDevice::new(file).at(path.display())?;
    let mut b = [0u8; BLOCK];
    ods_core::BlockDevice::read(&mut dev, lbn, &mut b).at(format!("LBN {lbn}"))?;
    Ok(b)
}

/// Advisory lock on the image file: exclusive for writers, shared for
/// readers.
fn lock(f: &File, mode: Mode) -> Result<()> {
    let r = match mode {
        Mode::ReadWrite => f.try_lock(),
        Mode::ReadOnly => f.try_lock_shared(),
    };
    r.map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => Error { kind: Kind::Locked, context: String::new() },
        std::fs::TryLockError::Error(e) => e.into(),
    })
}
