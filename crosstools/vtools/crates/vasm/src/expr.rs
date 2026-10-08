//! Expressions: C operators and precedence, VMS radix prefixes, symbols,
//! local labels (`10$`) and the location counter (`.`).
//!
//! With `Cursor::macro32`, MACRO-32's instead: binary operators apply left
//! to right with no precedence, `<>` groups, `!` is OR, `\` XOR, `@` an
//! arithmetic shift (right if negative), and `^C`, `^A/text/` and `^M<regs>`
//! are the complement, ASCII and register mask operators. `10.` is decimal.

use crate::lex::{Cursor, Result, err};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(i64),
    /// A symbol in upper case; local labels carry their block (`10$@3`).
    Sym(String),
    /// The location counter, `.`.
    Here,
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    And,
    Xor,
    Or,
    /// MACRO-32's `@`: left by a positive count, arithmetic right by a
    /// negative one.
    Ash,
}

/// A value: absolute, or relative to a psect of this module or to an
/// external symbol, which the linker resolves.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Abs(i64),
    Psect { psect: usize, offset: i64 },
    Ext { name: String, offset: i64 },
}

/// Symbols and the location counter, for evaluation.
pub trait Scope {
    fn lookup(&self, name: &str) -> Option<Value>;
    fn here(&self) -> Value;
}

/// Parses an expression; `block` numbers the local label block.
pub fn parse(c: &mut Cursor, block: u32) -> Result<Expr> {
    if c.macro32 {
        return flat(c, block);
    }
    binary(c, block, 0)
}

/// MACRO-32 has no `<<` and `>>`, whose `>` would end a `<>` group.
const FLAT: [(&str, Op); 11] = [
    ("+", Op::Add),
    ("-", Op::Sub),
    ("*", Op::Mul),
    ("/", Op::Div),
    ("%", Op::Rem),
    ("&", Op::And),
    ("!", Op::Or),
    ("|", Op::Or),
    ("\\", Op::Xor),
    ("^", Op::Xor),
    ("@", Op::Ash),
];

/// MACRO-32: operators left to right, all of the same precedence.
fn flat(c: &mut Cursor, block: u32) -> Result<Expr> {
    let mut left = unary(c, block)?;
    'more: loop {
        c.skip_ws();
        for &(tok, op) in &FLAT {
            if c.rest().starts_with(tok) {
                c.at += tok.len();
                let right = unary(c, block)?;
                left = Expr::Bin(op, Box::new(left), Box::new(right));
                continue 'more;
            }
        }
        return Ok(left);
    }
}

const LEVELS: [&[(&str, Op)]; 6] = [
    &[("|", Op::Or)],
    &[("^", Op::Xor)],
    &[("&", Op::And)],
    &[("<<", Op::Shl), (">>", Op::Shr)],
    &[("+", Op::Add), ("-", Op::Sub)],
    &[("*", Op::Mul), ("/", Op::Div), ("%", Op::Rem)],
];

fn binary(c: &mut Cursor, block: u32, level: usize) -> Result<Expr> {
    if level == LEVELS.len() {
        return unary(c, block);
    }
    let mut left = binary(c, block, level + 1)?;
    'more: loop {
        c.skip_ws();
        for &(tok, op) in LEVELS[level] {
            if c.rest().starts_with(tok) {
                c.at += tok.len();
                let right = binary(c, block, level + 1)?;
                left = Expr::Bin(op, Box::new(left), Box::new(right));
                continue 'more;
            }
        }
        return Ok(left);
    }
}

fn unary(c: &mut Cursor, block: u32) -> Result<Expr> {
    if c.eat('-') {
        return Ok(Expr::Neg(Box::new(unary(c, block)?)));
    }
    if c.eat('~') {
        return Ok(Expr::Not(Box::new(unary(c, block)?)));
    }
    if c.eat('+') {
        return unary(c, block);
    }
    if c.eat('(') {
        let e = parse(c, block)?;
        c.expect(')')?;
        return Ok(e);
    }
    if c.macro32
        && let Some(e) = macro32_unary(c, block)?
    {
        return Ok(e);
    }
    let col = c.col();
    let rest = c.rest();
    if let Some(ch) = rest.strip_prefix('\'').and_then(|r| r.chars().next()) {
        if rest[1 + ch.len_utf8()..].starts_with('\'') {
            c.at += 2 + ch.len_utf8();
            return Ok(Expr::Num(ch as i64));
        }
        return err(col, "expected a character constant like 'A'");
    }
    if rest.starts_with(|ch: char| ch.is_ascii_digit()) || rest.starts_with('^') {
        let (radix, skip) = match rest.get(..2).map(str::to_ascii_uppercase).as_deref() {
            Some("0X") | Some("^X") => (16, 2),
            Some("0B") | Some("^B") => (2, 2),
            Some("0O") | Some("^O") => (8, 2),
            Some("^D") => (10, 2),
            _ => (10, 0),
        };
        let digits = &rest[skip..];
        let len = digits
            .find(|ch: char| !ch.is_ascii_alphanumeric())
            .unwrap_or(digits.len());
        let text = &digits[..len];
        // A decimal number followed by `$` is a local label.
        if skip == 0 && digits[len..].starts_with('$') {
            c.at += len + 1;
            return Ok(Expr::Sym(local_name(text, block)));
        }
        let Ok(n) = u64::from_str_radix(text, radix) else {
            return err(col, format!("bad number '{}'", &rest[..skip + len]));
        };
        c.at += skip + len;
        // MACRO-32's explicitly decimal `10.`.
        if c.macro32 && skip == 0 && c.rest().starts_with('.') {
            c.at += 1;
        }
        return Ok(Expr::Num(n as i64));
    }
    match c.name() {
        Some(name) if name == "." => Ok(Expr::Here),
        Some(name) => Ok(Expr::Sym(name)),
        None => err(col, "expected an expression"),
    }
}

