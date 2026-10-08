//! Whole directory trees between the host and the volume. Host files hold
//! the bytes (binary, up to the end of file) under `NAME.TYPE;VERSION`; a
//! manifest beside them keeps what host files cannot: record attributes,
//! protection, owner, dates, characteristics.

use std::fs;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use ods_core::{Attributes, Fid, RecordAttrs, fch};
use serde::{Deserialize, Serialize};

use crate::{Context, Conversion, Error, Image, Result};

/// Name of the manifest in an exported tree.
pub const MANIFEST: &str = "ods-manifest.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub volume: String,
    pub structure_level: u8,
    /// The directory that was exported.
    pub root: String,
    pub entries: Vec<ManifestEntry>,
}

/// One file or directory. Times are VMS times, 100 ns units since
/// 17-Nov-1858.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Host path, relative to the tree, `/`-separated.
    pub path: String,
    pub directory: bool,
    /// "NAME.TYPE", ISO Latin-1 read as Unicode code points.
    pub name: String,
    pub version: u16,
    pub bytes: u64,
    pub rtype: u8,
    pub rattrib: u8,
    pub rsize: u16,
    pub bktsize: u8,
    pub vfcsize: u8,
    pub maxrec: u16,
    pub defext: u16,
    pub gbc: u16,
    pub reserved: [u8; 8],
    pub version_limit: u16,
    pub filechar: u32,
    pub owner: u32,
    pub protection: u16,
    pub revision: u16,
    pub created: u64,
    pub revised: u64,
    pub expires: u64,
    pub backup: u64,
    pub accessed: u64,
    pub attr_changed: u64,
}

impl ManifestEntry {
    fn new(path: String, name: String, version: u16, bytes: u64, a: &Attributes) -> ManifestEntry {
        let r = &a.record;
        ManifestEntry {
            path,
            directory: a.filechar & fch::DIRECTORY != 0,
            name,
            version,
            bytes,
            rtype: r.rtype,
            rattrib: r.rattrib,
            rsize: r.rsize,
            bktsize: r.bktsize,
            vfcsize: r.vfcsize,
            maxrec: r.maxrec,
            defext: r.defext,
            gbc: r.gbc,
            reserved: r.reserved,
            version_limit: r.versions,
            filechar: a.filechar,
            owner: a.owner,
            protection: a.protection,
            revision: a.revision,
            created: a.created,
            revised: a.revised,
            expires: a.expires,
            backup: a.backup,
            accessed: a.accessed,
            attr_changed: a.attr_changed,
        }
    }

    fn record(&self) -> RecordAttrs {
        RecordAttrs {
            rtype: self.rtype,
            rattrib: self.rattrib,
            rsize: self.rsize,
            bktsize: self.bktsize,
            vfcsize: self.vfcsize,
            maxrec: self.maxrec,
            defext: self.defext,
            gbc: self.gbc,
            reserved: self.reserved,
            versions: self.version_limit,
            ..RecordAttrs::default()
        }
    }

    /// Puts the saved attributes on a file just copied in; its size and end
    /// of file stay as the copy made them.
    fn apply(&self, a: &mut Attributes) {
        let (efblk, ffbyte, hiblk) = (a.record.efblk, a.record.ffbyte, a.record.hiblk);
        a.record = RecordAttrs { efblk, ffbyte, hiblk, ..self.record() };
        a.filechar = self.filechar;
        a.owner = self.owner;
        a.protection = self.protection;
        a.revision = self.revision;
        a.created = self.created;
        a.revised = self.revised;
        a.expires = self.expires;
        a.backup = self.backup;
        a.accessed = self.accessed;
        a.attr_changed = self.attr_changed;
    }
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// A single path component that stays where it is put.
pub fn safe_host_name(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\0'])
}

impl Image {
    /// Copies the tree under directory `spec` to `host`, which must not
    /// exist yet, with a manifest. Returns the number of files.
    pub fn export(&mut self, spec: &str, host: &Path) -> Result<Manifest> {
        let s = self.parse(spec)?;
        if s.file.is_some() || s.is_wild() {
            return Err(Error::usage("export takes a directory, like [A.B] or /a/b/").at(spec));
        }
        let root = self.lookup_dirs(&s.dirs).at(spec)?;
        fs::create_dir(host).at(host.display())?;
        let info = self.info()?;
        let mut m = Manifest {
            volume: info.label,
            structure_level: info.level.number(),
            root: self.dir_spec(&s.dirs),
            entries: Vec::new(),
        };
        let mut seen = vec![root];
        self.export_dir(root, host, "", &mut m, &mut seen)?;
        let path = host.join(MANIFEST);
        let f = fs::File::create_new(&path).at(path.display())?;
        let mut w = BufWriter::new(f);
        serde_json::to_writer_pretty(&mut w, &m).map_err(|e| Error::usage(e.to_string()).at(path.display()))?;
        w.flush().at(path.display())?;
        Ok(m)
    }

