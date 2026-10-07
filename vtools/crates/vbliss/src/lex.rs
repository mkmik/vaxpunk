//! The lexer: source text into lexemes (LRM chapter 2). Names are folded
//! to upper case, decimal numbers and quoted strings read, comments
//! (`! ...` and `%( ... )%`) dropped. What a `%` name does, a radix
//! (`%X'1F'`) or a lexical function, is the lexical processor's.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// A name or a reserved word, upper case; `%NAME` keeps its `%`.
    Name(String),
    /// A name bound to a declaration ahead of its use, by `%UNQUOTE`.
    Bound(String, usize),
    Num(i64),
    /// A quoted string, `''` read as one quote.
    Str(Vec<u8>),
    /// A special character: `+ - * / . = , ; : ( ) [ ] < > ^`.
    Punct(char),
    /// `%` alone, which ends a macro body.
    Percent,
    Eof,
}

/// A lexeme and where it is: the file's number, the line in it and the
/// column, from 0.
#[derive(Clone, Debug)]
pub struct Lexeme {
    pub tok: Tok,
    pub file: u16,
    pub line: u32,
    pub col: u32,
}

/// Splits `source`, file number `file`, into lexemes, ending with `Eof`.
pub fn lex(source: &str, file: u16) -> Result<Vec<Lexeme>, (u32, String)> {
    let s = source.as_bytes();
    let (mut i, mut line, mut bol, mut out) = (0, 1u32, 0, Vec::new());
    while i < s.len() {
        let c = s[i];
        let (start_line, col) = (line, (i - bol) as u32);
        let mut push = |tok| {
            out.push(Lexeme {
                tok,
                file,
                line: start_line,
                col,
            })
        };
        match c {
            b'\n' => {
                line += 1;
                i += 1;
                bol = i;
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
                        Some(b'\n') => {
                            line += 1;
                            bol = i + 1;
                        }
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
            b'%' if !s
                .get(i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'$' || *c == b'_') =>
            {
                push(Tok::Percent);
                i += 1;
            }
            b'%' | b'$' | b'_' | b'A'..=b'Z' | b'a'..=b'z' => {
                let start = i;
                i += 1;
                while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == b'$' || s[i] == b'_')
                {
                    i += 1;
                }
                push(Tok::Name(source[start..i].to_ascii_uppercase()));
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
        file,
        line,
        col: (i - bol) as u32,
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
pub fn number(text: &str, radix: u32) -> Option<i64> {
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
        lex(s, 0).unwrap().into_iter().map(|l| l.tok).collect()
    }

    #[test]
    fn lexemes() {
        use Tok::*;
        assert_eq!(
            toks("Own x: long; ! comment\n%( more\n )% .x = %X'1F' - 'it''s' %;"),
            vec![
                Name("OWN".into()),
                Name("X".into()),
                Punct(':'),
                Name("LONG".into()),
                Punct(';'),
                Punct('.'),
                Name("X".into()),
                Punct('='),
                Name("%X".into()),
                Str(b"1F".to_vec()),
                Punct('-'),
                Str(b"it's".to_vec()),
                Percent,
                Punct(';'),
                Eof
            ]
        );
        let l = lex("A\n\n  B", 0).unwrap();
        assert_eq!((l[1].line, l[1].col), (3, 2));
    }
}
