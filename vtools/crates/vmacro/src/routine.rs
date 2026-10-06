//! Routine declarations, AMACRO's (`vtools/docs/amacro.md`), and what each
//! routine saves. vmacro reads a module twice: the first pass surveys each
//! routine, the registers it writes and the JSB routines it calls; the
//! second compiles it, saving what the survey says.

use std::collections::{HashMap, HashSet};

use crate::operand;

/// VAX registers R0-R11, bit n for Rn.
pub type Regs = u16;

/// R2-R11: what a routine keeps for its caller.
pub const KEPT: Regs = 0xffc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Kind {
    /// `.CALL_ENTRY`, or `.ENTRY` with its register mask.
    #[default]
    Call,
    /// `.JSB_ENTRY`: keeps all 64 bits of what it modifies but its outputs.
    Jsb,
    /// `.JSB32_ENTRY`: keeps nothing it isn't told to.
    Jsb32,
    /// `.EXCEPTION_ENTRY`: entered by the PAL or by `REI`, not called. The
    /// PAL's frame keeps every register, so it saves none.
    Exception,
}

/// A routine as declared, and what the survey found in it.
#[derive(Clone, Debug, Default)]
pub struct Routine {
    pub kind: Kind,
    /// `.ENTRY`'s register mask: always saved, and the registers it may write.
    pub mask: Option<Regs>,
    pub output: Regs,
    pub scratch: Regs,
    pub preserve: Regs,
    pub home_args: Option<bool>,
    pub quad_args: bool,
    pub max_args: Option<u32>,
    /// The registers its own instructions write.
    pub direct: Regs,
    /// How it reads its argument list, through AP: the arguments its own
    /// fixed offsets reach, and whether it uses AP as a list (an address,
    /// indexed or offset by a variable), which takes them all.
    pub ap_args: Option<u32>,
    pub ap_list: bool,
    /// Whether it calls: JSB, BSBx, CALLS or CALLG; and whether it uses SP
    /// or FP.
    pub calls_out: bool,
    pub stacked: bool,
    /// Whether a JSB routine takes its caller's argument list, `INPUT=<AP>`,
    /// and whether a routine calls it by name, in the module or in another
    /// compiled with it, once solved.
    pub takes_ap: bool,
    pub called_here: bool,
    /// The JSB routines it calls by name, with the linkage `.USE_LINKAGE`
    /// gave the call.
    pub calls: Vec<(String, Option<Regs>)>,
    /// What the JSB routines it calls through an address modify, by their
    /// linkages: all of R2-R11 for one without.
    pub elsewhere: Regs,
    /// The labels it branches to by name. One in another routine of the
    /// module, not its entry, starts code it shares: it modifies what that
    /// routine does. An entry is a tail call, which restores its own.
    pub branches: Vec<String>,
    /// What it saves and restores, and what it modifies, through the
    /// routines it calls, once solved.
    pub saved: Regs,
    pub modified: Regs,
    /// The arguments a CALL routine copies into its frame, which its AP
    /// points at, once solved: those it and the JSB routines it calls in
    /// the module read, or all up to `max_args` with `home_args` or a list.
    /// None if nothing reads AP.
    pub home: Option<u32>,
    /// Whether its code, or code it shares, reads AP, which it then keeps
    /// in x12, once solved, and what it and the JSB routines it calls read.
    pub ap: bool,
    pub need: (Option<u32>, bool),
    /// Whether a JSB routine saves x30, once solved: if it calls, or code
    /// it shares with another routine does, whose RSB restores it.
    pub lr: bool,
    /// Whether a CALL routine is frameless, once solved: it saves nothing,
    /// homes nothing, calls nothing, leaves SP and FP alone and shares no
    /// code (DESIGN-0004).
    pub frameless: bool,
}

impl Routine {
    /// What a JSB to it modifies among R2-R11, as its callers see it.
    fn effect(&self, modified: Regs) -> Regs {
        let changes = match self.kind {
            Kind::Jsb => self.output | self.scratch,
            Kind::Jsb32 => modified,
            Kind::Call | Kind::Exception => 0,
        };
        changes & !self.preserve & KEPT
    }
}

/// A linkage: what a JSB routine in another module modifies.
pub fn linkage_effect(output: Regs, scratch: Regs, preserve: Regs) -> Regs {
    (output | scratch) & !preserve & KEPT
}

