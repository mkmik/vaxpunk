//! RMS relative and prologue 3 indexed files: reading their records along
//! any key, checking their structure, and loading records into a new
//! indexed file the way `CONVERT` does. The layouts are in
//! `docs/indexed.md` and `docs/relative.md`.
//!
//! Everything works on a file's bytes, VBN 1 first, up to its end of file.
//! What only the file header says (organization, record format, record and
//! bucket size) comes in a [`RecordAttrs`]. Bad data is an [`RmsError`],
//! never a panic.

mod check;
pub mod fdl;
mod load;
mod read;

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

use crate::layout::{BLOCK, Field, checksum, layout};

pub use check::{Finding, KeyStats, Report, check};
pub use load::{AreaSpec, KeySpec, Spec, build};
pub use read::{Indexed, Relative, RmsFile, open};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RmsError {
    /// A structure failed validation.
    Corrupt { what: &'static str, vbn: u32 },
    /// Valid, but not something we read: prologue 1 or 2, sequential files.
    Unsupported(&'static str),
    /// A loader's spec or records, or an FDL file, that cannot be.
    Invalid(String),
}

impl fmt::Display for RmsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RmsError::Corrupt { what, vbn } => write!(f, "{what} at VBN {vbn}"),
            RmsError::Unsupported(what) => write!(f, "unsupported: {what}"),
            RmsError::Invalid(what) => write!(f, "invalid: {what}"),
        }
    }
}

pub type Result<T> = core::result::Result<T, RmsError>;

fn corrupt<T>(what: &'static str, vbn: u32) -> Result<T> {
    Err(RmsError::Corrupt { what, vbn })
}

/// Key descriptor size, and the stride `CONVERT` and `CREATE/FDL` pack
/// alternate key descriptors at: five to a prologue block.
pub const KEY_LEN: usize = 102;
/// Area descriptor size; eight to a block.
pub const AREA_LEN: usize = 64;
/// Bucket header size: data, SIDRs or index entries start here.
pub const BKT_HDR: usize = 14;

/// `KEY$B_FLAGS`.
pub mod keyflag {
    pub const DUPKEYS: u8 = 1 << 0;
    pub const CHGKEYS: u8 = 1 << 1;
    pub const NULKEYS: u8 = 1 << 2;
    pub const IDX_COMPR: u8 = 1 << 3;
    pub const INITIDX: u8 = 1 << 4;
    pub const KEY_COMPR: u8 = 1 << 6;
    pub const REC_COMPR: u8 = 1 << 7;
}

/// `KEY$B_DATATYPE`; [`DESCENDING`](dtype::DESCENDING) added makes the
/// descending form.
pub mod dtype {
    pub const STRING: u8 = 0;
    pub const INT2: u8 = 1;
    pub const BIN2: u8 = 2;
    pub const INT4: u8 = 3;
    pub const BIN4: u8 = 4;
    pub const DECIMAL: u8 = 5;
    pub const INT8: u8 = 6;
    pub const BIN8: u8 = 7;
    pub const COLLATED: u8 = 8;
    pub const DESCENDING: u8 = 32;
}

/// Record control byte bits (`IRC$`), in data records, RRVs and SIDR
/// pointers.
pub mod irc {
    pub const PTRSZ: u8 = 3;
    pub const DELETED: u8 = 1 << 2;
    pub const RRV: u8 = 1 << 3;
    pub const NOPTRSZ: u8 = 1 << 4;
    /// On a SIDR's first pointer.
    pub const FIRST: u8 = 1 << 7;
}

/// `BKT$B_BKTCB`.
pub mod bktcb {
    pub const LASTBKT: u8 = 1 << 0;
    pub const ROOTBKT: u8 = 1 << 1;
}

