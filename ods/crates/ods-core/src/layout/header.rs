use alloc::vec::Vec;

use super::{BLOCK, Block, Fid, Field, checksum, layout};
use crate::layout::NameType;

layout! {
    /// File header (FH2$): one block in INDEXF.SYS describing one file, or
    /// one more piece of a file's map. The four offsets, in words, split the
    /// block into the header, ident, map, access control and reserved areas.
    /// See docs/file-header.md.
    pub struct Header {
        idoffset, set_idoffset: u8 = 0;
        mpoffset, set_mpoffset: u8 = 1;
        acoffset, set_acoffset: u8 = 2;
        rsoffset, set_rsoffset: u8 = 3;
        seg_num, set_seg_num: u16 = 4;
        struclev, set_struclev: u16 = 6;
        fid, set_fid: Fid = 8;
        ext_fid, set_ext_fid: Fid = 14;
        recattr, set_recattr: [u8; 32] = 20;
        filechar, set_filechar: u32 = 52;
        /// Default record protection; VMS 7 writes 0xFE00 here.
        recprot, set_recprot: u16 = 56;
        map_inuse, set_map_inuse: u8 = 58;
        acc_mode, set_acc_mode: u8 = 59;
        fileowner, set_fileowner: u32 = 60;
        fileprot, set_fileprot: u16 = 64;
        backlink, set_backlink: Fid = 66;
        journal, set_journal: u8 = 72;
        ru_active, set_ru_active: u8 = 73;
        /// ODS-5 hard link count; reserved on ODS-2.
        linkcount, set_linkcount: u16 = 74;
        /// Only present when the header area reaches it; see `highwater_mark`.
        highwater, set_highwater: u32 = 76;
        checksum, set_checksum: u16 = 510;
    }
}

/// File characteristics bit for "marked for delete".
const MARKDEL: u32 = 1 << 15;

