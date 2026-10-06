//! The lexer: source text into lexemes (LRM chapter 2). Names are folded
//! to upper case; numbers with a radix (`%X'1F'`), characters (`%C'A'`)
//! and quoted strings are read here; comments (`! ...` and `%( ... )%`)
//! are dropped.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// A name or a reserved word, upper case; `%NAME` keeps its `%`.
    Name(String),
    Num(i64),
    /// A quoted string, `''` read as one quote.
    Str(Vec<u8>),
    /// A special character: `+ - * / . = , ; : ( ) [ ] < > ^`.
    Punct(char),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Lexeme {
    pub tok: Tok,
    pub line: u32,
}

/// Splits `source` into lexemes, ending with `Eof`.
pub fn lex(source: &str) -> Result<Vec<Lexeme>, (u32, String)> {
    let s = source.as_bytes();
    let (mut i, mut line, mut out) = (0, 1u32, Vec::new());
    while i < s.len() {
        let c = s[i];
        let start_line = line;
        let mut push = |tok| {
            out.push(Lexeme {
                tok,
                line: start_line,
            })
        };
        match c {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b' ' | b'\t' | b'\r' | b'\x0c' | b'\x0b' => i += 1,
            b'!' => {
                while i < s.len() && s[i] != b'\n' {
                    i += 1;
                }
            }
            b'%' if s.get(i + 1) == Some(&b'(') => {
                i += 2;
                loop {
                    match s.get(i) {
                        None => return Err((start_line, "unterminated %( comment".into())),
                        Some(b')') if s.get(i + 1) == Some(&b'%') => break i += 2,
                        Some(b'\n') => line += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'\'' => {
                let (text, next) = quoted(s, i).ok_or((line, "unterminated string".into()))?;
                push(Tok::Str(text));
                i = next;
            }
            b'0'..=b'9' => {
                let start = i;
                while i < s.len() && s[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let text = &source[start..i];
                push(Tok::Num(
                    number(text, 10).ok_or((line, format!("bad number {text}")))?,
                ));
            }
            b'%' | b'$' | b'_' | b'A'..=b'Z' | b'a'..=b'z' => {
                let start = i;
                i += 1;
                while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == b'$' || s[i] == b'_')
                {
                    i += 1;
                }
                let name = source[start..i].to_ascii_uppercase();
                let radix = match name.as_str() {
                    "%B" => Some(2),
                    "%O" => Some(8),
                    "%X" => Some(16),
                    "%DECIMAL" => Some(10),
                    _ => None,
                };
                if let Some(radix) = radix.filter(|_| s.get(i) == Some(&b'\'')) {
                    let (text, next) = quoted(s, i).ok_or((line, "unterminated number".into()))?;
                    let text = String::from_utf8_lossy(&text).trim().to_string();
                    let (neg, digits) = match text.strip_prefix('-') {
                        Some(d) => (true, d),
                        None => (false, text.strip_prefix('+').unwrap_or(&text)),
                    };
                    let n = number(digits.trim(), radix)
                        .ok_or((line, format!("bad number {name}'{text}'")))?;
                    push(Tok::Num(if neg { n.wrapping_neg() } else { n }));
                    i = next;
                } else if name == "%C" && s.get(i) == Some(&b'\'') {
                    let (text, next) = quoted(s, i).ok_or((line, "unterminated %C".into()))?;
                    if text.len() != 1 {
                        return Err((line, "%C takes one character".into()));
                    }
                    push(Tok::Num(text[0].into()));
                    i = next;
                } else if name == "%" {
                    return Err((line, "% alone".into()));
                } else {
                    push(Tok::Name(name));
                }
            }
            b'+' | b'-' | b'*' | b'/' | b'.' | b'=' | b',' | b';' | b':' | b'(' | b')' | b'['
            | b']' | b'<' | b'>' | b'^' => {
                push(Tok::Punct(c as char));
                i += 1;
            }
            _ => return Err((line, format!("unexpected character {:?}", c as char))),
        }
    }
    out.push(Lexeme {
        tok: Tok::Eof,
        line,
    });
    Ok(out)
}

/// The text of the quoted string at `s[i]`, and where it ends.
fn quoted(s: &[u8], mut i: usize) -> Option<(Vec<u8>, usize)> {
    let mut text = Vec::new();
    i += 1;
    loop {
        match *s.get(i)? {
            b'\'' if s.get(i + 1) == Some(&b'\'') => {
                text.push(b'\'');
                i += 2;
            }
            b'\'' => return Some((text, i + 1)),
            b'\n' => return None,
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
}

/// A number in `radix`, wrapping at 64 bits as the fullword does.
fn number(text: &str, radix: u32) -> Option<i64> {
    if text.is_empty() {
        return None;
    }
    let mut n: u64 = 0;
    for c in text.chars() {
        n = n
            .wrapping_mul(radix.into())
            .wrapping_add(c.to_digit(radix)?.into());
    }
    Some(n as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).unwrap().into_iter().map(|l| l.tok).collect()
    }

    #[test]
    fn lexemes() {
        use Tok::*;
        assert_eq!(
            toks("Own x: long; ! comment\n%( more\n )% .x = %X'1F' + %c'A' - 'it''s'"),
            vec![
                Name("OWN".into()),
                Name("X".into()),
                Punct(':'),
                Name("LONG".into()),
                Punct(';'),
                Punct('.'),
                Name("X".into()),
                Punct('='),
                Num(31),
                Punct('+'),
                Num(65),
                Punct('-'),
                Str(b"it's".to_vec()),
                Eof
            ]
        );
        assert_eq!(
            toks("%O'-17' %ASCID"),
            vec![Num(-15), Name("%ASCID".into()), Eof]
        );
        assert_eq!(lex("A\n\nB").unwrap()[1].line, 3);
    }
}
