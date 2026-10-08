//! Executable images (EXE), laid out as OpenVMS Alpha images: header blocks,
//! then the section contents. See `docs/image-format.md`.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::obj::{self, Gsd, Record};
use crate::record::{Field, Reader, ascic, from_ascic, record};
use crate::{ARCH_ARM64, Error};

/// Images are made of blocks of this size, as on VMS.
pub const BLOCK: usize = 512;

/// Image sections start on 64 KB boundaries, so an image loads with any ARM64
/// page size. `EIHD$L_VIRT_MEM_BLOCK_SIZE` records it as a power of two.
pub const SECTION_ALIGN: u64 = 1 << SECTION_SHIFT;
const SECTION_SHIFT: u32 = 16;

record! {
    /// Image header (`EIHD$`): the fixed part at the start of the file.
    pub struct Eihd {
        pub majorid: u32,
        pub minorid: u32,
        pub size: u32,
        pub isdoff: u32,
        pub activoff: u32,
        pub symdbgoff: u32,
        pub imgidoff: u32,
        pub patchoff: u32,
        pub iafva: u64,
        pub symvva: u64,
        pub version_array_off: u32,
        pub imgtype: u32,
        pub subtype: u32,
        pub imgiocnt: u32,
        pub iochancnt: u32,
        pub privreqs: u64,
        pub hdrblkcnt: u32,
        pub lnkflags: u32,
        pub ident: u32,
        pub sysver: u32,
        pub matchctl: u8,
        pub fill_1: [u8; 3],
        pub symvect_size: u32,
        pub virt_mem_block_size: u32,
        pub ext_fixup_off: u32,
        pub noopt_psect_off: u32,
        /// Architecture code, [`ARCH_ARM64`]. Unused fill on Alpha.
        pub arch: u32,
    }
}

impl Eihd {
    pub const K_MAJORID: u32 = 3;
    pub const K_MINORID: u32 = 0;
    /// `imgtype` of an executable image.
    pub const K_EXE: u32 = 1;
    /// `imgtype` of a shareable image, which others link against.
    pub const K_LIM: u32 = 2;
    /// `lnkflags`: the image has no transfer address.
    pub const M_LNKNOTFR: u32 = 0x2;
    /// `lnkflags`: the image may move; its fixup section says how.
    pub const M_PICIMG: u32 = 0x8;
}

record! {
    /// Image symbol table header (`EIHS$`): where the global symbol table
    /// is, for a shareable image. vaxpunk has no debugger tables.
    pub struct Eihs {
        pub majorid: u32,
        pub minorid: u32,
        pub dstvbn: u32,
        pub dstsize: u32,
        /// The first block of the GST, counting from 1.
        pub gstvbn: u32,
        /// How many records the GST has.
        pub gstsize: u32,
        pub dmtvbn: u32,
        pub dmtbytes: u32,
    }
}

impl Eihs {
    pub const K_MAJORID: u32 = 1;
    pub const K_MINORID: u32 = 1;
}

record! {
    /// A GST entry (`EGSD$C_SYMG`, `EGST$`), after the subrecord's type and
    /// size: a symbol of a shareable image's symbol vector.
    pub struct Egst {
        pub datyp: u8,
        pub temp: u8,
        pub flags: u16,
        /// The entry's offset in the symbol vector.
        pub value: u64,
        /// The procedure's entry point, as linked.
        pub lp_1: u64,
        /// Its value, as linked: the same.
        pub lp_2: u64,
        pub psindx: u32,
        pub name: String,
    }
}

/// GSD subrecord type of a GST entry.
pub const EGSD_C_SYMG: u16 = 8;

record! {
    /// Shareable image list entry (`SHL$`), in a fixup section: a shareable
    /// image the image calls, by name. The rest is the image activator's.
    pub struct Shl {
        pub baseva: u32,
        pub shlptr: u32,
        pub ident: u32,
        pub permctx: u32,
        /// Size of the entry, 64.
        pub size: u8,
        pub fill_1: [u8; 2],
        pub flags: u8,
        pub icb: u32,
        pub imgnam: [u8; 40],
    }
}

record! {
    /// Activation data (`EIHA$`): the transfer addresses.
    pub struct Eiha {
        pub size: u32,
        pub spare: u32,
        pub tfradr1: u64,
        pub tfradr2: u64,
        pub tfradr3: u64,
        pub tfradr4: u64,
        pub inishr: u64,
    }
}

record! {
    /// Image identification (`EIHI$`). The names are counted strings.
    pub struct Eihi {
        pub majorid: u32,
        pub minorid: u32,
        pub linktime: u64,
        pub imgnam: [u8; 40],
        pub imgid: [u8; 16],
        pub linkid: [u8; 16],
        pub imgbid: [u8; 16],
    }
}

impl Eihi {
    pub const K_MAJORID: u32 = 1;
    pub const K_MINORID: u32 = 2;
}

record! {
    /// Image section descriptor (`EISD$`), without the global section fields.
    pub struct Eisd {
        pub majorid: u32,
        pub minorid: u32,
        pub eisdsize: u32,
        pub secsize: u32,
        pub virt_addr: u64,
        pub flags: u32,
        pub vbn: u32,
        pub pfc: u8,
        pub matchctl: u8,
        /// `EISD$B_TYPE`: 0, a normal section.
        pub kind: u8,
        pub fill_1: u8,
    }
}

