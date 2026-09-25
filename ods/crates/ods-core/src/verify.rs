//! Consistency checker, in the spirit of ANALYZE/DISK_STRUCTURE: walks the
//! whole volume and reports what disagrees.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::bitmap::{BITS, bit, set_bit};
use crate::dir::MFD;
use crate::layout::{BLOCK, Header, HomeBlock};
use crate::name;
use crate::volume::{INDEXF, Run};
use crate::{BlockDevice, Error, Fid, Result, Volume, fch};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Can lose data or break the volume: blocks in use marked free,
    /// blocks in two files, directory entries naming no file, bad headers.
    Error,
    /// Space nobody can use: blocks allocated to no file, headers no
    /// directory reaches. What an interrupted write leaves behind.
    Leak,
    /// Harmless disagreement: a stale back link or backup header.
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub what: String,
    pub fid: Option<Fid>,
    pub lbn: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Valid primary headers.
    pub files: u64,
    pub directories: u64,
    /// Blocks mapped by files.
    pub used_blocks: u64,
    /// Clusters whose bitmap bit was rewritten by `repair_bitmap`.
    pub repaired: u64,
}

impl Report {
    pub fn count(&self, s: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == s).count()
    }

    /// No errors: leaks and warnings allowed.
    pub fn is_sound(&self) -> bool {
        self.count(Severity::Error) == 0
    }

    fn add(&mut self, severity: Severity, what: String, fid: Option<Fid>, lbn: Option<u64>) {
        self.findings.push(Finding { severity, what, fid, lbn });
    }
}

/// A valid header found in the index file.
struct Slot {
    lbn: u64,
    h: Header,
    runs: Vec<Run>,
}

impl<D: BlockDevice> Volume<D> {
    /// Checks the whole volume without changing it.
    pub fn verify(&mut self) -> Result<Report, D::Error> {
        self.check(false)
    }

    /// Checks the volume, then rewrites the storage and index file bitmaps
    /// to match what the headers use.
    pub fn repair_bitmap(&mut self) -> Result<Report, D::Error> {
        if !self.writable {
            return Err(Error::ReadOnly);
        }
        self.check(true)
    }