layout! {
    /// Key descriptor (`KEY$`), 102 bytes; key 0's is at the start of VBN 1.
    pub struct KeyDesc {
        /// VBN of the next key descriptor, 0 after the last.
        idxfl, set_idxfl: u32 = 0;
        noff, set_noff: u16 = 4;
        ianum, set_ianum: u8 = 6;
        lanum, set_lanum: u8 = 7;
        danum, set_danum: u8 = 8;
        rootlev, set_rootlev: u8 = 9;
        idxbktsz, set_idxbktsz: u8 = 10;
        datbktsz, set_datbktsz: u8 = 11;
        rootvbn, set_rootvbn: u32 = 12;
        flags, set_flags: u8 = 16;
        datatype, set_datatype: u8 = 17;
        segments, set_segments: u8 = 18;
        nullchar, set_nullchar: u8 = 19;
        keysz, set_keysz: u8 = 20;
        keyref, set_keyref: u8 = 21;
        minrecsz, set_minrecsz: u16 = 22;
        idxfill, set_idxfill: u16 = 24;
        datfill, set_datfill: u16 = 26;
        sizes, set_sizes: [u8; 8] = 44;
        keynam, set_keynam: [u8; 32] = 52;
        ldvbn, set_ldvbn: u32 = 84;
        types, set_types: [u8; 8] = 88;
    }
}

layout! {
    /// Area descriptor (`AREA$`), 64 bytes.
    pub struct AreaDesc {
        flags, set_flags: u8 = 0;
        areaid, set_areaid: u8 = 2;
        arbktsz, set_arbktsz: u8 = 3;
        /// First reclaimed bucket.
        avail, set_avail: u32 = 8;
        /// The current extent: its start, size, and blocks used.
        cvbn, set_cvbn: u32 = 12;
        cnblk, set_cnblk: u32 = 16;
        used, set_used: u32 = 20;
        nxtvbn, set_nxtvbn: u32 = 24;
        nxt, set_nxt: u32 = 28;
        nxblk, set_nxblk: u32 = 32;
        deq, set_deq: u16 = 36;
        total_alloc, set_total_alloc: u32 = 50;
    }
}

layout! {
    /// Bucket header (`BKT$`), 14 bytes.
    pub struct BucketHeader {
        checkchar, set_checkchar: u8 = 0;
        indexno, set_indexno: u8 = 1;
        adrsample, set_adrsample: u16 = 2;
        freespace, set_freespace: u16 = 4;
        nxtrecid, set_nxtrecid: u16 = 6;
        nxtbkt, set_nxtbkt: u32 = 8;
        level, set_level: u8 = 12;
        bktcb, set_bktcb: u8 = 13;
    }
}

/// A key descriptor as it is stored.
pub type Key = KeyDesc<[u8; KEY_LEN]>;
/// An area descriptor as it is stored.
pub type Area = AreaDesc<[u8; AREA_LEN]>;

impl<B: AsRef<[u8]>> KeyDesc<B> {
    /// Offset of segment `i` in the record.
    pub fn position(&self, i: usize) -> u16 {
        u16::get(&self.0.as_ref()[28 + 2 * (i & 7)..])
    }

    /// (position, size) of each segment.
    pub fn segs(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let n = (self.segments() as usize).min(8);
        (0..n).map(|i| (self.position(i) as usize, self.sizes()[i] as usize))
    }

    pub fn has(&self, flag: u8) -> bool {
        self.flags() & flag != 0
    }

    /// The key's name, without its padding.
    pub fn name(&self) -> &[u8] {
        let n = &self.0.as_ref()[52..84];
        let end = n.iter().rposition(|&c| c != b' ' && c != 0).map_or(0, |i| i + 1);
        &n[..end]
    }

    /// The key of a record: its segments, one after another. `None` if the
    /// record is too short to hold them.
    pub fn key_of(&self, rec: &[u8]) -> Option<Vec<u8>> {
        let mut k = Vec::with_capacity(self.keysz() as usize);
        for (pos, size) in self.segs() {
            k.extend_from_slice(rec.get(pos..pos + size)?);
        }
        Some(k)
    }

    /// Whether a key is all `NULLCHAR`, the null key.
    pub fn is_null(&self, key: &[u8]) -> bool {
        key.iter().all(|&c| c == self.nullchar())
    }

    /// Orders two keys as RMS does for this key's data type: strings (and
    /// segmented keys) as unsigned bytes, integers as integers, descending
    /// types the other way round.
    pub fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        compare(self.datatype(), self.segments(), a, b)
    }
}