/// What the first pass learned about a module.
#[derive(Default)]
pub struct Survey {
    pub routines: Vec<Routine>,
    /// Each declared routine's names: the labels on its declaration.
    pub entries: HashMap<String, usize>,
    /// Every label the module defines, and the routine it is in.
    pub labels: HashMap<String, Option<usize>>,
    /// `.GLOBAL_LABEL`s: labels other routines may branch to.
    pub globals: HashSet<String>,
    /// `.DEFINE_LINKAGE`s, by name, and `.CALL_LINKAGE`s, by routine.
    pub linkages: HashMap<String, Regs>,
    pub called: HashMap<String, Regs>,
    /// The global labels it defines, `NAME::`, which other modules call.
    pub exports: HashSet<String>,
    /// `.EXCEPTION_ENTRY` routines other modules export: they never return,
    /// so any routine may go to one.
    pub noreturn: HashSet<String>,
    /// Labels on code that loads SP from elsewhere: a long jump, which any
    /// routine may make, leaving the stack of those it is in.
    pub longjumps: HashSet<String>,
    /// Each label other modules export: whether it is a routine's entry,
    /// and how the routine it is in returns.
    pub foreign: HashMap<String, (bool, Returns)>,
    /// What the JSB routines other modules export read through AP, as
    /// `Routine::need`: their callers here home it.
    pub reads: HashMap<String, (Option<u32>, bool)>,
}

impl Survey {
    /// What each routine modifies, through the JSB routines it calls, and
    /// so what it saves.
    pub fn solve(&mut self) {
        self.solve_home();
        self.solve_lr();
        let modified = self.modified();
        for (r, m) in self.routines.iter_mut().zip(modified) {
            r.modified = m;
            let unless = r.output | r.scratch;
            r.saved = match r.kind {
                Kind::Call => ((r.mask.unwrap_or(0) | m) & KEPT & !unless) | r.preserve,
                Kind::Jsb => (m & KEPT & !unless) | r.preserve,
                Kind::Jsb32 => r.preserve,
                Kind::Exception => 0,
            };
        }
        let mut sharing = vec![false; self.routines.len()];
        for i in 0..self.routines.len() {
            for c in self.shared(i) {
                (sharing[i], sharing[c]) = (true, true);
            }
        }
        for (r, sharing) in self.routines.iter_mut().zip(sharing) {
            r.frameless = r.kind == Kind::Call
                && r.saved == 0
                && r.home.is_none()
                && !r.ap
                && !r.calls_out
                && !r.stacked
                && !sharing;
        }
    }