    fn export_dir(&mut self, dir: Fid, host: &Path, rel: &str, m: &mut Manifest, seen: &mut Vec<Fid>) -> Result<()> {
        for e in self.list(dir)? {
            let name = latin1(&e.name);
            // A damaged or hostile volume must not write outside `host`.
            if !safe_host_name(&name) {
                return Err(Error::usage("name unusable on the host").at(format!("{name};{}", e.version)));
            }
            let a = match self.attributes(e.fid) {
                Ok(a) => a,
                Err(err) => return Err(err.at(format!("{name};{}", e.version))),
            };
            if e.is_dir_name() && a.filechar & fch::DIRECTORY != 0 {
                if seen.contains(&e.fid) {
                    continue;
                }
                seen.push(e.fid);
                let stem = latin1(ods_core::name::split(&e.name).0);
                if !safe_host_name(&stem) {
                    return Err(Error::usage("directory name unusable on the host").at(&name));
                }
                let sub = host.join(&stem);
                fs::create_dir(&sub).at(sub.display())?;
                let path = format!("{rel}{stem}");
                m.entries.push(ManifestEntry::new(path.clone(), name, 1, 0, &a));
                self.export_dir(e.fid, &sub, &format!("{path}/"), m, seen)?;
                continue;
            }
            let file = format!("{name};{}", e.version);
            let target = host.join(&file);
            let mut out = BufWriter::new(fs::File::create_new(&target).at(target.display())?);
            let bytes = self.copy_out(e.fid, &mut out, Conversion::Binary)?;
            out.flush().at(target.display())?;
            m.entries.push(ManifestEntry::new(format!("{rel}{file}"), name, e.version, bytes, &a));
        }
        Ok(())
    }

    /// Copies a host tree (usually one `export` made) into directory
    /// `spec`, creating directories as needed. Files listed in a manifest
    /// get their versions and attributes back; others come in as binary,
    /// as the next version. Hidden host files are skipped. Returns the new
    /// file specifications.
    pub fn import(&mut self, host: &Path, spec: &str) -> Result<Vec<String>> {
        let s = self.parse(spec)?;
        if s.file.is_some() || s.is_wild() {
            return Err(Error::usage("import takes a directory, like [A.B] or /a/b/").at(spec));
        }
        let manifest: Manifest = match fs::File::open(host.join(MANIFEST)) {
            Ok(f) => {
                serde_json::from_reader(BufReader::new(f)).map_err(|e| Error::usage(e.to_string()).at(MANIFEST))?
            }
            Err(_) => Manifest::default(),
        };
        let mut out = Vec::new();
        self.import_dir(host, &s.dirs, "", &manifest, &mut out)?;
        Ok(out)
    }

    fn import_dir(
        &mut self,
        host: &Path,
        dirs: &[Vec<u8>],
        rel: &str,
        m: &Manifest,
        out: &mut Vec<String>,
    ) -> Result<()> {
        let mut items: Vec<PathBuf> =
            fs::read_dir(host).at(host.display())?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        items.sort();
        self.lookup_dirs(dirs).at(self.dir_spec(dirs))?;
        for p in items {
            let fname = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let path = format!("{rel}{fname}");
            let entry = m.entries.iter().find(|e| e.path == path);
            if rel.is_empty() && fname == MANIFEST || fname.starts_with('.') && entry.is_none() {
                continue;
            }
            if p.is_dir() {
                let mut sub = dirs.to_vec();
                sub.push(to_latin1(&fname)?);
                let spec = self.dir_spec(&sub);
                let fid = match self.lookup_dirs(&sub) {
                    Ok(f) => f,
                    Err(_) => self.mkdir(&spec)?,
                };
                if let Some(e) = entry {
                    let mut a = self.attributes(fid)?;
                    a.owner = e.owner;
                    a.protection = e.protection;
                    a.record.versions = e.version_limit;
                    self.set_attributes(fid, &a)?;
                }
                self.import_dir(&p, &sub, &format!("{path}/"), m, out)?;
                continue;
            }
            let (name, version) = match entry {
                Some(e) => (e.name.clone(), Some(e.version)),
                None => match fname.rsplit_once(';') {
                    Some((n, v)) if v.parse::<u16>().is_ok() => (n.to_string(), v.parse().ok()),
                    _ => (fname.clone(), None),
                },
            };
            let mut target = self.dir_spec(dirs);
            target.push_str(&escape_name(&name));
            if let Some(v) = version {
                target.push_str(&format!(";{v}"));
            }
            let mut f = BufReader::new(fs::File::open(&p).at(p.display())?);
            let size = fs::metadata(&p).map(|md| md.len()).ok();
            let (fid, spec) = self.copy_in(&mut f, &target, Conversion::Binary, size, entry.map(|e| e.record()))?;
            if let Some(e) = entry {
                let mut a = self.attributes(fid)?;
                e.apply(&mut a);
                self.set_attributes(fid, &a).at(&spec)?;
            }
            out.push(spec);
        }
        Ok(())
    }

    fn lookup_dirs(&mut self, dirs: &[Vec<u8>]) -> std::result::Result<Fid, crate::OdsError> {
        self.vol.find_dir(dirs)
    }
}

fn to_latin1(s: &str) -> Result<Vec<u8>> {
    s.chars()
        .map(|c| u8::try_from(c as u32).map_err(|_| Error::usage(format!("{s:?}: character outside ISO Latin-1"))))
        .collect()
}

/// A stored name in VMS syntax: dots but the last escaped, and the
/// characters VMS gives a meaning to.
fn escape_name(name: &str) -> String {
    let last = name.rfind('.');
    let mut out = String::new();
    for (i, c) in name.char_indices() {
        match c {
            '.' if Some(i) != last => out.push_str("^."),
            ',' | ';' | '[' | ']' | '<' | '>' | ':' | '^' | '&' | '%' => {
                out.push('^');
                out.push(c);
            }
            ' ' => out.push_str("^_"),
            _ => out.push(c),
        }
    }
    out
}
