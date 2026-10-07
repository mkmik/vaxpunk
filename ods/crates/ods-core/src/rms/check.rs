//! Structure checker for relative and indexed files, in the spirit of
//! ANALYZE/RMS_FILE/CHECK: walks every key's index and buckets and reports
//! what disagrees.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::read::{DLC_DELETED, DLC_REC, Item, Rfa};
use super::{BKT_HDR, BucketHeader, Indexed, Key, Relative, RmsError, RmsFile, bktcb, checksum_ok, irc, keyflag, open};
use crate::layout::{BLOCK, Field, RecordAttrs};
use crate::verify::Severity;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub what: String,
    /// The key whose structure it is in.
    pub key: Option<u8>,
    pub vbn: Option<u32>,
}

/// One key's shape, as the checker found it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyStats {
    /// Buckets at each level, level 0 (data or SIDR buckets) first.
    pub levels: Vec<usize>,
    /// Records, or SIDRs, in each level 0 bucket along the chain.
    pub entries: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Records in the file, deleted ones left out.
    pub records: u64,
    /// Per key; empty for a relative file.
    pub keys: Vec<KeyStats>,
}

impl Report {
    pub fn count(&self, s: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == s).count()
    }

    /// No errors: warnings allowed.
    pub fn is_sound(&self) -> bool {
        self.count(Severity::Error) == 0
    }

    fn add(&mut self, severity: Severity, what: impl Into<String>, key: Option<u8>, vbn: Option<u32>) {
        self.findings.push(Finding { severity, what: what.into(), key, vbn });
    }

    fn error(&mut self, what: impl Into<String>, key: Option<u8>, vbn: u32) {
        self.add(Severity::Error, what, key, Some(vbn));
    }
}

/// Checks a file's bytes. A file that doesn't open is a report with the
/// reason as its one error.
pub fn check(data: &[u8], attrs: &RecordAttrs) -> Report {
    match open(data, attrs) {
        Ok(f) => f.check(),
        Err(e) => {
            let mut r = Report::default();
            let vbn = match e {
                RmsError::Corrupt { vbn, .. } => Some(vbn),
                _ => None,
            };
            r.add(Severity::Error, e.to_string(), None, vbn);
            r
        }
    }
}

impl RmsFile<'_> {
    pub fn check(&self) -> Report {
        match self {
            RmsFile::Relative(r) => r.check(),
            RmsFile::Indexed(x) => x.check(),
        }
    }
}

impl Relative<'_> {
    pub fn check(&self) -> Report {
        let mut r = Report::default();
        if !checksum_ok(self.prologue()) {
            r.error("prologue checksum", None, 1);
        }
        if self.eof < self.dvbn {
            r.error("end of file before the first bucket", None, 1);
        }
        for c in self.cells() {
            let (n, vbn, cell) = match c {
                Ok(c) => c,
                Err(e) => {
                    r.error(e.to_string(), None, self.eof);
                    break;
                }
            };
            if cell[0] & !(DLC_DELETED | DLC_REC) != 0 {
                r.error(format!("record {n}: control byte {:#04x}", cell[0]), None, vbn);
                continue;
            }
            match self.cell_record(cell, vbn) {
                Ok(Some(_)) if self.mrn != 0 && n > self.mrn => {
                    r.error(format!("record {n} above the highest record number, {}", self.mrn), None, vbn)
                }
                Ok(Some(_)) => r.records += 1,
                Ok(None) => {}
                Err(e) => r.error(format!("record {n}: {e}"), None, vbn),
            }
        }
        r
    }
}

/// A record as the checker knows it: where it is, and whether it is live.
struct Placed {
    vbn: u32,
    id: u16,
    live: bool,
    rec: Vec<u8>,
}

/// The index keys around each level 0 bucket: the one before it and its
/// own, `None` past either end.
type Bounds = BTreeMap<u32, (Option<Vec<u8>>, Option<Vec<u8>>)>;

struct Checker<'a, 'b> {
    x: &'b Indexed<'a>,
    r: Report,
    /// Blocks some bucket or the prologue holds.
    owned: Vec<bool>,
    /// Primary records by RFA.
    by_rfa: BTreeMap<Rfa, Placed>,
    /// RRVs: where, which ID, where they point.
    rrvs: Vec<(u32, u16, Rfa)>,
}

