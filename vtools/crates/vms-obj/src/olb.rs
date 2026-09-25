//! Object libraries (OLB), laid out as OpenVMS Alpha object libraries, library
//! format 3.0: a header block, two index B-trees (module names and global
//! symbols), then each module's records in a chain of data blocks. See
//! `docs/library-format.md`.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::Error;
use crate::exe::BLOCK;
use crate::record::{Field, Reader, ascic, from_ascic, record};

/// `LHD$B_TYPE` of a library of EOBJ object modules (`LBR$C_TYP_EOBJ`).
pub const TYP_EOBJ: u8 = 7;
/// `LHD$L_SANEID` of library format 3.
const SANEID3: u32 = 233_579_905;
const MAJORID: u16 = 3;
/// The longest key: module or symbol name.
pub const MAX_KEY: usize = 128;
/// Index descriptor flags: ASCII keys of variable length, stored and compared
/// as they are (`IDD$M_ASCII`, `VARLENIDX`, `NOCASECMP`, `NOCASENTR`).
const IDD_FLAGS: u16 = 1 | 4 | 8 | 16;
/// Index blocks: a used byte count, the parent's block number, 6 bytes of fill,
/// then 500 bytes of entries.
const INDEX_KEYS: usize = 12;
const INDEX_SPACE: usize = 500;
/// The RFA offset of an index entry that points at another index block.
const RFA_INDEX: u16 = 0xffff;
/// Data blocks: a record count, fill, the next block's number, then data.
const DATA_DATA: usize = 6;
const MHD_ID: u8 = 0xad;
/// The record that ends a module's data.
const EOT: [u8; 3] = [0x77, 0, 0x77];
/// Index B-trees deeper than this are rejected; 3 or more keys fit a block.
const MAX_DEPTH: usize = 32;

record! {
    /// Library header (`LHD$`), at the start of block 1. The free block lists,
    /// update history and compression are unused.
    pub struct Lhd {
        /// `LHD$B_TYPE`: [`TYP_EOBJ`].
        pub kind: u8,
        /// Number of indexes: 2, module names and global symbols.
        pub nindex: u8,
        pub fill_1: [u8; 2],
        pub sanity: u32,
        pub majorid: u16,
        pub minorid: u16,
        /// The librarian that created the library, a counted string.
        pub lbrver: [u8; 32],
        pub credat: u64,
        pub updtim: u64,
        /// Size of a module header past its first 16 bytes.
        pub mhdusz: u8,
        /// `IDXBLKF` to `FREEBLK`.
        pub fill_2: [u8; 15],
        /// The first free block, as an RFA and as a block number.
        pub nextrfa_vbn: u32,
        pub nextrfa_offset: u16,
        pub nextvbn: u32,
        /// `FREIDXBLK`, `FREEIDX`.
        pub fill_3: [u8; 8],
        /// The last index block, preallocated and used.
        pub hipreal: u32,
        pub hiprusd: u32,
        pub idxblks: u32,
        /// Index entries: modules plus symbols.
        pub idxcnt: u32,
        pub modcnt: u32,
        pub fill_4: [u8; 2],
        pub modhdrs: u32,
        /// `IDXOVH` to the end: history and compression.
        pub fill_5: [u8; 76],
    }
}

record! {
    /// Index descriptor (`IDD$`), one per index after the library header.
    pub struct Idd {
        pub flags: u16,
        /// The longest key allowed.
        pub keylen: u16,
        /// The root block of the index, 0 if it is empty.
        pub vbn: u32,
    }
}

record! {
    /// Module header (`MHD$`), the first record of a module's data.
    pub struct Mhd {
        pub lbrflag: u8,
        pub id: u8,
        pub fill_1: [u8; 2],
        /// Index entries that point at the module: its name and its symbols.
        pub refcnt: u32,
        /// When the module was inserted.
        pub datim: u64,
        pub objstat: u8,
        /// The module's ident, a counted string.
        pub objid: [u8; 32],
    }
}

/// An object library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    /// The librarian that created it, up to 31 characters.
    pub creator: String,
    /// Creation and last update times, in VMS format.
    pub created: u64,
    pub updated: u64,
    pub modules: Vec<Module>,
}

/// An object module in a library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    /// Up to 31 characters.
    pub ident: String,
    /// Insertion time, in VMS format.
    pub inserted: u64,
    /// The global symbols the library's symbol index gives to this module.
    pub symbols: Vec<String>,
    /// The module's records, as in an object file.
    pub object: Vec<u8>,
}

