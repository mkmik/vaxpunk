//! File names and file specifications: parsing, validation, wildcard
//! matching and directory order, for both structure levels.
//!
//! Names are bytes as stored on disk: "NAME.TYPE", the dot always present.
//! ODS-2 names are uppercase A-Z, 0-9, `$`, `-`, `_`, at most 39.39.
//! ODS-5 names are ISO Latin-1, case preserved but compared case-blind, at
//! most 236 bytes. See docs/names.md.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::layout::NameType;
use crate::volume::Level;

/// `*` in a parsed pattern. Control characters are never valid in names,
/// so they can stand for wildcards without ambiguity.
pub const ANY: u8 = 0x01;
/// `%` or `?` in a parsed pattern.
pub const ONE: u8 = 0x02;
/// A `...` directory component.
pub const ELLIPSIS: &[u8] = &[0x03];

pub const MAX_VERSION: u16 = 32767;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// No version, `;` or `;0`: the highest existing one.
    Highest,
    Exact(u16),
    /// `;-n`: n versions below the highest.
    Relative(u16),
    /// `;*`
    All,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileName {
    /// "NAME.TYPE", possibly with wildcards.
    pub name: Vec<u8>,
    pub version: Version,
}

/// A parsed file specification: the directory path below the MFD, and
/// the file name if there is one. Any device name is dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    pub dirs: Vec<Vec<u8>>,
    pub file: Option<FileName>,
}

impl Spec {
    pub fn is_wild(&self) -> bool {
        let wild = |n: &[u8]| n.iter().any(|&c| c == ANY || c == ONE || c == ELLIPSIS[0]);
        self.dirs.iter().any(|d| wild(d))
            || self.file.as_ref().is_some_and(|f| wild(&f.name) || f.version == Version::All)
    }
}

/// A lexical token: a character to take literally, or an unescaped
/// delimiter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tok {
    Lit(u8),
    Sep(u8),
}

fn hex(c: char) -> Option<u8> {
    c.to_digit(16).map(|d| d as u8)
}

fn tokenize(level: Level, s: &str) -> Result<Vec<Tok>, &'static str> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    let latin1 = |c: char| u8::try_from(c as u32).map_err(|_| "character outside ISO Latin-1");
    while let Some(c) = it.next() {
        let t = match c {
            '^' if level == Level::Ods5 => match it.next().ok_or("dangling ^")? {
                '_' | ' ' => Tok::Lit(b' '),
                'U' | 'u' => {
                    let mut v = 0u32;
                    for _ in 0..4 {
                        v = v << 4 | hex(it.next().ok_or("short ^U escape")?).ok_or("bad ^U escape")? as u32;
                    }
                    Tok::Lit(u8::try_from(v).map_err(|_| "UCS-2 names are not supported")?)
                }
                d if hex(d).is_some() => {
                    let lo = it.next().and_then(hex).ok_or("bad ^ hex escape")?;
                    Tok::Lit(hex(d).unwrap_or(0) << 4 | lo)
                }
                other => Tok::Lit(latin1(other)?),
            },
            '[' | ']' | '<' | '>' | '.' | ';' | ':' | '*' | '%' | '?' => Tok::Sep(c as u8),
            _ if level == Level::Ods2 => Tok::Lit(latin1(c.to_ascii_uppercase())?),
            _ => Tok::Lit(latin1(c)?),
        };
        out.push(t);
    }
    Ok(out)
}

fn literal(t: Tok) -> u8 {
    match t {
        Tok::Lit(c) => c,
        Tok::Sep(b'*') => ANY,
        Tok::Sep(b'%' | b'?') => ONE,
        Tok::Sep(c) => c,
    }
}

