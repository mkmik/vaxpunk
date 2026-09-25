//! Executable images (EXE), laid out as OpenVMS Alpha images: header blocks,
//! then the section contents. See `docs/image-format.md`.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::record::{Field, record};
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
    /// `lnkflags`: the image has no transfer address.
    pub const M_LNKNOTFR: u32 = 0x2;
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
    pub const M_EXE: u32 = 0x0800;
    /// The end-of-list marker: majorid, minorid and a zero size.
    const LENEND: usize = 12;
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

/// An executable image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// Image name, up to 39 characters.
    pub name: String,
    /// Image ident, up to 15 characters.
    pub ident: String,
    /// Link time in VMS format: 100 ns units since 17-Nov-1858.
    pub link_time: u64,
    /// Entry point, the first transfer address; 0 for none.
    pub transfer: u64,
    pub sections: Vec<Section>,
}

impl Image {
    /// Serializes the image.
    ///
    /// Panics if a name is too long, or a section's `data` isn't `size` bytes
    /// (empty for demand-zero sections).
    pub fn write(&self) -> Vec<u8> {
        let activoff = Eihd::SIZE.next_multiple_of(8);
        let imgidoff = activoff + Eiha::SIZE;
        let isdoff = imgidoff + Eihi::SIZE;

        // A descriptor never straddles a block, and the last word of block 0
        // is the alias. Unused header bytes are 0xFF, which a reader takes as
        // "continue in the next block".
        let mut at = isdoff;
        let mut eisd_at = Vec::new();
        for _ in &self.sections {
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
            symdbgoff: 0,
            imgidoff: imgidoff as u32,
            patchoff: 0,
            iafva: 0,
            symvva: 0,
            version_array_off: 0,
            imgtype: Eihd::K_EXE,
            subtype: 0,
            imgiocnt: 0,
            iochancnt: 0,
            privreqs: u64::MAX,
            hdrblkcnt: hdr_blocks as u32,
            lnkflags: if self.transfer == 0 {
                Eihd::M_LNKNOTFR
            } else {
                0
            },
            ident: 0,
            sysver: 0,
            matchctl: 0,
            fill_1: [0; 3],
            symvect_size: 0,
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

        for (s, &at) in self.sections.iter().zip(&eisd_at) {
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
            let size = from(hdr, at + 8)?.get(..4).ok_or(Error::Truncated)?;
            match u32::get(size) {
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

        Ok(Image {
            name: from_ascic(&eihi.imgnam),
            ident: from_ascic(&eihi.imgid),
            link_time: eihi.linktime,
            transfer: eiha.tfradr1,
            sections,
        })
    }
}

fn from(b: &[u8], at: usize) -> Result<&[u8], Error> {
    b.get(at..).ok_or(Error::Truncated)
}

/// Overwrites `file` at `at` with what `write` produces.
fn put(file: &mut [u8], at: usize, write: impl FnOnce(&mut Vec<u8>)) {
    let mut v = Vec::new();
    write(&mut v);
    file[at..at + v.len()].copy_from_slice(&v);
}

/// A counted string in an N-byte field.
fn ascic<const N: usize>(s: &str) -> [u8; N] {
    assert!(s.len() < N, "name too long: {s}");
    let mut b = [0; N];
    b[0] = s.len() as u8;
    b[1..=s.len()].copy_from_slice(s.as_bytes());
    b
}

fn from_ascic(b: &[u8]) -> String {
    let n = (b[0] as usize).min(b.len() - 1);
    String::from_utf8_lossy(&b[1..=n]).into_owned()
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
        }
    }

    #[test]
    fn round_trip() {
        // 40 descriptors don't fit in the first header block.
        for n in [0, 1, 3, 40] {
            let image = sample(n);
            let bytes = image.write();
            assert_eq!(bytes.len() % BLOCK, 0);
            let parsed = Image::parse(&bytes).unwrap();
            assert_eq!(parsed, image);
            assert_eq!(parsed.write(), bytes);
        }
    }

    #[test]
    fn alpha_layout() {
        let bytes = sample(1).write();
        let u32_at = |at: usize| u32::get(&bytes[at..]);
        assert_eq!((u32_at(0), u32_at(4)), (3, 0), "EIHD majorid, minorid");
        assert_eq!(u32_at(52), Eihd::K_EXE, "EIHD$L_IMGTYPE");
        assert_eq!(u32_at(76), 1, "EIHD$L_HDRBLKCNT");
        assert_eq!(u32_at(100), 16, "EIHD$L_VIRT_MEM_BLOCK_SIZE");
        assert_eq!(u32_at(112), ARCH_ARM64, "EIHD$L_ARCH");
        assert_eq!(&bytes[510..512], &[0xff, 0xff], "EIHD$W_ALIAS");
        let isd = u32_at(12) as usize;
        assert_eq!(
            u32_at(isd + 28),
            2,
            "the section's contents start in block 2"
        );
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
