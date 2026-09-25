//! Scanning one source line.

/// An error at a column of the current line.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub col: usize,
    pub msg: String,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn err<T>(col: usize, msg: impl Into<String>) -> Result<T> {
    Err(Error {
        col,
        msg: msg.into(),
    })
}

/// A position in a line. Columns count from 1.
#[derive(Clone)]
pub struct Cursor<'a> {
    pub line: &'a str,
    pub at: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(line: &'a str) -> Self {
        Cursor { line, at: 0 }
    }

    pub fn col(&self) -> usize {
        self.at + 1
    }

    pub fn rest(&self) -> &'a str {
        &self.line[self.at..]
    }

    pub fn skip_ws(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    /// The next character after blanks, without consuming it.
    pub fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.rest().chars().next()
    }

    /// Whether only blanks remain.
    pub fn at_end(&mut self) -> bool {
        self.peek().is_none()
    }

    /// Consumes `c` if it comes next.
    pub fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.at += c.len_utf8();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, c: char) -> Result<()> {
        if self.eat(c) {
            Ok(())
        } else {
            err(self.col(), format!("expected '{c}'"))
        }
    }

    /// A name: letters, digits, `$`, `_` and `.`, not starting with a digit.
    /// Returned in upper case, as VMS names are case-insensitive.
    pub fn name(&mut self) -> Option<String> {
        self.skip_ws();
        let rest = self.rest();
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '$' | '_' | '.')))
            .unwrap_or(rest.len());
        if len == 0 || rest.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.at += len;
        Some(rest[..len].to_ascii_uppercase())
    }
}

/// Removes a comment: `;` (MACRO) or `//` (GNU), outside of quotes. A
/// MACRO-style `/.../` string can't contain either.
pub fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, ';') => return &line[..i],
            (None, '/') if line[i..].starts_with("//") => return &line[..i],
            _ => {}
        }
    }
    line
}
