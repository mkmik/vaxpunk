//! The file system as FUSE sees it: inodes, attributes, listings, reads and
//! extended attributes over an image. `Vfs` holds the logic and returns
//! plain results, so it can be tested without a kernel; `Fs` hands its
//! answers to fuser.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::Mutex;
use std::time::Duration;

use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo, LockOwner, OpenFlags,
    ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyXattr, Request,
};
use ods_image::{Conversion, DirEntry, Error, Fid, Image, MFD, OdsError, TextView, fch};

use crate::map;

/// Inode 1 is the MFD; other files are their file number plus one.
pub const ROOT: u64 = 1;
const TTL: Duration = Duration::from_secs(60);

fn ino(fid: Fid) -> u64 {
    if fid == MFD { ROOT } else { fid.num as u64 + 1 }
}

fn errno(e: &Error) -> Errno {
    match e.ods() {
        Some(OdsError::NotFound | OdsError::DirNotFound) => Errno::ENOENT,
        Some(OdsError::NotDirectory) => Errno::ENOTDIR,
        Some(OdsError::ReadOnly) => Errno::EROFS,
        Some(OdsError::Stale(_)) => Errno::ESTALE,
        Some(OdsError::BadName(_) | OdsError::Invalid(_)) => Errno::EINVAL,
        Some(OdsError::Unsupported(_)) => Errno::ENOTSUP,
        _ => Errno::EIO,
    }
}

pub struct Vfs {
    img: Image,
    list_versions: bool,
    uid: u32,
    gid: u32,
    fids: HashMap<u64, Fid>,
    /// Directory listings, read once (the mount is read-only): entries,
    /// and which of them are directories.
    dirs: HashMap<u64, (Vec<DirEntry>, Vec<bool>)>,
    parents: HashMap<u64, u64>,
    /// Text files' views; `None` for files read as bytes.
    text: HashMap<u64, Option<TextView>>,
    versions: HashMap<u64, u16>,
}

impl Vfs {
    pub fn new(img: Image, list_versions: bool, uid: u32, gid: u32) -> Vfs {
        Vfs {
            img,
            list_versions,
            uid,
            gid,
            fids: HashMap::from([(ROOT, MFD)]),
            dirs: HashMap::new(),
            parents: HashMap::new(),
            text: HashMap::new(),
            versions: HashMap::new(),
        }
    }

    fn fid(&self, ino: u64) -> Result<Fid, Errno> {
        self.fids.get(&ino).copied().ok_or(Errno::ENOENT)
    }

    fn listing(&mut self, dir: u64) -> Result<(Vec<DirEntry>, Vec<bool>), Errno> {
        if !self.dirs.contains_key(&dir) {
            let fid = self.fid(dir)?;
            let mut entries = self.img.list(fid).map_err(|e| errno(&e))?;
            // The MFD lists itself (000000.DIR;1): that is the mount root.
            entries.retain(|e| e.fid != fid);
            let kinds = entries
                .iter()
                .map(|e| {
                    e.is_dir_name()
                        && e.fid != fid
                        && self.img.stat(e.fid).is_ok_and(|i| i.attrs.filechar & fch::DIRECTORY != 0)
                })
                .collect();
            self.dirs.insert(dir, (entries, kinds));
        }
        Ok(self.dirs[&dir].clone())
    }

    fn remember(&mut self, e: &DirEntry, dir: bool, parent: u64) -> u64 {
        let child = ino(e.fid);
        self.fids.insert(child, e.fid);
        self.versions.insert(child, e.version);
        if dir {
            self.parents.insert(child, parent);
        }
        child
    }

    fn text_view(&mut self, ino: u64, fid: Fid) -> Result<Option<TextView>, Errno> {
        if !self.text.contains_key(&ino) {
            let info = self.img.stat(fid).map_err(|e| errno(&e))?;
            let view = match Conversion::default_out(&info.attrs.record) {
                Conversion::RecordsToLines => self.img.text_view(fid).ok(),
                _ => None,
            };
            self.text.insert(ino, view);
        }
        Ok(self.text[&ino].clone())
    }

    pub fn attr(&mut self, ino: u64) -> Result<FileAttr, Errno> {
        let fid = self.fid(ino)?;
        let info = self.img.stat(fid).map_err(|e| errno(&e))?;
        let dir = info.attrs.filechar & fch::DIRECTORY != 0;
        let size = if dir {
            info.allocated * 512
        } else {
            match self.text_view(ino, fid)? {
                Some(v) => v.size,
                None => info.attrs.record.eof_bytes(),
            }
        };
        let (atime, mtime, ctime, crtime) = map::times(&info.attrs);
        Ok(FileAttr {
            ino: INodeNo(ino),
            size,
            blocks: info.allocated,
            atime,
            mtime,
            ctime,
            crtime,
            kind: if dir { FileType::Directory } else { FileType::RegularFile },
            perm: map::mode(info.attrs.protection),
            nlink: if dir { 2 } else { 1 },
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 512,
            flags: 0,
        })
    }