impl Eisd {
    pub const K_MAJORID: u32 = 1;
    pub const K_MINORID: u32 = 1;
    /// Global section: the descriptor continues with a name.
    pub const M_GBL: u32 = 0x0001;
    /// Copy on reference: each process gets a private copy.
    pub const M_CRF: u32 = 0x0002;
    /// Demand zero: no contents in the file.
    pub const M_DZRO: u32 = 0x0004;
    pub const M_WRT: u32 = 0x0008;
    /// The image activator fixup section.
    pub const M_FIXUPVEC: u32 = 0x0040;
    pub const M_EXE: u32 = 0x0800;
    /// The end-of-list marker: majorid, minorid and a zero size.
    const LENEND: usize = 12;
}

record! {
    /// Image activator fixup section header (`EIAF$`): the Alpha fields,
    /// then the vaxpunk extension. Offsets count from the header's start.
    pub struct Eiaf {
        pub majorid: u32,
        pub minorid: u32,
        pub iaflink: u64,
        pub fixuplnk: u64,
        /// Size of the header.
        pub size: u32,
        pub flags: u32,
        /// The image's own quadword and longword addresses.
        pub qrelfixoff: u32,
        pub lrelfixoff: u32,
        /// The quadwords that get addresses in shareable images.
        pub qdotadroff: u32,
        /// Alpha's other lists, which vaxpunk doesn't use.
        pub ldotadroff: u32,
        pub codeadroff: u32,
        pub lpfixoff: u32,
        pub chgprtoff: u32,
        /// The shareable image list, and its entries.
        pub shlstoff: u32,
        pub shrimgcnt: u32,
        pub shlextra: u32,
        pub permctx: u32,
        pub base_va: u32,
        pub lppsbfixoff: u32,
        /// vaxpunk: the lowest and highest longword address, as linked.
        pub lw_min: i32,
        pub lw_max: i32,
    }
}

/// What a loader patches to move an image, from its fixup section: where
/// the image holds its own addresses, as offsets from its lowest section
/// address, in increasing order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fixups {
    pub quad: Vec<u32>,
    pub long: Vec<u32>,
    /// The lowest and highest of the longword addresses, as linked.
    pub long_min: i32,
    pub long_max: i32,
}

/// A shareable image that an image calls, from its fixup section's
/// shareable image list, and where the image holds addresses in it, which
/// the image activator sets once it has mapped that image.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShlRef {
    /// The shareable image's name, which the image activator looks for in
    /// `SYS$SHARE:`, with `.EXE`.
    pub name: String,
    /// Each quadword to set: its offset from the image's lowest section
    /// address, and the offset in the shareable image's symbol vector of
    /// the entry that holds the address. In increasing order of offset.
    pub quad: Vec<(u32, u32)>,
}

/// A shareable image's symbol vector: the procedures other images may
/// call, each a quadword holding its address. Entry i is at `addr` + 8i.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SymbolVector {
    /// Where the vector is, as linked.
    pub addr: u64,
    /// Each entry's procedure: its name and its address, as linked.
    pub entries: Vec<(String, u64)>,
}

/// A relocation record's bitmap skips at most this many slots; past that, a
/// new record is smaller.
const MAX_GAP: u32 = 64;

/// The fixup section: the header, the quadword and longword relocation
/// records of `own`, the shareable image list and the `.ADDRESS` fixups
/// of `shl`.
pub fn write_eiaf(own: Option<&Fixups>, shl: &[ShlRef]) -> Vec<u8> {
    let mut out = vec![0; Eiaf::SIZE];
    let empty = Fixups::default();
    let f = own.unwrap_or(&empty);
    let qrelfixoff = records(&mut out, &f.quad, 8);
    let lrelfixoff = records(&mut out, &f.long, 4);
    let shlstoff = if shl.is_empty() { 0 } else { out.len() as u32 };
    for s in shl {
        let entry = Shl {
            baseva: 0,
            shlptr: 0,
            ident: 0,
            permctx: 0,
            size: Shl::SIZE as u8,
            fill_1: [0; 2],
            flags: 0,
            icb: 0,
            imgnam: ascic(&s.name),
        };
        entry.write(&mut out);
    }
    // A group for each image with fixups: the count and its index in the
    // list, then the pairs. A zero count ends them.
    let qdotadroff = if shl.iter().all(|s| s.quad.is_empty()) {
        0
    } else {
        let at = out.len() as u32;
        for (i, s) in shl.iter().enumerate().filter(|(_, s)| !s.quad.is_empty()) {
            (s.quad.len() as u32).put(&mut out);
            (i as u32).put(&mut out);
            for &(off, entry) in &s.quad {
                off.put(&mut out);
                entry.put(&mut out);
            }
        }
        out.extend([0; 8]);
        at
    };
    let eiaf = Eiaf {
        majorid: 0,
        minorid: 0,
        iaflink: 0,
        fixuplnk: 0,
        size: Eiaf::SIZE as u32,
        flags: 0,
        qrelfixoff,
        lrelfixoff,
        qdotadroff,
        ldotadroff: 0,
        codeadroff: 0,
        lpfixoff: 0,
        chgprtoff: 0,
        shlstoff,
        shrimgcnt: shl.len() as u32,
        shlextra: 0,
        permctx: 0,
        base_va: 0,
        lppsbfixoff: 0,
        lw_min: f.long_min,
        lw_max: f.long_max,
    };
    put(&mut out, 0, |v| eiaf.write(v));
    out
}

