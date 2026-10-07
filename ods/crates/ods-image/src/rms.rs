//! Relative and indexed files on a volume: their records along any key,
//! their structure checked, and new indexed files loaded from records, as
//! the build writes `SYSUAF.DAT`. The work is `ods_core::rms`'s, over the
//! file's bytes.

use std::io::Read;

pub use ods_core::rms::{
    AreaSpec, Finding, Indexed, KeySpec, KeyStats, Relative, Report, RmsError, RmsFile, Spec, build, check, dtype, fdl,
    keyflag, open,
};
use ods_core::{BLOCK, Fid, RecordAttrs, rfm};

use crate::{Context, Conversion, Error, Image, Result};

impl Image {
    /// A file's bytes up to its end of file, and its record attributes.
    fn file_bytes(&mut self, fid: Fid) -> Result<(Vec<u8>, RecordAttrs)> {
        let attrs = self.stat(fid)?.attrs.record;
        let mut data = Vec::new();
        self.reader(fid)?.read_to_end(&mut data)?;
        Ok((data, attrs))
    }

    /// A file's records along key `key`: an indexed file's along any of
    /// its keys, a relative file's in record number order, a sequential
    /// file's in order (key 0 for both of those).
    pub fn read_records(&mut self, fid: Fid, key: usize) -> Result<Vec<Vec<u8>>> {
        let (data, attrs) = self.file_bytes(fid)?;
        if attrs.rtype >> 4 == 0 {
            if key != 0 {
                return Err(Error::usage("a sequential file has no keys").at(fid));
            }
            return self.records(fid)?.collect::<std::io::Result<_>>().map_err(|e| Error::from(e).at(fid));
        }
        open(&data, &attrs).and_then(|f| f.records(key)).at(fid)
    }

    /// Checks a relative or indexed file's structure.
    pub fn check_file(&mut self, fid: Fid) -> Result<Report> {
        let (data, attrs) = self.file_bytes(fid)?;
        if attrs.rtype >> 4 == 0 {
            return Err(Error::usage("only relative and indexed files have a structure to check").at(fid));
        }
        Ok(check(&data, &attrs))
    }

    /// Builds an indexed file as `fdl` says, holding `records`, and copies
    /// it in as `spec`, a new file or version. Returns it and its
    /// specification.
    pub fn load_indexed<R: AsRef<[u8]>>(&mut self, fdl: &Spec, records: &[R], spec: &str) -> Result<(Fid, String)> {
        let mut s = fdl.clone();
        s.cluster = self.vol.cluster() as u32;
        let bytes = build(&s, records).at(spec)?;
        let attrs = s.record_attrs((bytes.len() / BLOCK) as u32);
        self.copy_in(&mut &bytes[..], spec, Conversion::Binary, Some(bytes.len() as u64), Some(attrs))
    }
}

/// Host text as records of a file `spec` describes: a record per line,
/// without its LF or CR LF; `FIX` records padded with spaces to their size.
pub fn lines_to_records(spec: &Spec, text: &[u8]) -> Result<Vec<Vec<u8>>> {
    let text = text.strip_suffix(b"\n").unwrap_or(text);
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (i, line) in text.split(|&c| c == b'\n').enumerate() {
        let mut r = line.strip_suffix(b"\r").unwrap_or(line).to_vec();
        let mrs = spec.mrs as usize;
        if mrs > 0 && r.len() > mrs {
            return Err(Error::usage(format!("line {} is longer than the {mrs}-byte records", i + 1)));
        }
        if spec.rfm == rfm::FIX {
            r.resize(mrs, b' ');
        }
        out.push(r);
    }
    Ok(out)
}