/// Where a module's data starts: block number and offset in the block.
type Rfa = (u32, u16);

/// Whether `file` starts with an object library header.
pub fn is_library(file: &[u8]) -> bool {
    Lhd::parse(file).is_ok_and(|h| h.kind == TYP_EOBJ && h.sanity == SANEID3)
}

impl Library {
    /// Serializes the library, with the modules in name order.
    ///
    /// Panics if a name is longer than [`MAX_KEY`] or not ASCII, a module or
    /// symbol name appears twice, or an `object` isn't a sequence of records.
    pub fn write(&self) -> Vec<u8> {
        let mut modules: Vec<&Module> = self.modules.iter().collect();
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<(&str, usize)> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.as_str(), i))
            .collect();
        let mut symbols: Vec<(&str, usize)> = modules
            .iter()
            .enumerate()
            .flat_map(|(i, m)| m.symbols.iter().map(move |s| (s.as_str(), i)))
            .collect();
        symbols.sort();
        for keys in [&names, &symbols] {
            if let Some(w) = keys.windows(2).find(|w| w[0].0 == w[1].0) {
                panic!("{} is in the library twice", w[0].0);
            }
        }

        // The indexes come first. Their size doesn't depend on where the
        // modules are, so a trial run tells where the modules start.
        let nowhere = [(0, 0)].repeat(modules.len());
        let name_blocks = index(&names, &nowhere, 0).0.len() / BLOCK;
        let symbol_blocks = index(&symbols, &nowhere, 0).0.len() / BLOCK;
        let first_data = 2 + name_blocks + symbol_blocks;
        let mut data = Vec::new();
        let mut rfas = Vec::new();
        for m in &modules {
            let vbn = first_data + data.len() / BLOCK;
            rfas.push((vbn as u32, DATA_DATA as u16));
            m.write_data(vbn, &mut data);
        }
        let (name_index, name_root) = index(&names, &rfas, 2);
        let (symbol_index, symbol_root) = index(&symbols, &rfas, 2 + name_blocks);

        let next = (first_data + data.len() / BLOCK) as u32;
        let lhd = Lhd {
            kind: TYP_EOBJ,
            nindex: 2,
            fill_1: [0; 2],
            sanity: SANEID3,
            majorid: MAJORID,
            minorid: 0,
            lbrver: ascic(&self.creator),
            credat: self.created,
            updtim: self.updated,
            mhdusz: (Mhd::SIZE - 16) as u8,
            fill_2: [0; 15],
            nextrfa_vbn: next,
            nextrfa_offset: 0,
            nextvbn: next,
            fill_3: [0; 8],
            hipreal: (first_data - 1) as u32,
            hiprusd: (first_data - 1) as u32,
            idxblks: (name_blocks + symbol_blocks) as u32,
            idxcnt: (names.len() + symbols.len()) as u32,
            modcnt: modules.len() as u32,
            fill_4: [0; 2],
            modhdrs: modules.len() as u32,
            fill_5: [0; 76],
        };
        let mut file = Vec::new();
        lhd.write(&mut file);
        for vbn in [name_root, symbol_root] {
            let idd = Idd {
                flags: IDD_FLAGS,
                keylen: MAX_KEY as u16,
                vbn,
            };
            idd.write(&mut file);
        }
        file.resize(BLOCK, 0);
        file.extend(name_index);
        file.extend(symbol_index);
        file.extend(data);
        file
    }

    /// Parses an object library. Modules come in name order, and their
    /// symbols too. The modules' records are not checked.
    pub fn parse(file: &[u8]) -> Result<Library, Error> {
        let lhd = Lhd::parse(file)?;
        if lhd.kind != TYP_EOBJ || lhd.nindex != 2 || lhd.sanity != SANEID3 {
            return Err(Error::Invalid("library type"));
        }
        if lhd.majorid != MAJORID {
            return Err(Error::Invalid("library format"));
        }
        if usize::from(lhd.mhdusz) + 16 != Mhd::SIZE {
            return Err(Error::Invalid("module header size"));
        }
        let mut r = Reader(file.get(Lhd::SIZE..).ok_or(Error::Truncated)?);
        let (names, symbols) = (Idd::read(&mut r)?, Idd::read(&mut r)?);
        let mut seen = BTreeSet::new();
        let names = entries(file, names.vbn, &mut seen)?;
        let symbols = entries(file, symbols.vbn, &mut seen)?;

        let mut at: BTreeMap<Rfa, usize> = BTreeMap::new();
        let mut modules = Vec::new();
        for (name, rfa) in names {
            if at.insert(rfa, modules.len()).is_some() {
                return Err(Error::Invalid("module index"));
            }
            let (mhd, object) = module_data(file, rfa)?;
            modules.push(Module {
                name,
                ident: from_ascic(&mhd.objid),
                inserted: mhd.datim,
                symbols: Vec::new(),
                object,
            });
        }
        for (name, rfa) in symbols {
            let &m = at.get(&rfa).ok_or(Error::Invalid("symbol index"))?;
            modules[m].symbols.push(name);
        }
        Ok(Library {
            creator: from_ascic(&lhd.lbrver),
            created: lhd.credat,
            updated: lhd.updtim,
            modules,
        })
    }
}

