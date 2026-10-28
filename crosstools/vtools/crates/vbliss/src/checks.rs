//! The checks `/CHECK`, the `CHECK_x` switches and `LANGUAGE` ask for
//! (RN 2.19.2 to 2.19.8, 2.30; LRM appendix C), with BLISSA64's messages
//! and severities (oracle CHECKS, CHK*, LANG*): the parser calls them as
//! it builds the declarations and expressions they look at.

use crate::lex::Lexeme;
use crate::parse::{Expr, Kind, Parser, Storage};

/// The dialects LANGUAGE names that the compiler in use is (LRM 18.2.5).
fn dialect_names(a32: bool) -> &'static [&'static str] {
    if a32 {
        &["BLISS32", "BLISS32E", "BLISS32M"]
    } else {
        &["BLISS64E"]
    }
}

/// What LANGUAGE's list asks the compiler to check: 2 for full
/// transportability checking, 1 for the BLISS-16/BLISS-32 subset, 0 for
/// none (LRM C).
pub fn language_level(list: &[String]) -> i64 {
    let has = |n: &str| list.iter().any(|l| l == n);
    if has("COMMON") || has("BLISS36") && (has("BLISS16") || has("BLISS32")) {
        2
    } else if has("BLISS16") && has("BLISS32") {
        1
    } else {
        0
    }
}