    pub fn lookup(&mut self, parent: u64, name: &str) -> Result<FileAttr, Errno> {
        let (entries, kinds) = self.listing(parent)?;
        let is_dir = |e: &DirEntry| entries.iter().position(|x| x == e).is_some_and(|i| kinds[i]);
        let e = map::resolve(&entries, name, is_dir).ok_or(Errno::ENOENT)?;
        let child = self.remember(e, is_dir(e), parent);
        self.attr(child)
    }

    /// "." and "..", then what the directory lists.
    pub fn entries(&mut self, dir: u64) -> Result<Vec<(u64, FileType, String)>, Errno> {
        let (entries, kinds) = self.listing(dir)?;
        let is_dir = |e: &DirEntry| entries.iter().position(|x| x == e).is_some_and(|i| kinds[i]);
        let shown = map::listing(&entries, is_dir, self.list_versions);
        let parent = self.parents.get(&dir).copied().unwrap_or(ROOT);
        let mut out = vec![(dir, FileType::Directory, ".".into()), (parent, FileType::Directory, "..".into())];
        for s in shown {
            let child = self.remember(&s.entry, s.dir, dir);
            out.push((child, if s.dir { FileType::Directory } else { FileType::RegularFile }, s.host));
        }
        Ok(out)
    }

    pub fn read(&mut self, ino: u64, offset: u64, size: u32) -> Result<Vec<u8>, Errno> {
        let fid = self.fid(ino)?;
        let mut buf = vec![0u8; size as usize];
        let n = match self.text_view(ino, fid)? {
            Some(v) => self.img.read_text(fid, &v, offset, &mut buf),
            None => self.img.read_at(fid, offset, &mut buf),
        }
        .map_err(|e| errno(&e))?;
        buf.truncate(n);
        Ok(buf)
    }

    fn xattrs(&mut self, ino: u64) -> Result<Vec<(&'static str, String)>, Errno> {
        let fid = self.fid(ino)?;
        let info = self.img.stat(fid).map_err(|e| errno(&e))?;
        Ok(map::xattrs(&info, self.versions.get(&ino).copied().unwrap_or(1)))
    }

    pub fn xattr(&mut self, ino: u64, name: &OsStr) -> Result<Vec<u8>, Errno> {
        let all = self.xattrs(ino)?;
        let v = all.into_iter().find(|(k, _)| OsStr::new(k) == name).ok_or(Errno::NO_XATTR)?;
        Ok(v.1.into_bytes())
    }

    /// Attribute names, each followed by a NUL, as listxattr wants them.
    pub fn xattr_names(&mut self, ino: u64) -> Result<Vec<u8>, Errno> {
        let mut out = Vec::new();
        for (k, _) in self.xattrs(ino)? {
            out.extend_from_slice(k.as_bytes());
            out.push(0);
        }
        Ok(out)
    }

    /// (blocks, free blocks, files, free files).
    pub fn statfs(&mut self) -> Result<(u64, u64, u64, u64), Errno> {
        let i = self.img.info().map_err(|e| errno(&e))?;
        let max = i.max_files as u64;
        Ok((i.volume_blocks, i.free_blocks, max, max - i.files.min(max)))
    }
}

/// The adapter fuser calls; one request at a time.
pub struct Fs(pub Mutex<Vfs>);

impl Fs {
    fn vfs(&self) -> std::sync::MutexGuard<'_, Vfs> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn xattr_reply(r: Result<Vec<u8>, Errno>, size: u32, reply: ReplyXattr) {
    match r {
        Ok(v) if size == 0 => reply.size(v.len() as u32),
        Ok(v) if v.len() <= size as usize => reply.data(&v),
        Ok(_) => reply.error(Errno::ERANGE),
        Err(e) => reply.error(e),
    }
}

impl Filesystem for Fs {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        match self.vfs().lookup(parent.0, &name.to_string_lossy()) {
            Ok(a) => reply.entry(&TTL, &a, Generation(0)),
            Err(e) => reply.error(e),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match self.vfs().attr(ino.0) {
            Ok(a) => reply.attr(&TTL, &a),
            Err(e) => reply.error(e),
        }
    }

