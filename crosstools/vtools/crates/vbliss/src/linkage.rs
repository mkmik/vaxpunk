//! LINKAGE declarations (LRM 13.3, `docs/bliss64.md` *Linkages*): a
//! routine's linkage says how calls reach it. CALL is the calling standard
//! (DESIGN-0004); JSB passes parameters in VAX registers R0-R11, mapped as
//! vmacro maps them (R0, R1 to x0, x1; R2-R11 to x19-x28), and keeps
//! R2-R11 unless the linkage says NOPRESERVE.
//!
//! Register data (LRM 10.7-10.9) lives in the routine's frame, as LOCAL
//! data does. A GLOBAL REGISTER's value goes into its register for a call
//! whose linkage names it in GLOBAL, and comes back from it after; a
//! routine that declares it EXTERNAL REGISTER reads it from the register
//! at entry and leaves it there when it returns. Output parameters come
//! back the same way.

use std::rc::Rc;

use crate::lex::Tok;
use crate::parse::{Expr, Kind, Link, Parser, R, Storage};

/// A linkage: its type and where its parameters go.
#[derive(Debug, PartialEq)]
pub struct Linkage {
    pub jsb: bool,
    /// Each input parameter's VAX register, or None for STANDARD; past
    /// the list, parameters are STANDARD, which JSB can't pass.
    pub params: Vec<Option<u8>>,
    /// The output parameters' registers.
    pub outs: Vec<u8>,
    /// The global register segments: names and registers.
    pub globals: Vec<(String, u8)>,
    /// Registers the callee needn't keep, besides R0 and R1.
    pub nopreserve: Vec<u8>,
}