impl Header {
    /// Why this block is not a valid file header, if it is not.
    pub fn invalid(&self) -> Option<&'static str> {
        let (id, mp, ac, rs) = (self.idoffset(), self.mpoffset(), self.acoffset(), self.rsoffset());
        let level = self.struclev() >> 8;
        if self.checksum() != checksum(&self.0, 255) {
            Some("bad checksum")
        } else if !(level == 2 || level == 5) || self.struclev() & 0xff == 0 {
            Some("unsupported structure level")
        } else if id < 38 || id > mp || mp > ac || ac > rs {
            Some("area offsets out of order")
        } else if self.map_inuse() > ac - mp {
            Some("map area overflows")
        } else if self.fid().num == 0 {
            Some("zero file number")
        } else {
            None
        }
    }

    /// A deleted header keeps its sequence number for the next user of the
    /// slot, and has a zero file number and checksum.
    pub fn is_deleted(&self) -> bool {
        self.filechar() & MARKDEL != 0 && self.fid().num == 0 && self.checksum() == 0
    }

    pub fn update_checksum(&mut self) {
        self.set_checksum(checksum(&self.0, 255));
    }

    /// Bytes `from..to` (word offsets), clipped to the checksum word.
    fn area(&self, from: u8, to: u8) -> &[u8] {
        let (a, b) = ((from as usize * 2).min(BLOCK - 2), (to as usize * 2).min(BLOCK - 2));
        &self.0[a..b.max(a)]
    }

    pub fn ident_area(&self) -> &[u8] {
        self.area(self.idoffset(), self.mpoffset())
    }

    pub fn map_area(&self) -> &[u8] {
        let mp = self.mpoffset();
        self.area(mp, mp.saturating_add(self.map_inuse()).min(self.acoffset()))
    }

    /// Map area capacity in bytes: everything up to the access control list.
    pub fn map_capacity(&self) -> usize {
        self.area(self.mpoffset(), self.acoffset()).len()
    }

    pub fn acl_area(&self) -> &[u8] {
        self.area(self.acoffset(), self.rsoffset())
    }

    /// Replaces the map area contents. The caller checks the capacity.
    pub fn set_map(&mut self, map: &[u8]) {
        let at = self.mpoffset() as usize * 2;
        let cap = self.map_capacity();
        self.0[at..at + cap].fill(0);
        self.0[at..at + map.len()].copy_from_slice(map);
        self.set_map_inuse((map.len() / 2) as u8);
    }

    /// The highwater mark, when the header area is long enough to hold it
    /// (volumes older than VMS V4 have shorter headers).
    pub fn highwater_mark(&self) -> Option<u32> {
        (self.idoffset() >= 40).then(|| self.highwater())
    }

    pub fn record_attrs(&self) -> RecordAttrs {
        RecordAttrs::from_bytes(&self.recattr())
    }

    pub fn set_record_attrs(&mut self, r: &RecordAttrs) {
        self.set_recattr(r.to_bytes());
    }

    /// The ident area, decoded. Short areas read as if padded with zeros.
    pub fn ident(&self) -> Option<Ident> {
        let area = self.ident_area();
        if area.is_empty() {
            return None;
        }
        let mut buf = [0u8; FI5_MAX];
        let n = area.len().min(FI5_MAX);
        buf[..n].copy_from_slice(&area[..n]);
        Some(if self.struclev() >> 8 == 5 {
            let f = Fi5(&buf[..]);
            let len = (f.namelen() as usize).min(FI5_MAX - FI5_NAME);
            Ident {
                name: buf[FI5_NAME..FI5_NAME + len].to_vec(),
                name_type: NameType::from_code(f.control() & 3),
                revision: f.revision(),
                credate: f.credate(),
                revdate: f.revdate(),
                expdate: f.expdate(),
                bakdate: f.bakdate(),
                accdate: f.accdate(),
                attdate: f.attdate(),
            }
        } else {
            let f = Fi2(&buf[..]);
            let mut name = f.filename().to_vec();
            name.extend_from_slice(&f.filenamext());
            while name.last().is_some_and(|&c| c == b' ' || c == 0) {
                name.pop();
            }
            Ident {
                name,
                name_type: NameType::Ods2,
                revision: f.revision(),
                credate: f.credate(),
                revdate: f.revdate(),
                expdate: f.expdate(),
                bakdate: f.bakdate(),
                accdate: 0,
                attdate: 0,
            }
        })
    }

    /// Ident area size in words needed to hold `name` at this header's
    /// structure level.
    pub fn ident_words(level: u8, name_len: usize) -> u8 {
        let bytes = if level == 5 { (FI5_NAME + name_len).max(FI5_MIN) } else { FI2_LENGTH };
        bytes.div_ceil(2) as u8
    }

    /// Writes the known ident fields, leaving the others as they are. Names
    /// are cut to what the area holds: size it first with `ident_words`.
    pub fn set_ident(&mut self, id: &Ident) {
        let from = self.idoffset() as usize * 2;
        let n = self.ident_area().len().min(FI5_MAX);
        let mut buf = [0u8; FI5_MAX];
        buf[..n].copy_from_slice(&self.0[from..from + n]);
        if self.struclev() >> 8 == 5 {
            let len = id.name.len().min(n.saturating_sub(FI5_NAME));
            let mut f = Fi5(&mut buf[..]);
            f.set_control(f.control() & !3 | id.name_type.code());
            f.set_namelen(len as u8);
            f.set_revision(id.revision);
            f.set_credate(id.credate);
            f.set_revdate(id.revdate);
            f.set_expdate(id.expdate);
            f.set_bakdate(id.bakdate);
            f.set_accdate(id.accdate);
            f.set_attdate(id.attdate);
            buf[FI5_NAME..].fill(0);
            buf[FI5_NAME..FI5_NAME + len].copy_from_slice(&id.name[..len]);
        } else {
            let mut name = [b' '; 86];
            let len = id.name.len().min(86);
            name[..len].copy_from_slice(&id.name[..len]);
            let mut f = Fi2(&mut buf[..]);
            f.set_filename(Field::get(&name[..]));
            f.set_filenamext(Field::get(&name[20..]));
            f.set_revision(id.revision);
            f.set_credate(id.credate);
            f.set_revdate(id.revdate);
            f.set_expdate(id.expdate);
            f.set_bakdate(id.bakdate);
        }
        self.0[from..from + n].copy_from_slice(&buf[..n]);
    }
}

impl Default for Header {
    fn default() -> Self {
        Header([0; BLOCK])
    }
}

