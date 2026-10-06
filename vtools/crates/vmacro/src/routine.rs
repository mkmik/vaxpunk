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
    /// The registers its own instructions write.
    pub direct: Regs,
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
    /// and how the routine it is in returns: whether it is a CALL routine,
    /// and what it restores.
    pub foreign: HashMap<String, (bool, bool, Regs)>,
}

impl Survey {
    /// What each routine modifies, through the JSB routines it calls, and
    /// so what it saves.
    pub fn solve(&mut self) {
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
    loop {
        for (s, explicit) in surveys.iter_mut().zip(&explicit) {
            for (name, effect) in &known {
                if !explicit.contains(name) && !s.labels.contains_key(name) {
                    s.called.insert(name.clone(), *effect);
                }
            }
            s.solve();
        }
        // Two that export the same name: either may be the one called.
        let mut now: HashMap<String, Regs> = HashMap::new();
        for (name, effect) in surveys.iter().flat_map(|s| s.effects()) {
            *now.entry(name.clone()).or_default() |= effect;
        }
        if now == known {
            break;
        }
        known = now;
    }
    let foreign: HashMap<String, (bool, bool, Regs)> = surveys
        .iter()
        .flat_map(|s| {
            s.exports.iter().filter_map(|n| {
                let r = &s.routines[(*s.labels.get(n)?)?];
                let entry = s.entries.contains_key(n);
                Some((n.clone(), (entry, r.kind == Kind::Call, r.saved)))
            })
        })
        .collect();
    for s in surveys.iter_mut() {
        s.foreign = foreign.clone();
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

    pub fn regs(&self, key: &str) -> Result<Regs, String> {
        self.get(key).map_or(Ok(0), regs)
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
            Some(n) if n < 12 => set |= 1 << n,
            _ => return Err(format!("{name} isn't one of R0-R11")),
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

    #[test]
    fn register_sets() {
        assert_eq!(regs("<R2, R3>"), Ok(0b1100));
        assert_eq!(regs("R11"), Ok(1 << 11));
        assert!(regs("<AP>").is_err());
    }
}
