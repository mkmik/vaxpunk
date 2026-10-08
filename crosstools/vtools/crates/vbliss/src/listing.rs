//! The listing (`/LIST`), laid out as BLISSA64 lays out its source part
//! with `/SOURCE_LIST=(EXPAND_MACROS,REQUIRE)`, so that the oracle's
//! listings compare with ours line for line once their page headers are
//! dropped (`crosstools/vtools/docs/vbliss.md`).

/// A lexeme's column when it has none: one a macro body gave.
pub const NOPOS: u32 = u32::MAX;

/// Source lines are cut at this many characters.
const TEXT_WIDTH: usize = 116;

/// Expansion lines end by this column.
const EXPANSION_WIDTH: usize = 84;

pub enum Entry {
    /// A source line: `flag` is 'P' inside a macro call's actuals and 'L'
    /// inside a lexical function's; `depth` the block depth.
    Line {
        flag: char,
        require: bool,
        depth: u32,
        number: u32,
        file: u16,
        line: u32,
        text: String,
    },
    /// A macro's expansion: `[NAME]=` and its lexemes, or None for null.
    Expansion {
        level: usize,
        name: String,
        text: Option<Vec<String>>,
    },
    /// `%PRINT`'s text.
    Print(String),
}

/// A diagnostic: severity `I`, `W` or `E`, and where.
#[derive(Clone, Debug)]
pub struct Diag {
    pub sev: char,
    pub file: u16,
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

/// The text from column `from` to column `to` in tabs and spaces.
fn pad(from: usize, to: usize) -> String {
    let mut s = String::new();
    let mut col = from;
    while (col / 8 + 1) * 8 <= to {
        s.push('\t');
        col = (col / 8 + 1) * 8;
    }
    s.extend(std::iter::repeat_n(' ', to.saturating_sub(col)));
    s
}

/// Whether a lexeme stays on the line of the one before it.
fn attached(t: &str) -> bool {
    t.len() == 1 && !t.starts_with(|c: char| c.is_alphanumeric() || "([<.'%".contains(c))
}

fn expansion(out: &mut String, level: usize, name: &str, text: &Option<Vec<String>>) {
    let indent = 22 + 4 * level;
    out.push_str("\t\t;;");
    out.push_str(&pad(18, indent));
    out.push_str(&format!("[{name}]="));
    let Some(text) = text else {
        out.push_str(" null\n");
        return;
    };
    out.push(' ');
    let start = indent + name.len() + 4;
    let width = EXPANSION_WIDTH as isize - start as isize;
    let (mut len, mut first) = (0, true);
    for t in text {
        if len > 0 && !attached(t) && len + 1 + t.len() as isize > width {
            out.push_str("\n\t\t;;");
            out.push_str(&pad(18, start));
            len = 0;
            first = false;
        }
        if len > 0 || first {
            out.push(' ');
        }
        out.push_str(t);
        len += t.len() as isize + 1;
    }
    out.push('\n');
}

/// The diagnostics on one line: the marker line under it and a message
/// each.
fn diags(out: &mut String, text: &str, ds: &[&Diag]) {
    let mut marker: Vec<char> = vec![' '; text.len() + 1];
    for (i, d) in ds.iter().enumerate() {
        let col = (d.col as usize).min(text.len());
        let digit = char::from_digit((i + 1) as u32 % 10, 10).unwrap();
        if col >= marker.len() {
            marker.resize(col + 1, ' ');
        }
        for c in marker[..col].iter_mut().filter(|c| **c == ' ') {
            *c = '.';
        }
        marker[col] = digit;
    }
    out.push_str("\t\t");
    out.extend(marker);
    out.push('\n');
    for (i, d) in ds.iter().enumerate() {
        out.push_str(&format!("%BLS64-{}-TEXT, ({}) {}\n", d.sev, i + 1, d.msg));
    }
    out.push('\n');
}

/// The listing's text: a header, then the entries with each line's
/// diagnostics after it.
pub fn render(header: &str, entries: &[Entry], all: &[Diag]) -> String {
    let mut out = String::from(header);
    let mut shown = vec![false; all.len()];
    for e in entries {
        match e {
            Entry::Line {
                flag,
                require,
                depth,
                number,
                file,
                line,
                text,
            } => {
                let prefix = format!(
                    "{flag}{}{depth:>3}\t{number:>7} ",
                    if *require { 'R' } else { ' ' }
                );
                let chars: Vec<char> = text.chars().collect();
                let mut chunks = chars.chunks(TEXT_WIDTH).peekable();
                if chunks.peek().is_none() {
                    out.push_str(&prefix);
                    out.push('\n');
                }
                for c in chunks {
                    out.push_str(&prefix);
                    out.extend(c);
                    out.push('\n');
                }
                let mut ds = Vec::new();
                for (i, d) in all.iter().enumerate() {
                    if !shown[i] && d.file == *file && d.line == *line {
                        shown[i] = true;
                        ds.push(d);
                    }
                }
                if !ds.is_empty() {
                    diags(&mut out, text, &ds);
                }
            }
            Entry::Expansion { level, name, text } => expansion(&mut out, *level, name, text),
            Entry::Print(text) => out.push_str(&format!("\t\t; %PRINT:\t{text}\n")),
        }
    }
    for (i, d) in all.iter().enumerate() {
        if !shown[i] {
            out.push_str(&format!("\n%BLS64-{}-TEXT, {}\n", d.sev, d.msg));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_lines() {
        let mut out = String::new();
        let mut text = vec!["1".to_string()];
        for _ in 0..28 {
            text.extend(["+".to_string(), "0".to_string()]);
        }
        expansion(&mut out, 0, "W", &Some(text));
        expansion(&mut out, 1, "SUM", &None);
        assert_eq!(
            out,
            "\t\t;;    [W]=  1 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 +\n\
             \t\t;;\t   0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 +\n\
             \t\t;;\t   0\n\
             \t\t;;\t  [SUM]= null\n"
        );
    }
}
