//! Records of sequential files, as RMS lays them out: what text files are
//! made of on VMS.
//!
//! - VAR: a 16-bit length, the data, a pad byte if the length is odd. In
//!   files where records may not span blocks (the BLK attribute), a length
//!   of 0xFFFF means "the rest of this block is unused".
//! - VFC: VAR with a fixed-size control area (print control) at the front
//!   of each record.
//! - FIX: records of the file's record size, padded to even length.
//! - STMLF, STMCR, STM: bytes, records ended by LF, CR or CR LF.

use std::io::{self, Read, Write};

use ods_core::{BLOCK, RecordAttrs, rat, rfm};

/// Record layout, from a file's record attributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Var { vfc: usize },
    Fix(usize),
    Stream(&'static [u8]),
}

impl Format {
    /// `None` for formats without records a text reader can use (UDF,
    /// relative and indexed files).
    pub fn of(r: &RecordAttrs) -> Option<Format> {
        if r.rtype >> 4 != 0 {
            return None;
        }
        match r.rtype & 0xf {
            rfm::VAR => Some(Format::Var { vfc: 0 }),
            rfm::VFC => Some(Format::Var { vfc: if r.vfcsize == 0 { 2 } else { r.vfcsize as usize } }),
            rfm::FIX if r.rsize > 0 => Some(Format::Fix(r.rsize as usize)),
            rfm::STMLF => Some(Format::Stream(b"\n")),
            rfm::STMCR => Some(Format::Stream(b"\r")),
            rfm::STM => Some(Format::Stream(b"\r\n")),
            _ => None,
        }
    }
}

/// Whether a file is text beyond doubt: records, with carriage control.
pub fn is_text(r: &RecordAttrs) -> bool {
    let cc = r.rattrib & (rat::CR | rat::FTN | rat::PRN) != 0;
    match r.rtype & 0xf {
        rfm::STMLF | rfm::STMCR | rfm::STM => r.rtype >> 4 == 0,
        rfm::VAR | rfm::VFC => cc && r.rtype >> 4 == 0,
        _ => false,
    }
}

/// Iterates over the records in a byte stream (a file's data up to its end
/// of file). VFC records keep their control area.
pub struct Records<R> {
    r: R,
    format: Format,
    pos: u64,
}

impl<R: Read> Records<R> {
    pub fn new(r: R, format: Format) -> Records<R> {
        Records { r, format, pos: 0 }
    }

    /// Records from `pos` on, where `r` stands. `pos` must be where a
    /// record starts (VAR files skip to block boundaries by it).
    pub fn at(r: R, format: Format, pos: u64) -> Records<R> {
        Records { r, format, pos }
    }

    /// Byte offset of the next record.
    pub fn position(&self) -> u64 {
        self.pos
    }

    pub fn format(&self) -> Format {
        self.format
    }

    /// The control area size, to skip when printing a record as text.
    pub fn control_size(&self) -> usize {
        match self.format {
            Format::Var { vfc } => vfc,
            _ => 0,
        }
    }

    fn byte(&mut self) -> io::Result<Option<u8>> {
        let mut b = [0u8];
        match self.r.read(&mut b)? {
            0 => Ok(None),
            _ => {
                self.pos += 1;
                Ok(Some(b[0]))
            }
        }
    }

    fn exact(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut v = vec![0u8; n];
        self.r.read_exact(&mut v).map_err(|_| truncated())?;
        self.pos += n as u64;
        Ok(v)
    }

    fn skip(&mut self, n: u64) -> io::Result<()> {
        for _ in 0..n {
            if self.byte()?.is_none() {
                break;
            }
        }
        Ok(())
    }
}

fn truncated() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "record cut short by the end of file")
}