/// Parses "[DIR.SUB]NAME.TYPE;VERSION" (any part optional) for a volume of
/// the given level. ODS-2 input is uppercased; ODS-5 escapes are resolved.
pub fn parse(level: Level, s: &str) -> Result<Spec, &'static str> {
    let mut toks = tokenize(level, s)?;
    if let Some(i) = toks.iter().position(|&t| t == Tok::Sep(b':')) {
        toks.drain(..=i);
    }
    let mut dirs = Vec::new();
    if let Some(&Tok::Sep(open @ (b'[' | b'<'))) = toks.first() {
        let close = if open == b'[' { b']' } else { b'>' };
        let end = toks.iter().position(|&t| t == Tok::Sep(close)).ok_or("unterminated directory")?;
        let inner: Vec<Tok> = toks.drain(..=end).skip(1).take(end - 1).collect();
        let mut comp = Vec::new();
        let mut i = 0;
        while i < inner.len() {
            match inner[i] {
                Tok::Sep(b'.') => {
                    let dots = inner[i..].iter().take_while(|&&t| t == Tok::Sep(b'.')).count();
                    if !comp.is_empty() {
                        dirs.push(core::mem::take(&mut comp));
                    }
                    match dots {
                        1 => {}
                        3 => dirs.push(ELLIPSIS.to_vec()),
                        _ => return Err("bad directory delimiter"),
                    }
                    i += dots;
                }
                Tok::Sep(b'[' | b']' | b'<' | b'>' | b';' | b':') => return Err("bad character in directory"),
                t => {
                    comp.push(literal(t));
                    i += 1;
                }
            }
        }
        if !comp.is_empty() {
            dirs.push(comp);
        }
        if dirs.first().is_some_and(|d| d == b"000000") {
            dirs.remove(0);
        }
    }
    if toks.iter().any(|t| matches!(t, Tok::Sep(b'[' | b']' | b'<' | b'>' | b':'))) {
        return Err("unexpected delimiter");
    }
    let file = if toks.is_empty() { None } else { Some(parse_file(level, &toks)?) };
    Ok(Spec { dirs, file })
}

/// Parses just "NAME.TYPE;VERSION".
pub fn parse_name(level: Level, s: &str) -> Result<FileName, &'static str> {
    let spec = parse(level, s)?;
    match (spec.dirs.is_empty(), spec.file) {
        (true, Some(f)) => Ok(f),
        _ => Err("expected a file name"),
    }
}

fn parse_file(level: Level, toks: &[Tok]) -> Result<FileName, &'static str> {
    let dots: Vec<usize> = (0..toks.len()).filter(|&i| toks[i] == Tok::Sep(b'.')).collect();
    let (body, ver) = if let Some(i) = toks.iter().rposition(|&t| t == Tok::Sep(b';')) {
        (&toks[..i], Some(&toks[i + 1..]))
    } else if let [.., _, b] = dots[..] {
        // NAME.TYPE.VERSION, when what follows the last dot is a number.
        let tail = &toks[b + 1..];
        let numeric = tail.iter().all(|&t| matches!(t, Tok::Lit(b'0'..=b'9' | b'-')) || t == Tok::Sep(b'*'));
        if numeric && !tail.is_empty() { (&toks[..b], Some(tail)) } else { (toks, None) }
    } else {
        (toks, None)
    };
    let version = match ver {
        None => Version::Highest,
        Some(v) => parse_version(v)?,
    };
    let split = body.iter().rposition(|&t| t == Tok::Sep(b'.'));
    let mut name: Vec<u8> = Vec::new();
    for (i, &t) in body.iter().enumerate() {
        match t {
            Tok::Sep(b'.') if Some(i) != split && level == Level::Ods2 => return Err("more than one dot"),
            Tok::Sep(b';') => return Err("more than one semicolon"),
            t => name.push(literal(t)),
        }
    }
    if split.is_none() {
        name.push(b'.');
    }
    Ok(FileName { name, version })
}

fn parse_version(v: &[Tok]) -> Result<Version, &'static str> {
    if v == [Tok::Sep(b'*')] {
        return Ok(Version::All);
    }
    let s: Vec<u8> = v.iter().map(|&t| literal(t)).collect();
    let (neg, digits) = match s.split_first() {
        Some((b'-', rest)) => (true, rest),
        _ => (false, &s[..]),
    };
    if digits.is_empty() {
        return if neg { Err("bad version") } else { Ok(Version::Highest) };
    }
    if digits.len() > 5 || !digits.iter().all(u8::is_ascii_digit) {
        return Err("bad version");
    }
    let n = digits.iter().fold(0u32, |n, &d| n * 10 + (d - b'0') as u32);
    match (neg, n) {
        (_, 0) => Ok(Version::Highest),
        (_, n) if n > MAX_VERSION as u32 => Err("version above 32767"),
        (true, n) => Ok(Version::Relative(n as u16)),
        (false, n) => Ok(Version::Exact(n as u16)),
    }
}

/// Splits "NAME.TYPE" at the last dot.
pub fn split(name: &[u8]) -> (&[u8], &[u8]) {
    match name.iter().rposition(|&c| c == b'.') {
        Some(i) => (&name[..i], &name[i + 1..]),
        None => (name, &[]),
    }
}