pub(crate) fn compare(datatype: u8, segments: u8, a: &[u8], b: &[u8]) -> Ordering {
    let int = |k: &[u8], signed: bool| -> i128 {
        let mut v = [0u8; 16];
        v[..k.len()].copy_from_slice(k);
        if signed && k.last().is_some_and(|&c| c & 0x80 != 0) {
            v[k.len()..].fill(0xff);
        }
        i128::from_le_bytes(v)
    };
    let width = match datatype & !dtype::DESCENDING {
        dtype::INT2 | dtype::BIN2 => 2,
        dtype::INT4 | dtype::BIN4 => 4,
        dtype::INT8 | dtype::BIN8 => 8,
        _ => 0,
    };
    let o = if segments == 1 && width > 0 && a.len() == width && b.len() == width {
        let signed = matches!(datatype & !dtype::DESCENDING, dtype::INT2 | dtype::INT4 | dtype::INT8);
        int(a, signed).cmp(&int(b, signed))
    } else {
        a.cmp(b)
    };
    if datatype & dtype::DESCENDING != 0 { o.reverse() } else { o }
}

/// Whether a prologue block's checksum, the sum of its first 255 words, is
/// right.
pub fn checksum_ok(b: &[u8]) -> bool {
    b.len() >= BLOCK && checksum(b, 255) == u16::get(&b[510..])
}

fn set_checksum(b: &mut [u8]) {
    let s = checksum(b, 255);
    s.put(&mut b[510..]);
}

/// Expands a compressed key (length, front count, bytes) at the start of
/// `b` against the key before it. Returns the key and the bytes it took.
fn expand_key(b: &[u8], prev: &[u8], keysz: usize) -> Option<(Vec<u8>, usize)> {
    let (&len, &front) = (b.first()?, b.get(1)?);
    let (len, front) = (len as usize, front as usize);
    let stored = b.get(2..2 + len)?;
    if front > prev.len() || front + len > keysz || front + len == 0 {
        return None;
    }
    let mut k = Vec::with_capacity(keysz);
    k.extend_from_slice(&prev[..front]);
    k.extend_from_slice(stored);
    let last = *k.last()?;
    k.resize(keysz, last);
    Some((k, 2 + len))
}

/// Compresses a key against the one before it in the bucket: the bytes they
/// share in front are counted, a run of the last byte is cut to one.
fn compress_key(key: &[u8], prev: Option<&[u8]>, out: &mut Vec<u8>) {
    let shared = prev.map_or(0, |p| p.iter().zip(key).take_while(|(a, b)| a == b).count());
    let front = shared.min(key.len().saturating_sub(1));
    let mut end = key.len();
    while end > front + 1 && key[end - 2] == key[end - 1] {
        end -= 1;
    }
    out.push((end - front) as u8);
    out.push(front as u8);
    out.extend_from_slice(&key[front..end]);
}

/// The longest record RMS has: anything that expands past it is corrupt.
const MAX_RECORD: usize = 65535;

/// Expands data record compression: segments of a length word, bytes, and
/// a count of more repeats of the last of them.
fn expand_segments(mut b: &[u8], out: &mut Vec<u8>) -> Option<()> {
    while !b.is_empty() {
        let len = u16::from_le_bytes([*b.first()?, *b.get(1)?]) as usize;
        let lit = b.get(2..2 + len)?;
        let count = *b.get(2 + len)? as usize;
        out.extend_from_slice(lit);
        if count > 0 {
            let last = *lit.last()?;
            out.resize(out.len() + count, last);
        }
        if out.len() > MAX_RECORD {
            return None;
        }
        b = &b[3 + len..];
    }
    Some(())
}

