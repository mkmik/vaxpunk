//! Guest memory for one run: the stub, the page tables that put the image at
//! its link addresses, and what the image sees at EL0 (docs/runner-abi.md).

use vms_obj::exe::{Eisd, Image};

use crate::layout::{BOOT, EL1_STACK, LOAD_BASE, RAM_BASE, STUB_MAX, UART};

const PAGE: u64 = 4096;
const MB: u64 = 1 << 20;

/// The runner's EL0 pages, at the top of VMS P1 space: the info block, the
/// return page, and the stack with an unmapped guard below it.
pub const RUNNER_VA: u64 = 0x7ff0_0000;
pub const RUNNER_END: u64 = 0x8000_0000;
pub const INFO_VA: u64 = RUNNER_VA;
pub const RETURN_VA: u64 = RUNNER_VA + PAGE;
pub const STACK_TOP: u64 = 0x7fff_0000;
const STACK_SIZE: u64 = 0xe_0000;

/// The page tables go right after the stub region; nothing maps them.
const TABLES: u64 = EL1_STACK;
const TABLES_MAX: u64 = MB;
/// Pages the image sees come after the tables.
const DATA: u64 = TABLES + TABLES_MAX;

/// `svc #1`, the exit monitor call: the whole return page.
const SVC_EXIT: u32 = 0xd400_0021;

/// Guest RAM for one run.
pub struct Plan {
    /// RAM contents from `LOAD_BASE` on. RAM past the end is zero.
    pub ram: Vec<u8>,
    /// RAM size QEMU needs, in MB.
    pub ram_mb: u64,
    /// What is mapped where.
    pub map: Vec<Region>,
}