/// Parses a fixup section: the image's own fixups, if it may move, and the
/// shareable images it calls.
fn parse_eiaf(b: &[u8], movable: bool) -> Result<(Option<Fixups>, Vec<ShlRef>), Error> {
    let h = Eiaf::parse(b)?;
    if (h.size as usize) < Eiaf::SIZE {
        return Err(Error::Invalid("fixup section header size"));
    }
    let unsupported = [
        h.ldotadroff,
        h.codeadroff,
        h.lpfixoff,
        h.chgprtoff,
        h.lppsbfixoff,
    ];
    if unsupported.iter().any(|&f| f != 0) {
        return Err(Error::Invalid("fixup kind"));
    }
    let own = Fixups {
        quad: slots(b, h.qrelfixoff, 8)?,
        long: slots(b, h.lrelfixoff, 4)?,
        long_min: h.lw_min,
        long_max: h.lw_max,
    };
    if !movable && own != Fixups::default() {
        return Err(Error::Invalid("fixups in an image that can't move"));
    }
    let mut shl = Vec::new();
    if h.shrimgcnt != 0 {
        let mut r = Reader(from(b, h.shlstoff as usize)?);
        for _ in 0..h.shrimgcnt {
            let entry = Shl::read(&mut r)?;
            if usize::from(entry.size) != Shl::SIZE {
                return Err(Error::Invalid("shareable image list entry size"));
            }
            shl.push(ShlRef {
                name: from_ascic(&entry.imgnam),
                quad: Vec::new(),
            });
        }
    }
    if h.qdotadroff != 0 {
        let mut r = Reader(from(b, h.qdotadroff as usize)?);
        loop {
            let count = u32::read(&mut r)?;
            let image = u32::read(&mut r)?;
            if count == 0 {
                break;
            }
            let s = shl
                .get_mut(image as usize)
                .ok_or(Error::Invalid("shareable image index"))?;
            for _ in 0..count {
                let pair = (u32::read(&mut r)?, u32::read(&mut r)?);
                if s.quad.last().is_some_and(|l| l.0 >= pair.0) {
                    return Err(Error::Invalid("fixup order"));
                }
                s.quad.push(pair);
            }
        }
    }
    Ok((movable.then_some(own), shl))
}

impl Fixups {
    /// Steps 1 and 2 of moving an image (docs/image-format.md): checks that
    /// the image, linked with its lowest section at `linked`, may have it at
    /// `base` instead, and returns the displacement.
    pub fn displacement(&self, linked: u64, base: u64) -> Result<i64, Error> {
        if !base.is_multiple_of(SECTION_ALIGN) {
            return Err(Error::Invalid("base address, not a multiple of 64 KB"));
        }
        let d = base.wrapping_sub(linked) as i64;
        let fits = |v: i32| {
            i64::from(v)
                .checked_add(d)
                .is_some_and(|v| i32::try_from(v).is_ok())
        };
        if !self.long.is_empty() && !(fits(self.long_min) && fits(self.long_max)) {
            return Err(Error::Invalid(
                "base address, too far for the longword addresses",
            ));
        }
        Ok(d)
    }

    /// Step 4: adds `d` to each address the fixups name in `data`, which
    /// holds the image's contents from offset `at` on. Other fixups are left
    /// alone, so a loader can go one section at a time.
    pub fn apply(&self, d: i64, at: u64, data: &mut [u8]) -> Result<(), Error> {
        for (list, size) in [(&self.quad, 8), (&self.long, 4)] {
            for &off in list {
                let Some(i) = u64::from(off)
                    .checked_sub(at)
                    .filter(|&i| i < data.len() as u64)
                else {
                    continue;
                };
                let field = data
                    .get_mut(i as usize..i as usize + size)
                    .ok_or(Error::Invalid("fixup across the end of a section"))?;
                if size == 8 {
                    let v = u64::from_le_bytes(field.try_into().unwrap());
                    field.copy_from_slice(&v.wrapping_add_signed(d).to_le_bytes());
                } else {
                    let v = i32::from_le_bytes(field.try_into().unwrap());
                    let v = i64::from(v)
                        .checked_add(d)
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or(Error::Invalid("displacement for a longword address"))?;
                    field.copy_from_slice(&v.to_le_bytes());
                }
            }
        }
        Ok(())
    }
}

/// Appends the relocation records for `offsets` and returns where they
/// start, or 0 if there are none. A record is a bit count (a multiple of 32)
/// and a base, then the bits, 32 to a longword: bit i stands for the
/// `stride`-byte slot at base + i × stride. A zero count ends the list.
fn records(out: &mut Vec<u8>, offsets: &[u32], stride: u32) -> u32 {
    if offsets.is_empty() {
        return 0;
    }
    let start = out.len() as u32;
    let mut rest = offsets;
    while let Some(&base) = rest.first() {
        let n = 1 + rest
            .windows(2)
            .take_while(|w| (w[1] - base) % stride == 0 && (w[1] - w[0]) / stride <= MAX_GAP)
            .count();
        let (these, more) = rest.split_at(n);
        let bits = ((these[n - 1] - base) / stride + 1).next_multiple_of(32);
        let mut map = vec![0u32; bits as usize / 32];
        for &o in these {
            let slot = (o - base) / stride;
            map[slot as usize / 32] |= 1 << (slot % 32);
        }
        bits.put(out);
        base.put(out);
        map.iter().for_each(|w| w.put(out));
        rest = more;
    }
    out.extend([0; 8]);
    start
}