impl From<Block> for Header {
    fn from(b: Block) -> Self {
        Header(b)
    }
}

pub(crate) const FI2_LENGTH: usize = 120;
const FI5_NAME: usize = 76;
const FI5_MIN: usize = 120;
const FI5_MAX: usize = 324;

layout! {
    /// ODS-2 ident area (FI2$): name, revision count and dates.
    pub struct Fi2 {
        filename, set_filename: [u8; 20] = 0;
        revision, set_revision: u16 = 20;
        credate, set_credate: u64 = 22;
        revdate, set_revdate: u64 = 30;
        expdate, set_expdate: u64 = 38;
        bakdate, set_bakdate: u64 = 46;
        filenamext, set_filenamext: [u8; 66] = 54;
    }
}

layout! {
    /// ODS-5 ident area (FI5$). The name starts at byte 76 and runs for
    /// `namelen` bytes, up to 248: the area grows to fit it.
    pub struct Fi5 {
        /// Bits 0-1: name type (0 ODS-2, 1 ISO Latin-1, 3 UCS-2).
        control, set_control: u8 = 0;
        namelen, set_namelen: u8 = 1;
        revision, set_revision: u16 = 2;
        credate, set_credate: u64 = 4;
        revdate, set_revdate: u64 = 12;
        expdate, set_expdate: u64 = 20;
        bakdate, set_bakdate: u64 = 28;
        accdate, set_accdate: u64 = 36;
        attdate, set_attdate: u64 = 44;
        ex_recattr, set_ex_recattr: [u8; 8] = 52;
        length_hint, set_length_hint: [u8; 16] = 60;
    }
}

/// The ident area, decoded: the file's primary name and its dates. The name
/// is raw, as stored: "NAME.TYPE;VERSION".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ident {
    pub name: Vec<u8>,
    pub name_type: NameType,
    pub revision: u16,
    pub credate: u64,
    pub revdate: u64,
    pub expdate: u64,
    pub bakdate: u64,
    /// ODS-5 only: last access and last attribute change.
    pub accdate: u64,
    pub attdate: u64,
}

/// A retrieval pointer from a header's map area. `count` is in blocks (the
/// on-disk field holds count - 1), `format` is how it is encoded (1-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// Format 0: placement control word, maps nothing.
    Placement(u16),
    Extent {
        format: u8,
        count: u32,
        lbn: u32,
    },
}

impl Pointer {
    /// An extent in the smallest format that can encode it, `None` if even
    /// format 3 cannot (count above 2^30).
    pub fn extent(count: u32, lbn: u32) -> Option<Pointer> {
        let format = match (count, lbn) {
            (1..=256, 0..0x40_0000) => 1,
            (1..=0x4000, _) => 2,
            (1..=0x4000_0000, _) => 3,
            _ => return None,
        };
        Some(Pointer::Extent { format, count, lbn })
    }

    /// Encoded size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Pointer::Placement(_) => 2,
            Pointer::Extent { format, .. } => 2 + 2 * *format as usize,
        }
    }

    /// Whether `count` blocks still fit the encoding this pointer uses.
    pub fn format_holds(format: u8, count: u32) -> bool {
        count >= 1 && count <= [0, 256, 0x4000, 0x4000_0000][format as usize & 3]
    }
}

/// Decodes a map area. Fails on a truncated pointer.
pub fn decode_map(mut m: &[u8]) -> Result<Vec<Pointer>, &'static str> {
    let mut out = Vec::new();
    while m.len() >= 2 {
        let w0 = u16::from_le_bytes([m[0], m[1]]);
        let word = |i: usize| m.get(2 * i..2 * i + 2).map(|w| u16::from_le_bytes([w[0], w[1]]) as u32);
        let (p, len) = match w0 >> 14 {
            0 => (Pointer::Placement(w0), 2),
            1 => {
                let lbn = ((w0 as u32 >> 8) & 0x3f) << 16 | word(1).ok_or("truncated map pointer")?;
                (Pointer::Extent { format: 1, count: (w0 & 0xff) as u32 + 1, lbn }, 4)
            }
            2 => {
                let lbn = word(1).zip(word(2)).ok_or("truncated map pointer")?;
                (Pointer::Extent { format: 2, count: (w0 & 0x3fff) as u32 + 1, lbn: lbn.0 | lbn.1 << 16 }, 6)
            }
            _ => {
                let (lo, lbn) = word(1).zip(word(2).zip(word(3))).ok_or("truncated map pointer")?;
                let count = ((w0 & 0x3fff) as u32) << 16 | lo;
                (Pointer::Extent { format: 3, count: count + 1, lbn: lbn.0 | lbn.1 << 16 }, 8)
            }
        };
        out.push(p);
        m = &m[len..];
    }
    Ok(out)
}