    /// How many arguments each CALL routine homes: what it and the JSB
    /// routines it calls, in this module, read through AP. Routines that
    /// share code have one frame, so they home alike.
    fn solve_home(&mut self) {
        let n = self.routines.len();
        // (arguments reached, all of them) for each routine, through its
        // JSB routines.
        let mut need: Vec<(Option<u32>, bool)> = self
            .routines
            .iter()
            .map(|r| (r.ap_args, r.ap_list))
            .collect();
        loop {
            let mut changed = false;
            for i in 0..n {
                let (mut args, mut list) = need[i];
                for (name, _) in &self.routines[i].calls {
                    let theirs = match self.entries.get(name) {
                        Some(&c) if self.routines[c].kind != Kind::Call => Some(need[c]),
                        Some(_) => None,
                        None => self.reads.get(name).copied(),
                    };
                    if let Some((a, l)) = theirs {
                        args = args.max(a);
                        list |= l;
                    }
                }
                for c in self.shared(i) {
                    args = args.max(need[c].0);
                    list |= need[c].1;
                    if need[c] != (args, list) {
                        need[c] = (args, list);
                        changed = true;
                    }
                }
                if (args, list) != need[i] {
                    need[i] = (args, list);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let called: Vec<usize> = self
            .routines
            .iter()
            .flat_map(|r| &r.calls)
            .filter_map(|(name, _)| self.entries.get(name).copied())
            .collect();
        for c in called {
            self.routines[c].called_here = true;
        }
        let mut reads: Vec<bool> = self
            .routines
            .iter()
            .map(|r| r.ap_args.is_some() || r.ap_list)
            .collect();
        loop {
            let mut changed = false;
            for i in 0..n {
                for c in self.shared(i) {
                    if reads[i] != reads[c] {
                        (reads[i], reads[c]) = (true, true);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for (r, ap) in self.routines.iter_mut().zip(reads) {
            r.ap = ap;
        }
        for (r, (args, list)) in self.routines.iter_mut().zip(need) {
            r.need = (args, list);
            let all = (list || r.home_args == Some(true)).then(|| r.max_args.unwrap_or(8));
            r.home = match (args, all) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(0).max(b.unwrap_or(0))),
            };
        }
        loop {
            let mut changed = false;
            for i in 0..n {
                for c in self.shared(i) {
                    let home = self.routines[i].home.max(self.routines[c].home);
                    if (self.routines[i].home, self.routines[c].home) != (home, home) {
                        (self.routines[i].home, self.routines[c].home) = (home, home);
                        changed = true;
                    }
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// The other routines routine `i` shares code with: those it branches
    /// into the middle of, but for long jumps.
    fn shared(&self, i: usize) -> Vec<usize> {
        self.routines[i]
            .branches
            .iter()
            .filter(|name| !self.longjumps.contains(*name) && !self.entries.contains_key(*name))
            .filter_map(|name| *self.labels.get(name)?)
            .filter(|&c| c != i)
            .collect()
    }

    /// Which JSB routines save x30: those that call, and those that share
    /// code with one that does, which must build the same save area.
    fn solve_lr(&mut self) {
        let jsb = |r: &Routine| matches!(r.kind, Kind::Jsb | Kind::Jsb32);
        for r in self.routines.iter_mut() {
            r.lr = jsb(r) && r.calls_out;
        }
        loop {
            let mut changed = false;
            for i in 0..self.routines.len() {
                for name in &self.routines[i].branches {
                    if let Some(Some(c)) = self.labels.get(name)
                        && *c != i
                        && !self.longjumps.contains(name)
                        && !self.entries.contains_key(name)
                        && jsb(&self.routines[i])
                        && jsb(&self.routines[*c])
                        && self.routines[i].lr != self.routines[*c].lr
                    {
                        changed = true;
                        (self.routines[i].lr, self.routines[*c].lr) = (true, true);
                        break;
                    }
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// What each routine modifies, through the JSB routines it calls: all
    /// of R2-R11 for one elsewhere whose linkage it doesn't know.
    fn modified(&self) -> Vec<Regs> {
        let n = self.routines.len();
        let mut modified: Vec<Regs> = self
            .routines
            .iter()
            .map(|r| r.direct | r.elsewhere)
            .collect();
        loop {
            let mut changed = false;
            for i in 0..n {
                let mut m = modified[i];
                for (name, linkage) in &self.routines[i].calls {
                    m |= match self.entries.get(name) {
                        Some(&c) => self.routines[c].effect(modified[c]),
                        // Not a routine: an error the second pass reports.
                        None if self.labels.contains_key(name) => 0,
                        None => linkage
                            .or_else(|| self.called.get(name).copied())
                            .unwrap_or(KEPT),
                    };
                }
                for name in &self.routines[i].branches {
                    if let Some(Some(c)) = self.labels.get(name)
                        && *c != i
                        && !self.longjumps.contains(name)
                        && !self.entries.contains_key(name)
                    {
                        m |= modified[*c];
                    }
                }
                if m != modified[i] {
                    modified[i] = m;
                    changed = true;
                }
            }
            if !changed {
                return modified;
            }
        }
    }

    /// What each JSB routine it exports reads through AP, once solved, if
    /// anything.
    pub fn reads(&self) -> impl Iterator<Item = (&String, (Option<u32>, bool))> {
        self.exports.iter().filter_map(|name| {
            let r = &self.routines[*self.entries.get(name)?];
            (r.kind != Kind::Call && r.need != (None, false)).then_some((name, r.need))
        })
    }

    /// What a JSB to each routine it exports modifies, once solved.
    pub fn effects(&self) -> impl Iterator<Item = (&String, Regs)> {
        self.exports.iter().filter_map(|name| {
            let r = &self.routines[*self.entries.get(name)?];
            Some((name, r.effect(r.modified)))
        })
    }
}

/// Solves modules compiled together: a JSB from one to a routine another
/// exports modifies what that routine's declaration says, as if each had
/// a `.CALL_LINKAGE` for it. Explicit linkages win.
// ponytail: names live in one space; programs linked apart that export the
// same name get the union of their effects.
pub fn solve_together(surveys: &mut [Survey]) {
    let explicit: Vec<HashSet<String>> = surveys
        .iter()
        .map(|s| s.called.keys().cloned().collect())
        .collect();
    let noreturn: HashSet<String> = surveys
        .iter()
        .flat_map(|s| {
            s.exports
                .iter()
                .filter(|n| {
                    s.entries
                        .get(*n)
                        .is_some_and(|&r| s.routines[r].kind == Kind::Exception)
                })
                .cloned()
        })
        .collect();
    for s in surveys.iter_mut() {
        s.noreturn = noreturn.clone();
    }
    let mut known: HashMap<String, Regs> = HashMap::new();
    let mut reads: HashMap<String, (Option<u32>, bool)> = HashMap::new();
    loop {
        for (s, explicit) in surveys.iter_mut().zip(&explicit) {
            for (name, effect) in &known {
                if !explicit.contains(name) && !s.labels.contains_key(name) {
                    s.called.insert(name.clone(), *effect);
                }
            }
            s.reads = reads.clone();
            s.solve();
        }
        // Two that export the same name: either may be the one called.
        let mut now: HashMap<String, Regs> = HashMap::new();
        for (name, effect) in surveys.iter().flat_map(|s| s.effects()) {
            *now.entry(name.clone()).or_default() |= effect;
        }
        let mut now_reads: HashMap<String, (Option<u32>, bool)> = HashMap::new();
        for (name, (a, l)) in surveys.iter().flat_map(|s| s.reads()) {
            let e = now_reads.entry(name.clone()).or_default();
            *e = (e.0.max(a), e.1 || l);
        }
        if now == known && now_reads == reads {
            break;
        }
        (known, reads) = (now, now_reads);
    }
    // A JSB routine another module calls has a caller that homes for it.
    let called: HashSet<String> = surveys
        .iter()
        .flat_map(|s| s.routines.iter().flat_map(|r| &r.calls))
        .map(|(name, _)| name.clone())
        .collect();
    for s in surveys.iter_mut() {
        for (name, &i) in &s.entries {
            if called.contains(name) {
                s.routines[i].called_here = true;
            }
        }
    }
    let foreign: HashMap<String, (bool, Returns)> = surveys
        .iter()
        .flat_map(|s| {
            s.exports.iter().filter_map(|n| {
                let r = &s.routines[(*s.labels.get(n)?)?];
                let entry = s.entries.contains_key(n);
                Some((n.clone(), (entry, r.returns())))
            })
        })
        .collect();
    for s in surveys.iter_mut() {
        s.foreign = foreign.clone();
    }
}

/// How a routine returns, which code that branches into its middle must
/// match: whether with RET, what it restores, x30 too, and the argument
/// list in its frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Returns {
    pub call: bool,
    pub saved: Regs,
    pub lr: bool,
    pub home: Option<u32>,
    pub frameless: bool,
}

impl Routine {
    pub fn returns(&self) -> Returns {
        Returns {
            call: self.kind == Kind::Call,
            saved: self.saved,
            lr: self.lr,
            home: self.home,
            frameless: self.frameless,
        }
    }
}

/// A declaration's parameters: `key=value`, each value a register set
/// `<R2,R3>`, a number or TRUE or FALSE.
pub struct Params {
    pub positional: Vec<String>,
    pub keyed: Vec<(String, String)>,
}

impl Params {
    pub fn parse(rest: &str) -> Params {
        let mut p = Params {
            positional: Vec::new(),
            keyed: Vec::new(),
        };
        for arg in operand::split(rest) {
            match arg.split_once('=') {
                Some((k, v)) => p
                    .keyed
                    .push((k.trim().to_ascii_uppercase(), v.trim().to_string())),
                None if !arg.is_empty() => p.positional.push(arg),
                None => {}
            }
        }
        p
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.keyed
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// A register set: R0-R11, and AP in INPUT, bit 12, for a JSB routine
    /// that takes its caller's argument list.
    pub fn regs(&self, key: &str) -> Result<Regs, String> {
        let set = self.get(key).map_or(Ok(0), regs)?;
        if set & 1 << 12 != 0 && key != "INPUT" {
            return Err(format!("AP can only be an INPUT, not {key}"));
        }
        Ok(set)
    }

    pub fn flag(&self, key: &str) -> Result<Option<bool>, String> {
        match self.get(key).map(str::to_ascii_uppercase).as_deref() {
            None => Ok(None),
            Some("TRUE" | "YES") => Ok(Some(true)),
            Some("FALSE" | "NO") => Ok(Some(false)),
            Some(v) => Err(format!("{key} is TRUE or FALSE, not {v}")),
        }
    }

    pub fn number(&self, key: &str) -> Result<Option<i64>, String> {
        self.get(key)
            .map(|v| {
                v.trim_end_matches('.')
                    .parse()
                    .map_err(|_| format!("{key} must be a number"))
            })
            .transpose()
    }

    /// Every key must be one of `known`.
    pub fn check(&self, known: &[&str]) -> Result<(), String> {
        match self
            .keyed
            .iter()
            .find(|(k, _)| !known.contains(&k.as_str()))
        {
            Some((k, _)) => Err(format!("unknown parameter {k}")),
            None => Ok(()),
        }
    }
}

/// A register set, `<R2,R3>` or a single register: R0-R11 only.
pub fn regs(text: &str) -> Result<Regs, String> {
    let t = text.trim();
    let inner = t
        .strip_prefix('<')
        .and_then(|t| t.strip_suffix('>'))
        .unwrap_or(t);
    let mut set = 0;
    for name in inner.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match operand::reg(name) {
            Some(n) if n <= 12 => set |= 1 << n,
            _ => return Err(format!("{name} isn't one of R0-R11 or AP")),
        }
    }
    Ok(set)
}

/// The registers in `set`, R0 first.
pub fn list(set: Regs) -> Vec<u8> {
    (0..12).filter(|r| set & 1 << r != 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine(kind: Kind) -> Routine {
        Routine {
            kind,
            ..Routine::default()
        }
    }

    /// A CALL routine saves what it and its JSB routines modify, but what
    /// a .JSB_ENTRY routine keeps; a .JSB32_ENTRY routine keeps nothing.
    #[test]
    fn solve() {
        let mut s = Survey::default();
        let mut call = routine(Kind::Call);
        call.mask = Some(1 << 2);
        call.direct = 1 << 3 | 1;
        call.calls = vec![("J".into(), None), ("J32".into(), None)];
        let mut j = routine(Kind::Jsb);
        j.direct = 1 << 4 | 1 << 5;
        j.output = 1 << 5;
        let mut j32 = routine(Kind::Jsb32);
        j32.direct = 1 << 6;
        j32.calls = vec![("J".into(), None)];
        s.routines = vec![call, j, j32];
        s.entries = [("J".into(), 1), ("J32".into(), 2)].into();
        s.solve();
        assert_eq!(list(s.routines[0].saved), [2, 3, 5, 6]);
        assert_eq!(list(s.routines[1].saved), [4]);
        assert_eq!(list(s.routines[2].saved), Vec::<u8>::new());
    }

    /// A JSB to another module modifies all of R2-R11 but what its linkage
    /// says it doesn't.
    #[test]
    fn elsewhere() {
        let mut s = Survey::default();
        let mut call = routine(Kind::Call);
        call.calls = vec![("EXT".into(), None), ("LINKED".into(), Some(1 << 7))];
        let mut j = routine(Kind::Jsb);
        j.calls = vec![("LINKED".into(), Some(1 << 7))];
        j.scratch = 1 << 2;
        s.routines = vec![call, j];
        s.solve();
        assert_eq!(s.routines[0].saved, KEPT);
        assert_eq!(list(s.routines[1].saved), [7]);
    }

    /// A CALL routine homes what its JSB routines read through AP, in its
    /// module or in another compiled with it; code it shares with another
    /// routine has that one's list.
    #[test]
    fn homes() {
        let mut lib = Survey::default();
        let mut get = routine(Kind::Jsb);
        get.ap_args = Some(3);
        lib.routines = vec![get];
        lib.entries = [("GET".into(), 0)].into();
        lib.exports = ["GET".into()].into();
        let mut main = Survey::default();
        let mut call = routine(Kind::Call);
        call.calls = vec![("GET".into(), None)];
        let mut first = routine(Kind::Call);
        first.branches = vec!["TAIL".into()];
        let mut last = routine(Kind::Call);
        last.ap_list = true;
        last.max_args = Some(10);
        main.routines = vec![call, first, last];
        main.entries = [("CALL".into(), 0), ("FIRST".into(), 1), ("LAST".into(), 2)].into();
        main.labels = [("TAIL".into(), Some(2))].into();
        let mut surveys = [lib, main];
        solve_together(&mut surveys);
        let main = &surveys[1];
        assert_eq!(main.routines[0].home, Some(3));
        assert_eq!(
            (main.routines[1].home, main.routines[1].ap),
            (Some(10), true)
        );
        assert!(surveys[0].routines[0].called_here);
    }

    #[test]
    fn register_sets() {
        assert_eq!(regs("<R2, R3>"), Ok(0b1100));
        assert_eq!(regs("R11"), Ok(1 << 11));
        assert_eq!(regs("<AP>"), Ok(1 << 12));
        assert!(regs("<FP>").is_err());
        let p = Params::parse("INPUT=<AP>, OUTPUT=<AP>");
        assert_eq!(p.regs("INPUT"), Ok(1 << 12));
        assert!(p.regs("OUTPUT").is_err());
    }
}