/// The offsets that the relocation records at `off` stand for.
fn slots(b: &[u8], off: u32, stride: u32) -> Result<Vec<u32>, Error> {
    let mut out: Vec<u32> = Vec::new();
    if off == 0 {
        return Ok(out);
    }
    const BAD: Error = Error::Invalid("fixup record");
    let mut r = Reader(from(b, off as usize)?);
    loop {
        let bits = u32::read(&mut r)?;
        if bits == 0 {
            return Ok(out);
        }
        let base = u32::read(&mut r)?;
        if bits % 32 != 0 {
            return Err(BAD);
        }
        for word in 0..bits / 32 {
            let map = u32::read(&mut r)?;
            for bit in (0..32).filter(|b| map & 1 << b != 0) {
                let at = (word * 32 + bit)
                    .checked_mul(stride)
                    .and_then(|s| s.checked_add(base))
                    .ok_or(BAD)?;
                if out.last().is_some_and(|&last| last >= at) {
                    return Err(Error::Invalid("fixup order"));
                }
                out.push(at);
            }
        }
    }
}

/// One image section: where it goes, its protection, and its contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Virtual address, a multiple of [`SECTION_ALIGN`].
    pub vaddr: u64,
    /// Size in bytes.
    pub size: u32,
    /// `Eisd::M_*` flags.
    pub flags: u32,
    /// Contents: `size` bytes, or none for a demand-zero section.
    pub data: Vec<u8>,
}

/// An executable or shareable image.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Image {
    /// Image name, up to 39 characters.
    pub name: String,
    /// Image ident, up to 15 characters.
    pub ident: String,
    /// Link time in VMS format: 100 ns units since 17-Nov-1858.
    pub link_time: u64,
    /// Entry point, the first transfer address; 0 for none.
    pub transfer: u64,
    /// The sections, without the fixup section.
    pub sections: Vec<Section>,
    /// What a loader patches to move the image; None if it can't move.
    pub fixups: Option<Fixups>,
    /// The shareable images it calls, in the order of their indexes.
    pub shareables: Vec<ShlRef>,
    /// For a shareable image, its symbol vector.
    pub vector: Option<SymbolVector>,
}

impl Image {
    /// Serializes the image. The fixup section, if any, goes after the other
    /// sections.
    ///
    /// Panics if a name is too long, or a section's `data` isn't `size` bytes
    /// (empty for demand-zero sections).
    pub fn write(&self) -> Vec<u8> {
        let activoff = Eihd::SIZE.next_multiple_of(8);
        let imgidoff = activoff + Eiha::SIZE;
        let symdbgoff = imgidoff + Eihi::SIZE;
        let isdoff = symdbgoff + if self.vector.is_some() { Eihs::SIZE } else { 0 };

        let eiaf = self.fixups.is_some() || !self.shareables.is_empty();
        let fixups = eiaf.then(|| {
            let data = write_eiaf(self.fixups.as_ref(), &self.shareables);
            let end = self.sections.iter().map(|s| s.vaddr + u64::from(s.size));
            // Never at 0, which would say that there is none.
            let end = end.max().unwrap_or(0).max(1);
            Section {
                vaddr: end.next_multiple_of(SECTION_ALIGN),
                size: data.len() as u32,
                flags: Eisd::M_FIXUPVEC,
                data,
            }
        });
        let sections: Vec<&Section> = self.sections.iter().chain(&fixups).collect();

        // A descriptor never straddles a block, and the last word of block 0
        // is the alias. Unused header bytes are 0xFF, which a reader takes as
        // "continue in the next block".
        let mut at = isdoff;
        let mut eisd_at = Vec::new();
        for _ in &sections {
            let room = BLOCK - at % BLOCK - if at < BLOCK { 2 } else { 0 };
            if room < Eisd::SIZE + Eisd::LENEND {
                at = at.next_multiple_of(BLOCK);
            }
            eisd_at.push(at);
            at += Eisd::SIZE;
        }
        let hdr_size = at + Eisd::LENEND;
        let hdr_blocks = hdr_size.div_ceil(BLOCK);
        let mut file = vec![0xff; hdr_blocks * BLOCK];

        let eihd = Eihd {
            majorid: Eihd::K_MAJORID,
            minorid: Eihd::K_MINORID,
            size: hdr_size as u32,
            isdoff: isdoff as u32,
            activoff: activoff as u32,
            symdbgoff: if self.vector.is_some() {
                symdbgoff as u32
            } else {
                0
            },
            imgidoff: imgidoff as u32,
            patchoff: 0,
            iafva: fixups.as_ref().map_or(0, |s| s.vaddr),
            symvva: self.vector.as_ref().map_or(0, |v| v.addr),
            version_array_off: 0,
            imgtype: if self.vector.is_some() {
                Eihd::K_LIM
            } else {
                Eihd::K_EXE
            },
            subtype: 0,
            imgiocnt: 0,
            iochancnt: 0,
            privreqs: u64::MAX,
            hdrblkcnt: hdr_blocks as u32,
            lnkflags: if self.transfer == 0 {
                Eihd::M_LNKNOTFR
            } else {
                0
            } | if self.fixups.is_some() {
                Eihd::M_PICIMG
            } else {
                0
            },
            ident: 0,
            sysver: 0,
            matchctl: 0,
            fill_1: [0; 3],
            symvect_size: self.vector.as_ref().map_or(0, |v| v.entries.len() as u32),
            virt_mem_block_size: SECTION_SHIFT,
            ext_fixup_off: 0,
            noopt_psect_off: 0,
            arch: ARCH_ARM64,
        };
        put(&mut file, 0, |v| eihd.write(v));
        let eiha = Eiha {
            size: Eiha::SIZE as u32,
            spare: 0,
            tfradr1: self.transfer,
            tfradr2: 0,
            tfradr3: 0,
            tfradr4: 0,
            inishr: 0,
        };
        put(&mut file, activoff, |v| eiha.write(v));
        let eihi = Eihi {
            majorid: Eihi::K_MAJORID,
            minorid: Eihi::K_MINORID,
            linktime: self.link_time,
            imgnam: ascic(&self.name),
            imgid: ascic(&self.ident),
            linkid: ascic(""),
            imgbid: ascic(""),
        };
        put(&mut file, imgidoff, |v| eihi.write(v));
        put(&mut file, at, |v| v.extend([0; Eisd::LENEND]));

        for (s, &at) in sections.iter().zip(&eisd_at) {
            let vbn = if s.flags & Eisd::M_DZRO != 0 {
                assert!(s.data.is_empty(), "demand-zero section with contents");
                0
            } else {
                assert_eq!(s.data.len(), s.size as usize, "section contents");
                let vbn = file.len() / BLOCK + 1;
                file.extend_from_slice(&s.data);
                file.resize(file.len().next_multiple_of(BLOCK), 0);
                vbn
            };
            let eisd = Eisd {
                majorid: Eisd::K_MAJORID,
                minorid: Eisd::K_MINORID,
                eisdsize: Eisd::SIZE as u32,
                secsize: s.size,
                virt_addr: s.vaddr,
                flags: s.flags,
                vbn: vbn as u32,
                pfc: 0,
                matchctl: 0,
                kind: 0,
                fill_1: 0,
            };
            put(&mut file, at, |v| eisd.write(v));
        }
        if let Some(vector) = &self.vector {
            let gst = gst(&self.name, &self.ident, vector);
            let eihs = Eihs {
                majorid: Eihs::K_MAJORID,
                minorid: Eihs::K_MINORID,
                dstvbn: 0,
                dstsize: 0,
                gstvbn: (file.len() / BLOCK + 1) as u32,
                gstsize: gst.len() as u32,
                dmtvbn: 0,
                dmtbytes: 0,
            };
            put(&mut file, symdbgoff, |v| eihs.write(v));
            file.extend(obj::write(&gst));
            file.resize(file.len().next_multiple_of(BLOCK), 0);
        }
        file
    }

