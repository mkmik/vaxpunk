//! The dot lint (PRD-0004 *The dot lint*): a data name is its address and
//! `.X` its value, so a missing or extra dot compiles silently. The parser
//! calls these checks as it builds the expressions they look at, where it
//! knows the place:
//!
//! 1. a test of a scalar's address: `IF NOT STATUS THEN`;
//! 2. arithmetic or a comparison on a scalar's address, outside address
//!    arithmetic: `COUNT + 1`, `X GTR 5`;
//! 3. a fetch from a LITERAL or a routine's name: `.LIT`;
//! 4. a system service or run-time library argument passed the wrong way
//!    for its mechanism, by the table vdefs writes from the macro library:
//!    a scalar's address where it takes a value, a constant where it
//!    takes an address;
//! 5. a scalar's address stored in a scalar the module uses as a value:
//!    `Y = X`, then `.Y + 1`;
//! 6. an argument passed the other way from how the routine, in the same
//!    module, uses its formal: a scalar's address to a formal it uses as
//!    a value (`PRINT(X)`), a constant other than 0 to one it uses as an
//!    address. A formal is a value if the routine compares it,
//!    multiplies or divides it or tests it, and an address if it fetches
//!    through it, takes a field of it or maps it REF, but not both. Rules 5
//!    and 6 are checked at the end of the module, when the uses are known.
//!
//! A line ending with `! LINT: ADDRESS` is left alone. Lint warnings are
//! kept apart from the compiler's diagnostics and out of the listing.

use std::collections::HashMap;

use crate::lex::Lexeme;
use crate::listing::{Diag, NOPOS};
use crate::parse::{BOp, Expr, Kind, Parser, Storage};

/// How a routine takes an argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mechanism {
    Value,
    Address,
}

/// The mechanisms of the system services and run-time library routines,
/// from `services.txt` (vdefs): a line each, the routine's name and `V` or
/// `A` for each argument.
pub fn parse_services(text: &str) -> HashMap<String, Vec<Mechanism>> {
    text.lines()
        .filter(|l| !l.starts_with('!'))
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let name = words.next()?.to_string();
            let m = words
                .map(|w| {
                    if w == "V" {
                        Mechanism::Value
                    } else {
                        Mechanism::Address
                    }
                })
                .collect();
            Some((name, m))
        })
        .collect()
}

/// What a call passes, as rule 6 sees it.
#[derive(Clone, Debug)]
pub enum Passed {
    /// A scalar's address: its name.
    Address(String),
    /// A constant other than 0.
    Constant(i64),
    Other,
}

/// How a routine uses one of its formals, so far: as a value, as an
/// address.
#[derive(Clone, Copy, Debug, Default)]
pub struct Use {
    pub value: bool,
    pub address: bool,
}

