//! The listing (`/LIST`), laid out as BLISSA64 lays it out: pages with
//! their headers, the source part as /SOURCE_LIST and LIST say, the
//! machine code if /MACHINE_CODE asks, and the command line at the end,
//! so that the oracle's listings compare with ours line for line once
//! their page headers are dropped (`crosstools/vtools/docs/vbliss.md`).

/// A lexeme's column when it has none: one a macro body gave.
pub const NOPOS: u32 = u32::MAX;

/// Source lines are cut at this many characters.
const TEXT_WIDTH: usize = 116;

/// Expansion lines end by this column.
const EXPANSION_WIDTH: usize = 84;

pub enum Entry {
    /// A source line: `flag` is 'P' inside a macro call's actuals and 'L'
    /// inside a lexical function's; `depth` the block depth. One not
    /// `listed`, as LIST says, shows only if it has diagnostics.
    Line {
        listed: bool,
        flag: char,
        require: bool,
        depth: u32,
        number: u32,
        file: u16,
        line: u32,
        text: String,
    },
    /// A macro's expansion: `[NAME]=` and its lexemes, or None for null,
    /// with `label` `[NAME]`; `[NAME](1)` and the like for TRACE's
    /// parameter bindings.
    Expansion {
        level: usize,
        label: String,
        text: Option<Vec<String>>,
    },
    /// A step of TRACE's, `[NAME]: Expansion` and the like.
    Trace {
        level: usize,
        text: String,
    },
    /// A `;;` line of LIBRARY's.
    Note(String),
    /// `%PRINT`'s text.
    Print(String),
    /// %TITLE's and %SBTTL's text, for the next pages' headers.
    Title(String),
    Subtitle(String),
}

/// What the listing's pages show besides the entries.
pub struct Layout<'a> {
    /// The files lines come from, by number: a page names its first's.
    pub files: Vec<String>,
    pub module: String,
    pub date: String,
    /// Lines a page has, its header's three among them.
    pub page: usize,
    /// /MACHINE_CODE's: the assembly.
    pub code: Option<&'a str>,
    /// The libraries: each one's name, how many names it declares, how
    /// many the module used, and its source's size in bytes.
    pub libraries: Vec<(String, usize, usize, usize)>,
    pub command: String,
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