/// MACRO-32's `<expr>`, `^C`, `^A` and `^M`, if one comes next.
fn macro32_unary(c: &mut Cursor, block: u32) -> Result<Option<Expr>> {
    if c.eat('<') {
        let e = parse(c, block)?;
        c.expect('>')?;
        return Ok(Some(e));
    }
    let col = c.col();
    let op = c.rest().get(..2).map(str::to_ascii_uppercase);
    match op.as_deref() {
        Some("^C") => {
            c.at += 2;
            Ok(Some(Expr::Not(Box::new(unary(c, block)?))))
        }
        Some("^A") => {
            // ^A/text/: up to 8 characters, the first in the low byte.
            c.at += 2;
            let rest = c.rest();
            let Some(d) = rest.chars().next() else {
                return err(col, "expected ^A/text/");
            };
            let Some(len) = rest[d.len_utf8()..].find(d) else {
                return err(col, format!("missing closing {d}"));
            };
            let text = &rest.as_bytes()[d.len_utf8()..d.len_utf8() + len];
            if text.len() > 8 {
                return err(col, "^A takes at most 8 characters");
            }
            let n = text.iter().rev().fold(0u64, |n, &b| n << 8 | u64::from(b));
            c.at += len + 2 * d.len_utf8();
            Ok(Some(Expr::Num(n as i64)))
        }
        Some("^M") => {
            // ^M<R2,R3,AP>: a register mask.
            c.at += 2;
            c.expect('<')?;
            let mut mask = 0i64;
            while !c.eat('>') {
                let rcol = c.col();
                let bit = match c.name().as_deref() {
                    Some("AP") => 12,
                    Some("FP") => 13,
                    Some("SP") => 14,
                    Some("PC") => 15,
                    Some(r) => match r.strip_prefix('R').and_then(|n| n.parse::<u32>().ok()) {
                        Some(n) if n < 16 => n,
                        _ => return err(rcol, "expected a register"),
                    },
                    None => return err(rcol, "expected a register"),
                };
                mask |= 1 << bit;
                if !c.eat(',') {
                    c.expect('>')?;
                    break;
                }
            }
            Ok(Some(Expr::Num(mask)))
        }
        _ => Ok(None),
    }
}

/// Evaluates `e`. Relocatable values support adding and subtracting
/// constants; two addresses in the same psect subtract to a constant.
pub fn eval(e: &Expr, s: &impl Scope) -> std::result::Result<Value, String> {
    use Value::*;
    Ok(match e {
        Expr::Num(n) => Abs(*n),
        Expr::Sym(name) => match s.lookup(name) {
            Some(v) => v,
            None => return Err(format!("undefined symbol {}", display(name))),
        },
        Expr::Here => s.here(),
        Expr::Neg(e) => Abs(abs(eval(e, s)?)?.wrapping_neg()),
        Expr::Not(e) => Abs(!abs(eval(e, s)?)?),
        Expr::Bin(op, l, r) => {
            let (l, r) = (eval(l, s)?, eval(r, s)?);
            match (op, l, r) {
                (Op::Add, Psect { psect, offset }, Abs(n))
                | (Op::Add, Abs(n), Psect { psect, offset }) => Psect {
                    psect,
                    offset: offset.wrapping_add(n),
                },
                (Op::Add, Ext { name, offset }, Abs(n))
                | (Op::Add, Abs(n), Ext { name, offset }) => Ext {
                    name,
                    offset: offset.wrapping_add(n),
                },
                (Op::Sub, Psect { psect, offset }, Abs(n)) => Psect {
                    psect,
                    offset: offset.wrapping_sub(n),
                },
                (Op::Sub, Ext { name, offset }, Abs(n)) => Ext {
                    name,
                    offset: offset.wrapping_sub(n),
                },
                (
                    Op::Sub,
                    Psect {
                        psect: p,
                        offset: a,
                    },
                    Psect {
                        psect: q,
                        offset: b,
                    },
                ) if p == q => Abs(a.wrapping_sub(b)),
                (Op::Sub, Ext { name: m, offset: a }, Ext { name: n, offset: b }) if m == n => {
                    Abs(a.wrapping_sub(b))
                }
                (op, l, r) => Abs(arith(*op, abs(l)?, abs(r)?)?),
            }
        }
    })
}