impl Module {
    /// Appends the module's data blocks, the first being block `vbn`: its
    /// header, its object records, then the end marker. Each is a record: a
    /// length word, the bytes, and a pad byte if the length is odd.
    fn write_data(&self, vbn: usize, out: &mut Vec<u8>) {
        let mut stream = Vec::new();
        let mut starts = Vec::new();
        let mut record = |bytes: &[u8]| {
            starts.push(stream.len());
            (bytes.len() as u16).put(&mut stream);
            stream.extend_from_slice(bytes);
            stream.resize(stream.len().next_multiple_of(2), 0);
        };
        let mhd = Mhd {
            lbrflag: 0,
            id: MHD_ID,
            fill_1: [0; 2],
            refcnt: 1 + self.symbols.len() as u32,
            datim: self.inserted,
            objstat: 0,
            objid: ascic(&self.ident),
        };
        let mut header = Vec::new();
        mhd.write(&mut header);
        record(&header);
        let mut rest = &self.object[..];
        while !rest.is_empty() {
            let size = rest
                .get(2..4)
                .map_or(0, |s| usize::from(u16::from_le_bytes([s[0], s[1]])));
            assert!(
                (4..=rest.len()).contains(&size),
                "module {}: not a sequence of object records",
                self.name
            );
            record(&rest[..size]);
            rest = &rest[size..];
        }
        record(&EOT);

        let chunk = BLOCK - DATA_DATA;
        let blocks = stream.len().div_ceil(chunk);
        for (i, data) in stream.chunks(chunk).enumerate() {
            let range = i * chunk..(i + 1) * chunk;
            let recs = starts.iter().filter(|s| range.contains(s)).count();
            let link = if i + 1 < blocks { vbn + i + 1 } else { 0 };
            out.extend([recs as u8, 0]);
            (link as u32).put(out);
            out.extend_from_slice(data);
            out.resize(out.len().next_multiple_of(BLOCK), 0);
        }
    }
}

/// An index block being built.
struct Node<'a> {
    entries: Vec<u8>,
    parent: u32,
    last: &'a str,
}

