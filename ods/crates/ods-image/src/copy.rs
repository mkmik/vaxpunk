//! Byte streams over files, and copying files in and out of the volume
//! without holding them in memory.

use std::io::{self, Read, Write};

use ods_core::{Alloc, BLOCK, Fid, NewFile, RecordAttrs, rat, rfm};

use crate::records::{Format, Records, VarWriter, is_text};
use crate::{Context, Error, Image, Result, Version};

/// How data changes crossing between host and volume. Nothing is ever
/// chosen silently: callers pass one, or ask [`Conversion::default_out`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// Bytes as they are, up to the end of file.
    Binary,
    /// Copy out: each record becomes a line ending in LF (VAR, VFC, FIX
    /// and stream files; VFC control bytes are dropped).
    RecordsToLines,
    /// Copy in: each line becomes a VAR record with CR carriage control.
    LinesToRecords,
}

impl Conversion {
    /// The copy-out default: records to lines for files that are text
    /// beyond doubt, binary for everything else.
    pub fn default_out(r: &RecordAttrs) -> Conversion {
        if is_text(r) { Conversion::RecordsToLines } else { Conversion::Binary }
    }
}

/// Transfer size for streaming, in blocks.
const CHUNK: usize = 64;

/// Reads a file's bytes up to its end of file.
pub struct FileReader<'a> {
    img: &'a mut Image,
    fid: Fid,
    pos: u64,
    eof: u64,
    buf: Vec<u8>,
    buf_at: u64,
}

impl Read for FileReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.eof || out.is_empty() {
            return Ok(0);
        }
        let end = self.buf_at + self.buf.len() as u64;
        if self.pos < self.buf_at || self.pos >= end {
            let first = self.pos / BLOCK as u64;
            let blocks = ((self.eof - first * BLOCK as u64).div_ceil(BLOCK as u64) as usize).min(CHUNK);
            self.buf.resize(blocks * BLOCK, 0);
            self.img
                .vol
                .read_blocks(self.fid, first + 1, &mut self.buf)
                .map_err(|e| io::Error::other(Error::from(e).at(self.fid)))?;
            self.buf_at = first * BLOCK as u64;
        }
        let from = (self.pos - self.buf_at) as usize;
        let n = out.len().min(self.buf.len() - from).min((self.eof - self.pos) as usize);
        out[..n].copy_from_slice(&self.buf[from..from + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl FileReader<'_> {
    /// Moves to byte `pos` of the file.
    pub fn seek_to(&mut self, pos: u64) {
        self.pos = pos.min(self.eof);
    }

    /// Bytes up to the end of file.
    pub fn len(&self) -> u64 {
        self.eof
    }

    pub fn is_empty(&self) -> bool {
        self.eof == 0
    }
}

/// A text file seen as lines: its size, and every 64 kB of lines the record
/// they start with, so reading at an offset converts only from there.
#[derive(Clone, Debug)]
pub struct TextView {
    pub size: u64,
    /// (offset in the text, offset in the file) of record starts.
    marks: Vec<(u64, u64)>,
}

const MARK_EVERY: u64 = 64 * 1024;

impl Image {
    /// Scans a file's records once to size its text view.
    pub fn text_view(&mut self, fid: Fid) -> Result<TextView> {
        let mut recs = self.records(fid)?;
        let skip = recs.control_size();
        let (mut size, mut marks) = (0u64, vec![(0, 0)]);
        loop {
            let at = recs.position();
            let Some(r) = recs.next() else { break };
            if size - marks[marks.len() - 1].0 >= MARK_EVERY {
                marks.push((size, at));
            }
            size += r?.len().saturating_sub(skip) as u64 + 1;
        }
        Ok(TextView { size, marks })
    }

    /// Reads the text view at `offset`. Returns the bytes read.
    pub fn read_text(&mut self, fid: Fid, view: &TextView, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let i = view.marks.partition_point(|m| m.0 <= offset).saturating_sub(1);
        let (mut pos, raw) = view.marks[i];
        let format = self.records(fid)?.format();
        let mut rd = self.reader(fid)?;
        rd.seek_to(raw);
        let mut recs = Records::at(rd, format, raw);
        let skip = recs.control_size();
        let end = offset + buf.len() as u64;
        let mut n = 0;
        while pos < end {
            let Some(r) = recs.next() else { break };
            let mut line = r?;
            line.drain(..skip.min(line.len()));
            line.push(b'\n');
            let (from, to) = (pos.max(offset), (pos + line.len() as u64).min(end));
            if from < to {
                let src = &line[(from - pos) as usize..(to - pos) as usize];
                buf[(from - offset) as usize..(to - offset) as usize].copy_from_slice(src);
                n = (to - offset) as usize;
            }
            pos += line.len() as u64;
        }
        Ok(n)
    }

    /// A reader over the file's bytes, up to its end of file.
    pub fn reader(&mut self, fid: Fid) -> Result<FileReader<'_>> {
        let eof = self.stat(fid)?.attrs.record.eof_bytes();
        Ok(FileReader { img: self, fid, pos: 0, eof, buf: Vec::new(), buf_at: 0 })
    }

    /// An iterator over the file's records. Fails for files whose records
    /// are not a text reader's business (UDF, relative, indexed).
    pub fn records(&mut self, fid: Fid) -> Result<Records<FileReader<'_>>> {
        let r = self.stat(fid)?.attrs.record;
        let format = Format::of(&r).ok_or_else(|| {
            Error::usage(format!(
                "{} {} files have no records to read as text",
                crate::attrs::rfm_name(r.rtype),
                crate::attrs::org_name(r.rtype)
            ))
            .at(fid)
        })?;
        Ok(Records::new(self.reader(fid)?, format))
    }

    /// Copies a file out. Returns the bytes written.
    pub fn copy_out(&mut self, fid: Fid, w: &mut dyn Write, conv: Conversion) -> Result<u64> {
        match conv {
            Conversion::Binary => Ok(io::copy(&mut self.reader(fid)?, w)?),
            Conversion::RecordsToLines => {
                let mut recs = self.records(fid)?;
                let skip = recs.control_size();
                let mut n = 0;
                for r in &mut recs {
                    let r = r?;
                    let line = r.get(skip..).unwrap_or(&[]);
                    w.write_all(line)?;
                    w.write_all(b"\n")?;
                    n += line.len() as u64 + 1;
                }
                Ok(n)
            }
            Conversion::LinesToRecords => Err(Error::usage("lines-to-records converts host files coming in")),
        }
    }

    /// Copies host data in as a new file (a new version unless `spec` gives
    /// one). `size` is a hint for the first allocation. `record` overrides
    /// the record attributes the conversion implies (UDF for binary, VAR
    /// with CR carriage control for lines). Returns the new file and its
    /// specification. A failed copy leaves nothing behind.
    pub fn copy_in(
        &mut self,
        r: &mut dyn Read,
        spec: &str,
        conv: Conversion,
        size: Option<u64>,
        record: Option<RecordAttrs>,
    ) -> Result<(Fid, String)> {
        let (dir, f) = self.parse_file(spec)?;
        let version = match f.version {
            Version::Exact(v) => Some(v),
            Version::Highest => None,
            _ => return Err(Error::usage("a new file takes an explicit version or none").at(spec)),
        };
        let mut ra = match conv {
            Conversion::Binary => RecordAttrs::default(),
            Conversion::LinesToRecords => RecordAttrs { rtype: rfm::VAR, rattrib: rat::CR, ..RecordAttrs::default() },
            Conversion::RecordsToLines => return Err(Error::usage("records-to-lines converts files going out")),
        };
        if let Some(r) = record {
            ra = r;
        }
        let blocks = size.map_or(0, |s| (s + s / 16).div_ceil(BLOCK as u64));
        let new = NewFile { record: ra, blocks, alloc: Alloc::Any, ..NewFile::default() };
        let (fid, v) = self.vol.create(dir, &f.name, version, &new).at(spec)?;
        let name = self.spec_of(&self.parse(spec)?.dirs, fid)?;
        let result = (|| -> Result<()> {
            let mut sink = BlockSink { img: self, fid, pending: Vec::new(), written: 0, bytes: 0 };
            let longest = match conv {
                Conversion::LinesToRecords => {
                    let mut w = VarWriter::new(&mut sink);
                    io::copy(r, &mut w)?;
                    w.finish()?.1
                }
                _ => {
                    io::copy(r, &mut sink)?;
                    0
                }
            };
            let bytes = sink.finish()?;
            let mut a = self.attributes(fid)?;
            a.record = ra;
            if conv == Conversion::LinesToRecords && record.is_none() {
                a.record.maxrec = longest as u16;
            }
            a.record.efblk = (bytes / BLOCK as u64) as u32 + 1;
            a.record.ffbyte = (bytes % BLOCK as u64) as u16;
            a.revised = crate::time::now();
            self.set_attributes(fid, &a)
        })();
        match result {
            Ok(()) => Ok((fid, name)),
            Err(e) => {
                let _ = self.vol.delete(dir, &f.name, v);
                Err(e.at(spec))
            }
        }
    }
}