/// Checks that `name` ("NAME.TYPE", no wildcards) may be created.
pub fn validate(level: Level, name: &[u8]) -> Result<(), &'static str> {
    if !name.contains(&b'.') {
        return Err("no dot between name and type");
    }
    let (n, t) = split(name);
    match level {
        Level::Ods2 => {
            let ok = |c: &u8| matches!(c, b'A'..=b'Z' | b'0'..=b'9' | b'$' | b'-' | b'_');
            if n.len() > 39 || t.len() > 39 {
                Err("ODS-2 names and types are at most 39 characters")
            } else if !n.iter().chain(t).all(ok) {
                Err("ODS-2 names use only A-Z, 0-9, $, - and _")
            } else {
                Ok(())
            }
        }
        Level::Ods5 => {
            let bad = |&c: &u8| c < 0x20 || b"\"*\\:<>/?|".contains(&c);
            if name.len() > 236 {
                Err("ODS-5 names are at most 236 characters")
            } else if name.iter().any(bad) {
                Err("character not allowed in an ODS-5 name")
            } else {
                Ok(())
            }
        }
    }
}

/// ISO Latin-1 uppercase: ASCII letters and the accented letters, except
/// the division sign and y with diaeresis, which have no uppercase in
/// Latin-1.
pub fn upcase(c: u16) -> u16 {
    match c {
        0x61..=0x7a | 0xe0..=0xf6 | 0xf8..=0xfe => c - 0x20,
        _ => c,
    }
}

/// A stored name as 16-bit characters, whatever its encoding.
pub fn chars(name: &[u8], t: NameType) -> Vec<u16> {
    match t {
        NameType::Ucs2 => name.as_chunks::<2>().0.iter().map(|p| u16::from_le_bytes([p[0], p[1]])).collect(),
        _ => name.iter().map(|&c| c as u16).collect(),
    }
}

/// Directory order. ODS-2 names are uppercase and compare as bytes; ODS-5
/// names compare case-blind.
pub fn cmp(a: &[u16], b: &[u16]) -> Ordering {
    a.iter().map(|&c| upcase(c)).cmp(b.iter().map(|&c| upcase(c)))
}

/// Whether a name matches a pattern with `ANY` and `ONE` wildcards; name
/// and type are matched separately, case-blind.
pub fn matches(pattern: &[u16], name: &[u16]) -> bool {
    let at = |s: &[u16]| s.iter().rposition(|&c| c == b'.' as u16).unwrap_or(s.len());
    let (pn, pt) = pattern.split_at(at(pattern));
    let (nn, nt) = name.split_at(at(name));
    glob(pn, nn) && glob(pt, nt)
}

fn glob(p: &[u16], s: &[u16]) -> bool {
    // Iterative wildcard match: remember the last `*` and retry from there.
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == ONE as u16 || p[pi] != ANY as u16 && upcase(p[pi]) == upcase(s[si])) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == ANY as u16 {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == ANY as u16)
}

/// A stored name for display: ODS-2 as is, ODS-5 with `^` escapes where
/// VMS would print them, so the result parses back to the same name.
pub fn display(level: Level, name: &[u8], t: NameType) -> String {
    let cs = chars(name, t);
    let type_dot = cs.iter().rposition(|&c| c == b'.' as u16);
    let mut out = String::new();
    for (i, &c) in cs.iter().enumerate() {
        match char::from_u32(c as u32) {
            _ if level == Level::Ods2 => out.push(char::from(c as u8)),
            Some('.') if Some(i) != type_dot => out.push_str("^."),
            Some(' ') => out.push_str("^_"),
            Some(ch @ (',' | ';' | '[' | ']' | '%' | '^' | '&' | '<' | '>' | ':')) => {
                out.push('^');
                out.push(ch);
            }
            Some(ch) if c > 0xff => {
                let _ = core::fmt::Write::write_fmt(&mut out, format_args!("^U{:04X}", ch as u32));
            }
            Some(ch) if (ch as u32) < 0x20 || (0x7f..0xa0).contains(&(ch as u32)) => {
                let _ = core::fmt::Write::write_fmt(&mut out, format_args!("^{:02X}", ch as u32));
            }
            Some(ch) => out.push(ch),
            None => out.push('?'),
        }
    }
    out
}