    /// Parses an image and checks what vaxpunk requires of it: ARM64, no
    /// global sections, no writable code, sections 64 KB aligned.
    pub fn parse(file: &[u8]) -> Result<Image, Error> {
        let eihd = Eihd::parse(file)?;
        if (eihd.majorid, eihd.minorid) != (Eihd::K_MAJORID, Eihd::K_MINORID) {
            return Err(Error::Invalid("image header version"));
        }
        if eihd.arch != ARCH_ARM64 {
            return Err(Error::Invalid("architecture"));
        }
        let hdr = file.get(..eihd.size as usize).ok_or(Error::Truncated)?;
        let eiha = Eiha::parse(from(hdr, eihd.activoff as usize)?)?;
        let eihi = Eihi::parse(from(hdr, eihd.imgidoff as usize)?)?;

        let mut sections = Vec::new();
        let mut at = eihd.isdoff as usize;
        loop {
            match u32_at(hdr, at + 8)? {
                0 => break,
                u32::MAX => {
                    at = (at + BLOCK) & !(BLOCK - 1);
                    continue;
                }
                _ => {}
            }
            let eisd = Eisd::parse(from(hdr, at)?)?;
            if (eisd.eisdsize as usize) < Eisd::SIZE {
                return Err(Error::Invalid("section descriptor size"));
            }
            at += eisd.eisdsize as usize;
            if eisd.flags & Eisd::M_GBL != 0 {
                return Err(Error::Invalid("global section"));
            }
            if eisd.flags & Eisd::M_EXE != 0 && eisd.flags & Eisd::M_WRT != 0 {
                return Err(Error::Invalid("writable code section"));
            }
            if eisd.virt_addr % SECTION_ALIGN != 0 {
                return Err(Error::Invalid("section alignment"));
            }
            let data = if eisd.flags & Eisd::M_DZRO != 0 {
                Vec::new()
            } else {
                let block = (eisd.vbn as usize)
                    .checked_sub(1)
                    .ok_or(Error::Invalid("section block number"))?;
                let start = block * BLOCK;
                file.get(start..start + eisd.secsize as usize)
                    .ok_or(Error::Truncated)?
                    .to_vec()
            };
            sections.push(Section {
                vaddr: eisd.virt_addr,
                size: eisd.secsize,
                flags: eisd.flags,
                data,
            });
        }

        let movable = eihd.lnkflags & Eihd::M_PICIMG != 0;
        let (fixups, shareables) = match eihd.iafva {
            0 if movable => return Err(Error::Invalid("movable image without fixups")),
            0 => (None, Vec::new()),
            va => {
                let i = sections
                    .iter()
                    .position(|s| s.vaddr == va)
                    .ok_or(Error::Invalid("fixup section address"))?;
                let (f, shl) = parse_eiaf(&sections.remove(i).data, movable)?;
                let slots = shl.iter().flat_map(|s| s.quad.iter().map(|q| q.0));
                let images = Fixups {
                    quad: slots.collect(),
                    ..Fixups::default()
                };
                if !f.iter().chain([&images]).all(|f| within(f, &sections)) {
                    return Err(Error::Invalid("fixup outside the image's contents"));
                }
                (f, shl)
            }
        };
        let vector = match eihd.imgtype {
            Eihd::K_EXE => None,
            Eihd::K_LIM => {
                let eihs = Eihs::parse(from(hdr, eihd.symdbgoff as usize)?)?;
                let block = (eihs.gstvbn as usize)
                    .checked_sub(1)
                    .ok_or(Error::Invalid("GST block number"))?;
                let entries = parse_gst(from(file, block * BLOCK)?, eihs.gstsize)?;
                if entries.len() != eihd.symvect_size as usize {
                    return Err(Error::Invalid("symbol vector size"));
                }
                Some(SymbolVector {
                    addr: eihd.symvva,
                    entries,
                })
            }
            _ => return Err(Error::Invalid("image type")),
        };

        Ok(Image {
            name: from_ascic(&eihi.imgnam),
            ident: from_ascic(&eihi.imgid),
            link_time: eihi.linktime,
            transfer: eiha.tfradr1,
            sections,
            fixups,
            shareables,
            vector,
        })
    }
}