    fn check(&mut self, repair: bool) -> Result<Report, D::Error> {
        use Severity::{Error as Bad, Leak, Warning};
        let mut r = Report::default();
        let v = self.cluster();
        let clusters = self.clusters();
        let home = self.home;

        // Home block copies and the backup index file header.
        match self.read_block(home.alhomelbn() as u64) {
            Ok(b) => {
                let alt = HomeBlock(b);
                if alt.invalid().is_some() || !same_home(&alt, &home) {
                    r.add(
                        Warning,
                        "alternate home block missing or different".into(),
                        None,
                        Some(home.alhomelbn() as u64),
                    );
                }
            }
            Err(_) => r.add(Warning, "alternate home block unreadable".into(), None, Some(home.alhomelbn() as u64)),
        }
        let ilbn = self.header_lbn(1).unwrap_or(0);
        let primary_index = self.read_block(ilbn)?;
        if self.read_block(home.altidxlbn() as u64).ok() != Some(primary_index) {
            r.add(
                Warning,
                "backup index file header differs from INDEXF.SYS".into(),
                Some(INDEXF),
                Some(home.altidxlbn() as u64),
            );
        }

        // Every header slot the index file maps. Past the end of file slots
        // count as never used, so a valid header there is in danger: VMS
        // takes such slots for new files without looking.
        let ih = Header(primary_index);
        let eof = ih.record_attrs().efblk as u64;
        let vbn0 = home.header_vbn0() as u64;
        let (ibm_lbns, mut ibits) = self.read_index_bitmap()?;
        let mut slots: BTreeMap<u32, Slot> = BTreeMap::new();
        let mut new_eof = eof;
        let mut last = 0;
        for num in 1..=home.maxfiles() {
            let Some(lbn) = self.header_lbn(num) else { break };
            last = num;
            let past_eof = vbn0 + num as u64 > eof;
            let h = Header(self.read_block(lbn)?);
            let in_use = bit(&ibits, num as u64 - 1);
            if let Some(why) = h.invalid() {
                // Free slots: zeros, deleted headers, and the empty template
                // headers (file number 0) VMS writes when it grows the index.
                let blank = h.fid().num == 0 || h.0.iter().all(|&b| b == 0) || past_eof;
                if !blank {
                    r.add(Bad, format!("bad file header: {why}"), Some(Fid::new(num, h.fid().seq)), Some(lbn));
                } else if in_use {
                    r.add(Leak, format!("file number {num} marked in use, header free"), None, Some(lbn));
                }
                continue;
            }
            if past_eof {
                r.add(Bad, "valid file header past the index file's end of file".into(), Some(h.fid()), Some(lbn));
                new_eof = vbn0 + num as u64;
            }
            if h.fid().num != num {
                r.add(Bad, format!("header for {} in the slot of file {num}", h.fid()), Some(h.fid()), Some(lbn));
                continue;
            }
            if !in_use {
                r.add(Warning, "file header not marked in the index file bitmap".into(), Some(h.fid()), Some(lbn));
            }
            let runs = match self.header_runs(&h, lbn) {
                Ok(runs) => runs,
                Err(Error::Corrupt { what, .. }) => {
                    r.add(Bad, what.into(), Some(h.fid()), Some(lbn));
                    Vec::new()
                }
                Err(e) => return Err(e),
            };
            slots.insert(num, Slot { lbn, h, runs });
        }
        for num in last + 1..=home.maxfiles() {
            if bit(&ibits, num as u64 - 1) {
                r.add(Leak, format!("file number {num} marked in use, beyond the index file"), None, None);
            }
        }

        // Files: chains of headers, their maps, and who owns each cluster.
        let mut owner: Vec<u32> = vec![0; clusters as usize];
        let mut claimed: Vec<u32> = Vec::new();
        let mut dirs: Vec<Fid> = Vec::new();
        let nums: Vec<u32> = slots.keys().copied().collect();
        for &num in &nums {
            let s = &slots[&num];
            if s.h.seg_num() != 0 {
                continue;
            }
            let fid = s.h.fid();
            r.files += 1;
            if s.h.filechar() & fch::DIRECTORY != 0 {
                dirs.push(fid);
                r.directories += 1;
            }
            let mut runs = s.runs.clone();
            let (mut cur, mut seg) = (num, 0u16);
            loop {
                let next = slots[&cur].h.ext_fid();
                if next.is_zero() {
                    break;
                }
                let ok =
                    slots.get(&next.num).filter(|e| e.h.fid().seq == next.seq && e.h.seg_num() == seg.wrapping_add(1));
                let Some(e) = ok else {
                    r.add(Bad, format!("broken extension header chain at {next}"), Some(fid), Some(slots[&cur].lbn));
                    break;
                };
                if claimed.contains(&next.num) {
                    r.add(Bad, format!("extension header {next} in two chains"), Some(fid), Some(e.lbn));
                    break;
                }
                if e.h.backlink() != fid {
                    r.add(Warning, format!("extension header {next} does not link back"), Some(fid), Some(e.lbn));
                }
                claimed.push(next.num);
                runs.extend(e.runs.iter().copied());
                (cur, seg) = (next.num, seg + 1);
            }
            let mapped: u64 = runs.iter().map(|x| x.count).sum();
            r.used_blocks += mapped;
            let ra = s.h.record_attrs();
            match (ra.hiblk as u64).cmp(&mapped) {
                Ordering::Less => r.add(
                    Leak,
                    format!("allocated size {} below the {mapped} blocks mapped", ra.hiblk),
                    Some(fid),
                    None,
                ),
                Ordering::Greater => r.add(
                    Warning,
                    format!("allocated size {} above the {mapped} blocks mapped", ra.hiblk),
                    Some(fid),
                    None,
                ),
                Ordering::Equal => {}
            }
            if ra.efblk as u64 > mapped + 1 {
                r.add(Warning, format!("end of file (block {}) past the allocation", ra.efblk), Some(fid), None);
            }
            if s.h.filechar() & fch::CONTIG != 0 && runs.windows(2).any(|w| w[0].lbn + w[0].count != w[1].lbn) {
                r.add(Warning, "marked contiguous but is not".into(), Some(fid), None);
            }
            for run in &runs {
                if run.lbn % v != 0 || run.count % v != 0 {
                    r.add(
                        Warning,
                        format!("extent at LBN {} not in whole clusters", run.lbn),
                        Some(fid),
                        Some(run.lbn),
                    );
                }
                let (first, end) = (run.lbn / v, (run.lbn + run.count).div_ceil(v));
                for c in first..end.min(clusters) {
                    match owner[c as usize] {
                        0 => owner[c as usize] = num,
                        o => r.add(Bad, format!("blocks at LBN {} also in file {o}", c * v), Some(fid), Some(c * v)),
                    }
                }
            }
        }
        for (&num, s) in &slots {
            if s.h.seg_num() != 0 && !claimed.contains(&num) {
                r.add(Leak, "extension header of no file".into(), Some(s.h.fid()), Some(s.lbn));
            }
        }

        // Storage bitmap against what the files use.
        let (bmap, mut bits) = self.read_storage_bitmap()?;
        let mut bad = Vec::new();
        for c in 0..clusters {
            let (free, used) = (bit(&bits, c), owner[c as usize] != 0);
            if free == used {
                bad.push((c, used));
            }
        }
        for (start, n, used) in ranges(&bad) {
            let what = if used { "blocks in use marked free" } else { "blocks allocated to no file" };
            r.add(
                if used { Bad } else { Leak },
                format!("{what}: LBN {} to {}", start * v, (start + n) * v - 1),
                None,
                Some(start * v),
            );
        }

        // Directories, from the MFD down.
        let mut listed: BTreeMap<u32, Vec<Fid>> = BTreeMap::new();
        let mut todo = vec![MFD];
        let mut seen = Vec::new();
        while let Some(d) = todo.pop() {
            if seen.contains(&d) {
                continue;
            }
            seen.push(d);
            let blocks = match self.read_dir(d) {
                Ok((_, _, b)) => b,
                Err(Error::Corrupt { what, lbn }) => {
                    r.add(Bad, format!("directory: {what}"), Some(d), Some(lbn));
                    continue;
                }
                Err(e) => return Err(e),
            };
            let recs: Vec<_> = blocks.iter().flat_map(|b| &b.records).collect();
            for w in recs.windows(2) {
                let (a, b) = (name::chars(&w[0].name, w[0].name_type()), name::chars(&w[1].name, w[1].name_type()));
                let order = name::cmp(&a, &b);
                let versions_down = w[0].entries.last().zip(w[1].entries.first()).is_some_and(|(x, y)| x.0 > y.0);
                if order == Ordering::Greater || order == Ordering::Equal && !versions_down {
                    r.add(
                        Bad,
                        format!("directory out of order at {}", String::from_utf8_lossy(&w[1].name)),
                        Some(d),
                        None,
                    );
                }
            }
            for rec in &recs {
                if rec.entries.windows(2).any(|w| w[0].0 <= w[1].0) {
                    r.add(
                        Bad,
                        format!("versions of {} out of order", String::from_utf8_lossy(&rec.name)),
                        Some(d),
                        None,
                    );
                }
                for &(ver, fid) in &rec.entries {
                    let ok = slots.get(&fid.num).filter(|s| s.h.fid().seq == fid.seq && s.h.seg_num() == 0);
                    let Some(s) = ok else {
                        r.add(
                            Bad,
                            format!("{};{ver} names missing file {fid}", String::from_utf8_lossy(&rec.name)),
                            Some(d),
                            None,
                        );
                        continue;
                    };
                    listed.entry(fid.num).or_default().push(d);
                    let dir_name = name::split(&rec.name).1.eq_ignore_ascii_case(b"DIR") && ver == 1;
                    if dir_name && s.h.filechar() & fch::DIRECTORY != 0 && fid != d {
                        todo.push(fid);
                    }
                }
            }
        }
        for (&num, s) in &slots {
            let fid = s.h.fid();
            if s.h.seg_num() != 0 {
                continue;
            }
            match listed.get(&num) {
                None if num > home.resfiles() as u32 => {
                    r.add(Leak, "lost file: in no directory".into(), Some(fid), Some(s.lbn))
                }
                None => r.add(Bad, "reserved file missing from the MFD".into(), Some(fid), Some(s.lbn)),
                Some(ds) if !ds.contains(&s.h.backlink()) && s.h.idoffset() >= 40 => {
                    r.add(
                        Warning,
                        format!("back link {} is not a directory listing the file", s.h.backlink()),
                        Some(fid),
                        None,
                    );
                }
                Some(_) => {}
            }
        }
        for num in 1..=home.resfiles().min(9) as u32 {
            if !slots.contains_key(&num) {
                r.add(Bad, format!("reserved file {num} missing"), None, None);
            }
        }

        if repair {
            if new_eof > eof {
                let mut ih = self.read_header(INDEXF)?;
                let mut ra = ih.record_attrs();
                ra.efblk = new_eof as u32;
                ra.ffbyte = 0;
                ih.set_record_attrs(&ra);
                self.write_header(ilbn, &mut ih)?;
            }
            let mut touched: Vec<u64> = Vec::new();
            for c in 0..clusters {
                let want = owner[c as usize] == 0;
                if bit(&bits, c) != want {
                    set_bit(&mut bits, c, want);
                    r.repaired += 1;
                    touched.push(c / BITS);
                }
            }
            touched.dedup();
            for b in touched {
                if let Some(run) = bmap.get(b as usize) {
                    let mut blk = [0u8; BLOCK];
                    blk.copy_from_slice(&bits[b as usize * BLOCK..(b as usize + 1) * BLOCK]);
                    self.write_block(run.lbn, &blk)?;
                }
            }
            let mut itouched: Vec<u64> = Vec::new();
            for num in 1..=home.maxfiles() {
                let want = slots.contains_key(&num);
                if bit(&ibits, num as u64 - 1) != want {
                    set_bit(&mut ibits, num as u64 - 1, want);
                    itouched.push((num as u64 - 1) / BITS);
                }
            }
            itouched.dedup();
            for b in itouched {
                let mut blk = [0u8; BLOCK];
                blk.copy_from_slice(&ibits[b as usize * BLOCK..(b as usize + 1) * BLOCK]);
                self.write_block(ibm_lbns[b as usize], &blk)?;
            }
        }
        r.findings.sort_by_key(|f| f.severity);
        Ok(r)
    }
}

/// Whether two home block copies agree, apart from where each one is.
fn same_home(a: &HomeBlock, b: &HomeBlock) -> bool {
    let (mut a, mut b) = (*a, *b);
    for h in [&mut a, &mut b] {
        h.set_homelbn(0);
        h.set_homevbn(0);
        h.set_checksum1(0);
        h.set_checksum2(0);
    }
    a == b
}

/// Groups consecutive clusters with the same flag: (start, count, flag).
fn ranges(items: &[(u64, bool)]) -> Vec<(u64, u64, bool)> {
    let mut out: Vec<(u64, u64, bool)> = Vec::new();
    for &(c, f) in items {
        match out.last_mut() {
            Some((s, n, g)) if *g == f && *s + *n == c => *n += 1,
            _ => out.push((c, 1, f)),
        }
    }
    out
}