fn expansion(out: &mut String, level: usize, label: &str, text: &Option<Vec<String>>) {
    let indent = 22 + 4 * level;
    out.push_str("\t\t;;");
    out.push_str(&pad(18, indent));
    out.push_str(&format!("{label}="));
    let Some(text) = text else {
        out.push_str(" null\n");
        return;
    };
    out.push(' ');
    let start = indent + label.len() + 2;
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
    // A number a place: diagnostics at the same column share one, as
    // BLISSA64 numbers them (oracle QLIT).
    let mut cols: Vec<usize> = Vec::new();
    let mut numbers = Vec::new();
    for d in ds {
        let col = (d.col as usize).min(text.len());
        let n = match cols.iter().position(|&c| c == col) {
            Some(k) => k + 1,
            None => {
                cols.push(col);
                cols.len()
            }
        };
        numbers.push(n);
        let digit = char::from_digit(n as u32 % 10, 10).unwrap();
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
    // A blank line after each message (oracle QLIT).
    for (d, n) in ds.iter().zip(numbers) {
        out.push_str(&format!("%BLS64-{}-TEXT, ({n}) {}\n\n", d.sev, d.msg));
    }
}

/// The source part: the entries, each line's diagnostics after it, and
/// the diagnostics of no line at the end; with the file each output line
/// came from, and the titles where they change.
fn source(entries: &[Entry], all: &[Diag]) -> Vec<Out> {
    let mut out = Vec::new();
    let mut file_now = 0;
    let mut shown = vec![false; all.len()];
    for e in entries {
        let mut s = String::new();
        match e {
            Entry::Line {
                listed,
                flag,
                require,
                depth,
                number,
                file,
                line,
                text,
            } => {
                file_now = *file;
                let mut ds = Vec::new();
                for (i, d) in all.iter().enumerate() {
                    if !shown[i] && d.file == *file && d.line == *line {
                        shown[i] = true;
                        ds.push(d);
                    }
                }
                if !listed && ds.is_empty() {
                    continue;
                }
                let prefix = format!(
                    "{flag}{}{depth:>3}\t{number:>7} ",
                    if *require { 'R' } else { ' ' }
                );
                let chars: Vec<char> = text.chars().collect();
                let mut chunks = chars.chunks(TEXT_WIDTH).peekable();
                if chunks.peek().is_none() {
                    s.push_str(&prefix);
                    s.push('\n');
                }
                for c in chunks {
                    s.push_str(&prefix);
                    s.extend(c);
                    s.push('\n');
                }
                if !ds.is_empty() {
                    diags(&mut s, text, &ds);
                }
            }
            Entry::Expansion { level, label, text } => expansion(&mut s, *level, label, text),
            Entry::Trace { level, text } => {
                s.push_str("\t\t;;");
                s.push_str(&pad(18, 22 + 4 * level));
                s.push_str(text);
                s.push('\n');
            }
            Entry::Note(text) => s.push_str(&format!("\t\t;; {text}\n")),
            Entry::Print(text) => s.push_str(&format!("\t\t; %PRINT:\t{text}\n")),
            Entry::Title(t) => out.push(Out::Title(t.clone())),
            Entry::Subtitle(t) => out.push(Out::Subtitle(t.clone())),
        }
        if let Some(s) = s.strip_suffix('\n') {
            out.extend(s.split('\n').map(|l| Out::Text(l.into(), file_now)));
        }
    }
    let rest: Vec<_> = all.iter().zip(&shown).filter(|(_, s)| !**s).collect();
    if !rest.is_empty() {
        out.push(Out::Text(String::new(), file_now));
        for (d, _) in rest {
            out.push(Out::Text(
                format!("%BLS64-{}-TEXT, {}", d.sev, d.msg),
                file_now,
            ));
        }
        out.push(Out::Text(String::new(), file_now));
    }
    out
}

/// An output line of the source part: text from a file, or a title.
enum Out {
    Text(String, u16),
    Title(String),
    Subtitle(String),
}

/// The listing's text: the source part in pages, then the machine code
/// and the command line, each on pages of their own.
pub fn render(layout: &Layout, entries: &[Entry], all: &[Diag]) -> String {
    let mut out = String::new();
    let mut page = 0;
    let header =
        |out: &mut String, page: &mut usize, module: &str, title: &str, sub: &str, file: &str| {
            if *page > 0 {
                out.push('\n');
            }
            *page += 1;
            out.push_str(&format!(
                "{module:<32}{title:<32}{}  {:<34}Page {page}\n{:<32}{sub:<32}{}  {file}\n\n",
                layout.date,
                crate::TOOL,
                "",
                layout.date
            ));
        };
    let per_page = layout.page.saturating_sub(3).max(1);
    let (mut title, mut sub) = ("Source Listing".to_string(), String::new());
    let mut used = per_page;
    for o in source(entries, all) {
        match o {
            Out::Title(t) => title = t,
            Out::Subtitle(t) => sub = t,
            Out::Text(s, file) => {
                if used == per_page {
                    // BLISS writes the first page's header before it
                    // knows the module's name.
                    let module = if page == 0 { "" } else { &layout.module };
                    let name = layout
                        .files
                        .get(usize::from(file))
                        .map_or("", String::as_str);
                    header(&mut out, &mut page, module, &title, &sub, name);
                    used = 0;
                }
                out.push_str(&s);
                out.push('\n');
                used += 1;
            }
        }
    }
    let main = layout.files.first().map_or("", String::as_str);
    if let Some(code) = layout.code {
        used = per_page;
        for l in code.lines() {
            if used == per_page {
                header(
                    &mut out,
                    &mut page,
                    &layout.module,
                    "Machine Code Listing",
                    "",
                    main,
                );
                used = 0;
            }
            out.push_str(l);
            out.push('\n');
            used += 1;
        }
    }
    out.push('\n');
    if !layout.libraries.is_empty() {
        out.push_str("\n;\t\t\t\tLibrary Statistics\n;\n");
        out.push_str(";\t\t\t\t\t     -------- Symbols --------\t    Pages\n");
        out.push_str(";\tFile\t\t\t\t     Total    Loaded   Percent      Mapped\n;\n");
        for (name, total, loaded, bytes) in &layout.libraries {
            let percent = (loaded * 100).checked_div(*total).unwrap_or(0);
            out.push_str(&format!(
                ";  {name}\t{total:>10}{loaded:>10}{percent:>10}{:>10}\n",
                bytes.div_ceil(512)
            ));
        }
        out.push_str("\n\n\n");
    }
    header(
        &mut out,
        &mut page,
        "",
        "Source Listing",
        "Source Listing",
        main,
    );
    out.push_str(&format!("COMMAND LINE:\n\n{}\n", layout.command));
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
        expansion(&mut out, 0, "[W]", &Some(text));
        expansion(&mut out, 1, "[SUM]", &None);
        assert_eq!(
            out,
            "\t\t;;    [W]=  1 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 +\n\
             \t\t;;\t   0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 + 0 +\n\
             \t\t;;\t   0\n\
             \t\t;;\t  [SUM]= null\n"
        );
    }
}