/// A shareable image's GST: a module whose global symbol directory has an
/// entry for each procedure of the symbol vector, as the Alpha linker
/// writes it.
fn gst(name: &str, ident: &str, vector: &SymbolVector) -> Vec<Record> {
    let abs = Gsd::Psc(obj::Psc {
        align: 0,
        temp: 0,
        flags: obj::psc::PIC | obj::psc::LIB | obj::psc::RD,
        alloc: 0,
        name: ".$$ABS$$.".into(),
    });
    let entries: Vec<Gsd> = (vector.entries.iter().enumerate())
        .map(|(i, (name, addr))| {
            let mut data = Vec::new();
            Egst {
                datyp: 0,
                temp: 0,
                flags: obj::sym::DEF | obj::sym::UNI | obj::sym::REL | obj::sym::NORM,
                value: 8 * i as u64,
                lp_1: *addr,
                lp_2: *addr,
                psindx: 0,
                name: name.clone(),
            }
            .write(&mut data);
            Gsd::Other {
                gsdtyp: EGSD_C_SYMG,
                data,
            }
        })
        .collect();
    let mut records = vec![
        Record::Mhd(obj::Mhd {
            strlvl: obj::STRLVL,
            temp: 0,
            arch1: ARCH_ARM64,
            arch2: 0,
            recsiz: obj::MAX_RECORD as u32,
            name: name.into(),
            version: ident.into(),
            date: [b' '; 17],
        }),
        Record::Gsd(vec![abs]),
    ];
    // An entry is at most 80 bytes: 100 to a record fit in its 8 KB.
    records.extend(entries.chunks(100).map(|c| Record::Gsd(c.to_vec())));
    records.push(Record::Eom(
        obj::Eom {
            total_lps: 0,
            comcod: 0,
        },
        None,
    ));
    records
}

/// The symbol vector's entries, from a GST of `count` records at the
/// start of `b`. Every entry is a procedure's, and entry i is at 8i.
fn parse_gst(b: &[u8], count: u32) -> Result<Vec<(String, u64)>, Error> {
    // The records' own sizes say where the GST ends.
    let mut r = Reader(b);
    let mut len = 0;
    for _ in 0..count {
        let _rectyp = u16::read(&mut r)?;
        let size = u16::read(&mut r)? as usize;
        r.take(
            size.checked_sub(4)
                .ok_or(Error::Invalid("GST record size"))?,
        )?;
        len += size;
    }
    let mut entries = Vec::new();
    for record in obj::parse(&b[..len])? {
        let Record::Gsd(subrecords) = record else {
            continue;
        };
        for sub in subrecords {
            let Gsd::Other {
                gsdtyp: EGSD_C_SYMG,
                data,
            } = sub
            else {
                continue;
            };
            let e = Egst::parse(&data)?;
            if e.flags & obj::sym::NORM == 0 {
                return Err(Error::Invalid("symbol vector entry that isn't a procedure"));
            }
            if e.value != 8 * entries.len() as u64 {
                return Err(Error::Invalid("symbol vector entry order"));
            }
            entries.push((e.name, e.lp_1));
        }
    }
    Ok(entries)
}

/// Whether each fixup names bytes that a section holds in the file.
fn within(f: &Fixups, sections: &[Section]) -> bool {
    let origin = sections.iter().map(|s| s.vaddr).min().unwrap_or(0);
    let held = |off: u32, size: u64| {
        let Some(at) = origin.checked_add(u64::from(off)) else {
            return false;
        };
        sections.iter().any(|s| {
            // Offsets in the section, which can end at the top of memory.
            let inside = at.checked_sub(s.vaddr).and_then(|i| i.checked_add(size));
            s.flags & Eisd::M_DZRO == 0 && inside.is_some_and(|end| end <= u64::from(s.size))
        })
    };
    f.quad.iter().all(|&o| held(o, 8)) && f.long.iter().all(|&o| held(o, 4))
}

fn from(b: &[u8], at: usize) -> Result<&[u8], Error> {
    b.get(at..).ok_or(Error::Truncated)
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    u32::read(&mut Reader(from(b, at)?))
}