impl Parser<'_> {
    /// Records a lint warning at `at`, unless its line says not to.
    pub(crate) fn lint(&mut self, at: &Lexeme, msg: String) {
        let quiet = self.lx.files[at.file as usize]
            .lines
            .get((at.line as usize).wrapping_sub(1))
            .is_some_and(|l| l.contains("! LINT: ADDRESS"));
        if !quiet {
            self.lints.push(Diag {
                sev: 'W',
                file: at.file,
                line: at.line,
                col: if at.col == NOPOS { 0 } else { at.col },
                msg,
            });
        }
    }

    /// The scalar data `e` names, if it is a scalar's name: its address.
    /// A BIND to a PLIT or a descriptor names no scalar.
    fn scalar(&self, e: &Expr) -> Option<&str> {
        let Expr::Name(id) = e else {
            return None;
        };
        if self.undeclared.contains(id) {
            return None;
        }
        let s = &self.m.syms[*id];
        if let Kind::Data {
            storage: Storage::Bind(i),
            ..
        } = s.kind
            && matches!(self.m.binds[i as usize], Expr::Plit(..) | Expr::Ascid(_))
        {
            return None;
        }
        match &s.kind {
            Kind::Data {
                structure: None,
                storage,
                ..
            } if !s.name.is_empty()
                && matches!(
                    storage,
                    Storage::Own
                        | Storage::Global
                        | Storage::External
                        | Storage::Local(_)
                        | Storage::Bind(_)
                ) =>
            {
                Some(&s.name)
            }
            _ => None,
        }
    }

    /// Notes that the data `e` fetches, `.F`, is used as a value or an
    /// address, for rule 6.
    pub(crate) fn note_use(&mut self, e: &Expr, address: bool) {
        let e = match e {
            Expr::Bin(BOp::Add | BOp::Sub, a, _) if address => a,
            e => e,
        };
        if let Expr::Fetch(inner) = e
            && let Expr::Name(id) = **inner
        {
            let id = self.mapped.get(&id).copied().unwrap_or(id);
            let u = self.uses.entry(id).or_default();
            if address {
                u.address = true;
            } else {
                u.value = true;
            }
        }
    }

    /// Rule 5: a scalar's address stored in a scalar, unless a macro's
    /// body stores it, as `$FAOL(OUTLEN = X)` passes X by reference.
    pub(crate) fn lint_store(&mut self, place: &Expr, v: &Expr, at: &Lexeme) {
        if self.address_context > 0 || at.col == NOPOS || self.scalar(place).is_none() {
            return;
        }
        if let (Some(n), Expr::Name(id)) = (self.scalar(v), place) {
            let id = self.mapped.get(id).copied().unwrap_or(*id);
            self.stores.push((id, n.to_string(), at.clone()));
        }
    }

    /// Rule 6, at a call: records what it passes, checked once the module
    /// has every routine's formals.
    pub(crate) fn note_call(&mut self, target: &Expr, args: &[Expr], at: &Lexeme) {
        let Expr::Name(id) = target else {
            return;
        };
        if !matches!(self.m.syms[*id].kind, Kind::Routine { .. }) {
            return;
        }
        let passed = args
            .iter()
            .map(|a| match (self.scalar(a), crate::parse::fold(a)) {
                (Some(n), _) => Passed::Address(n.to_string()),
                (None, Some(v)) if v != 0 => Passed::Constant(v),
                _ => Passed::Other,
            })
            .collect();
        self.calls.push((*id, passed, at.clone()));
    }

    /// Rules 5 and 6, once the module is parsed: each store of an address
    /// and each call's arguments against how the data is used.
    pub(crate) fn lint_formals(&mut self) {
        for (id, n, at) in std::mem::take(&mut self.stores) {
            let u = self.uses.get(&id).copied().unwrap_or_default();
            if u.value && !u.address {
                let msg = format!("stores the address of {n}, not its value: a missing dot?");
                self.lint(&at, msg);
            }
        }
        let formals: HashMap<usize, Vec<usize>> = self
            .m
            .routines
            .iter()
            .map(|r| (r.sym, r.formals.clone()))
            .collect();
        for (id, passed, at) in std::mem::take(&mut self.calls) {
            let Some(fs) = formals.get(&id) else {
                continue;
            };
            let routine = self.m.syms[id].name.clone();
            for (i, (p, f)) in passed.iter().zip(fs).enumerate() {
                let u = self.uses.get(f).copied().unwrap_or_default();
                let formal = &self.m.syms[*f].name;
                let msg = match p {
                    Passed::Address(n) if u.value && !u.address => format!(
                        "passes the address of {n} as argument {} of {routine}, which uses {formal} as a value: a missing dot?",
                        i + 1
                    ),
                    Passed::Constant(v) if u.address && !u.value => format!(
                        "passes {v} as argument {} of {routine}, which uses {formal} as an address",
                        i + 1
                    ),
                    _ => continue,
                };
                self.lint(&at, msg);
            }
        }
    }

    /// Rule 1: a test of a scalar's address.
    pub(crate) fn lint_test(&mut self, c: &Expr, at: &Lexeme) {
        let mut found = None;
        let mut tested = Vec::new();
        let mut look = |e: &Expr, p: &Self| {
            let mut e = e;
            while let Expr::Not(inner) = e {
                e = inner;
            }
            tested.push(e.clone());
            if let Some(n) = p.scalar(e) {
                found.get_or_insert(n.to_string());
            }
        };
        match c {
            Expr::Bin(BOp::And | BOp::Or | BOp::Xor | BOp::Eqv, a, b) => {
                look(a, self);
                look(b, self);
            }
            c => look(c, self),
        }
        for e in tested {
            self.note_use(&e, false);
        }
        if let Some(n) = found {
            self.lint(
                at,
                format!("tests the address of {n}, not its value: a missing dot?"),
            );
        }
    }

    /// Rule 2: arithmetic or a comparison on a scalar's address.
    pub(crate) fn lint_operands(&mut self, op: BOp, a: &Expr, b: &Expr, at: &Lexeme) {
        if self.address_context > 0 || matches!(op, BOp::And | BOp::Or | BOp::Xor | BOp::Eqv) {
            return;
        }
        // An address is added to and subtracted from, but not compared,
        // multiplied or divided.
        if matches!(op, BOp::Rel(..) | BOp::Mul | BOp::Div | BOp::Mod) {
            self.note_use(a, false);
            self.note_use(b, false);
        }
        if let Some(n) = self.scalar(a).or_else(|| self.scalar(b)) {
            let what = if matches!(op, BOp::Rel(..)) {
                "compares"
            } else {
                "does arithmetic on"
            };
            let msg = format!("{what} the address of {n}, not its value: a missing dot?");
            self.lint(at, msg);
        }
    }

    /// Rule 3: a fetch from a literal's or a routine's name.
    pub(crate) fn lint_fetch(&mut self, operand: &Expr, start: Option<usize>, at: &Lexeme) {
        let name = match start.map(|id| (&self.m.syms[id].name, &self.m.syms[id].kind)) {
            Some((n, Kind::Literal(_) | Kind::Compiletime(_)))
                if matches!(operand, Expr::Num(_)) =>
            {
                format!("LITERAL {n}")
            }
            _ => match operand {
                Expr::Name(id) if matches!(self.m.syms[*id].kind, Kind::Routine { .. }) => {
                    format!("routine {}", self.m.syms[*id].name)
                }
                _ => return,
            },
        };
        self.lint(at, format!("fetches from {name}: an extra dot?"));
    }

    /// Rule 4: a call's arguments against its routine's mechanisms.
    pub(crate) fn lint_call(&mut self, target: &Expr, args: &[Expr], at: &Lexeme) {
        let Expr::Name(id) = target else {
            return;
        };
        let routine = self.m.syms[*id].asm.clone();
        if !routine.contains('$') {
            return;
        }
        if self.services.is_none() {
            let table = (self.load)("SERVICES.TXT").unwrap_or_default();
            self.services = Some(parse_services(&table));
        }
        let Some(mechs) = self.services.as_ref().unwrap().get(&routine).cloned() else {
            return;
        };
        for (i, (a, m)) in args.iter().zip(mechs).enumerate() {
            let msg = match (m, self.scalar(a), crate::parse::fold(a)) {
                (Mechanism::Value, Some(n), _) => format!(
                    "passes the address of {n} as argument {} of {routine}, which takes a value: a missing dot?",
                    i + 1
                ),
                (Mechanism::Address, None, Some(v)) if v != 0 => format!(
                    "passes {v} as argument {} of {routine}, which takes an address",
                    i + 1
                ),
                _ => continue,
            };
            self.lint(at, msg);
        }
    }
}