impl Parser<'_> {
    /// `LANGUAGE(list)`, at `at`, in the module head or a block: the
    /// checking it asks for, from here to the block's end.
    pub(crate) fn set_language(&mut self, list: &[String], at: &Lexeme) {
        let names = dialect_names(self.m.dialect.a32);
        if !list.is_empty()
            && !list
                .iter()
                .any(|l| l == "COMMON" || names.contains(&l.as_str()))
        {
            let w = if self.m.dialect.a32 {
                "BLISS-32E"
            } else {
                "BLISS-64E"
            };
            self.diag(
                'W',
                at,
                format!("LANGUAGE switch specification excludes {w}"),
            );
        }
        let id = self.m.syms.len();
        self.m.syms.push(crate::parse::Sym {
            name: "%LANGUAGE".into(),
            asm: String::new(),
            kind: Kind::Literal(language_level(list)),
        });
        self.scopes
            .last_mut()
            .unwrap()
            .insert("%LANGUAGE".into(), id);
    }

    /// LANGUAGE's warning of `feature` at `at`: under full checking, or
    /// under the subset's too if `subset` says it checks it.
    pub(crate) fn transport(&mut self, feature: &str, at: &Lexeme, subset: bool) {
        let level = match self.lookup("%LANGUAGE").map(|id| &self.m.syms[id].kind) {
            Some(Kind::Literal(l)) => *l,
            _ => language_level(&self.opts.language),
        };
        if level == 2 || level == 1 && subset {
            let msg = if feature.is_empty() {
                "Language feature not transportable".to_string()
            } else {
                format!("Language feature not transportable: {feature}")
            };
            self.diag('W', at, msg);
        }
    }

    /// LANGUAGE's check of a GLOBAL or EXTERNAL name, at `at`: unique in
    /// its first six characters.
    pub(crate) fn transport_global(&mut self, name: &str, at: &Lexeme) {
        let six: String = name.chars().take(6).collect();
        let clash = self
            .global_names
            .iter()
            .any(|g| g != name && g.chars().take(6).eq(six.chars()));
        if !self.global_names.iter().any(|g| g == name) {
            self.global_names.push(name.to_string());
        }
        if clash {
            let level = match self.lookup("%LANGUAGE").map(|id| &self.m.syms[id].kind) {
                Some(Kind::Literal(l)) => *l,
                _ => language_level(&self.opts.language),
            };
            if level > 0 {
                let msg = format!("GLOBAL or EXTERNAL name not unique in 6 characters: {name}");
                self.diag('W', at, msg);
            }
        }
    }

    /// CHECK_LONGWORD, at a data declaration whose attributes are `LONG`
    /// with neither SIGNED nor UNSIGNED (RN 2.19.5).
    pub(crate) fn check_long(&mut self, a: &crate::data::Attrs) {
        if a.unit == Some(4)
            && a.signed.is_none()
            && a.structure.is_none()
            && self.switch_on("CHECK_LONGWORD")
        {
            let at = self.here();
            self.diag(
                'W',
                &at,
                "The LONGWORD attribute defaults to UNSIGNED".into(),
            );
        }
    }

    /// CHECK_ALIGNMENT, at static data placed `off` bytes into its psect
    /// by an ALIGN(`align`) less than its natural alignment, `natural`:
    /// a warning if it is misaligned, else a note (RN 2.19.4).
    pub(crate) fn check_align(&mut self, off: u32, align: u8, natural: u8) {
        if align >= natural || !self.switch_on("CHECK_ALIGNMENT") {
            return;
        }
        let at = self.here();
        if off.is_multiple_of(1 << natural) {
            let msg = "Alignment less than default may cause reduced performance";
            self.diag('I', &at, msg.into());
        } else {
            let msg = "Alignment less than default will cause reduced performance";
            self.diag('W', &at, msg.into());
        }
    }

    /// CHECK_ALIGNMENT, at a structure reference `e` whose field is a
    /// word, longword or quadword not at a multiple of its size from the
    /// data's start, as BLOCK[1, 8, 16, 0] (oracle STRUC10).
    pub(crate) fn check_field_align(&mut self, e: &Expr, at: &Lexeme) {
        let Expr::Field(addr, pos, size, _) = e else {
            return;
        };
        let (Some(pos), Some(size)) = (crate::parse::fold(pos), crate::parse::fold(size)) else {
            return;
        };
        let name = match size {
            16 => "Word",
            32 => "Longword",
            64 => "Quadword",
            _ => return,
        };
        // The constant part of the address: the field's byte offset.
        fn offset(e: &Expr) -> i64 {
            match e {
                Expr::Num(n) => *n,
                Expr::Bin(crate::parse::BOp::Add, a, b) => offset(a) + offset(b),
                Expr::Bin(crate::parse::BOp::Sub, a, b) => offset(a) - offset(b),
                _ => 0,
            }
        }
        if (offset(addr) * 8 + pos).rem_euclid(size) != 0 && self.switch_on("CHECK_ALIGNMENT") {
            self.diag(
                'W',
                at,
                format!("{name} field reference is not {name} aligned"),
            );
        }
    }

    /// CHECK_SHARE, at an INITIAL value that is an address, which the
    /// image activator must fix up, in psect `psect` (RN 2.19.6).
    pub(crate) fn check_share(&mut self, psect: &str, at: &Lexeme) {
        let shared = self
            .m
            .psect_attrs
            .iter()
            .any(|(n, a)| n == psect && a.split(", ").any(|w| w == "SHR"));
        if shared && self.switch_on("CHECK_SHARE") {
            self.diag(
                'W',
                at,
                format!("Non-shareable value in SHARE PSECT {psect}"),
            );
        }
    }

    /// CHECK_REDECLARE, as `name` is declared in a block inside another
    /// that declares it (RN 2.30).
    pub(crate) fn check_redeclare(&mut self, name: &str) {
        let n = self.scopes.len();
        if n < 3 || !self.switch_on("CHECK_REDECLARE") {
            return;
        }
        let outer = self.scopes[1..n - 1]
            .iter()
            .rev()
            .find_map(|s| s.get(name).copied());
        let Some(id) = outer else {
            return;
        };
        let what = match &self.m.syms[id].kind {
            Kind::Data { storage, .. } => match storage {
                Storage::Own | Storage::Global => "OWN/GLOBAL",
                Storage::External => "EXTERNAL",
                Storage::Local(_) => "LOCAL",
                Storage::Bind(_) => "BIND",
            },
            Kind::Routine { .. } => "ROUTINE",
            Kind::Literal(_) => "LITERAL",
            _ => return,
        };
        let at = self.last();
        self.diag(
            'I',
            &at,
            format!("Symbol {name} is declared {what} in an outer block"),
        );
    }

    /// `left = right`, at `at`: CHECK_LONGWORD's 64-bit value stored in
    /// less, CHECK_SHORT_ADDRESS's store through an address held in less
    /// than 64 bits, and CHECK_ADDRESS_TAKEN's address stored as a value.
    pub(crate) fn check_store(&mut self, left: &Expr, right: &Expr, at: &Lexeme) {
        self.check_zero_size(left, at);
        if self.switch_on("CHECK_LONGWORD")
            && self.scalar_size(left).is_some_and(|n| n < 8)
            && self.wide(right)
        {
            let msg = "Storing 64 bits in a field of less than 64 bits".to_string();
            self.diag('I', at, msg);
        }
        if let Expr::Fetch(a) = left
            && self.scalar_size(a).is_some_and(|n| n < 8)
            && self.switch_on("CHECK_SHORT_ADDRESS")
        {
            self.diag('I', at, "Storing to a short address".into());
        }
        // A NOVALUE routine's call (oracle MSGS).
        if let Expr::Call(t, ..) = right
            && let Expr::Name(id) = **t
            && let Kind::Routine { novalue: true, .. } = self.m.syms[id].kind
            && let Some(at) = self.call_at.clone()
        {
            let msg = format!("ROUTINE {} has no value", self.m.syms[id].name);
            self.late.push(self.diags.len());
            self.diag('W', &at, msg);
        }
        // A field's value is its address only if it starts a byte's
        // first bit (LRM 5.1.2).
        if let Expr::Field(_, pos, ..) = right
            && crate::parse::fold(pos).is_none_or(|p| p != 0)
            && let Some(at) = self.field_at.clone()
        {
            self.diag(
                'W',
                &at,
                "Field reference used as an expression has no value".into(),
            );
        }
        self.address_taken(right);
    }

    /// CHECK_SHORT_ADDRESS, at a fetch `.operand` at `at`.
    pub(crate) fn check_fetch(&mut self, operand: &Expr, at: &Lexeme) {
        let operand = match operand {
            Expr::Block(es, true) if es.len() == 1 => &es[0],
            e => e,
        };
        self.check_zero_size(operand, at);
        if let Expr::Fetch(a) = operand
            && self.scalar_size(a).is_some_and(|n| n < 8)
            && self.switch_on("CHECK_SHORT_ADDRESS")
        {
            self.diag('I', at, "Fetching from a short address".into());
        }
    }

    /// A field of a constant size of 0, which a fetch reads as 0 and a
    /// store leaves alone, is BLISSA64's warning (oracle RZ).
    fn check_zero_size(&mut self, e: &Expr, at: &Lexeme) {
        if let Expr::Field(_, _, s, _) = e
            && crate::parse::fold(s) == Some(0)
        {
            let msg = "Fetch or store applied to field of zero size".to_string();
            self.diag('W', at, msg);
        }
    }

    /// Whether `e` is a 64-bit value: a quadword's fetched, or arithmetic
    /// on one.
    fn wide(&self, e: &Expr) -> bool {
        match e {
            Expr::Fetch(a) => self.scalar_size(a) == Some(8),
            Expr::Bin(_, a, b) => self.wide(a) || self.wide(b),
            Expr::Neg(a) | Expr::Not(a) => self.wide(a),
            _ => false,
        }
    }

    /// The size of the scalar data `e` names, if it names one.
    fn scalar_size(&self, e: &Expr) -> Option<u8> {
        match e {
            Expr::Name(id) => match &self.m.syms[*id].kind {
                Kind::Data {
                    size,
                    structure: None,
                    ..
                } => Some(*size),
                _ => None,
            },
            _ => None,
        }
    }

    /// CHECK_ADDRESS_TAKEN: a variable's address stored as a value, the
    /// first time for each, unless it is ALIAS or VOLATILE (RN 2.19.3).
    fn address_taken(&mut self, e: &Expr) {
        match e {
            Expr::Name(id) => {
                let id = *id;
                let variable = matches!(
                    self.m.syms[id].kind,
                    Kind::Data {
                        storage: Storage::Own
                            | Storage::Global
                            | Storage::External
                            | Storage::Local(_),
                        ..
                    }
                );
                if variable
                    && !self.volatile.contains(&id)
                    && !self.alias.contains(&id)
                    && !self.taken.contains(&id)
                    && self.switch_on("CHECK_ADDRESS_TAKEN")
                {
                    self.taken.push(id);
                    let at = self
                        .name_at
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| self.last());
                    let msg = format!(
                        "Address of variable {} in value context; assuming ALIAS attribute",
                        self.m.syms[id].name
                    );
                    self.diag('I', &at, msg);
                }
            }
            Expr::If(_, t, e) => {
                self.address_taken(t);
                if let Some(e) = e {
                    self.address_taken(e);
                }
            }
            Expr::Block(es, _) => {
                if let Some(e) = es.last() {
                    self.address_taken(e);
                }
            }
            _ => {}
        }
    }

    /// At a call of `target` with `args`, each starting at `starts`:
    /// CHECK_PARAMETERS's actuals beyond the routine's formals and
    /// undotted variables smaller than the REF structure its prototype's
    /// formal expects (RN 2.19.8), and CHECK_LONGWORD's 64-bit value
    /// passed to a formal of less.
    pub(crate) fn check_call(&mut self, target: &Expr, args: &[Expr], starts: &[Lexeme]) {
        let Expr::Name(id) = target else {
            return;
        };
        let formals = self.formals.get(id).cloned().unwrap_or_default();
        if self.switch_on("CHECK_LONGWORD") {
            for (k, (a, f)) in args.iter().zip(&formals).enumerate() {
                if f.unit.is_some_and(|u| u < 8) && self.wide(a) {
                    self.diag('I', &starts[k], "Passing a 64-bit parameter".into());
                }
            }
        }
        if !self.switch_on("CHECK_PARAMETERS") {
            return;
        }
        if let Some(&(n, false)) = self.formal_counts.get(id)
            && args.len() > n
        {
            let msg = format!(
                "Extra actual parameters in call to {}",
                self.m.syms[*id].name
            );
            self.diag('I', &starts[n], msg);
        }
        for (k, (a, f)) in args.iter().zip(&formals).enumerate() {
            let (Expr::Name(v), Some(want)) = (a, f.ref_bytes) else {
                continue;
            };
            if let Kind::Data { bytes, .. } = self.m.syms[*v].kind
                && bytes < want
            {
                let msg = format!(
                    "Passing {bytes} byte parameter where {want} byte parameter is expected (parameter {})",
                    k + 1
                );
                self.diag('I', &starts[k], msg);
            }
        }
    }
}

/// What the checks know of a routine's formal: its allocation unit, and
/// the size of the REF structure a prototype gives it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Formal {
    pub unit: Option<u8>,
    pub ref_bytes: Option<u32>,
}
