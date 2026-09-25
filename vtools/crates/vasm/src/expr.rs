//! Expressions: C operators and precedence, VMS radix prefixes, symbols,
//! local labels (`10$`) and the location counter (`.`).

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
    binary(c, block, 0)
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
        return Ok(Expr::Num(n as i64));
    }
    match c.name() {
        Some(name) if name == "." => Ok(Expr::Here),
        Some(name) => Ok(Expr::Sym(name)),
        None => err(col, "expected an expression"),
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
