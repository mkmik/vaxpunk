use super::{BLOCK, checksum, layout};

layout! {
    /// Home block (HM2$): identifies the volume and locates the index file.
    /// The primary copy is normally LBN 1. See docs/home-block.md.
    pub struct HomeBlock {
        homelbn, set_homelbn: u32 = 0;
        alhomelbn, set_alhomelbn: u32 = 4;
        altidxlbn, set_altidxlbn: u32 = 8;
        /// High byte: structure level (2 or 5). Low byte: version (1).
        struclev, set_struclev: u16 = 12;
        cluster, set_cluster: u16 = 14;
        homevbn, set_homevbn: u16 = 16;
        alhomevbn, set_alhomevbn: u16 = 18;
        altidxvbn, set_altidxvbn: u16 = 20;
        ibmapvbn, set_ibmapvbn: u16 = 22;
        ibmaplbn, set_ibmaplbn: u32 = 24;
        maxfiles, set_maxfiles: u32 = 28;
        ibmapsize, set_ibmapsize: u16 = 32;
        resfiles, set_resfiles: u16 = 34;
        devtype, set_devtype: u16 = 36;
        rvn, set_rvn: u16 = 38;
        setcount, set_setcount: u16 = 40;
        volchar, set_volchar: u16 = 42;
        volowner, set_volowner: u32 = 44;
        protect, set_protect: u16 = 52;
        fileprot, set_fileprot: u16 = 54;
        /// Default record protection; VMS 7 writes 0xFE00 here.
        recprot, set_recprot: u16 = 56;
        checksum1, set_checksum1: u16 = 58;
        credate, set_credate: u64 = 60;
        window, set_window: u8 = 68;
        lru_lim, set_lru_lim: u8 = 69;
        extend, set_extend: u16 = 70;
        retainmin, set_retainmin: u64 = 72;
        retainmax, set_retainmax: u64 = 80;
        revdate, set_revdate: u64 = 88;
        serialnum, set_serialnum: u32 = 456;
        strucname, set_strucname: [u8; 12] = 460;
        volname, set_volname: [u8; 12] = 472;
        ownername, set_ownername: [u8; 12] = 484;
        format, set_format: [u8; 12] = 496;
        checksum2, set_checksum2: u16 = 510;
    }
}

impl HomeBlock {
    /// Why this block is not a valid home block, if it is not.
    pub fn invalid(&self) -> Option<&'static str> {
        let b = &self.0;
        let (level, version) = (self.struclev() >> 8, self.struclev() & 0xff);
        if self.checksum1() != checksum(b, 29) || self.checksum2() != checksum(b, 255) {
            Some("bad checksum")
        } else if !(level == 2 || level == 5) || version < 1 {
            Some("unsupported structure level")
        } else if self.homelbn() == 0 || self.alhomelbn() == 0 || self.altidxlbn() == 0 {
            Some("zero home or backup LBN")
        } else if self.cluster() == 0 || self.homevbn() == 0 || self.ibmapvbn() == 0 {
            Some("zero cluster factor or VBN")
        } else if self.ibmaplbn() == 0 || self.ibmapsize() == 0 {
            Some("no index file bitmap")
        } else if self.resfiles() < 5 || self.maxfiles() <= self.resfiles() as u32 {
            Some("bad reserved or maximum file count")
        } else if self.maxfiles() >= 1 << 24 || self.maxfiles() > self.ibmapsize() as u32 * 4096 {
            Some("index file bitmap too small for maximum files")
        } else {
            None
        }
    }

    pub fn update_checksums(&mut self) {
        self.set_checksum1(checksum(&self.0, 29));
        self.set_checksum2(checksum(&self.0, 255));
    }

    /// Structure level: 2 or 5.
    pub fn level(&self) -> u8 {
        (self.struclev() >> 8) as u8
    }

    /// VBN of the first file header in the index file: header n is at this
    /// VBN plus n - 1.
    pub fn header_vbn0(&self) -> u32 {
        self.ibmapvbn() as u32 + self.ibmapsize() as u32
    }
}

impl Default for HomeBlock {
    fn default() -> Self {
        HomeBlock([0; BLOCK])
    }
}