impl Indexed<'_> {
    pub fn check(&self) -> Report {
        let mut c = Checker {
            x: self,
            r: Report::default(),
            owned: vec![false; self.blocks() as usize + 1],
            by_rfa: BTreeMap::new(),
            rrvs: Vec::new(),
        };
        c.prologue();
        for n in 0..self.keys.len() {
            let stats = c.key(n);
            c.r.keys.push(stats);
        }
        c.rrvs();
        c.r
    }
}

impl Checker<'_, '_> {
    fn prologue(&mut self) {
        let x = self.x;
        let mut blocks = BTreeSet::from([1u32]);
        blocks.extend(x.key_at.iter().map(|a| a.0));
        if !x.areas.is_empty() {
            blocks.extend((0..x.areas.len().div_ceil(8)).map(|i| x.avbn + i as u32));
        }
        for vbn in blocks {
            match x.block(vbn) {
                Ok(b) if checksum_ok(b) => {}
                Ok(_) => self.r.error("prologue checksum", None, vbn),
                Err(e) => self.r.error(e.to_string(), None, vbn),
            }
            if let Some(o) = self.owned.get_mut(vbn as usize) {
                *o = true;
            }
        }
        for (i, a) in x.areas.iter().enumerate() {
            let vbn = x.avbn + i as u32 / 8;
            if a.areaid() as usize != i {
                self.r.error(format!("area {i}'s descriptor says it is area {}", a.areaid()), None, vbn);
            }
            if !(1..=63).contains(&a.arbktsz()) {
                self.r.error(format!("area {i}: bucket size {}", a.arbktsz()), None, vbn);
            }
            if a.used() > a.cnblk() {
                self.r.error(format!("area {i}: more blocks used than its extent has"), None, vbn);
            }
            if a.nxtvbn() != a.cvbn().wrapping_add(a.used()) {
                let w = format!(
                    "area {i}: next VBN {} is not the extent's start {} plus {} used",
                    a.nxtvbn(),
                    a.cvbn(),
                    a.used()
                );
                self.r.add(Severity::Warning, w, None, Some(vbn));
            }
        }
    }

    /// Checks a bucket's header and claims its blocks.
    fn header(&mut self, n: usize, vbn: u32, b: &[u8], level: u8, area: u8) {
        let key = Some(n as u8);
        let h = BucketHeader(&b[..BKT_HDR]);
        if b[0] != b[b.len() - 1] {
            self.r.error("check characters differ", key, vbn);
        }
        if h.adrsample() != vbn as u16 {
            self.r.error(format!("address sample {} is not the bucket's VBN", h.adrsample()), key, vbn);
        }
        if h.indexno() as usize != n {
            self.r.error(format!("bucket of key {}", h.indexno()), key, vbn);
        }
        if h.level() != level {
            self.r.error(format!("bucket level {} where {level} belongs", h.level()), key, vbn);
        }
        let end = vbn as u64 + (b.len() / BLOCK) as u64;
        match self.x.areas.get(area as usize) {
            Some(a) if end > a.nxtvbn() as u64 => {
                self.r.error(format!("bucket past area {area}'s next VBN {}", a.nxtvbn()), key, vbn)
            }
            Some(_) => {}
            None => self.r.error(format!("bucket in area {area}, which the file doesn't have"), key, vbn),
        }
        let mut overlap = false;
        for blk in vbn as usize..end as usize {
            if let Some(o) = self.owned.get_mut(blk) {
                overlap |= *o;
                *o = true;
            }
        }
        if overlap {
            self.r.error("bucket overlaps another bucket or the prologue", key, vbn);
        }
    }

