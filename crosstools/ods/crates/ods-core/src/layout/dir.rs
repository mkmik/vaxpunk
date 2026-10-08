use alloc::vec::Vec;

use super::{BLOCK, Block, Fid, Field};

/// Largest record a directory block holds: the block, less the -1 word that
/// always ends the records.
pub const MAX_RECORD: usize = BLOCK - 2;

/// How a name is encoded, both in directory records and ident areas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NameType {
    /// ODS-2 rules: uppercase A-Z, 0-9, $ - _.
    #[default]
    Ods2,
    /// ODS-5, one byte per character, ISO Latin-1.
    Isl1,
    /// ODS-5, two bytes per character, UCS-2 little-endian.
    Ucs2,
}

impl NameType {
    pub fn from_code(c: u8) -> NameType {
        match c {
            1 => NameType::Isl1,
            3 => NameType::Ucs2,
            _ => NameType::Ods2,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            NameType::Ods2 => 0,
            NameType::Isl1 => 1,
            NameType::Ucs2 => 3,
        }
    }
}

/// One directory record (DIR$): a name and some of its versions, highest
/// first. A name with too many versions for one record continues in the
/// next ones. See docs/directory.md.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirRecord {
    /// "NAME.TYPE", without version.
    pub name: Vec<u8>,
    pub verlimit: u16,
    /// Bits 0-2: record type, only 0 (version and FID list) is supported.
    /// Bits 3-5: name type on ODS-5 (see [`NameType`]).
    pub flags: u8,
    /// (version, FID), in decreasing version order.
    pub entries: Vec<(u16, Fid)>,
    /// The byte that pads an odd-length name, kept for round trips.
    pub pad: u8,
}

impl DirRecord {
    /// Encoded size, including the leading size word.
    pub fn size(&self) -> usize {
        6 + self.name.len().next_multiple_of(2) + 8 * self.entries.len()
    }

    pub fn name_type(&self) -> NameType {
        NameType::from_code(self.flags >> 3 & 7)
    }

    fn write(&self, b: &mut [u8]) {
        ((self.size() - 2) as u16).put(b);
        self.verlimit.put(&mut b[2..]);
        b[4] = self.flags;
        b[5] = self.name.len() as u8;
        let mut o = 6 + self.name.len();
        b[6..o].copy_from_slice(&self.name);
        if o % 2 == 1 {
            b[o] = self.pad;
            o += 1;
        }
        for (v, fid) in &self.entries {
            v.put(&mut b[o..]);
            fid.put(&mut b[o + 2..]);
            o += 8;
        }
    }
}

/// The records of one directory block, plus whatever follows them (the -1
/// word and stale bytes), so an unmodified block writes back identically.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirBlock {
    pub records: Vec<DirRecord>,
    pub tail: Vec<u8>,
}

impl DirBlock {
    pub fn parse(b: &Block) -> Result<DirBlock, &'static str> {
        let mut records = Vec::new();
        let mut o = 0;
        while o + 2 <= BLOCK {
            let size = u16::get(&b[o..]) as usize;
            if size == 0xffff {
                break;
            }
            let end = o + 2 + size;
            if !size.is_multiple_of(2) || size < 4 || end > BLOCK {
                return Err("bad directory record size");
            }
            let rec = &b[o..end];
            let (flags, len) = (rec[4], rec[5] as usize);
            let values = 6 + len.next_multiple_of(2);
            if values > rec.len() {
                return Err("directory record name overflows the record");
            }
            if flags & 7 != 0 {
                return Err("unsupported directory record type");
            }
            if !(rec.len() - values).is_multiple_of(8) || rec.len() == values {
                return Err("bad directory record value list");
            }
            let entries = rec[values..].as_chunks::<8>().0.iter().map(|e| (u16::get(e), Fid::get(&e[2..]))).collect();
            records.push(DirRecord {
                name: rec[6..6 + len].to_vec(),
                verlimit: u16::get(&rec[2..]),
                flags,
                entries,
                pad: if len % 2 == 1 { rec[6 + len] } else { 0 },
            });
            o = end;
        }
        Ok(DirBlock { records, tail: b[o..].to_vec() })
    }

    /// Bytes taken by the records.
    pub fn used(&self) -> usize {
        self.records.iter().map(DirRecord::size).sum()
    }

    /// Encodes the block, `None` if the records do not fit. The old tail is
    /// kept when the records still end where they did; otherwise the block
    /// ends with the -1 word and zeros.
    pub fn to_block(&self) -> Option<Block> {
        let used = self.used();
        let same_end = self.tail.len() == BLOCK.checked_sub(used)?;
        if !same_end && used > MAX_RECORD {
            return None;
        }
        let mut b = [0u8; BLOCK];
        let mut o = 0;
        for r in &self.records {
            r.write(&mut b[o..]);
            o += r.size();
        }
        if same_end {
            b[o..].copy_from_slice(&self.tail);
        } else {
            0xffffu16.put(&mut b[o..]);
        }
        Some(b)
    }
}