/// Overwrites `file` at `at` with what `write` produces.
fn put(file: &mut [u8], at: usize, write: impl FnOnce(&mut Vec<u8>)) {
    let mut v = Vec::new();
    write(&mut v);
    file[at..at + v.len()].copy_from_slice(&v);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(sections: usize) -> Image {
        let sections = (0..sections)
            .map(|i| {
                let vaddr = SECTION_ALIGN * (i as u64 + 1);
                match i % 3 {
                    0 => Section {
                        vaddr,
                        size: 8,
                        flags: Eisd::M_EXE,
                        // mov x0, #1; ret
                        data: vec![0x20, 0x00, 0x80, 0xd2, 0xc0, 0x03, 0x5f, 0xd6],
                    },
                    1 => Section {
                        vaddr,
                        size: 3,
                        flags: Eisd::M_WRT | Eisd::M_CRF,
                        data: vec![1, 2, 3],
                    },
                    _ => Section {
                        vaddr,
                        size: 0x2000,
                        flags: Eisd::M_WRT | Eisd::M_DZRO,
                        data: vec![],
                    },
                }
            })
            .collect();
        Image {
            name: "TINY".into(),
            ident: "V1.0".into(),
            link_time: 0x00a1_b2c3_d4e5_f607,
            transfer: SECTION_ALIGN,
            sections,
            ..Image::default()
        }
    }

    /// Where `movable` has addresses: quadwords in two records of aligned
    /// slots, far apart, then one off their alignment; two longwords.
    const QUAD: [u32; 5] = [0x10000, 0x10008, 0x10018, 0x10800, 0x10814];
    const LONG: [u32; 2] = [0x10020, 0x10024];

    /// `sample(3)` with 4 KB of data holding addresses, 64 KB above the
    /// code: offset 0x10000.
    fn movable() -> Image {
        let mut image = sample(3);
        let data = &mut image.sections[1];
        data.size = 0x1000;
        data.data = vec![0; 0x1000];
        for o in QUAD {
            let at = (o - 0x10000) as usize;
            data.data[at..at + 8].copy_from_slice(&(0x1_0000_0000 + u64::from(o)).to_le_bytes());
        }
        for (o, v) in LONG.into_iter().zip([0x20008i32, 0x20010]) {
            let at = (o - 0x10000) as usize;
            data.data[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        image.fixups = Some(Fixups {
            quad: QUAD.to_vec(),
            long: LONG.to_vec(),
            long_min: 0x20008,
            long_max: 0x20010,
        });
        image
    }

    #[test]
    fn round_trip() {
        // 40 descriptors don't fit in the first header block.
        let images = [0, 1, 3, 40].map(sample);
        for image in images.into_iter().chain([movable()]) {
            let bytes = image.write();
            assert_eq!(bytes.len() % BLOCK, 0);
            let parsed = Image::parse(&bytes).unwrap();
            assert_eq!(parsed, image);
            assert_eq!(parsed.write(), bytes);
        }
    }

    #[test]
    fn fixup_section_layout() {
        let mut bytes = movable().write();
        let word = |b: &[u8], at: usize| u32_at(b, at).unwrap();
        // The fourth section, after the demand-zero one that ends at 0x32000.
        let eisd = word(&bytes, 12) as usize + 3 * Eisd::SIZE;
        assert_eq!(word(&bytes, 32), 0x40000, "EIHD$Q_IAFVA");
        assert_eq!(word(&bytes, eisd + 16), 0x40000, "EISD$Q_VIRT_ADDR");
        assert_eq!(word(&bytes, eisd + 24), Eisd::M_FIXUPVEC, "EISD$L_FLAGS");
        let fix = (word(&bytes, eisd + 28) as usize - 1) * BLOCK;
        let eiaf = |at: usize| word(&bytes, fix + at);
        assert_eq!(
            eiaf(24),
            92,
            "EIAF$L_SIZE: 84 Alpha bytes, then the vaxpunk ones"
        );
        assert_eq!(eiaf(32), 92, "EIAF$L_QRELFIXOFF");
        assert_eq!((eiaf(84), eiaf(88)), (0x20008, 0x20010), "longword range");
        let records = [
            32, 0x10000, 0b1011, // slots 0, 1 and 3 of the first record
            32, 0x10800, 1, // 253 slots on
            32, 0x10814, 1, // not a multiple of 8 from 0x10800
            0, 0,
        ];
        let got: Vec<u32> = (0..records.len()).map(|i| eiaf(92 + 4 * i)).collect();
        assert_eq!(got, records);
        assert_eq!(eiaf(36), 92 + 4 * records.len() as u32, "EIAF$L_LRELFIXOFF");

        bytes[fix + 44] = 1; // EIAF$L_LDOTADROFF
        let err = Image::parse(&bytes);
        assert_eq!(err, Err(Error::Invalid("fixup kind")));
    }

    /// `movable()` as a shareable image with two procedures in its vector,
    /// and an image that can't move calling it, through a quadword in its
    /// data section.
    #[test]
    fn shareable_images() {
        let mut shr = movable();
        shr.transfer = 0;
        shr.vector = Some(SymbolVector {
            addr: 0x20000,
            entries: vec![("LIB$ONE".into(), 0x10000), ("LIB$TWO".into(), 0x10004)],
        });
        let bytes = shr.write();
        let word = |b: &[u8], at: usize| u32_at(b, at).unwrap();
        assert_eq!(word(&bytes, 52), Eihd::K_LIM, "EIHD$L_IMGTYPE");
        assert_eq!(word(&bytes, 80), Eihd::M_LNKNOTFR | Eihd::M_PICIMG);
        assert_eq!(word(&bytes, 40), 0x20000, "EIHD$Q_SYMVVA");
        assert_eq!(word(&bytes, 96), 2, "EIHD$L_SYMVECT_SIZE");
        let eihs = word(&bytes, 20) as usize;
        assert_eq!(word(&bytes, eihs + 20), 4, "GST records: MHD, 2 EGSD, EEOM");
        assert_eq!(Image::parse(&bytes), Ok(shr.clone()));

        let mut main = movable();
        main.fixups = None;
        main.shareables = vec![
            ShlRef {
                name: "UNUSED".into(),
                quad: vec![],
            },
            ShlRef {
                name: "LIBRTL".into(),
                quad: vec![(0x10000, 8)],
            },
        ];
        let bytes = main.write();
        assert_eq!(word(&bytes, 80) & Eihd::M_PICIMG, 0, "can't move");
        assert_eq!(Image::parse(&bytes), Ok(main.clone()));
        let fix = (word(&bytes, 12) as usize + 3 * Eisd::SIZE + 28, 32);
        let fix = (word(&bytes, fix.0) as usize - 1) * BLOCK;
        let eiaf = |at: usize| word(&bytes, fix + at);
        assert_eq!((eiaf(32), eiaf(36)), (0, 0), "no relocation fixups");
        assert_eq!((eiaf(60), eiaf(64)), (92, 2), "SHLSTOFF, SHRIMGCNT");
        assert_eq!(
            bytes[fix + 92 + 64 + 24..fix + 92 + 64 + 31],
            *b"\x06LIBRTL"
        );
        let groups: Vec<u32> = (0..6).map(|i| eiaf(eiaf(40) as usize + 4 * i)).collect();
        assert_eq!(groups, [1, 1, 0x10000, 8, 0, 0], "QDOTADROFF's group");

        // A slot in the demand-zero section.
        main.shareables[1].quad[0].0 = 0x20000;
        let err = Image::parse(&main.write());
        assert_eq!(
            err,
            Err(Error::Invalid("fixup outside the image's contents"))
        );
    }

    #[test]
    fn moves() {
        let mut image = movable();
        let f = image.fixups.take().unwrap();
        let bad = |base| f.displacement(0x10000, base).is_err();
        assert!(bad(0x7ffd_1000), "not 64 KB aligned");
        assert!(bad(0x7fff_0000), "0x20010 would move past 2 GB");
        assert!(bad(0xffff_ffff_7ffe_0000), "0x20008 would move below -2 GB");
        let d = f.displacement(0x10000, 0x7ffe_0000).unwrap();
        assert_eq!(d, 0x7ffd_0000);
        for s in &mut image.sections {
            f.apply(d, s.vaddr - 0x10000, &mut s.data).unwrap();
        }
        let data = &image.sections[1].data;
        let quad = |at: usize| u64::from_le_bytes(data[at..at + 8].try_into().unwrap());
        let long = |at: usize| i32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        assert_eq!(quad(0x18), 0x1_0001_0018 + 0x7ffd_0000);
        assert_eq!((long(0x20), long(0x24)), (0x7fff_0008, 0x7fff_0010));
        assert_eq!(
            &image.sections[0].data[..4],
            &[0x20, 0, 0x80, 0xd2],
            "code untouched"
        );

        // A loader that skips the check still can't wrap a longword.
        let mut data = data.clone();
        let err = f.apply(0x10000, 0x10000, &mut data);
        assert_eq!(
            err,
            Err(Error::Invalid("displacement for a longword address"))
        );
    }

    #[test]
    fn rejects_fixups_outside_the_contents() {
        let mut image = sample(3);
        // Offset 0x20000 is in the demand-zero section.
        image.fixups = Some(Fixups {
            quad: vec![0x20000],
            ..Fixups::default()
        });
        let err = Image::parse(&image.write());
        assert_eq!(
            err,
            Err(Error::Invalid("fixup outside the image's contents"))
        );

        // A section at the top of memory, with a quadword at `at`.
        let top = |size: u32, at: u32| {
            let image = Image {
                sections: vec![Section {
                    vaddr: SECTION_ALIGN,
                    size,
                    flags: Eisd::M_WRT,
                    data: vec![0; size as usize],
                }],
                fixups: Some(Fixups {
                    quad: vec![at],
                    ..Fixups::default()
                }),
                ..sample(0)
            };
            let mut bytes = image.write();
            let eisd = u32_at(&bytes, 12).unwrap() as usize;
            bytes[eisd + 16..eisd + 24].copy_from_slice(&0xffff_ffff_ffff_0000u64.to_le_bytes());
            Image::parse(&bytes)
        };
        assert!(top(0xfffe, 0xfffc).is_err(), "past the end of the section");
        assert!(top(0x10000, 0xfff8).is_ok(), "up to the end of memory");
    }

    #[test]
    fn fixups_without_sections() {
        let image = Image {
            fixups: Some(Fixups::default()),
            ..sample(0)
        };
        let bytes = image.write();
        assert_eq!(u32_at(&bytes, 32), Ok(0x10000), "IAFVA can't be 0");
        assert_eq!(Image::parse(&bytes), Ok(image));
    }

    #[test]
    fn alpha_layout() {
        let bytes = sample(1).write();
        let word = |at: usize| u32_at(&bytes, at).unwrap();
        assert_eq!((word(0), word(4)), (3, 0), "EIHD majorid, minorid");
        assert_eq!(word(52), Eihd::K_EXE, "EIHD$L_IMGTYPE");
        assert_eq!(word(76), 1, "EIHD$L_HDRBLKCNT");
        assert_eq!(word(100), 16, "EIHD$L_VIRT_MEM_BLOCK_SIZE");
        assert_eq!(word(112), ARCH_ARM64, "EIHD$L_ARCH");
        assert_eq!(&bytes[510..512], &[0xff, 0xff], "EIHD$W_ALIAS");
        let isd = word(12) as usize;
        assert_eq!(word(isd + 28), 2, "the section's contents start in block 2");
        assert_eq!(&bytes[512..516], &[0x20, 0x00, 0x80, 0xd2]);
    }

    #[test]
    fn rejects_bad_images() {
        let mut image = sample(1);
        image.sections[0].flags |= Eisd::M_WRT;
        let err = Image::parse(&image.write());
        assert_eq!(err, Err(Error::Invalid("writable code section")));

        let mut bytes = sample(1).write();
        bytes[112] = 0;
        assert_eq!(Image::parse(&bytes), Err(Error::Invalid("architecture")));

        let bytes = sample(1).write();
        assert_eq!(Image::parse(&bytes[..516]), Err(Error::Truncated));
    }
}
