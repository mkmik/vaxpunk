//! A small FDL reader: the attributes that say what a relative or indexed
//! file is, as `ANALYZE/RMS_FILE/FDL` writes them and as they are written
//! by hand. Every other attribute, and the `IDENT`, `SYSTEM` and
//! `ANALYSIS_OF_*` sections, are passed over. Keywords are spelled out in
//! full, not abbreviated as VMS allows.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::{AreaSpec, KeySpec, Result, RmsError, Spec, dtype};
use crate::layout::{BLOCK, RecordAttrs};
use crate::{rat, rfm};

/// What an FDL file says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fdl {
    /// `FILE ORGANIZATION`, as the high nibble of `FAT$B_RTYPE`: 0
    /// sequential, 1 relative, 2 indexed.
    pub org: u8,
    /// `FILE BUCKET_SIZE`, 0 if not given.
    pub bktsz: u8,
    /// `FILE MAX_RECORD_NUMBER`, 0 if not given.
    pub mrn: u32,
    /// `RECORD CONTROL_FIELD_SIZE`.
    pub vfc: u8,
    /// Record format, size and attributes; areas and keys of an indexed
    /// file. `cluster` is 0.
    pub spec: Spec,
}

impl Fdl {
    /// The record attributes of a file the FDL describes, end of file
    /// aside: enough to read its records.
    pub fn record_attrs(&self) -> RecordAttrs {
        let bktsz = match self.org {
            2 => self.spec.areas.iter().map(|a| a.bktsz).max().unwrap_or(0),
            _ => self.bktsz,
        };
        RecordAttrs {
            rtype: self.org << 4 | self.spec.rfm,
            rattrib: self.spec.rat,
            rsize: if self.spec.rfm == rfm::FIX { self.spec.mrs } else { 0 },
            bktsize: bktsz,
            vfcsize: self.vfc,
            maxrec: self.spec.mrs,
            ..RecordAttrs::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    File,
    Record,
    Area(u8),
    Key(u8),
    Other,
}

/// A key as written: what wasn't given gets its default at the end.
#[derive(Default)]
struct RawKey {
    seg_len: [Option<u8>; 8],
    seg_pos: [Option<u16>; 8],
    datatype: Option<u8>,
    dups: Option<bool>,
    changes: Option<bool>,
    null_key: Option<bool>,
    null_value: u8,
    key_compr: Option<bool>,
    rec_compr: Option<bool>,
    idx_compr: Option<bool>,
    data_fill: Option<u32>,
    index_fill: Option<u32>,
    data_area: u8,
    index_area: u8,
    level1_area: Option<u8>,
    name: Vec<u8>,
}

const SKIPPED: [&str; 10] = [
    "IDENT",
    "TITLE",
    "SYSTEM",
    "ACCESS",
    "SHARING",
    "CONNECT",
    "DATE",
    "JOURNALING",
    "ANALYSIS_OF_AREA",
    "ANALYSIS_OF_KEY",
];

const TYPES: [(&str, u8); 9] = [
    ("STRING", dtype::STRING),
    ("INT2", dtype::INT2),
    ("BIN2", dtype::BIN2),
    ("INT4", dtype::INT4),
    ("BIN4", dtype::BIN4),
    ("DECIMAL", dtype::DECIMAL),
    ("INT8", dtype::INT8),
    ("BIN8", dtype::BIN8),
    ("COLLATED", dtype::COLLATED),
];

/// Reads an FDL file.
pub fn parse(text: &str) -> Result<Fdl> {
    let mut fdl = Fdl {
        org: 0,
        bktsz: 0,
        mrn: 0,
        vfc: 0,
        spec: Spec { rfm: rfm::VAR, mrs: 0, rat: rat::CR, areas: Vec::new(), keys: Vec::new(), cluster: 0 },
    };
    let mut areas: BTreeMap<u8, AreaSpec> = BTreeMap::new();
    let mut keys: BTreeMap<u8, RawKey> = BTreeMap::new();
    let mut section = Section::None;
    for (i, line) in text.lines().enumerate() {
        let bad = |m: &str| RmsError::Invalid(format!("line {}: {m}", i + 1));
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        let (word, value) = line.split_once(|c: char| c.is_ascii_whitespace()).unwrap_or((line, ""));
        let word = word.to_ascii_uppercase();
        let value = value.trim();
        let number = || number(value).ok_or_else(|| bad("expected a number"));
        let small = |max: u32| number().and_then(|n| if n <= max { Ok(n) } else { Err(bad("number too big")) });
        let yes = || match value.to_ascii_uppercase().as_str() {
            "YES" | "TRUE" => Ok(true),
            "NO" | "FALSE" => Ok(false),
            _ => Err(bad("expected yes or no")),
        };
        match word.as_str() {
            "FILE" => section = Section::File,
            "RECORD" => section = Section::Record,
            "AREA" => {
                let n = small(254)? as u8;
                areas.entry(n).or_insert(AreaSpec { bktsz: 0, alloc: 0, deq: 0 });
                section = Section::Area(n);
            }
            "KEY" => {
                let n = small(254)? as u8;
                keys.entry(n).or_default();
                section = Section::Key(n);
            }
            w if SKIPPED.contains(&w) => section = Section::Other,
            _ => match section {
                Section::None => return Err(bad("attribute outside a section")),
                Section::File => match word.as_str() {
                    "ORGANIZATION" => {
                        fdl.org = match value.to_ascii_uppercase().as_str() {
                            "SEQUENTIAL" => 0,
                            "RELATIVE" => 1,
                            "INDEXED" => 2,
                            _ => return Err(bad("organization is sequential, relative or indexed")),
                        }
                    }
                    "BUCKET_SIZE" => fdl.bktsz = small(63)? as u8,
                    "MAX_RECORD_NUMBER" => fdl.mrn = number()?,
                    _ => {}
                },
                Section::Record => match word.as_str() {
                    "FORMAT" => {
                        fdl.spec.rfm = match value.to_ascii_uppercase().as_str() {
                            "FIXED" => rfm::FIX,
                            "VARIABLE" => rfm::VAR,
                            "VFC" => rfm::VFC,
                            "STREAM" => rfm::STM,
                            "STREAM_LF" => rfm::STMLF,
                            "STREAM_CR" => rfm::STMCR,
                            "UNDEFINED" => rfm::UDF,
                            _ => return Err(bad("unknown record format")),
                        }
                    }
                    "SIZE" => fdl.spec.mrs = small(65535)? as u16,
                    "CONTROL_FIELD_SIZE" => fdl.vfc = small(255)? as u8,
                    "CARRIAGE_CONTROL" => {
                        fdl.spec.rat = fdl.spec.rat & rat::BLK
                            | match value.to_ascii_uppercase().as_str() {
                                "CARRIAGE_RETURN" => rat::CR,
                                "FORTRAN" => rat::FTN,
                                "PRINT" => rat::PRN,
                                "NONE" => 0,
                                _ => return Err(bad("unknown carriage control")),
                            }
                    }
                    "BLOCK_SPAN" => {
                        fdl.spec.rat = fdl.spec.rat & !rat::BLK | if yes()? { 0 } else { rat::BLK };
                    }
                    _ => {}
                },
                Section::Area(n) => {
                    let a = areas.get_mut(&n).ok_or_else(|| bad("no such area"))?;
                    match word.as_str() {
                        "BUCKET_SIZE" => a.bktsz = small(63)? as u8,
                        "ALLOCATION" => a.alloc = number()?,
                        "EXTENSION" => a.deq = small(65535)? as u16,
                        _ => {}
                    }
                }
                Section::Key(n) => {
                    let k = keys.get_mut(&n).ok_or_else(|| bad("no such key"))?;
                    let seg = |w: &str, suffix: &str| {
                        w.strip_prefix("SEG")?.strip_suffix(suffix)?.parse::<usize>().ok().filter(|&s| s < 8)
                    };
                    match word.as_str() {
                        w if seg(w, "_LENGTH").is_some() => {
                            k.seg_len[seg(w, "_LENGTH").unwrap_or(0)] = Some(small(255)? as u8)
                        }
                        w if seg(w, "_POSITION").is_some() => {
                            k.seg_pos[seg(w, "_POSITION").unwrap_or(0)] = Some(small(65535)? as u16)
                        }
                        "TYPE" => {
                            let t = value.to_ascii_uppercase();
                            let (desc, t) = match t.strip_prefix('D').filter(|r| TYPES.iter().any(|x| x.0 == *r)) {
                                Some(r) if t != "DECIMAL" => (dtype::DESCENDING, r.to_string()),
                                _ => (0, t.clone()),
                            };
                            let base = TYPES.iter().find(|x| x.0 == t).ok_or_else(|| bad("unknown key type"))?.1;
                            k.datatype = Some(base | desc);
                        }
                        "DUPLICATES" => k.dups = Some(yes()?),
                        "CHANGES" => k.changes = Some(yes()?),
                        "NULL_KEY" => k.null_key = Some(yes()?),
                        "NULL_VALUE" => {
                            k.null_value = match quoted(value) {
                                Some(s) if s.len() == 1 => s[0],
                                Some(_) => return Err(bad("a null value is one character")),
                                None => small(255)? as u8,
                            }
                        }
                        "DATA_KEY_COMPRESSION" => k.key_compr = Some(yes()?),
                        "DATA_RECORD_COMPRESSION" => k.rec_compr = Some(yes()?),
                        "INDEX_COMPRESSION" => k.idx_compr = Some(yes()?),
                        "DATA_FILL" => k.data_fill = Some(number()?),
                        "INDEX_FILL" => k.index_fill = Some(number()?),
                        "DATA_AREA" => k.data_area = small(254)? as u8,
                        "INDEX_AREA" => k.index_area = small(254)? as u8,
                        "LEVEL1_INDEX_AREA" => k.level1_area = Some(small(254)? as u8),
                        "NAME" => k.name = quoted(value).ok_or_else(|| bad("expected a quoted name"))?,
                        "PROLOG" | "PROLOGUE" if number()? != 3 => {
                            return Err(RmsError::Unsupported("prologues other than 3"));
                        }
                        _ => {}
                    }
                }
                Section::Other => {}
            },
        }
    }

    // Areas: as given, or one of the file's bucket size.
    if areas.is_empty() {
        let bktsz = match fdl.bktsz {
            0 if fdl.spec.mrs == 0 => 2,
            0 => (BKT_OVERHEAD + fdl.spec.mrs as usize).div_ceil(BLOCK).min(63) as u8,
            b => b,
        };
        areas.insert(0, AreaSpec { bktsz, alloc: 0, deq: 0 });
    }
    for (i, (&n, a)) in areas.iter_mut().enumerate() {
        if n as usize != i {
            return Err(RmsError::Invalid(format!("area {i} missing")));
        }
        if a.bktsz == 0 {
            a.bktsz = fdl.bktsz.max(1);
        }
    }
    fdl.spec.areas = areas.into_values().collect();

    for (i, (&n, k)) in keys.iter().enumerate() {
        if n as usize != i {
            return Err(RmsError::Invalid(format!("key {i} missing")));
        }
        let segments: Vec<(u16, u8)> =
            (0..8).map_while(|s| k.seg_len[s].map(|len| (k.seg_pos[s].unwrap_or(0), len))).collect();
        if segments.is_empty() {
            return Err(RmsError::Invalid(format!("key {n} has no SEG0_LENGTH")));
        }
        let datatype = k.datatype.unwrap_or(dtype::STRING);
        let size: usize = segments.iter().map(|s| s.1 as usize).sum();
        // Unless told, compression is on for string keys of 6 bytes and
        // more: the fixtures' keys of 8 and more have it, of 5 don't.
        let compress = datatype & !dtype::DESCENDING == dtype::STRING && size >= 6;
        let bucket = |a: u8| fdl.spec.areas.get(a as usize).map_or(BLOCK, |a| a.bktsz as usize * BLOCK);
        let fill = |f: Option<u32>, a: u8| match f {
            None => 0,
            Some(p) if p <= 100 => (bucket(a) * p as usize / 100) as u16,
            Some(b) => b.min(65535) as u16,
        };
        fdl.spec.keys.push(KeySpec {
            segments,
            datatype,
            dups: k.dups.unwrap_or(false),
            changes: k.changes.unwrap_or(false),
            null_key: k.null_key.unwrap_or(false),
            null_value: k.null_value,
            key_compr: k.key_compr.unwrap_or(compress),
            rec_compr: n == 0 && k.rec_compr.unwrap_or(compress),
            idx_compr: k.idx_compr.unwrap_or(compress),
            data_fill: fill(k.data_fill, k.data_area),
            index_fill: fill(k.index_fill, k.index_area),
            data_area: k.data_area,
            index_area: k.index_area,
            level1_area: k.level1_area.unwrap_or(k.index_area),
            name: k.name.clone(),
        });
    }
    if fdl.org == 2 && fdl.spec.keys.is_empty() {
        return Err(RmsError::Invalid("an indexed file needs KEY 0".into()));
    }
    Ok(fdl)
}

/// A bucket header, a record's overhead and the check byte: what a bucket
/// needs beyond a record.
const BKT_OVERHEAD: usize = 14 + 11 + 1;

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '!' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

/// A number: decimal, or `%X`, `%O` or `%D` and digits.
fn number(s: &str) -> Option<u32> {
    let s = s.trim();
    let (radix, digits) = match s.get(..2).map(|p| p.to_ascii_uppercase()) {
        Some(p) if p == "%X" => (16, &s[2..]),
        Some(p) if p == "%O" => (8, &s[2..]),
        Some(p) if p == "%D" => (10, &s[2..]),
        _ => (10, s),
    };
    u32::from_str_radix(digits, radix).ok()
}

/// The text between double or single quotes, a doubled quote standing for
/// one.
fn quoted(s: &str) -> Option<Vec<u8>> {
    let q = s.chars().next().filter(|&c| c == '"' || c == '\'')?;
    let inner = s[1..].strip_suffix(q)?;
    let mut doubled = String::new();
    doubled.push(q);
    doubled.push(q);
    Some(inner.replace(&doubled, &q.to_string()).into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_written() {
        let f = parse(
            "FILE\n\tORGANIZATION\tindexed\nRECORD\n\tFORMAT\tfixed\n\tSIZE\t40\n! a comment\n\
             AREA 0\n\tBUCKET_SIZE\t2\nAREA 1\n\tBUCKET_SIZE 3\n\tALLOCATION 30\n\tEXTENSION 6\n\
             KEY 0\n\tSEG0_LENGTH\t6\n\tSEG0_POSITION\t0\n\tNAME\t\"NUM\"\"BER\" ! trailing\n\
             \tINDEX_AREA 1\n\tLEVEL1_INDEX_AREA 1\n\tDATA_FILL 50\n\tINDEX_FILL 256\n\
             KEY 1\n\tSEG0_LENGTH 4\n\tSEG0_POSITION 36\n\tTYPE dbin4\n\tDUPLICATES yes\n\tNULL_KEY yes\n\tNULL_VALUE ' '\n",
        )
        .unwrap();
        assert_eq!(f.org, 2);
        assert_eq!((f.spec.rfm, f.spec.mrs), (rfm::FIX, 40));
        assert_eq!(f.spec.areas, [AreaSpec { bktsz: 2, alloc: 0, deq: 0 }, AreaSpec { bktsz: 3, alloc: 30, deq: 6 }]);
        let k = &f.spec.keys[0];
        assert_eq!((k.name.as_slice(), k.data_fill, k.index_fill), (&b"NUM\"BER"[..], 512, 256));
        assert!(k.key_compr && k.rec_compr && k.idx_compr);
        let k = &f.spec.keys[1];
        assert_eq!((k.datatype, k.dups, k.null_key, k.null_value), (dtype::BIN4 | dtype::DESCENDING, true, true, b' '));
        assert!(!k.key_compr && !k.rec_compr);
        assert!(parse("KEY 1\n\tSEG0_LENGTH 4\n").is_err());
        assert!(parse("FILE\n\tORGANIZATION heap\n").is_err());
    }
}