/// Collects bytes into whole blocks and writes them to the end of a file,
/// growing it as needed.
struct BlockSink<'a> {
    img: &'a mut Image,
    fid: Fid,
    pending: Vec<u8>,
    written: u64,
    bytes: u64,
}

impl BlockSink<'_> {
    fn put(&mut self, blocks: &[u8]) -> Result<()> {
        let need = self.written + (blocks.len() / BLOCK) as u64;
        let have = self.img.stat(self.fid)?.allocated;
        if need > have {
            let grow = (need - have).max(have / 2).max(CHUNK as u64);
            self.img.vol.extend(self.fid, grow, Alloc::Any).at(self.fid)?;
        }
        self.img.vol.write_blocks(self.fid, self.written + 1, blocks).at(self.fid)?;
        self.written = need;
        Ok(())
    }

    /// Writes the last partial block, trims the allocation to what was
    /// used, and returns the byte count.
    fn finish(mut self) -> Result<u64> {
        if !self.pending.is_empty() {
            let mut last = std::mem::take(&mut self.pending);
            last.resize(last.len().next_multiple_of(BLOCK), 0);
            self.put(&last)?;
        }
        self.img.vol.truncate(self.fid, self.written).at(self.fid)?;
        Ok(self.bytes)
    }
}

impl Write for BlockSink<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        self.bytes += buf.len() as u64;
        let full = self.pending.len() / (CHUNK * BLOCK) * (CHUNK * BLOCK);
        if full > 0 {
            let chunk: Vec<u8> = self.pending.drain(..full).collect();
            self.put(&chunk).map_err(io::Error::other)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