impl<R: Read> Iterator for Records<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let r = (|| -> io::Result<Option<Vec<u8>>> {
            match self.format {
                Format::Var { .. } => loop {
                    let Some(lo) = self.byte()? else { return Ok(None) };
                    let hi = self.byte()?.ok_or_else(truncated)?;
                    let len = u16::from_le_bytes([lo, hi]);
                    if len == 0xffff {
                        self.skip((BLOCK as u64 - self.pos % BLOCK as u64) % BLOCK as u64)?;
                        continue;
                    }
                    let data = self.exact(len as usize)?;
                    if len % 2 == 1 {
                        self.skip(1)?;
                    }
                    return Ok(Some(data));
                },
                Format::Fix(n) => {
                    let Some(first) = self.byte()? else { return Ok(None) };
                    let mut data = vec![first];
                    data.extend(self.exact(n - 1)?);
                    if n % 2 == 1 {
                        self.skip(1)?;
                    }
                    Ok(Some(data))
                }
                Format::Stream(end) => {
                    let mut data = Vec::new();
                    loop {
                        match self.byte()? {
                            None if data.is_empty() => return Ok(None),
                            None => return Ok(Some(data)),
                            Some(b) => {
                                data.push(b);
                                if data.ends_with(end) || end == b"\r\n" && data.ends_with(b"\n") {
                                    let cut = if data.ends_with(end) { end.len() } else { 1 };
                                    data.truncate(data.len() - cut);
                                    return Ok(Some(data));
                                }
                            }
                        }
                    }
                }
            }
        })();
        r.transpose()
    }
}

/// Writes lines as VAR records: each line (without its LF, or CR LF)
/// becomes one record. Tracks the longest record for the file's
/// attributes.
pub struct VarWriter<W> {
    w: W,
    pub longest: usize,
    partial: Vec<u8>,
}

impl<W: Write> VarWriter<W> {
    pub fn new(w: W) -> VarWriter<W> {
        VarWriter { w, longest: 0, partial: Vec::new() }
    }

    fn record(&mut self, line: &[u8]) -> io::Result<()> {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let len = u16::try_from(line.len()).ok().filter(|&n| n < 0x8000).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "line longer than 32767 bytes, the largest record")
        })?;
        self.longest = self.longest.max(line.len());
        self.w.write_all(&len.to_le_bytes())?;
        self.w.write_all(line)?;
        if len % 2 == 1 {
            self.w.write_all(&[0])?;
        }
        Ok(())
    }

    /// Writes a final unterminated line, if any, and returns the inner
    /// writer.
    pub fn finish(mut self) -> io::Result<(W, usize)> {
        if !self.partial.is_empty() {
            let p = std::mem::take(&mut self.partial);
            self.record(&p)?;
        }
        Ok((self.w, self.longest))
    }
}

impl<W: Write> Write for VarWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut rest = buf;
        while let Some(i) = rest.iter().position(|&b| b == b'\n') {
            let mut line = std::mem::take(&mut self.partial);
            line.extend_from_slice(&rest[..i]);
            self.record(&line)?;
            rest = &rest[i + 1..];
        }
        self.partial.extend_from_slice(rest);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(format: Format, data: &[u8]) -> Vec<Vec<u8>> {
        Records::new(data, format).collect::<io::Result<_>>().unwrap()
    }

    #[test]
    fn var_round_trip() {
        let mut w = VarWriter::new(Vec::new());
        w.write_all(b"hello\nodd\n\nlast").unwrap();
        let (bytes, longest) = w.finish().unwrap();
        assert_eq!(longest, 5);
        assert_eq!(bytes, b"\x05\0hello\0\x03\0odd\0\0\0\x04\0last");
        let r = lines(Format::Var { vfc: 0 }, &bytes);
        assert_eq!(r, [&b"hello"[..], b"odd", b"", b"last"]);
    }

    #[test]
    fn var_skips_rest_of_block() {
        let mut data = vec![0u8; 1024];
        data[..6].copy_from_slice(b"\x02\0hi\xff\xff");
        data[512..516].copy_from_slice(b"\x01\0x\0");
        let r = lines(Format::Var { vfc: 0 }, &data[..516]);
        assert_eq!(r, [&b"hi"[..], b"x"]);
    }

    #[test]
    fn fix_and_stream() {
        assert_eq!(lines(Format::Fix(3), b"abc\0def\0"), [&b"abc"[..], b"def"]);
        assert_eq!(lines(Format::Stream(b"\n"), b"a\nb"), [&b"a"[..], b"b"]);
        assert_eq!(lines(Format::Stream(b"\r\n"), b"a\r\nb\n"), [&b"a"[..], b"b"]);
        assert!(Records::new(&b"\x05\0ab"[..], Format::Var { vfc: 0 }).next().unwrap().is_err());
    }
}