/// Builds the B-tree index of `keys`, sorted, each naming a module in `rfas`,
/// in blocks numbered from `first`. Leaves point at modules; each block above
/// has an entry per block below, keyed by that block's last key. Returns the
/// blocks and the root's number, 0 if there are no keys.
fn index<'a>(keys: &[(&'a str, usize)], rfas: &[Rfa], first: usize) -> (Vec<u8>, u32) {
    if keys.is_empty() {
        return (Vec::new(), 0);
    }
    let mut nodes: Vec<Node<'a>> = Vec::new();
    let mut level: Vec<(Rfa, &'a str, Option<usize>)> =
        keys.iter().map(|&(k, m)| (rfas[m], k, None)).collect();
    loop {
        let start = nodes.len();
        for ((vbn, offset), key, child) in level {
            assert!(key.len() <= MAX_KEY && key.is_ascii(), "bad key {key}");
            let size = 7 + key.len();
            if nodes.len() == start || nodes[nodes.len() - 1].entries.len() + size > INDEX_SPACE {
                nodes.push(Node {
                    entries: Vec::new(),
                    parent: 0,
                    last: key,
                });
            }
            let at = nodes.len() - 1;
            let node = &mut nodes[at];
            vbn.put(&mut node.entries);
            offset.put(&mut node.entries);
            (key.len() as u8).put(&mut node.entries);
            node.entries.extend_from_slice(key.as_bytes());
            node.last = key;
            if let Some(c) = child {
                nodes[c].parent = (first + at) as u32;
            }
        }
        if nodes.len() - start == 1 {
            break;
        }
        level = (start..nodes.len())
            .map(|i| (((first + i) as u32, RFA_INDEX), nodes[i].last, Some(i)))
            .collect();
    }
    let mut out = Vec::new();
    for n in &nodes {
        (n.entries.len() as u16).put(&mut out);
        n.parent.put(&mut out);
        out.extend([0; INDEX_KEYS - 6]);
        out.extend_from_slice(&n.entries);
        out.resize(out.len().next_multiple_of(BLOCK), 0);
    }
    (out, (first + nodes.len() - 1) as u32)
}

/// The keys of the index rooted at block `vbn`, with the RFAs they point at.
/// `seen` holds the index blocks read so far: each may be read once.
fn entries(file: &[u8], vbn: u32, seen: &mut BTreeSet<u32>) -> Result<Vec<(String, Rfa)>, Error> {
    let mut out = Vec::new();
    if vbn != 0 {
        walk(file, vbn, 0, seen, &mut out)?;
    }
    Ok(out)
}

fn walk(
    file: &[u8],
    vbn: u32,
    depth: usize,
    seen: &mut BTreeSet<u32>,
    out: &mut Vec<(String, Rfa)>,
) -> Result<(), Error> {
    if depth > MAX_DEPTH || !seen.insert(vbn) {
        return Err(Error::Invalid("index"));
    }
    let b = block(file, vbn)?;
    let used = usize::from(u16::from_le_bytes([b[0], b[1]]));
    if used > INDEX_SPACE {
        return Err(Error::Invalid("index block"));
    }
    let mut r = Reader(&b[INDEX_KEYS..INDEX_KEYS + used]);
    while !r.0.is_empty() {
        let rfa = (u32::read(&mut r)?, u16::read(&mut r)?);
        let n = u8::read(&mut r)?;
        let key = r.take(n.into())?;
        if rfa.1 == RFA_INDEX {
            walk(file, rfa.0, depth + 1, seen, out)?;
        } else if key.is_ascii() {
            out.push((key.iter().map(|&c| char::from(c)).collect(), rfa));
        } else {
            return Err(Error::Invalid("index key"));
        }
    }
    Ok(())
}

/// Block `vbn` of `file`; blocks are numbered from 1.
fn block(file: &[u8], vbn: u32) -> Result<&[u8], Error> {
    let start = (vbn as usize)
        .checked_sub(1)
        .and_then(|b| b.checked_mul(BLOCK))
        .ok_or(Error::Invalid("block number"))?;
    file.get(start..start + BLOCK).ok_or(Error::Truncated)
}

/// A module's header and records, from its data starting at `rfa`.
fn module_data(file: &[u8], (vbn, offset): Rfa) -> Result<(Mhd, Vec<u8>), Error> {
    if !(DATA_DATA..BLOCK).contains(&usize::from(offset)) {
        return Err(Error::Invalid("module address"));
    }
    let mut d = Data {
        file,
        block: block(file, vbn)?,
        at: offset.into(),
        hops: 0,
    };
    let header = d.record()?;
    if header.len() != Mhd::SIZE {
        return Err(Error::Invalid("module header size"));
    }
    let mhd = Mhd::parse(&header)?;
    if mhd.id != MHD_ID {
        return Err(Error::Invalid("module header"));
    }
    let mut object = Vec::new();
    loop {
        let record = d.record()?;
        if record == EOT {
            return Ok((mhd, object));
        }
        object.extend(record);
    }
}

/// Reads through a chain of data blocks.
struct Data<'a> {
    file: &'a [u8],
    block: &'a [u8],
    at: usize,
    hops: usize,
}