pub struct Region {
    pub va: u64,
    pub pa: u64,
    pub size: u64,
    pub prot: Prot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prot {
    StubCode,
    StubData,
    Device,
    Code,
    ReadOnly,
    ReadWrite,
}

impl Prot {
    /// Stage 1 page descriptor bits: AttrIndx, AP, SH, AF, PXN, UXN.
    fn attrs(self) -> u64 {
        const DEVICE: u64 = 0 << 2; // MAIR attribute 0
        const NORMAL: u64 = 1 << 2; // MAIR attribute 1
        const EL0: u64 = 1 << 6;
        const RO: u64 = 1 << 7;
        const ISH: u64 = 3 << 8;
        const AF: u64 = 1 << 10;
        const PXN: u64 = 1 << 53;
        const UXN: u64 = 1 << 54;
        AF | match self {
            Prot::StubCode => NORMAL | ISH | RO | UXN,
            Prot::StubData => NORMAL | ISH | PXN | UXN,
            Prot::Device => DEVICE | PXN | UXN,
            Prot::Code => NORMAL | ISH | EL0 | RO | PXN,
            Prot::ReadOnly => NORMAL | ISH | EL0 | RO | PXN | UXN,
            Prot::ReadWrite => NORMAL | ISH | EL0 | PXN | UXN,
        }
    }
}

/// Lays out guest RAM for `image`, with `args` as the argument string.
pub fn plan(image: &Image, args: &[u8], stub: &[u8]) -> Result<Plan, String> {
    if image.transfer == 0 {
        return Err("NOTFR, the image has no transfer address".into());
    }
    let mut b = Builder {
        ram: Vec::new(),
        tables: vec![[0; 512]],
        map: Vec::new(),
        next_pa: DATA,
    };

    b.write(LOAD_BASE, stub);
    b.map(LOAD_BASE, LOAD_BASE, STUB_MAX, Prot::StubCode)?;
    b.map(BOOT, BOOT, EL1_STACK - BOOT, Prot::StubData)?;
    b.map(UART, UART, PAGE, Prot::Device)?;

    let pa = b.alloc(PAGE);
    b.write(pa, &info_block(args)?);
    b.map(INFO_VA, pa, PAGE, Prot::ReadOnly)?;
    let pa = b.alloc(PAGE);
    b.write(pa, &SVC_EXIT.to_le_bytes());
    b.map(RETURN_VA, pa, PAGE, Prot::Code)?;
    let pa = b.alloc(STACK_SIZE);
    b.map(STACK_TOP - STACK_SIZE, pa, STACK_SIZE, Prot::ReadWrite)?;

    let reserved = [
        (LOAD_BASE, EL1_STACK),
        (UART, UART + PAGE),
        (RUNNER_VA, RUNNER_END),
    ];
    for s in &image.sections {
        let size = u64::from(s.size).next_multiple_of(PAGE);
        let end = s.vaddr + size;
        if end > 1 << 48 {
            return Err(format!(
                "BADVA, image section at {:016X} is outside the 48-bit lower half",
                s.vaddr
            ));
        }
        if let Some((lo, hi)) = reserved.iter().find(|&&(lo, hi)| s.vaddr < hi && lo < end) {
            return Err(format!(
                "BADVA, image section {:016X}-{:016X} overlaps vrun's range {lo:016X}-{:016X}",
                s.vaddr,
                end - 1,
                hi - 1
            ));
        }
        let prot = if s.flags & Eisd::M_EXE != 0 {
            Prot::Code
        } else if s.flags & Eisd::M_WRT != 0 {
            Prot::ReadWrite
        } else {
            Prot::ReadOnly
        };
        let pa = b.alloc(size);
        b.write(pa, &s.data);
        b.map(s.vaddr, pa, size, prot)?;
    }

    if b.tables.len() as u64 * PAGE > TABLES_MAX {
        return Err("BIGIMG, the image needs too many page tables".into());
    }
    let tables: Vec<u8> = b
        .tables
        .iter()
        .flatten()
        .flat_map(|e| e.to_le_bytes())
        .collect();
    b.write(TABLES, &tables);
    let boot = [TABLES, image.transfer, STACK_TOP, INFO_VA, RETURN_VA];
    b.write(
        BOOT,
        &boot
            .iter()
            .flat_map(|q| q.to_le_bytes())
            .collect::<Vec<_>>(),
    );

    // The stub's page tables give the CPU 32-bit physical addresses.
    if b.next_pa > 1 << 32 {
        return Err("BIGIMG, the image doesn't fit in 4 GB of guest RAM".into());
    }
    Ok(Plan {
        ram: b.ram,
        ram_mb: (b.next_pa - RAM_BASE).div_ceil(MB),
        map: b.map,
    })
}

/// The runner info block: its size, the runner version, flags, and the
/// argument string as a class S text descriptor pointing just past it.
fn info_block(args: &[u8]) -> Result<Vec<u8>, String> {
    const SIZE: u64 = 24;
    if args.len() as u64 > PAGE - SIZE {
        return Err(format!(
            "ARGLEN, the arguments are longer than {} bytes",
            PAGE - SIZE
        ));
    }
    let mut b = Vec::new();
    b.extend(SIZE.to_le_bytes());
    b.extend(1u32.to_le_bytes()); // runner version
    b.extend(0u32.to_le_bytes()); // flags
    b.extend((args.len() as u16).to_le_bytes()); // DSC$W_LENGTH
    b.push(14); // DSC$B_DTYPE: DSC$K_DTYPE_T, text
    b.push(1); // DSC$B_CLASS: DSC$K_CLASS_S, static
    b.extend(((INFO_VA + SIZE) as u32).to_le_bytes()); // DSC$A_POINTER
    b.extend(args);
    Ok(b)
}

struct Builder {
    ram: Vec<u8>,
    /// 4-level, 4 KB granule page tables; table n lives at TABLES + n pages.
    tables: Vec<[u64; 512]>,
    map: Vec<Region>,
    next_pa: u64,
}

impl Builder {
    /// Allocates guest RAM, 64 KB aligned like the image sections.
    fn alloc(&mut self, size: u64) -> u64 {
        let pa = self.next_pa;
        self.next_pa = (pa + size).next_multiple_of(vms_obj::exe::SECTION_ALIGN);
        pa
    }

