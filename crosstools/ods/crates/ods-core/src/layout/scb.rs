use super::{BLOCK, checksum, layout};

layout! {
    /// Storage control block (SCB$): VBN 1 of BITMAP.SYS, a summary of the
    /// volume in front of the storage bitmap. See docs/bitmap.md.
    pub struct Scb {
        struclev, set_struclev: u16 = 0;
        cluster, set_cluster: u16 = 2;
        volsize, set_volsize: u32 = 4;
        blksize, set_blksize: u32 = 8;
        sectors, set_sectors: u32 = 12;
        tracks, set_tracks: u32 = 16;
        cylinders, set_cylinders: u32 = 20;
        status, set_status: u32 = 24;
        status2, set_status2: u32 = 28;
        writecnt, set_writecnt: u16 = 32;
        volockname, set_volockname: [u8; 12] = 34;
        mounttime, set_mounttime: u64 = 46;
        backrev, set_backrev: u16 = 54;
        genernum, set_genernum: u64 = 56;
        checksum, set_checksum: u16 = 510;
    }
}

impl Scb {
    /// VMS V1 left the checksum zero, so zero passes.
    pub fn checksum_ok(&self) -> bool {
        self.checksum() == 0 || self.checksum() == checksum(&self.0, 255)
    }

    pub fn update_checksum(&mut self) {
        self.set_checksum(checksum(&self.0, 255));
    }
}

impl Default for Scb {
    fn default() -> Self {
        Scb([0; BLOCK])
    }
}
