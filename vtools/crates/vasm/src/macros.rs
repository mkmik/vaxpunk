//! Macros in the MACRO tradition: a body of text lines whose formal
//! arguments are replaced by the call's arguments. Also the line sources the
//! assembler reads from: files, macro expansions and repeat blocks.

use std::collections::HashMap;
use std::rc::Rc;

use crate::lex::Cursor;

/// A source line and where it came from.
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub text: String,
    pub loc: Loc,
}

/// A file and line, and the macro call or repeat block that produced it.
#[derive(Clone, Debug, Default)]
pub struct Loc {
    pub file: Rc<str>,
    pub line: usize,
    pub via: Option<Rc<(String, Loc)>>,
}

impl Loc {
    /// One entry per level of expansion, innermost first.
    pub fn context(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut via = self.via.clone();
        while let Some(v) = via {
            out.push(format!("in {}, at {}:{}", v.0, v.1.file, v.1.line));
            via = v.1.via.clone();
        }
        out
    }
}

pub fn lines(file: &Rc<str>, source: &str) -> Vec<Line> {
    source
        .lines()
        .enumerate()
        .map(|(i, text)| Line {
            text: text.to_string(),
            loc: Loc {
                file: file.clone(),
                line: i + 1,
                via: None,
            },
        })
        .collect()
}

pub struct Macro {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Line>,
}

pub struct Param {
    pub name: String,
    pub default: Option<String>,
    /// `?NAME`: a created local label unless the call gives one.
    pub created: bool,
}

/// The formal arguments after `.MACRO NAME`: `A, B=default, ?LABEL`.
pub fn params(text: &str) -> Result<Vec<Param>, String> {
    let mut out: Vec<Param> = Vec::new();
    if text.trim().is_empty() {
        return Ok(out);
    }
    for raw in split(text) {
        let (created, arg) = match raw.strip_prefix('?') {
            Some(a) => (true, a.trim()),
            None => (false, raw.as_str()),
        };
        let (name, default) = match arg.split_once('=') {
            Some((n, d)) => (n.trim(), Some(unbracket(d.trim()).to_string())),
            None => (arg, None),
        };
        if !is_name(name) {
            return Err(format!("bad formal argument '{raw}'"));
        }
        let name = name.to_ascii_uppercase();
        if out.iter().any(|p| p.name == name) {
            return Err(format!("formal argument {name} appears twice"));
        }
        out.push(Param {
            name,
            default,
            created,
        });
    }
    Ok(out)
}

/// Binds a call's arguments to the formals: keyword (`NAME=value`) or
/// positional, then defaults and created labels. Returns the substitutions
/// and how many positional arguments the call gave.
pub fn bind(
    m: &Macro,
    args: &[String],
    next_label: &mut u32,
) -> Result<(HashMap<String, String>, usize), String> {
    let mut given: HashMap<String, String> = HashMap::new();
    let mut positional = 0;
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let key = key.trim().to_ascii_uppercase();
            if m.params.iter().any(|p| p.name == key) {
                given.insert(key, unbracket(value.trim()).to_string());
                continue;
            }
        }
        let Some(p) = m.params.get(positional) else {
            return Err(format!("too many arguments for macro {}", m.name));
        };
        positional += 1;
        if !arg.is_empty() {
            given.insert(p.name.clone(), arg.clone());
        }
    }
    let mut values = HashMap::new();
    for p in &m.params {
        let value = match given.remove(&p.name) {
            Some(v) => v,
            None if p.created => {
                *next_label += 1;
                format!("{}$", *next_label - 1)
            }
            None => p.default.clone().unwrap_or_default(),
        };
        values.insert(p.name.clone(), value);
    }
    Ok((values, positional))
}