impl Data<'_> {
    fn take(&mut self, mut n: usize, out: &mut Vec<u8>) -> Result<(), Error> {
        while n > 0 {
            if self.at == BLOCK {
                let link = u32::from_le_bytes(self.block[2..6].try_into().unwrap());
                self.hops += 1;
                if link == 0 || self.hops > self.file.len() / BLOCK {
                    return Err(Error::Invalid("data block chain"));
                }
                self.block = block(self.file, link)?;
                self.at = DATA_DATA;
            }
            let k = n.min(BLOCK - self.at);
            out.extend_from_slice(&self.block[self.at..self.at + k]);
            self.at += k;
            n -= k;
        }
        Ok(())
    }

    /// The next record's bytes, without the length word and pad byte.
    fn record(&mut self) -> Result<Vec<u8>, Error> {
        let mut len = Vec::new();
        self.take(2, &mut len)?;
        let n = usize::from(u16::from_le_bytes([len[0], len[1]]));
        let mut record = Vec::new();
        self.take(n.next_multiple_of(2), &mut record)?;
        record.truncate(n);
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec;

    /// An object of `n` fake records, `size` bytes each (any size from 4).
    fn object(n: usize, size: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..n {
            v.extend([11, 0, size as u8, (size >> 8) as u8]);
            v.extend((4..size).map(|j| (i + j) as u8));
        }
        v
    }

    fn sample() -> Library {
        let module = |name: &str, symbols: Vec<String>, object| Module {
            name: name.into(),
            ident: "V1.0".into(),
            inserted: 0x00a1_b2c3_d4e5_f607,
            symbols,
            object,
        };
        Library {
            creator: "vlib test".into(),
            created: 1,
            updated: 2,
            modules: vec![
                module("A", vec!["A_ONE".into(), "A_TWO".into()], object(3, 5)),
                // Records that span data blocks.
                module("B", vec!["B".into()], object(40, 301)),
                // Enough long symbols for an index four levels deep.
                module(
                    "C",
                    (0..400).map(|i| format!("C_{i:0>60}")).collect(),
                    object(1, 4),
                ),
                module("D", vec![], Vec::new()),
            ],
        }
    }

    #[test]
    fn round_trip() {
        let lib = sample();
        let bytes = lib.write();
        assert_eq!(bytes.len() % BLOCK, 0);
        let parsed = Library::parse(&bytes).unwrap();
        assert_eq!(parsed, lib);
        assert_eq!(parsed.write(), bytes);
        assert!(is_library(&bytes));
        // The symbol index's root points at index blocks, not modules.
        let root = u32::from_le_bytes(bytes[208..212].try_into().unwrap()) as usize;
        let entry = (root - 1) * BLOCK + INDEX_KEYS;
        assert_eq!(&bytes[entry + 4..entry + 6], &RFA_INDEX.to_le_bytes());

        let empty = Library {
            modules: vec![],
            ..lib
        };
        assert_eq!(Library::parse(&empty.write()).unwrap(), empty);
    }

    #[test]
    fn alpha_layout() {
        let lib = Library {
            modules: vec![sample().modules.remove(0)],
            ..sample()
        };
        let bytes = lib.write();
        let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let long = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!((bytes[0], bytes[1]), (TYP_EOBJ, 2), "LHD$B_TYPE, NINDEX");
        assert_eq!((long(4), word(8)), (SANEID3, 3), "LHD$L_SANEID, MAJORID");
        assert_eq!(&bytes[12..22], b"\x09vlib test", "LHD$T_LBRVER");
        assert_eq!(bytes[60], 33, "LHD$B_MHDUSZ");
        assert_eq!((long(106), long(110)), (3, 1), "LHD$L_IDXCNT, MODCNT");
        // Module names at block 2, symbols at 3, the module at 4.
        assert_eq!((word(196), word(198), long(200)), (0x1d, 128, 2), "IDD 1");
        assert_eq!(long(208), 3, "IDD 2");
        let entry = BLOCK + INDEX_KEYS;
        assert_eq!((long(entry), word(entry + 4)), (4, 6), "RFA of A");
        assert_eq!(&bytes[entry + 6..entry + 8], b"\x01A");
        let data = 3 * BLOCK;
        assert_eq!(long(data + 2), 0, "the module fits one block");
        assert_eq!(word(data + 6), 49, "MHD length");
        assert_eq!(bytes[data + 9], MHD_ID);
        assert_eq!(long(data + 12), 3, "MHD$L_REFCNT: name and 2 symbols");
    }

    #[test]
    fn rejects_bad_libraries() {
        let bytes = sample().write();
        let root = u32::from_le_bytes(bytes[200..204].try_into().unwrap()) as usize;
        let mut looped = bytes.clone();
        let entry = (root - 1) * BLOCK + INDEX_KEYS;
        looped[entry..entry + 4].copy_from_slice(&(root as u32).to_le_bytes());
        looped[entry + 4..entry + 6].copy_from_slice(&RFA_INDEX.to_le_bytes());
        assert_eq!(Library::parse(&looped), Err(Error::Invalid("index")));

        assert_eq!(
            Library::parse(&bytes[..bytes.len() - BLOCK]),
            Err(Error::Truncated)
        );
        let mut other = bytes.clone();
        other[0] = 1;
        assert!(!is_library(&other));
        assert_eq!(Library::parse(&other), Err(Error::Invalid("library type")));
    }
}
