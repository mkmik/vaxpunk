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
//!    takes an address.
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
    fn scalar(&self, e: &Expr) -> Option<&str> {
        let Expr::Name(id) = e else {
            return None;
        };
        if self.undeclared.contains(id) {
            return None;
        }
        let s = &self.m.syms[*id];
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

    /// Rule 1: a test of a scalar's address.
    pub(crate) fn lint_test(&mut self, c: &Expr, at: &Lexeme) {
        let mut found = None;
        let mut look = |e: &Expr, p: &Self| {
            let mut e = e;
            while let Expr::Not(inner) = e {
                e = inner;
            }
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