    /// Checks one key: its descriptor, index, chains and level 0 buckets.
    fn key(&mut self, n: usize) -> KeyStats {
        let k = self.x.keys[n];
        let key = Some(n as u8);
        let at = self.x.key_at[n].0;
        let mut stats = KeyStats::default();
        if k.keyref() as usize != n {
            self.r.error(format!("key {n}'s descriptor says it is key {}", k.keyref()), key, at);
        }
        let minrec = k.segs().map(|(p, s)| p + s).max().unwrap_or(0);
        if k.minrecsz() as usize != minrec {
            self.r.error(format!("minimum record size {} where the key ends at {minrec}", k.minrecsz()), key, at);
        }
        for (what, a, size) in [
            ("index", k.ianum(), k.idxbktsz()),
            ("level 1", k.lanum(), k.idxbktsz()),
            ("data", k.danum(), k.datbktsz()),
        ] {
            match self.x.areas.get(a as usize) {
                Some(area) if area.arbktsz() != size && !k.has(keyflag::INITIDX) => {
                    self.r.error(format!("{what} bucket size {size} but area {a}'s is {}", area.arbktsz()), key, at)
                }
                Some(_) => {}
                None => self.r.error(format!("{what} area {a} doesn't exist"), key, at),
            }
        }
        if k.has(keyflag::INITIDX) {
            if k.rootvbn() != 0 || k.ldvbn() != 0 {
                self.r.add(Severity::Warning, "index not initialized, but it has a root", key, Some(at));
            }
            if n > 0 {
                self.coverage(n, &[]);
            }
            return stats;
        }

        // The index, from the root down.
        let levels = k.rootlev() as usize;
        let mut reached: Vec<Vec<u32>> = vec![Vec::new(); levels + 1];
        let mut bounds = Bounds::new();
        let mut seen = BTreeSet::new();
        self.walk(n, k.rootvbn(), levels, None, None, &mut reached, &mut bounds, &mut seen);
        match self.x.bucket(k.rootvbn(), k.idxbktsz()) {
            Ok(b) if b[13] & (bktcb::ROOTBKT | bktcb::LASTBKT) != bktcb::ROOTBKT | bktcb::LASTBKT => {
                self.r.error("root bucket not marked root and last", key, k.rootvbn())
            }
            _ => {}
        }

        // Each level's chain must be the buckets the index reaches, in order.
        let mut level0 = reached[0].clone();
        for (l, list) in reached.iter().enumerate() {
            let first = match list.first() {
                _ if l == 0 => k.ldvbn(),
                Some(&f) => f,
                None => continue,
            };
            let size = if l == 0 { k.datbktsz() } else { k.idxbktsz() };
            if let Some(&f) = list.first()
                && f != first
            {
                self.r.error(format!("first data bucket is VBN {f}, not LDVBN {first}"), key, at);
            }
            let chain = match self.x.chain(first, size) {
                Ok(c) => c,
                Err(e) => {
                    self.r.error(format!("level {l} chain: {e}"), key, first);
                    continue;
                }
            };
            if let Some(&last) = chain.last()
                && let Ok(b) = self.x.bucket(last, size)
                && u32::get(&b[8..]) != chain[0]
            {
                self.r.error(format!("last bucket of level {l} doesn't point back to the first"), key, last);
            }
            if chain != *list {
                for &v in chain.iter().filter(|v| !list.contains(v)) {
                    self.r.error(format!("level {l} bucket the index doesn't reach"), key, v);
                }
                for &v in list.iter().filter(|v| !chain.contains(v)) {
                    self.r.error(format!("level {l} bucket missing from its chain"), key, v);
                }
                if chain.iter().filter(|v| list.contains(v)).ne(list.iter().filter(|v| chain.contains(v))) {
                    self.r.error(format!("level {l} chain out of the index's order"), key, first);
                }
                if l == 0 {
                    // Read what the chain has: it is what a reader goes by.
                    level0 = chain;
                }
            }
        }
        stats.levels = reached.iter().map(|l| l.len()).collect();

        // Level 0: records or SIDRs, in order and within their index keys.
        let mut prev: Option<Vec<u8>> = None;
        let mut pointers = Vec::new();
        for &vbn in &level0 {
            let Ok(b) = self.x.bucket(vbn, k.datbktsz()) else { continue };
            if !seen.contains(&vbn) {
                self.header(n, vbn, b, 0, k.danum());
            }
            let (lo, hi) = bounds.get(&vbn).cloned().unwrap_or((None, None));
            let keys: Vec<Vec<u8>> = if n == 0 {
                match self.x.data_bucket(&k, vbn, b) {
                    Ok(items) => self.data_items(&k, vbn, b, items),
                    Err(e) => {
                        self.r.error(e.to_string(), key, vbn);
                        continue;
                    }
                }
            } else {
                match self.x.sidr_bucket(&k, vbn, b) {
                    Ok(sidrs) => {
                        let mut ks = Vec::new();
                        for s in sidrs {
                            if s.ptrs.is_empty() {
                                self.r.error("SIDR without pointers", key, vbn);
                            }
                            pointers.extend(s.ptrs.iter().map(|&(c, rfa)| (vbn, c, rfa, s.key.clone())));
                            ks.push(s.key);
                        }
                        if ks.windows(2).any(|w| k.compare(&w[0], &w[1]) != Ordering::Less) {
                            self.r.error("SIDR keys out of order or repeated in a bucket", key, vbn);
                        }
                        ks
                    }
                    Err(e) => {
                        self.r.error(e.to_string(), key, vbn);
                        continue;
                    }
                }
            };
            stats.entries.push(keys.len());
            for kv in keys {
                if let Some(p) = &prev {
                    match k.compare(p, &kv) {
                        Ordering::Greater => self.r.error("keys out of order", key, vbn),
                        Ordering::Equal if !k.has(keyflag::DUPKEYS) => self.r.error("duplicate key", key, vbn),
                        _ => {}
                    }
                }
                if hi.as_ref().is_some_and(|h| k.compare(&kv, h) == Ordering::Greater) {
                    self.r.error("key above the index key for its bucket", key, vbn);
                }
                if lo.as_ref().is_some_and(|l| k.compare(&kv, l) == Ordering::Less) {
                    self.r.error("key below the index key for the bucket before", key, vbn);
                }
                prev = Some(kv);
            }
        }
        if n > 0 {
            self.coverage(n, &pointers);
        }
        stats
    }