/// Replaces formal names in `text` by their values. An apostrophe next to a
/// formal name concatenates, and disappears.
pub fn substitute(text: &str, values: &HashMap<String, String>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        if is_name_start(chars[i]) {
            while i < chars.len() && is_name_char(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            match values.get(&word.to_ascii_uppercase()) {
                Some(value) => {
                    if out.ends_with('\'') {
                        out.pop();
                    }
                    out.push_str(value);
                    if chars.get(i) == Some(&'\'') {
                        i += 1;
                    }
                }
                None => out.push_str(&word),
            }
        } else if chars[i].is_ascii_digit() {
            // Numbers and local labels are never formal names.
            while i < chars.len() && (is_name_char(chars[i])) {
                i += 1;
            }
            out.extend(&chars[start..i]);
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// One argument, up to a comma outside `<>`, `[]`, `()` and quotes. An
/// argument that is all inside `<...>` loses the brackets, as in MACRO.
pub fn arg(c: &mut Cursor) -> String {
    c.skip_ws();
    let rest = c.rest();
    let mut depth = 0i32;
    let mut quote = false;
    let mut end = rest.len();
    for (i, ch) in rest.char_indices() {
        match ch {
            '"' => quote = !quote,
            _ if quote => {}
            '<' | '[' | '(' => depth += 1,
            '>' | ']' | ')' => depth -= 1,
            ',' if depth <= 0 => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    c.at += end;
    unbracket(rest[..end].trim()).to_string()
}

/// Arguments separated by commas.
pub fn split(text: &str) -> Vec<String> {
    let mut c = Cursor::new(text);
    let mut out = vec![arg(&mut c)];
    while c.eat(',') {
        out.push(arg(&mut c));
    }
    out
}

/// `<text>` without its brackets, if they enclose all of it.
pub fn unbracket(s: &str) -> &str {
    let Some(inner) = s.strip_prefix('<').and_then(|s| s.strip_suffix('>')) else {
        return s;
    };
    // `<a> <b>` starts and ends with brackets but isn't one group.
    let mut depth = 0;
    for ch in inner.chars() {
        match ch {
            '<' => depth += 1,
            '>' if depth == 0 => return s,
            '>' => depth -= 1,
            _ => {}
        }
    }
    inner
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '$' | '_' | '.')
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '$' | '_' | '.')
}

fn is_name(s: &str) -> bool {
    s.starts_with(is_name_start) && s.chars().all(is_name_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(params: &str) -> Macro {
        Macro {
            name: "M".into(),
            params: super::params(params).unwrap(),
            body: Vec::new(),
        }
    }

    #[test]
    fn binding() {
        let m = mac("A, B=7, C=<x, y>, ?L");
        let mut next = 30000;
        let (v, n) = bind(&m, &split("1, , C=[x1, #8]"), &mut next).unwrap();
        assert_eq!(
            (
                v["A"].as_str(),
                v["B"].as_str(),
                v["C"].as_str(),
                v["L"].as_str()
            ),
            ("1", "7", "[x1, #8]", "30000$")
        );
        assert_eq!(n, 2, "positional arguments, the empty one too");
        let (v, _) = bind(&m, &split("<a, b>"), &mut next).unwrap();
        assert_eq!(
            (v["A"].as_str(), v["C"].as_str(), v["L"].as_str()),
            ("a, b", "x, y", "30001$")
        );
        assert!(bind(&m, &split("1, 2, 3, 4, 5"), &mut next).is_err());
    }

    #[test]
    fn substitution() {
        let values: HashMap<String, String> = [("REG", "x3"), ("N", "12"), ("TEXT", "hi")]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into();
        assert_eq!(
            substitute("add reg, reg, #n ; N", &values),
            "add x3, x3, #12 ; 12"
        );
        assert_eq!(
            substitute("L'N: .ASCII \"TEXT\" 10$ B.EQ", &values),
            "L12: .ASCII \"hi\" 10$ B.EQ"
        );
        assert_eq!(substitute("REG'_SAVE", &values), "x3_SAVE");
        assert_eq!(unbracket("<a> <b>"), "<a> <b>");
    }
}