/// Encodes pointers, each in its own format. Counts must fit the format.
pub fn encode_map(ptrs: &[Pointer], out: &mut Vec<u8>) {
    for p in ptrs {
        let words: &[u16] = match *p {
            Pointer::Placement(w) => &[w],
            Pointer::Extent { format: 1, count, lbn } => {
                &[0x4000 | ((lbn >> 16) as u16 & 0x3f) << 8 | (count - 1) as u16 & 0xff, lbn as u16]
            }
            Pointer::Extent { format: 2, count, lbn } => {
                &[0x8000 | (count - 1) as u16 & 0x3fff, lbn as u16, (lbn >> 16) as u16]
            }
            Pointer::Extent { count, lbn, .. } => {
                &[0xc000 | ((count - 1) >> 16) as u16 & 0x3fff, (count - 1) as u16, lbn as u16, (lbn >> 16) as u16]
            }
        };
        for w in words {
            out.extend_from_slice(&w.to_le_bytes());
        }
    }
}

/// Record attributes (FAT$), the 32 bytes RMS keeps in every file header.
/// The file system stores them and only reads the end of file and allocated
/// size; every byte round-trips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordAttrs {
    /// Low nibble: record format (FAT$C_*). High nibble: file organization.
    pub rtype: u8,
    /// FAT$M_FORTRANCC, _IMPLIEDCC, _PRINTCC, _NOSPAN, _MSBRCW.
    pub rattrib: u8,
    pub rsize: u16,
    /// Highest allocated VBN. On disk the two words are swapped.
    pub hiblk: u32,
    /// End of file VBN, word-swapped like `hiblk`.
    pub efblk: u32,
    /// First free byte in `efblk`.
    pub ffbyte: u16,
    pub bktsize: u8,
    pub vfcsize: u8,
    pub maxrec: u16,
    pub defext: u16,
    pub gbc: u16,
    pub reserved: [u8; 8],
    /// Default version limit, used only in directory headers.
    pub versions: u16,
}

impl RecordAttrs {
    pub fn from_bytes(b: &[u8; 32]) -> RecordAttrs {
        let w = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let swapped = |i: usize| (w(i) as u32) << 16 | w(i + 2) as u32;
        let mut reserved = [0; 8];
        reserved.copy_from_slice(&b[22..30]);
        RecordAttrs {
            rtype: b[0],
            rattrib: b[1],
            rsize: w(2),
            hiblk: swapped(4),
            efblk: swapped(8),
            ffbyte: w(12),
            bktsize: b[14],
            vfcsize: b[15],
            maxrec: w(16),
            defext: w(18),
            gbc: w(20),
            reserved,
            versions: w(30),
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut b = [0; 32];
        let mut put = |i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
        put(2, self.rsize);
        put(4, (self.hiblk >> 16) as u16);
        put(6, self.hiblk as u16);
        put(8, (self.efblk >> 16) as u16);
        put(10, self.efblk as u16);
        put(12, self.ffbyte);
        put(16, self.maxrec);
        put(18, self.defext);
        put(20, self.gbc);
        put(30, self.versions);
        b[0] = self.rtype;
        b[1] = self.rattrib;
        b[14] = self.bktsize;
        b[15] = self.vfcsize;
        b[22..30].copy_from_slice(&self.reserved);
        b
    }

    /// Size of the file's data in bytes, from the end of file mark.
    pub fn eof_bytes(&self) -> u64 {
        match self.efblk {
            0 => 0,
            n => (n as u64 - 1) * BLOCK as u64 + self.ffbyte as u64,
        }
    }
}