    /// Descends the index from `vbn` at `level`, every key in it between
    /// `lo` and `hi` (`None`: unbounded), recording what it reaches.
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        n: usize,
        vbn: u32,
        level: usize,
        lo: Option<&[u8]>,
        hi: Option<&[u8]>,
        reached: &mut Vec<Vec<u32>>,
        bounds: &mut Bounds,
        seen: &mut BTreeSet<u32>,
    ) {
        let k = self.x.keys[n];
        let key = Some(n as u8);
        if !seen.insert(vbn) {
            self.r.error("bucket reached twice from the index", key, vbn);
            return;
        }
        reached[level].push(vbn);
        if level == 0 {
            if let Ok(b) = self.x.bucket(vbn, k.datbktsz()) {
                self.header(n, vbn, b, 0, k.danum());
            }
            bounds.insert(vbn, (lo.map(<[u8]>::to_vec), hi.map(<[u8]>::to_vec)));
            return;
        }
        let b = match self.x.bucket(vbn, k.idxbktsz()) {
            Ok(b) => b,
            Err(e) => {
                self.r.error(e.to_string(), key, vbn);
                return;
            }
        };
        self.header(n, vbn, b, level as u8, if level == 1 { k.lanum() } else { k.ianum() });
        if vbn != k.rootvbn() && b[13] & bktcb::ROOTBKT != 0 {
            self.r.error("bucket marked root that isn't", key, vbn);
        }
        let (keys, ptrs) = match self.x.index_bucket(&k, vbn, b) {
            Ok(e) => e,
            Err(e) => {
                self.r.error(e.to_string(), key, vbn);
                return;
            }
        };
        if keys.is_empty() {
            self.r.error("empty index bucket", key, vbn);
            return;
        }
        let ps = (b[13] >> 3 & 3) as usize + 2;
        let vfree = u16::get(&b[b.len() - 4..]) as usize;
        if vfree + 1 + keys.len() * ps + 4 != b.len() {
            self.r.error(format!("VBN free space {vfree} doesn't fit {} pointers", keys.len()), key, vbn);
        }
        if keys.windows(2).any(|w| k.compare(&w[0], &w[1]) == Ordering::Greater) {
            self.r.error("index keys out of order", key, vbn);
        }
        if let Some(h) = hi
            && keys.iter().any(|kv| k.compare(kv, h) == Ordering::Greater)
        {
            self.r.error("index key above the one leading here", key, vbn);
        }
        for i in 0..keys.len() {
            let clo = if i == 0 { lo } else { Some(&keys[i - 1][..]) };
            let chi = if i + 1 == keys.len() && hi.is_none() { None } else { Some(&keys[i][..]) };
            self.walk(n, ptrs[i], level - 1, clo, chi, reached, bounds, seen);
        }
    }

    /// Checks a primary data bucket's records and RRVs; returns their keys.
    fn data_items(&mut self, k: &Key, vbn: u32, b: &[u8], items: Vec<Item>) -> Vec<Vec<u8>> {
        let key = Some(0);
        let nxtrecid = u16::get(&b[6..]);
        let mut ids = BTreeSet::new();
        let mut keys = Vec::new();
        for item in items {
            let id = match &item {
                Item::Rec(r) => r.id,
                Item::Rrv { id, .. } => *id,
            };
            if !ids.insert(id) {
                self.r.error(format!("record ID {id} used twice"), key, vbn);
            }
            if id >= nxtrecid || id == 0 {
                self.r.error(format!("record ID {id} not below the next ID, {nxtrecid}"), key, vbn);
            }
            match item {
                Item::Rrv { ctrl, id, to } => {
                    if ctrl & irc::DELETED == 0 {
                        self.rrvs.push((vbn, id, to));
                    }
                }
                Item::Rec(r) => {
                    let live = r.ctrl & irc::DELETED == 0;
                    if live {
                        self.r.records += 1;
                        let len = r.rec.len();
                        match self.x.fix {
                            Some(size) if len != size => self.r.error(
                                format!("record ID {} is {len} bytes in a FIX {size} file", r.id),
                                key,
                                vbn,
                            ),
                            _ => {}
                        }
                        if len < k.minrecsz() as usize {
                            self.r.error(format!("record ID {} shorter than its primary key", r.id), key, vbn);
                        }
                    }
                    if self.by_rfa.insert(r.rfa, Placed { vbn, id: r.id, live, rec: r.rec }).is_some() {
                        self.r.error(format!("RFA ({},{}) used twice", r.rfa.0, r.rfa.1), key, vbn);
                    }
                    keys.push(r.key);
                }
            }
        }
        keys
    }

    /// Every SIDR pointer must lead to a record with that key, and every
    /// record that has the key must be pointed at once.
    fn coverage(&mut self, n: usize, pointers: &[(u32, u8, Rfa, Vec<u8>)]) {
        let k = self.x.keys[n];
        let key = Some(n as u8);
        let mut times: BTreeMap<Rfa, usize> = BTreeMap::new();
        let mut last_key: Option<&[u8]> = None;
        for (vbn, c, rfa, kv) in pointers {
            if c & (irc::DELETED | irc::NOPTRSZ) != 0 {
                continue;
            }
            if !k.has(keyflag::DUPKEYS) && last_key.is_some_and(|l| k.compare(l, kv) == Ordering::Equal) {
                self.r.error("duplicate key", key, *vbn);
            }
            last_key = Some(kv);
            match self.by_rfa.get(rfa) {
                None => self.r.error(format!("SIDR pointer to ({},{}), no record", rfa.0, rfa.1), key, *vbn),
                Some(p) if !p.live => self.r.error("SIDR pointer to a deleted record", key, *vbn),
                Some(p) => {
                    if k.key_of(&p.rec).is_none_or(|rk| k.compare(&rk, kv) != Ordering::Equal) {
                        self.r.error(format!("SIDR key differs from record ({},{})'s", rfa.0, rfa.1), key, *vbn);
                    }
                    *times.entry(*rfa).or_default() += 1;
                }
            }
        }
        let mut missing = 0;
        for (rfa, p) in &self.by_rfa {
            if !p.live || p.rec.len() < k.minrecsz() as usize {
                continue;
            }
            let null = k.has(keyflag::NULKEYS) && k.key_of(&p.rec).is_some_and(|kv| k.is_null(&kv));
            match times.get(rfa).copied().unwrap_or(0) {
                0 if null => {}
                0 => missing += 1,
                1 if !null => {}
                t => self.r.error(format!("record ({},{}) is in the index {t} times", rfa.0, rfa.1), key, p.vbn),
            }
        }
        if missing > 0 {
            let at = self.x.key_at[n].0;
            self.r.error(format!("{missing} records missing from the key's SIDRs"), key, at);
        }
    }

    /// RRVs must lead to the record with their RFA, and a record away from
    /// its RFA must have one there.
    fn rrvs(&mut self) {
        let key = Some(0);
        let mut found = BTreeSet::new();
        for &(vbn, id, to) in &self.rrvs {
            match self.by_rfa.get(&(vbn, id)) {
                Some(p) if (p.vbn, p.id) == to => {
                    found.insert((vbn, id));
                }
                _ => self.r.error(format!("RRV {id} doesn't lead to the record with its RFA"), key, vbn),
            }
        }
        for (rfa, p) in &self.by_rfa {
            if p.live && (p.vbn, p.id) != *rfa && !found.contains(rfa) {
                self.r.error(format!("record moved from ({},{}) with no RRV there", rfa.0, rfa.1), key, p.vbn);
            }
        }
    }
}