    fn write(&mut self, pa: u64, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let at = (pa - LOAD_BASE) as usize;
        if self.ram.len() < at + bytes.len() {
            self.ram.resize(at + bytes.len(), 0);
        }
        self.ram[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn map(&mut self, va: u64, pa: u64, size: u64, prot: Prot) -> Result<(), String> {
        for off in (0..size).step_by(PAGE as usize) {
            self.map_page(va + off, pa + off, prot)?;
        }
        self.map.push(Region { va, pa, size, prot });
        Ok(())
    }

    fn map_page(&mut self, va: u64, pa: u64, prot: Prot) -> Result<(), String> {
        let mut t = 0;
        for shift in [39, 30, 21] {
            let i = (va >> shift) as usize & 511;
            if self.tables[t][i] == 0 {
                self.tables.push([0; 512]);
                let next = TABLES + (self.tables.len() as u64 - 1) * PAGE;
                self.tables[t][i] = next | 0b11;
            }
            t = ((self.tables[t][i] & !0xfff) - TABLES) as usize / PAGE as usize;
        }
        let entry = &mut self.tables[t][(va >> 12) as usize & 511];
        if *entry != 0 {
            return Err(format!("BADVA, image sections overlap at {va:016X}"));
        }
        *entry = pa | prot.attrs() | 0b11;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vms_obj::exe::Section;

    /// Walks the tables in `plan.ram` like the MMU would.
    fn translate(ram: &[u8], va: u64) -> Option<u64> {
        let entry = |table: u64, i: u64| {
            let at = (table - LOAD_BASE + i * 8) as usize;
            u64::from_le_bytes(ram[at..at + 8].try_into().unwrap())
        };
        let mut table = TABLES;
        for shift in [39, 30, 21, 12] {
            let e = entry(table, (va >> shift) & 511);
            if e & 0b11 != 0b11 {
                return None;
            }
            table = e & 0x0000_ffff_ffff_f000;
            if shift == 12 {
                return Some(e);
            }
        }
        unreachable!()
    }

    #[test]
    fn maps_image_at_link_address() {
        let image = Image {
            name: "T".into(),
            ident: String::new(),
            link_time: 0,
            transfer: 0x10000,
            sections: vec![
                Section {
                    vaddr: 0x10000,
                    size: 8,
                    flags: Eisd::M_EXE,
                    data: vec![7; 8],
                },
                Section {
                    vaddr: 0x20000,
                    size: 0x3000,
                    flags: Eisd::M_WRT | Eisd::M_DZRO,
                    data: vec![],
                },
            ],
        };
        let ram = plan(&image, b"hi", &[0; 4]).unwrap().ram;

        let code = translate(&ram, 0x10004).unwrap();
        let pa = code & 0x0000_ffff_ffff_f000;
        assert_eq!(code & !0x0000_ffff_ffff_f000, Prot::Code.attrs() | 0b11);
        assert_eq!(ram[(pa - LOAD_BASE) as usize], 7);
        let bss = translate(&ram, 0x22000).unwrap();
        assert_eq!(
            bss & (1 << 54 | 1 << 53 | 3 << 6),
            1 << 54 | 1 << 53 | 1 << 6,
            "EL0 RW, XN"
        );
        assert_eq!(translate(&ram, 0x23000), None, "past the section");
        assert_eq!(translate(&ram, 0), None, "page 0");
        assert_eq!(
            translate(&ram, STACK_TOP - STACK_SIZE - PAGE),
            None,
            "stack guard"
        );
        let info = translate(&ram, INFO_VA).unwrap() & 0x0000_ffff_ffff_f000;
        let info = &ram[(info - LOAD_BASE) as usize..][..26];
        assert_eq!(
            &info[16..20],
            &[2, 0, 14, 1],
            "descriptor length, dtype, class"
        );
        assert_eq!(&info[24..], b"hi");

        let overlap = Section {
            vaddr: 0x7ff0_0000,
            size: 1,
            flags: 0,
            data: vec![0],
        };
        let bad = Image {
            sections: vec![overlap],
            ..image
        };
        assert!(plan(&bad, b"", &[]).is_err_and(|e| e.starts_with("BADVA")));
    }
}