fn arith(op: Op, l: i64, r: i64) -> std::result::Result<i64, String> {
    Ok(match op {
        Op::Mul => l.wrapping_mul(r),
        Op::Div | Op::Rem if r == 0 => return Err("division by zero".into()),
        Op::Div => l.wrapping_div(r),
        Op::Rem => l.wrapping_rem(r),
        Op::Add => l.wrapping_add(r),
        Op::Sub => l.wrapping_sub(r),
        Op::Shl => l.wrapping_shl(r as u32),
        Op::Shr => ((l as u64).wrapping_shr(r as u32)) as i64,
        Op::And => l & r,
        Op::Xor => l ^ r,
        Op::Or => l | r,
        Op::Ash if r >= 0 => l.wrapping_shl(r as u32),
        Op::Ash => l.wrapping_shr(r.unsigned_abs() as u32),
    })
}

fn abs(v: Value) -> std::result::Result<i64, String> {
    match v {
        Value::Abs(n) => Ok(n),
        _ => Err("the linker can only add or subtract a constant to an address".into()),
    }
}

/// The internal name of local label `digits$` in `block`. Labels from 30000$
/// up are the ones macros create; each is unique in the module, so they
/// belong to no block.
pub fn local_name(digits: &str, block: u32) -> String {
    if digits.parse::<u32>().is_ok_and(|n| n >= 30000) {
        format!("{digits}$@")
    } else {
        format!("{digits}$@{block}")
    }
}

/// A symbol name as written: local labels without their block.
pub fn display(name: &str) -> &str {
    name.split('@').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct S;
    impl Scope for S {
        fn lookup(&self, name: &str) -> Option<Value> {
            match name {
                "A" => Some(Value::Psect {
                    psect: 1,
                    offset: 16,
                }),
                "B" => Some(Value::Psect {
                    psect: 1,
                    offset: 4,
                }),
                "PUTS" => Some(Value::Ext {
                    name: "PUTS".into(),
                    offset: 0,
                }),
                _ => None,
            }
        }
        fn here(&self) -> Value {
            Value::Psect {
                psect: 1,
                offset: 100,
            }
        }
    }

    fn val(text: &str) -> std::result::Result<Value, String> {
        let mut c = Cursor::new(text);
        let e = parse(&mut c, 7).map_err(|e| e.msg)?;
        assert!(c.at_end(), "{text}: trailing text");
        eval(&e, &S)
    }

    fn val32(text: &str) -> std::result::Result<Value, String> {
        let mut c = Cursor::new(text);
        c.macro32 = true;
        let e = parse(&mut c, 7).map_err(|e| e.msg)?;
        assert!(c.at_end(), "{text}: trailing text");
        eval(&e, &S)
    }

    #[test]
    fn macro32() {
        assert_eq!(val32("1 + 2 * 3"), Ok(Value::Abs(9)));
        assert_eq!(val32("1 + <2 * 3>"), Ok(Value::Abs(7)));
        assert_eq!(val32("^X10 ! 1 \\ 3 & 6"), Ok(Value::Abs(2)));
        assert_eq!(val32("1 @ 4 + 10."), Ok(Value::Abs(26)));
        assert_eq!(val32("-32 @ -2"), Ok(Value::Abs(-8)));
        assert_eq!(val32("^C0 + ^A/AB/"), Ok(Value::Abs(0x4241 - 1)));
        assert_eq!(val32("^M<R2, R3, AP>"), Ok(Value::Abs(0x100c)));
        assert_eq!(val32("a - b"), Ok(Value::Abs(12)));
    }

    #[test]
    fn expressions() {
        assert_eq!(val("1 + 2 * 3"), Ok(Value::Abs(7)));
        assert_eq!(val("(1 + 2) * 3"), Ok(Value::Abs(9)));
        assert_eq!(val("1 << 4 | 1"), Ok(Value::Abs(17)));
        assert_eq!(
            val("^X10 + 0x10 + ^B11 + 'A'"),
            Ok(Value::Abs(0x20 + 3 + 65))
        );
        assert_eq!(val("-1 >> 60"), Ok(Value::Abs(15)));
        assert_eq!(
            val("a + 4"),
            Ok(Value::Psect {
                psect: 1,
                offset: 20
            })
        );
        assert_eq!(val("a - b"), Ok(Value::Abs(12)));
        assert_eq!(val(". - a"), Ok(Value::Abs(84)));
        assert_eq!(
            val("puts + 8"),
            Ok(Value::Ext {
                name: "PUTS".into(),
                offset: 8
            })
        );
        assert!(val("a * 2").is_err());
        assert_eq!(val("nope"), Err("undefined symbol NOPE".into()));
        let mut c = Cursor::new("10$ + 1");
        assert_eq!(
            parse(&mut c, 3),
            Ok(Expr::Bin(
                Op::Add,
                Box::new(Expr::Sym("10$@3".into())),
                Box::new(Expr::Num(1))
            ))
        );
    }
}
