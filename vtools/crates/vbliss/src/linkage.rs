//! LINKAGE declarations (LRM 13.3, `docs/bliss64.md` *Linkages*): a
//! routine's linkage says how calls reach it. CALL is the calling standard
//! (DESIGN-0004); JSB passes parameters in VAX registers R0-R11, mapped as
//! vmacro maps them (R0, R1 to x0, x1; R2-R11 to x19-x28), and keeps
//! R2-R11 unless the linkage says NOPRESERVE.

use std::rc::Rc;

use crate::lex::Tok;
use crate::parse::{Kind, Parser, R};

/// A linkage: its type and where its parameters go.
#[derive(Debug, PartialEq)]
pub struct Linkage {
    pub jsb: bool,
    /// The VAX register of each parameter, in order; past the list,
    /// parameters are STANDARD, which JSB can't pass.
    pub params: Vec<u8>,
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
                nopreserve: Vec::new(),
            };
            if self.eat_punct('(') {
                loop {
                    if self.at_punct(';') {
                        return self.err("output parameters are not supported yet");
                    }
                    if self.eat("REGISTER") {
                        self.expect_punct('=')?;
                        let r = self.register()?;
                        if l.params.contains(&r) {
                            return self.err(format!("register {r} is given twice"));
                        }
                        l.params.push(r);
                    } else if self.eat("STANDARD") || self.at_punct(',') || self.at_punct(')') {
                        if jsb {
                            return self.err("a JSB linkage passes parameters in registers");
                        }
                    } else {
                        let found = self.describe();
                        return self.err(format!("expected REGISTER or STANDARD, found {found}"));
                    }
                    if !self.eat_punct(',') {
                        break;
                    }
                }
                self.expect_punct(')')?;
            }
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
                        "GLOBAL" => return self.err("GLOBAL registers are not supported yet"),
                        _ => break,
                    }
                }
            }
            if !jsb && !l.params.is_empty() {
                return self.err("register parameters in a CALL linkage are not supported yet");
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
}