/// Compresses data the way RMS appears to: nothing for 8 bytes or fewer,
/// else runs of 6 or more of a byte, and of 5 at the end.
fn compress_segments(data: &[u8], out: &mut Vec<u8>) {
    let mut lit_start = 0;
    let mut i = 0;
    let n = data.len();
    let seg = |lit: &[u8], count: usize, out: &mut Vec<u8>| {
        out.extend_from_slice(&(lit.len() as u16).to_le_bytes());
        out.extend_from_slice(lit);
        out.push(count as u8);
    };
    if n <= 8 {
        if n > 0 {
            seg(data, 0, out);
        }
        return;
    }
    while i < n {
        let run = data[i..].iter().take_while(|&&c| c == data[i]).count();
        if run >= 6 || (i + run == n && run >= 5) {
            // The run's first byte ends the literal; the rest is a count,
            // 255 at most, so a longer run starts another segment.
            let mut left = run;
            let mut start = lit_start;
            let mut at = i;
            while left > 0 {
                let count = (left - 1).min(255);
                seg(&data[start..at + 1], count, out);
                at += 1 + count;
                left -= 1 + count;
                start = at;
            }
            i += run;
            lit_start = i;
        } else {
            i += run;
        }
    }
    if lit_start < n {
        seg(&data[lit_start..], 0, out);
    }
}

/// Little-endian unsigned integer of 1 to 8 bytes.
fn uint(b: &[u8]) -> u64 {
    b.iter().rev().fold(0, |v, &c| v << 8 | c as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ck(key: &[u8], prev: Option<&[u8]>) -> Vec<u8> {
        let mut v = Vec::new();
        compress_key(key, prev, &mut v);
        v
    }

    #[test]
    fn key_compression_as_documented() {
        assert_eq!(ck(b"K00000", None), b"\x02\x00K0");
        assert_eq!(ck(b"K00001", Some(b"K00000")), b"\x01\x051");
        assert_eq!(ck(b"K10000", Some(b"K00001")), b"\x02\x0110");
        assert_eq!(ck(b"K00100", Some(b"K00003")), b"\x02\x0310");
        assert_eq!(ck(&[0xff; 8], None), b"\x01\x00\xff");
        assert_eq!(ck(b"AAAA", Some(b"AAAA")), b"\x01\x03A");
        for (k, p) in [(&b"K00001"[..], &b"K00000"[..]), (b"AAAAAA", b"AAAAAB"), (b"ABBBBB", b"AB    ")] {
            let c = ck(k, Some(p));
            assert_eq!(expand_key(&c, p, 6), Some((k.to_vec(), c.len())));
        }
    }

    #[test]
    fn record_compression_round_trips() {
        let cases: [&[u8]; 6] =
            [b"", b"short", b"aaa::tail", b"ccc::ab    cd          ef", &[b'A'; 600], b"kkk::abc     "];
        for d in cases {
            let mut c = Vec::new();
            compress_segments(d, &mut c);
            let mut back = Vec::new();
            expand_segments(&c, &mut back).unwrap();
            assert_eq!(back, d);
        }
        // As RMS wrote them in fixtures/rms/comp.idx.
        let mut c = Vec::new();
        compress_segments(b"ccc::ab    cd          ef", &mut c);
        assert_eq!(c, b"\x0e\x00ccc::ab    cd \x09\x02\x00ef\x00");
        c.clear();
        compress_segments(&[b"fff::".as_slice(), &[b'A'; 268]].concat(), &mut c);
        assert_eq!(c, b"\x06\x00fff::A\xff\x01\x00A\x0b");
        c.clear();
        compress_segments(b"kkk::abc     ", &mut c);
        assert_eq!(c, b"\x09\x00kkk::abc \x04");
        c.clear();
        compress_segments(b" yyyyyyy", &mut c);
        assert_eq!(c, b"\x08\x00 yyyyyyy\x00");
    }

    #[test]
    fn integer_keys_compare_as_integers() {
        assert_eq!(compare(dtype::BIN4, 1, &[0, 1, 0, 0], &[0xff, 0, 0, 0]), Ordering::Greater);
        assert_eq!(compare(dtype::INT2, 1, &[0xff, 0xff], &[1, 0]), Ordering::Less);
        assert_eq!(compare(dtype::STRING | dtype::DESCENDING, 1, b"a", b"b"), Ordering::Greater);
        assert_eq!(uint(&[1, 2]), 0x201);
    }
}