    fn readdir(&self, _req: &Request, ino: INodeNo, _fh: FileHandle, offset: u64, mut reply: ReplyDirectory) {
        match self.vfs().entries(ino.0) {
            Ok(all) => {
                for (i, (child, kind, name)) in all.into_iter().enumerate().skip(offset as usize) {
                    if reply.add(INodeNo(child), i as u64 + 1, kind, name) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(e) => reply.error(e),
        }
    }

    fn open(&self, _req: &Request, _ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        if flags.0 & libc::O_ACCMODE != libc::O_RDONLY {
            return reply.error(Errno::EROFS);
        }
        reply.opened(FileHandle(0), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock: Option<LockOwner>,
        reply: ReplyData,
    ) {
        match self.vfs().read(ino.0, offset, size) {
            Ok(d) => reply.data(&d),
            Err(e) => reply.error(e),
        }
    }

    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        match self.vfs().statfs() {
            Ok((blocks, free, files, ffree)) => reply.statfs(blocks, free, free, files, ffree, 512, 255, 512),
            Err(e) => reply.error(e),
        }
    }

    fn getxattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, size: u32, reply: ReplyXattr) {
        xattr_reply(self.vfs().xattr(ino.0, name), size, reply);
    }

    fn listxattr(&self, _req: &Request, ino: INodeNo, size: u32, reply: ReplyXattr) {
        xattr_reply(self.vfs().xattr_names(ino.0), size, reply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ods_image::{InitParams, Level};

    fn vfs(versions: bool) -> Vfs {
        let dir = std::env::temp_dir().join(format!("ods-fuse-test-{}-{versions}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = InitParams { label: b"FUSE".to_vec(), level: Level::Ods2, ..InitParams::default() };
        let mut img = Image::create(dir.join("t.img"), 4000, &p).unwrap();
        img.mkdir("[SUB]").unwrap();
        for text in ["first\n", "second\nversion\n"] {
            img.copy_in(&mut text.as_bytes(), "[SUB]NOTES.TXT", Conversion::LinesToRecords, None, None).unwrap();
        }
        img.copy_in(&mut &b"\x00\x01\x02"[..], "[000000]MAKEFILE", Conversion::Binary, None, None).unwrap();
        Vfs::new(img, versions, 501, 20)
    }

    fn names(v: &mut Vfs, ino: u64) -> Vec<String> {
        v.entries(ino).unwrap().into_iter().map(|e| e.2).collect()
    }

    #[test]
    fn browse_and_read() {
        let mut v = vfs(false);
        let root = names(&mut v, ROOT);
        assert!(root.contains(&"SUB".to_string()) && root.contains(&"MAKEFILE".to_string()), "{root:?}");
        assert!(root.contains(&"INDEXF.SYS".to_string()));
        let sub = v.lookup(ROOT, "sub").unwrap();
        assert_eq!(sub.kind, FileType::Directory);
        assert_eq!(names(&mut v, sub.ino.0), [".", "..", "NOTES.TXT"]);
        let notes = v.lookup(sub.ino.0, "notes.txt").unwrap();
        assert_eq!(notes.size, 15);
        assert_eq!(v.read(notes.ino.0, 0, 100).unwrap(), b"second\nversion\n");
        assert_eq!(v.read(notes.ino.0, 7, 4).unwrap(), b"vers");
        let old = v.lookup(sub.ino.0, "NOTES.TXT;1").unwrap();
        assert_eq!(v.read(old.ino.0, 0, 100).unwrap(), b"first\n");
        assert_eq!(v.xattr(old.ino.0, OsStr::new("vms.version")).unwrap(), b"1");
        assert_eq!(v.xattr(notes.ino.0, OsStr::new("vms.rfm")).unwrap(), b"VAR");
        assert_eq!(v.xattr(notes.ino.0, OsStr::new("com.apple.FinderInfo")), Err(Errno::NO_XATTR));
        let mk = v.lookup(ROOT, "Makefile").unwrap();
        assert_eq!(v.read(mk.ino.0, 0, 10).unwrap(), b"\x00\x01\x02");
        assert_eq!(v.lookup(ROOT, "._Makefile"), Err(Errno::ENOENT));
        assert_eq!(v.lookup(ROOT, "nothing"), Err(Errno::ENOENT));
    }

    #[test]
    fn versions_listed_on_request() {
        let mut v = vfs(true);
        let sub = v.lookup(ROOT, "SUB").unwrap();
        assert_eq!(names(&mut v, sub.ino.0), [".", "..", "NOTES.TXT", "NOTES.TXT;1"]);
    }
}