impl Parser<'_> {
    /// LINKAGE definitions, after the word.
    pub(crate) fn linkages(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            self.expect_punct('=')?;
            let jsb = if self.eat("JSB") {
                true
            } else if self.eat("CALL") {
                false
            } else {
                let found = self.describe();
                return self.err(format!("expected CALL or JSB, found {found}"));
            };
            let mut l = Linkage {
                jsb,
                params: Vec::new(),
                outs: Vec::new(),
                globals: Vec::new(),
                nopreserve: Vec::new(),
            };
            let mut used = Vec::new();
            if self.eat_punct('(') {
                if !self.at_punct(';') && !self.at_punct(')') {
                    loop {
                        if self.eat("REGISTER") {
                            self.expect_punct('=')?;
                            let r = self.linkage_register(&mut used, !jsb)?;
                            l.params.push(Some(r));
                        } else if self.eat("STANDARD")
                            || self.at_punct(',')
                            || self.at_punct(';')
                            || self.at_punct(')')
                        {
                            if jsb {
                                return self.err("a JSB linkage passes parameters in registers");
                            }
                            l.params.push(None);
                        } else {
                            let found = self.describe();
                            return self
                                .err(format!("expected REGISTER or STANDARD, found {found}"));
                        }
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                }
                if self.eat_punct(';') {
                    loop {
                        self.expect("REGISTER")?;
                        self.expect_punct('=')?;
                        // The same register may be an input and an output.
                        let r = self.register()?;
                        if l.outs.contains(&r) {
                            return self.err(format!("register {r} is given twice"));
                        }
                        l.outs.push(r);
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                }
                self.expect_punct(')')?;
            }
            used.extend(l.outs.iter().copied());
            if self.eat_punct(':') {
                while let Tok::Name(option) = self.peek().clone() {
                    match option.as_str() {
                        "PRESERVE" | "NOPRESERVE" | "NOTUSED" => {
                            self.pos += 1;
                            self.expect_punct('(')?;
                            loop {
                                let r = self.register()?;
                                if option == "NOPRESERVE" {
                                    l.nopreserve.push(r);
                                }
                                if !self.eat_punct(',') {
                                    break;
                                }
                            }
                            self.expect_punct(')')?;
                        }
                        "GLOBAL" => {
                            self.pos += 1;
                            self.expect_punct('(')?;
                            loop {
                                let g = self.name()?;
                                self.expect_punct('=')?;
                                let r = self.linkage_register(&mut used, true)?;
                                l.globals.push((g, r));
                                if !self.eat_punct(',') {
                                    break;
                                }
                            }
                            self.expect_punct(')')?;
                        }
                        _ => break,
                    }
                }
            }
            self.declare(name, Kind::Linkage(Rc::new(l)))?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// A VAX register number, 0 to 11.
    fn register(&mut self) -> R<u8> {
        let r = self.ctce()?;
        if !(0..=11).contains(&r) {
            return self.err(format!(
                "register {r}: linkages name VAX registers R0-R11 (docs/bliss64.md)"
            ));
        }
        Ok(r as u8)
    }

    /// A register a linkage gives one parameter or global segment: not
    /// one given already, and with `high`, not R0 or R1, which the calling
    /// standard's arguments and the result use.
    fn linkage_register(&mut self, used: &mut Vec<u8>, high: bool) -> R<u8> {
        let r = self.register()?;
        if used.contains(&r) {
            return self.err(format!("register {r} is given twice"));
        }
        if high && r < 2 {
            return self.err(format!(
                "Register conflict: R{r} carries the calling standard's arguments and result"
            ));
        }
        used.push(r);
        Ok(r)
    }

    /// REGISTER, GLOBAL REGISTER (`global`) or EXTERNAL REGISTER
    /// (`external`) names, after the words: data in the routine's frame.
    pub(crate) fn registers(&mut self, global: bool, external: bool) -> R<()> {
        if !self.slots_open() {
            return self.err("register data must be declared in a routine");
        }
        loop {
            let name = self.name()?;
            let mut number = None;
            if self.eat_punct('=') {
                number = Some(self.register()?);
            }
            let mut a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                crate::data::Attrs::default()
            };
            if a.initial.is_some() || a.preset.is_some() {
                return self.err("INITIAL or PRESET on register data");
            }
            if external {
                let linked = self
                    .linkage_now()
                    .and_then(|l| l.globals.iter().find(|(g, _)| *g == name).map(|&(_, r)| r));
                let Some(r) = linked else {
                    return self.err(format!(
                        "EXTERNAL REGISTER {name} isn't a GLOBAL register of the routine's linkage"
                    ));
                };
                if number.is_some_and(|n| n != r) {
                    return self.err(format!(
                        "EXTERNAL REGISTER {name} isn't R{r} as the linkage says"
                    ));
                }
                number = Some(r);
            } else if global {
                match number {
                    Some(2..=11) => {}
                    Some(r) => {
                        return self.err(format!(
                            "Register conflict: R{r} carries the calling standard's arguments and result"
                        ));
                    }
                    None => {
                        return self.err(format!("GLOBAL REGISTER {name} needs a register number"));
                    }
                }
            }
            let kind = self.data_kind(Storage::Local(0), &mut a)?;
            if let Kind::Data { bytes, .. } = kind
                && bytes > u32::from(self.m.dialect.fullword())
            {
                return self.err(format!("register data {name} is more than a fullword"));
            }
            let id = self.declare(name, kind)?;
            if global || external {
                let r = number.unwrap();
                self.reg_data.insert(id, r);
                if external {
                    self.exits.last_mut().unwrap().0.push((r, id));
                }
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// A call of `target` with input actuals `args` and output actuals
    /// `outs` (None where one is left out): its standard actuals and what
    /// goes through registers, as the routine's linkage says.
    pub(crate) fn link_call(
        &mut self,
        target: Expr,
        args: Vec<Expr>,
        outs: Vec<Option<Expr>>,
    ) -> R<Expr> {
        let linkage = match &target {
            Expr::Name(id) => match &self.m.syms[*id].kind {
                Kind::Routine { linkage, .. } => linkage.clone(),
                _ => None,
            },
            _ => None,
        };
        let Some(l) = linkage else {
            if !outs.is_empty() {
                return self.err("output actuals need a linkage with output parameters");
            }
            return Ok(Expr::Call(Box::new(target), args, Vec::new()));
        };
        if l.jsb && args.len() > l.params.len() {
            return self.err("more actuals than the JSB linkage has registers");
        }
        if outs.len() > l.outs.len() {
            return self.err("more output actuals than the linkage has output parameters");
        }
        let (mut std, mut links) = (Vec::new(), Vec::new());
        for (i, a) in args.into_iter().enumerate() {
            match l.params.get(i).copied().flatten() {
                Some(reg) => links.push(Link {
                    reg,
                    input: Some(a),
                    output: None,
                }),
                None => std.push(a),
            }
        }
        let mut outs = outs.into_iter();
        for &reg in &l.outs {
            let output = outs.next().flatten();
            match links.iter_mut().find(|k| k.reg == reg) {
                Some(k) => k.output = output,
                None => links.push(Link {
                    reg,
                    input: None,
                    output,
                }),
            }
        }
        for (g, reg) in &l.globals {
            let found = self
                .lookup(g)
                .filter(|id| self.reg_data.get(id) == Some(reg));
            let Some(id) = found else {
                return self.err(format!(
                    "the linkage's global register {g} = {reg} isn't a GLOBAL or EXTERNAL REGISTER here"
                ));
            };
            links.push(Link {
                reg: *reg,
                input: Some(Expr::Fetch(Box::new(Expr::Name(id)))),
                output: Some(Expr::Name(id)),
            });
        }
        Ok(Expr::Call(Box::new(target), std, links))
    }
}
