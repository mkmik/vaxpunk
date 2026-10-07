//! Writes docs/api/index.html, the API reference, from the sources.
//!
//! Nothing in the page is written by hand: it is all read from the code and the
//! design documents, so keeping it current means keeping these current:
//! - the executive's routines and data (roottask/exec/*.mar, consolio.mar,
//!   and the `GLOBAL ROUTINE`s of roottask/exec/*.b64, BLISS-64), and
//!   the global ones of the libraries the system disk's images link
//!   (roottask/sysexe/lib/*.mar) and of DCL's own (roottask/sysexe/dcl.mar,
//!   roottask/sysexe/dcl/*.mar), and of the RMS utilities' (roottask/sysexe/rms/*.mar):
//!   the comment block right above each `NAME::`
//!   or `.ENTRY`, whose first line reads `NAME: what it does` or, for a system
//!   service, `$NAME args: what it does`;
//! - how the code reaches each of them, which is its linkage: `.ENTRY`, a
//!   `JSB` or `BSBx`, the SCB slot `EXEC$START` stores it in, or else the
//!   branches to it, the code that runs on into it and the routines that use
//!   its address, and whether it comes back with `RSB`;
//! - the system service vector, the SERVICE and STUB lines of syssrv.mar;
//! - the macro libraries (vtools/lib/*.mlb, roottask/sysexe/*.mlb): the comment
//!   block right above each `.MACRO`, and the `SYM = value ; meaning` lines of
//!   the $xxxDEF macros;
//! - the PAL calls: the tables of DESIGN-0001 (docs/design/0001-pal-interface.md).
//!
//! Run it to see the page locally; it isn't committed. CI runs it on every PR
//! and publishes main's to GitHub Pages:
//!
//!     cargo run -p apidoc

use regex::{Captures, Regex};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

macro_rules! re {
    ($s:expr) => {{
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new($s).unwrap());
        &*RE
    }};
}

const LABEL: &str = r"^([A-Za-z][\w$]*)(::?)(.*)";
const ENTRY: &str = r"(?i)^\s+\.ENTRY\s+([\w$]+)";
const DIRECTIVE: &str = r"(?i)\.(LONG|WORD|BYTE|QUAD|BLK[BWLQ]|ASCI[CDZI]|ADDRESS)\b[^;]*";
const CALL: &str = r"(?i)^(BSBB|BSBW|JSB|JMP|BRB|BRW|CALLS|CALLG)\s+(.*)";
const SYMBOL: &str = r"\$?[A-Za-z][\w$]*";
// An instruction the next one never follows, or a call that may not return:
// a label after it is reached some other way.
const NO_FALL: &str = r"(?i)^(RSB|RET|REI|HALT|BRB|BRW|JMP|CASE[BWL]|CALLS|CALLG)\b";
const BRANCH: &str = r"(?i)^(BRB|BRW|JMP|B(EQL|NEQ|GTRU?|LEQU?|GEQU?|LSSU?|CC|CS|VC|VS|LB[CS]|B[SC][SC]?)|SOB(GTR|GEQ)|AOB(LSS|LEQ)|ACB[BWL])$";
const PAL_DOC: &str = "docs/design/0001-pal-interface.md";

#[derive(Clone, Copy, PartialEq)]
enum K {
    P,
    Pre,
    Note,
}
type Doc = Vec<(K, String)>;

fn code(line: &str) -> &str {
    // ponytail: a ';' inside a string literal would cut the line; the
    // sources have none
    line.split(';').next().unwrap()
}

fn comment(line: &str) -> String {
    let text = &line.trim()[1..];
    text.strip_prefix(' ').unwrap_or(text).to_string()
}

/// Comment lines as paragraphs, indented examples and ponytail notes.
fn blocks(lines: &[String]) -> Doc {
    let (mut out, mut kind, mut buf) = (Doc::new(), None, Vec::<String>::new());
    let flush = |out: &mut Doc, kind: Option<K>, buf: &mut Vec<String>| {
        if !buf.is_empty() {
            let k = kind.unwrap();
            out.push((k, buf.join(if k == K::Pre { "\n" } else { " " })));
            buf.clear();
        }
    };
    for line in lines {
        if line.trim().is_empty() {
            flush(&mut out, kind, &mut buf);
            kind = None;
        } else if let Some(rest) = line.strip_prefix("ponytail:") {
            flush(&mut out, kind, &mut buf);
            kind = Some(K::Note);
            buf.push(rest.trim().to_string());
        } else if (line.starts_with("  ") || line.starts_with('\t')) && kind != Some(K::Note) {
            if kind != Some(K::Pre) {
                flush(&mut out, kind, &mut buf);
                kind = Some(K::Pre);
            }
            buf.push(line.clone());
        } else {
            if !matches!(kind, Some(K::P | K::Note)) {
                flush(&mut out, kind, &mut buf);
                kind = Some(K::P);
            }
            buf.push(line.trim().to_string());
        }
    }
    flush(&mut out, kind, &mut buf);
    out
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Splits `NAME args: text` off the first paragraph: (NAME, args, rest).
fn head(doc: &Doc) -> (Option<String>, String, Doc) {
    if let Some((K::P, text)) = doc.first()
        && let Some(m) = re!(r"(?s)^(\$?[\w$]+)([^:]*):\s+(.*)").captures(text)
    {
        let mut rest = vec![(K::P, capitalize(&m[3]))];
        rest.extend(doc[1..].iter().cloned());
        return (Some(m[1].to_string()), m[2].trim().to_string(), rest);
    }
    (None, String::new(), doc.clone())
}

/// Splits after each '.' that whitespace follows, like `re.split(r"(?<=\.)\s+")`.
fn sentences(text: &str) -> Vec<&str> {
    let (mut out, mut start, mut prev) = (vec![], 0, ' ');
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if c.is_whitespace() && prev == '.' {
            out.push(&text[start..i]);
            while it.next_if(|(_, c)| c.is_whitespace()).is_some() {}
            start = it.peek().map_or(text.len(), |(j, _)| *j);
            prev = ' ';
            continue;
        }
        prev = c;
    }
    out.push(&text[start..]);
    out
}

/// Moves "Uses R0.", "Keeps every register." and "IPL$_SYNCH." out of the
/// description: (description, registers, IPL).
fn facts(doc: Doc) -> (Doc, String, String) {
    let (mut regs, mut ipl, mut out) = (vec![], vec![], Doc::new());
    for (kind, mut text) in doc {
        if kind == K::P {
            let mut keep = vec![];
            for s in sentences(&text) {
                if re!(r"^(Uses|Keeps) .*\.$").is_match(s) {
                    regs.push(s.to_string());
                } else if re!(r"^IPL\$_\w+\.$").is_match(s) {
                    ipl.push(s[..s.len() - 1].to_string());
                } else {
                    keep.push(s);
                }
            }
            text = keep.join(" ");
            if text.is_empty() {
                continue;
            }
        }
        out.push((kind, text));
    }
    (out, regs.join(" "), ipl.join(", "))
}

// ---------------------------------------------------------------- sources

#[derive(Default)]
struct Module {
    /// One of the libraries images link, not the executive's.
    image: bool,
    path: String,
    stem: String,
    title: String,
    intro: Doc,
    routines: Vec<usize>,
    data: Vec<usize>,
}

#[derive(Default)]
struct Routine {
    name: String,
    line: usize,
    module: usize,
    entry: bool,
    global: bool,
    comment: Vec<String>,
    body: Vec<String>,
    labels: Vec<String>,
    args: String,
    doc: Doc,
    regs: String,
    ipl: String,
    status: BTreeSet<String>,
    edges: Vec<(usize, bool)>,
    pal: Vec<String>,
    svcs: Vec<String>,
    uses: BTreeSet<String>,
    returns: BTreeSet<String>,
    calls: Vec<usize>,
    // How the code reaches it, for its linkage: the routine whose last
    // instruction runs on into it, the SCB slot EXEC$START stores it in,
    // whether a BSBx or JSB calls it, and the routines that branch to it or
    // use its address.
    falls_from: Option<String>,
    scb: Option<String>,
    called: bool,
    comes_back: bool, // an RSB or RET ends it, or what it branches or runs on to

    branched: Vec<usize>,
    addressed: Vec<usize>,
}

struct Data {
    name: String,
    doc: Doc,
    storage: String,
}

struct Service {
    name: String,
    code: u32,
    nargs: u32,
    stub: bool,
    group: String,
    chme: bool,
    routine: Option<usize>,
    mac: Option<usize>,
}

struct Lib {
    path: String,
    stem: String,
    name: String,
    notes: Vec<Doc>,
    macros: Vec<usize>,
}

#[derive(Default)]
struct Macro {
    name: String,
    params: String,
    line: usize,
    lib: usize,
    comment: Vec<String>,
    body: Vec<String>,
    shares: Vec<String>,
    doc: Doc,
    shared: Option<usize>,
    defs: Vec<(String, String, String)>,
}

struct Pal {
    name: String,
    code: u32,
    cols: HashMap<String, String>,
    vax: Vec<String>,
    users: Vec<usize>,
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

fn glob(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect();
    out.sort();
    out
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("apidoc: {}: {e}", path.display()))
}

fn parse_mar(
    root: &Path,
    path: &Path,
    mi: usize,
    routines: &mut Vec<Routine>,
    data: &mut Vec<Data>,
) -> Module {
    let text = read(path);
    let lines: Vec<&str> = text.lines().collect();
    let mut m = Module {
        path: rel(root, path),
        stem: stem(path),
        ..Default::default()
    };
    let (mut block, mut cur, mut in_code, mut in_macro) =
        (Vec::<String>::new(), None::<usize>, true, false);
    // the module's macros that make an entry point, `.ENTRY PREFIX'NAME`,
    // by name: their prefix
    let (mut makers, mut macro_name) = (HashMap::<String, String>::new(), String::new());
    // the routine whose instruction came last, and that instruction, if
    // the next label follows it in the same psect
    let mut last: Option<(String, String)> = None;
    for (i, line) in lines.iter().enumerate() {
        let no = i + 1;
        let s = line.trim();
        if s.starts_with(';') {
            block.push(comment(line));
            continue;
        }
        if m.intro.is_empty() && !block.is_empty() && m.routines.is_empty() && m.data.is_empty() {
            m.intro = blocks(&block);
        }
        if s.is_empty() {
            block.clear();
            continue;
        }
        let up = s.to_uppercase();
        if up.starts_with(".MACRO") {
            in_macro = true;
            macro_name = up.split_whitespace().nth(1).unwrap_or("").to_string();
        }
        if in_macro {
            if let Some(c) = re!(r"(?i)^\s+\.ENTRY\s+([\w$]+)'NAME\b").captures(line) {
                makers.insert(macro_name.clone(), c[1].to_uppercase());
            }
            in_macro = !up.starts_with(".ENDM");
            block.clear();
            continue;
        }
        // an entry point a macro makes: `MACRO NAME, ...`
        if let Some((mac, name)) = up.split_once(char::is_whitespace)
            && let Some(prefix) = makers.get(mac)
        {
            let name = name.trim_start().split([',', ' ']).next().unwrap_or("");
            cur = None;
            m.routines.push(routines.len());
            routines.push(Routine {
                name: format!("{prefix}{name}"),
                line: no,
                module: mi,
                entry: true,
                global: true,
                comment: block.clone(),
                ..Default::default()
            });
            block.clear();
            continue;
        }
        if up.starts_with(".TITLE") {
            let words: Vec<&str> = s.split_whitespace().collect();
            m.title = if words.len() > 2 {
                let rest = s[words[0].len()..].trim_start();
                rest[words[1].len()..].trim_start().to_string()
            } else {
                String::new()
            };
        } else if up.starts_with(".PSECT") {
            in_code = !up.contains("NOEXE");
            last = None;
        }
        let (label, entry) = match re!(ENTRY).captures(line) {
            Some(c) => (Some((c[1].to_string(), String::new(), String::new())), true),
            None => (
                re!(LABEL)
                    .captures(line)
                    .map(|c| (c[1].to_string(), c[2].to_string(), c[3].to_string())),
                false,
            ),
        };
        if let Some((name, sep, rest)) = label {
            if !in_code {
                if sep == "::" {
                    let mut d = re!(DIRECTIVE)
                        .find(code(&rest))
                        .map(|d| d.as_str().trim().to_string());
                    if d.is_none() && no < lines.len() {
                        d = re!(DIRECTIVE)
                            .find(code(lines[no]).trim())
                            .filter(|d| d.start() == 0)
                            .map(|d| d.as_str().trim().to_string());
                    }
                    let mut b = block.clone();
                    if let Some((_, note)) = rest.split_once(';')
                        && !note.trim().is_empty()
                    {
                        b.push(note.trim().to_string());
                    }
                    m.data.push(data.len());
                    data.push(Data {
                        name,
                        doc: blocks(&b),
                        storage: d.unwrap_or_default(),
                    });
                }
            } else if entry || sep == "::" || !block.is_empty() {
                cur = Some(routines.len());
                m.routines.push(routines.len());
                routines.push(Routine {
                    name,
                    line: no,
                    module: mi,
                    entry,
                    global: entry || sep == "::",
                    comment: block.clone(),
                    falls_from: last
                        .take()
                        .filter(|(_, insn)| !re!(NO_FALL).is_match(insn))
                        .map(|(owner, _)| owner),
                    ..Default::default()
                });
            } else if let Some(c) = cur {
                routines[c].labels.push(name);
            }
        }
        if let (Some(c), false) = (cur, entry) {
            let insn = re!(r"^\s*(?:[\w$]+::?\s*)+").replace(code(line), "");
            let insn = insn.trim();
            if !insn.is_empty() && !insn.contains('=') && !insn.starts_with('.') {
                routines[c].body.push(insn.to_string());
                last = Some((routines[c].name.clone(), insn.to_string()));
            }
        }
        block.clear();
    }
    m
}

/// A BLISS-64 module: its `%TITLE`, the comment block after `MODULE` and
/// the one right above each `GLOBAL ROUTINE`, which is called.
fn parse_b64(root: &Path, path: &Path, mi: usize, routines: &mut Vec<Routine>) -> Module {
    let text = read(path);
    let mut m = Module {
        path: rel(root, path),
        stem: stem(path),
        ..Default::default()
    };
    let mut block = Vec::<String>::new();
    for (i, line) in text.lines().enumerate() {
        let s = line.trim();
        if s.starts_with('!') {
            block.push(comment(line));
            continue;
        }
        if let Some(t) = re!(r"^%TITLE\s+'([^']*)'").captures(s) {
            m.title = t[1].to_string();
        } else if let Some(g) = re!(r"(?i)^GLOBAL\s+ROUTINE\s+([\w$]+)").captures(s) {
            m.routines.push(routines.len());
            routines.push(Routine {
                name: g[1].to_string(),
                line: i + 1,
                module: mi,
                entry: true,
                global: true,
                comment: block.clone(),
                ..Default::default()
            });
        } else if m.intro.is_empty() && !block.is_empty() && m.routines.is_empty() {
            m.intro = blocks(&block);
        }
        block.clear();
    }
    m
}

fn parse_sstab(path: &Path) -> Vec<Service> {
    let (mut services, mut group, mut prev, mut code) = (vec![], String::new(), String::new(), 0);
    for line in read(path).lines() {
        if let Some(m) = re!(r"^\s+(SERVICE|STUB|ESERVICE)\s+(\w+),\s*(\d+)").captures(line) {
            if prev.starts_with(';') {
                group = comment(&prev);
            }
            let chme = &m[1] == "ESERVICE"; // the executive mode one: CHME #0
            services.push(Service {
                name: m[2].to_string(),
                code: if chme { 0 } else { code },
                nargs: m[3].parse().unwrap(),
                stub: &m[1] == "STUB",
                group: group.clone(),
                chme,
                routine: None,
                mac: None,
            });
            code += !chme as u32;
        }
        prev = line.trim().to_string();
    }
    services
}

fn parse_mlb(root: &Path, path: &Path, li: usize, macros: &mut Vec<Macro>) -> Lib {
    let mut lib = Lib {
        path: rel(root, path),
        stem: stem(path),
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        notes: vec![],
        macros: vec![],
    };
    let (mut block, mut mac) = (Vec::<String>::new(), None::<usize>);
    for (i, line) in read(path).lines().enumerate() {
        let s = line.trim();
        if let Some(c) = mac {
            if re!(r"(?i)^\.ENDM\b").is_match(s) {
                mac = None;
            } else {
                macros[c].body.push(line.trim_end().to_string());
            }
            continue;
        }
        if s.starts_with(';') {
            block.push(comment(line));
            continue;
        }
        if let Some(m) = re!(r"(?i)^\.MACRO\s+([\w$]+)\s*(.*)").captures(s) {
            mac = Some(macros.len());
            lib.macros.push(macros.len());
            macros.push(Macro {
                name: m[1].to_string(),
                params: m[2].to_string(),
                line: i + 1,
                lib: li,
                comment: block.clone(),
                ..Default::default()
            });
        } else if !block.is_empty() {
            lib.notes.push(blocks(&block));
        }
        block.clear();
    }
    for (n, &mi) in lib.macros.iter().enumerate() {
        // "SETIPL, DSBINT, ENBINT, SOFTINT: ..." documents all four
        let doc = blocks(&macros[mi].comment);
        let (name, args, rest) = head(&doc);
        let names: Vec<String> = match name {
            Some(name) if args.starts_with(',') => re!(r"[\w$]+")
                .find_iter(&format!("{name} {args}"))
                .map(|m| m.as_str().to_string())
                .collect(),
            Some(name) => vec![name],
            None => vec![],
        };
        let mac = &mut macros[mi];
        mac.doc = if names.contains(&mac.name) { rest } else { doc };
        if names.len() > 1 {
            mac.shares = names;
        }
        if mac.comment.is_empty() {
            let name = mac.name.clone();
            if let Some(&owner) = lib.macros[..n]
                .iter()
                .find(|&&o| macros[o].shares.contains(&name))
            {
                macros[mi].doc = macros[owner].doc.clone();
                macros[mi].shared = Some(owner);
            }
        }
        let mac = &mut macros[mi];
        if mac.name.ends_with("DEF") {
            for b in &mac.body {
                if let Some(d) = re!(r"^\s*([\w$]+)\s*=\s*([^;]*?)\s*(?:;\s*(.*))?$").captures(b) {
                    let note = d.get(3).map_or("", |m| m.as_str());
                    mac.defs
                        .push((d[1].to_string(), d[2].to_string(), note.to_string()));
                }
            }
        }
    }
    lib
}

type Table = Vec<Vec<String>>;

/// The tables under a heading, up to the next one: [(caption, rows)], the
/// first row the header.
fn md_tables(text: &str, heading: &str) -> Vec<(String, Table)> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| *l == heading).unwrap_or_else(|| {
        eprintln!("apidoc: no {heading:?} in {PAL_DOC}");
        std::process::exit(1)
    });
    let (mut tables, mut cur, mut caption) = (Vec::<(String, Table)>::new(), false, String::new());
    for line in &lines[start + 1..] {
        if line.starts_with('#') {
            break;
        }
        if line.starts_with('|') {
            let cells: Vec<String> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            if !cur {
                cur = true;
                tables.push((caption.clone(), vec![]));
            }
            if !cells.concat().chars().all(|c| "-: ".contains(c)) {
                tables.last_mut().unwrap().1.push(cells);
            }
        } else {
            cur = false;
            if !line.trim().is_empty() {
                caption = line.trim().to_string();
            }
        }
    }
    tables
}

// ---------------------------------------------------------------- analysis

struct Api {
    mods: Vec<Module>,
    routines: Vec<Routine>,
    data: Vec<Data>,
    libs: Vec<Lib>,
    macros: Vec<Macro>,
    services: Vec<Service>,
    paldoc: String,
    pals: Vec<Pal>,
    ssdef: usize,
    paldef: HashSet<String>,
    called_by: HashMap<usize, Vec<usize>>,
    syms: HashMap<String, String>,
}

type PalMap = (&'static LazyLock<Regex>, fn(&Captures) -> Vec<String>);

static PALMAP: LazyLock<Vec<PalMap>> = LazyLock::new(|| {
    macro_rules! pat {
        ($name:ident, $s:expr) => {
            static $name: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(concat!("(?i)^", $s)).unwrap());
        };
    }
    pat!(CALL_PAL, r"CALL_PAL\s+#PAL\$_(\w+)");
    pat!(MTPR, r"MTPR\s+.*#PR\$_(\w+)");
    pat!(MFPR, r"MFPR\s+#PR\$_(\w+)");
    pat!(DSBINT, r"DSBINT\b");
    pat!(SETIPL, r"(SETIPL|ENBINT)\b");
    pat!(SOFTINT, r"SOFTINT\b");
    pat!(PRIV, r"(REI|HALT|CHMK|CHME|CHMS|CHMU|PROBER|PROBEW)\b");
    pat!(IFNORD, r"IFNORD\b");
    pat!(IFNOWRT, r"IFNOWRT\b");
    vec![
        (&CALL_PAL, |m| vec![m[1].to_string()]),
        (&MTPR, |m| vec![format!("MTPR_{}", &m[1])]),
        (&MFPR, |m| vec![format!("MFPR_{}", &m[1])]),
        (&DSBINT, |_| vec!["MFPR_IPL".into(), "MTPR_IPL".into()]),
        (&SETIPL, |_| vec!["MTPR_IPL".into()]),
        (&SOFTINT, |_| vec!["MTPR_SIRR".into()]),
        (&PRIV, |m| vec![m[1].to_uppercase()]),
        (&IFNORD, |_| vec!["PROBER".into()]),
        (&IFNOWRT, |_| vec!["PROBEW".into()]),
    ]
});

const PAL_REFS: [(&str, &str); 8] = [
    ("Calling the PAL", "## Calling the PAL"),
    (
        "VAX instructions that are PAL calls",
        "## How MACRO-32 calls it",
    ),
    ("What the PAL sets up", "## What the PAL sets up"),
    (
        "The restart parameter block",
        "### The restart parameter block",
    ),
    ("Page table entries", "## Memory"),
    ("The interrupt frame", "## Interrupts and exceptions"),
    ("Faults and HALT", "### Faults and HALT"),
    ("The Alpha OpenVMS calls", "### The Alpha calls"),
];

fn push_new<T: PartialEq>(v: &mut Vec<T>, items: impl IntoIterator<Item = T>) {
    for i in items {
        if !v.contains(&i) {
            v.push(i);
        }
    }
}

fn closure(rs: &[Routine], r: usize, seen: &mut HashSet<usize>) -> BTreeSet<String> {
    if !seen.insert(r) {
        return BTreeSet::new();
    }
    let mut out = rs[r].status.clone();
    for &(callee, passes) in &rs[r].edges {
        if passes {
            out.extend(closure(rs, callee, seen));
        }
    }
    out
}

/// The global routines r calls, through its local ones.
fn calls(rs: &[Routine], r: usize, seen: &mut HashSet<usize>) -> Vec<usize> {
    let mut out = vec![];
    for &(callee, _) in &rs[r].edges {
        if seen.insert(callee) {
            if rs[callee].global {
                out.push(callee);
            } else {
                out.extend(calls(rs, callee, seen));
            }
        }
    }
    out
}

impl Api {
    fn load(root: &Path) -> Api {
        let (mut routines, mut data, mut macros) = (vec![], vec![], vec![]);
        let mut exec = glob(&root.join("roottask/exec"), "mar");
        exec.push(root.join("vtools/lib/consolio.mar"));
        let mut mods: Vec<Module> = exec
            .iter()
            .enumerate()
            .map(|(i, p)| parse_mar(root, p, i, &mut routines, &mut data))
            .collect();
        for p in glob(&root.join("roottask/exec"), "b64") {
            let m = parse_b64(root, &p, mods.len(), &mut routines);
            mods.push(m);
        }
        // The images' libraries, and DCL's: their global routines and data
        // only, since their local names may be the executive's too.
        let mut libs = glob(&root.join("roottask/sysexe/lib"), "mar");
        libs.push(root.join("roottask/sysexe/dcl.mar"));
        libs.extend(glob(&root.join("roottask/sysexe/dcl"), "mar"));
        libs.extend(glob(&root.join("roottask/sysexe/rms"), "mar"));
        for p in libs {
            let (mut rs, mut ds) = (vec![], vec![]);
            let mut m = parse_mar(root, &p, mods.len(), &mut rs, &mut ds);
            m.image = true;
            m.routines.clear();
            m.data.clear();
            for r in rs.into_iter().filter(|r| r.global) {
                m.routines.push(routines.len());
                routines.push(r);
            }
            for d in ds {
                m.data.push(data.len());
                data.push(d);
            }
            mods.push(m);
        }
        let mut lib_paths = glob(&root.join("vtools/lib"), "mlb");
        lib_paths.extend(glob(&root.join("roottask/sysexe"), "mlb"));
        let libs: Vec<Lib> = lib_paths
            .iter()
            .enumerate()
            .map(|(i, p)| parse_mlb(root, p, i, &mut macros))
            .collect();
        let mut services = parse_sstab(&root.join("roottask/exec/syssrv.mar"));
        let paldoc = read(&root.join(PAL_DOC));

        let mut by_name = HashMap::new();
        for (i, r) in routines.iter().enumerate() {
            assert!(
                by_name.insert(r.name.clone(), i).is_none(),
                "apidoc: two routines named {}",
                r.name
            );
        }
        let mut data_names = HashSet::new();
        for d in &data {
            assert!(
                data_names.insert(d.name.clone()),
                "apidoc: two data cells named {}",
                d.name
            );
        }
        let owner: HashMap<String, usize> = routines
            .iter()
            .enumerate()
            .flat_map(|(i, r)| r.labels.iter().map(move |l| (l.clone(), i)))
            .collect();
        let starlet: HashMap<String, usize> = macros
            .iter()
            .enumerate()
            .filter(|(_, m)| libs[m.lib].stem == "starlet")
            .map(|(i, m)| (m.name.clone(), i))
            .collect();
        let ssdef = macros
            .iter()
            .position(|m| m.name == "$SSDEF")
            .expect("apidoc: no $SSDEF");
        let paldef: HashSet<String> = macros
            .iter()
            .filter(|m| m.name == "$PALDEF")
            .flat_map(|m| m.defs.iter().map(|d| d.0.clone()))
            .collect();

        for svc in &mut services {
            svc.routine = by_name.get(&format!("EXE${}", svc.name)).copied();
            svc.mac = starlet.get(&format!("${}_S", svc.name)).copied();
        }

        let rs_global: Vec<bool> = routines.iter().map(|r| r.global).collect();
        let rs_module: Vec<usize> = routines.iter().map(|r| r.module).collect();
        // Who reaches each routine, and how: a local only from its module.
        let mut reach = vec![];
        for (from, r) in routines.iter().enumerate() {
            for insn in &r.body {
                let (op, rest) = insn.split_once(char::is_whitespace).unwrap_or((insn, ""));
                let ops: Vec<&str> = rest.split(',').map(str::trim).collect();
                let target = |o: &str| {
                    let name = re!(r"^[@#]|G\^").replace_all(o, "");
                    by_name
                        .get(&*name)
                        .copied()
                        .filter(|&t| t != from && (rs_global[t] || rs_module[t] == r.module))
                };
                let op = op.to_uppercase();
                if ["BSBB", "BSBW", "JSB"].contains(&op.as_str()) {
                    reach.extend(target(ops[0]).map(|t| (t, 'c', from, None)));
                } else if re!(BRANCH).is_match(&op) {
                    reach.extend(target(ops[ops.len() - 1]).map(|t| (t, 'b', from, None)));
                } else if re!(r"^(MOVA|PUSHA)[BWLQ]$").is_match(&op) {
                    let slot = ops.get(1).and_then(|o| o.strip_prefix("EXE$AL_SCB+"));
                    reach.extend(target(ops[0]).map(|t| (t, 'a', from, slot.map(String::from))));
                }
            }
        }
        for (t, how, from, slot) in reach {
            let r = &mut routines[t];
            match (how, slot) {
                ('c', _) => r.called = true,
                ('b', _) => push_new(&mut r.branched, [from]),
                (_, Some(slot)) => r.scb = Some(slot),
                _ => push_new(&mut r.addressed, [from]),
            }
        }
        let mut onward: Vec<Vec<usize>> = routines.iter().map(|r| r.branched.clone()).collect();
        for (ri, r) in routines.iter_mut().enumerate() {
            r.falls_from = r
                .falls_from
                .take()
                .filter(|f| by_name.get(f).is_some_and(|&f| rs_module[f] == r.module));
            r.comes_back = r.body.iter().any(|b| b == "RSB" || b == "RET");
            if let Some(f) = &r.falls_from {
                onward[by_name[f]].push(ri);
            }
        }
        // onward[t] holds who branches to t: it comes back if t does
        loop {
            let back: Vec<usize> = (0..routines.len())
                .filter(|&t| routines[t].comes_back)
                .flat_map(|t| onward[t].clone())
                .filter(|&f| !routines[f].comes_back)
                .collect();
            if back.is_empty() {
                break;
            }
            for f in back {
                routines[f].comes_back = true;
            }
        }
        for (ri, r) in routines.iter_mut().enumerate() {
            let (_, args, doc) = head(&blocks(&r.comment));
            let (doc, regs, ipl) = facts(doc);
            (r.args, r.doc, r.regs, r.ipl) = (args, doc, regs, ipl);
            for (i, insn) in r.body.iter().enumerate() {
                r.status.extend(
                    re!(r"SS\$_\w+")
                        .find_iter(insn)
                        .map(|m| m.as_str().to_string()),
                );
                r.uses.extend(
                    re!(SYMBOL)
                        .find_iter(insn)
                        .map(|m| m.as_str())
                        .filter(|t| data_names.contains(*t))
                        .map(String::from),
                );
                for (pat, f) in PALMAP.iter() {
                    if let Some(m) = pat.captures(insn) {
                        push_new(&mut r.pal, f(&m));
                    }
                }
                if let Some(m) = re!(r"^(\$\w+)_S\b").captures(insn) {
                    push_new(&mut r.svcs, [m[1].to_string()]);
                }
                let Some(m) = re!(CALL).captures(insn) else {
                    continue;
                };
                let target =
                    re!(r"^[@#]|G\^").replace_all(m[2].split(',').next_back().unwrap().trim(), "");
                let callee = by_name.get(&*target).or(owner.get(&*target)).copied();
                if let Some(svc) = target.strip_prefix("SYS$") {
                    push_new(&mut r.svcs, [format!("${svc}")]);
                }
                // a local routine, or a label in one, is only its own module's
                let mi = r.module;
                let Some(callee) = callee
                    .filter(|&c| c != ri)
                    .filter(|&c| rs_global[c] || rs_module[c] == mi)
                else {
                    continue;
                };
                let passes = if ["BRB", "BRW", "JMP"].contains(&m[1].to_uppercase().as_str()) {
                    true // a tail call: its status is ours
                } else {
                    // a call whose failure is ours: BLBC R0 right after it, or a return
                    let after = &r.body[i + 1..(i + 3).min(r.body.len())];
                    after.iter().any(|a| re!(r"(?i)^BLBC\s+R0\b").is_match(a))
                        || after.first().is_some_and(|a| a == "RET" || a == "RSB")
                };
                r.edges.push((callee, passes));
            }
        }

        for ri in 0..routines.len() {
            let returns = closure(&routines, ri, &mut HashSet::new());
            let calls = calls(&routines, ri, &mut HashSet::from([ri]));
            (routines[ri].returns, routines[ri].calls) = (returns, calls);
        }
        let mut called_by: HashMap<usize, Vec<usize>> = HashMap::new();
        for (ri, r) in routines.iter().enumerate() {
            for &c in &r.calls {
                called_by.entry(c).or_default().push(ri);
            }
        }
        for ri in 0..routines.len() {
            // locals pass their PAL calls up
            if routines[ri].global {
                continue;
            }
            let pal = routines[ri].pal.clone();
            for caller in &mut routines {
                if caller.edges.iter().any(|&(c, _)| c == ri) {
                    push_new(&mut caller.pal, pal.iter().cloned());
                }
            }
        }

        // PAL calls from DESIGN-0001
        let imp = md_tables(&paldoc, "### Implemented").swap_remove(0).1;
        let vaxmap = md_tables(&paldoc, "## How MACRO-32 calls it")
            .swap_remove(0)
            .1;
        let pals = imp[1..]
            .iter()
            .map(|row| {
                let cols: HashMap<String, String> =
                    imp[0].iter().cloned().zip(row.iter().cloned()).collect();
                let name = cols["Call"].trim_matches('`').to_string();
                let code = u32::from_str_radix(cols["Code"].trim_start_matches("0x"), 16).unwrap();
                let vax = vaxmap[1..]
                    .iter()
                    .filter(|v| re!(r"`([^`]+)`").captures_iter(&v[1]).any(|c| c[1] == name))
                    .map(|v| v[0].clone())
                    .collect();
                let users = (0..routines.len())
                    .filter(|&r| routines[r].global && routines[r].pal.contains(&name))
                    .collect();
                Pal {
                    name,
                    code,
                    cols,
                    vax,
                    users,
                }
            })
            .collect();

        let mut api = Api {
            mods,
            routines,
            data,
            libs,
            macros,
            services,
            paldoc,
            pals,
            ssdef,
            paldef,
            called_by,
            syms: HashMap::new(),
        };
        api.anchors();
        api
    }

    fn anchors(&mut self) {
        let mut syms = HashMap::new();
        let mut sym = |name: String, anchor: String| {
            syms.entry(name).or_insert(anchor);
        };
        for svc in &self.services {
            for n in ["$", "SYS$", "EXE$"] {
                sym(format!("{n}{}", svc.name), format!("svc-{}", svc.name));
            }
        }
        for r in &self.routines {
            sym(r.name.clone(), format!("r-{}", r.name));
            // a service that runs in its caller's mode, $UNWIND: SYS$name's own code
            if let Some(svc) = r.name.strip_prefix("SYS$") {
                sym(format!("${svc}"), format!("r-{}", r.name));
            }
        }
        for d in &self.data {
            sym(d.name.clone(), format!("d-{}", d.name));
        }
        for p in &self.pals {
            sym(p.name.clone(), format!("pal-{}", p.name));
            sym(format!("PAL$_{}", p.name), format!("pal-{}", p.name));
        }
        for (name, _, _) in &self.macros[self.ssdef].defs {
            sym(name.clone(), format!("ss-{name}"));
        }
        for mac in &self.macros {
            sym(
                mac.name.clone(),
                format!("m-{}-{}", self.libs[mac.lib].stem, mac.name),
            );
        }
        for mac in &self.macros {
            for (name, _, _) in &mac.defs {
                sym(name.clone(), format!("s-{name}"));
            }
        }
        self.syms = syms;
    }
}

// ---------------------------------------------------------------- HTML

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn esc(s: &str) -> String {
    esc_text(s).replace('"', "&quot;").replace('\'', "&#x27;")
}

fn src(path: &str, line: usize) -> String {
    format!(
        r#"<a class="src" href="../../{path}" title="open the source">{}:{line}</a>"#,
        esc(path)
    )
}

fn summary(doc: &Doc) -> String {
    let text = doc.iter().find(|(k, _)| *k == K::P).map_or("", |(_, t)| t);
    let mut prev = ' ';
    let end = text
        .char_indices()
        .find(|&(_, c)| {
            let hit = c.is_whitespace() && prev == '.';
            prev = c;
            hit
        })
        .map_or(text.len(), |(i, _)| i);
    let s = &text[..end];
    if s.chars().count() < 110 {
        s.to_string()
    } else {
        s.chars().take(107).chain(['…']).collect()
    }
}

fn hex(v: u32) -> String {
    format!("{v:02X}")
}

struct Page<'a> {
    api: &'a Api,
    index: Vec<[String; 4]>,
    toc: Vec<(u8, String, String)>, // (level, id, title)
    toc_mark: usize,                // where the next chapter's entry goes: before its sections
}

impl Page<'_> {
    fn link(&self, text: &str) -> String {
        re!(SYMBOL)
            .replace_all(&esc_text(text), |m: &Captures| {
                match self.api.syms.get(&m[0]) {
                    Some(a) => format!(r##"<a href="#{a}">{}</a>"##, &m[0]),
                    None => m[0].to_string(),
                }
            })
            .into_owned()
    }

    /// Markdown inline: code, links, bold and italics, and symbol links.
    fn md(&self, text: &str) -> String {
        let (mut out, mut pos) = (String::new(), 0);
        for m in re!(r"`([^`]+)`|\[([^\]]+)\]\(([^)]+)\)|\*\*([^*]+)\*\*|\*([^*]+)\*")
            .captures_iter(text)
        {
            let all = m.get(0).unwrap();
            out += &self.link(&text[pos..all.start()]);
            if let Some(c) = m.get(1) {
                out += &format!("<code>{}</code>", self.link(c.as_str()));
            } else if let Some(t) = m.get(2) {
                let u = &m[3];
                let url = if u.contains("://") || u.starts_with('#') {
                    u.to_string()
                } else {
                    format!("../design/{u}")
                };
                out += &format!(r#"<a href="{}">{}</a>"#, esc(&url), esc(t.as_str()));
            } else if let Some(b) = m.get(4) {
                out += &format!("<b>{}</b>", self.link(b.as_str()));
            } else {
                out += &format!("<i>{}</i>", self.link(&m[5]));
            }
            pos = all.end();
        }
        out + &self.link(&text[pos..])
    }

    fn prose(&self, doc: &Doc) -> String {
        doc.iter()
            .map(|(k, t)| match k {
                K::Pre => format!("<pre>{}</pre>", self.link(t)),
                K::Note => format!(r#"<p class="note"><b>Limit.</b> {}</p>"#, self.link(t)),
                K::P => format!("<p>{}</p>", self.link(t)),
            })
            .collect()
    }

    fn table(
        &self,
        rows: &Table,
        cell: fn(&Self, &str) -> String,
        ids: Option<&[Option<String>]>,
    ) -> String {
        let th: String = rows[0]
            .iter()
            .map(|c| format!("<th>{}</th>", esc(c)))
            .collect();
        let mut body = String::new();
        for (i, row) in rows[1..].iter().enumerate() {
            let rid = ids
                .and_then(|ids| ids[i].as_ref())
                .map(|id| format!(r#" id="{id}""#))
                .unwrap_or_default();
            body += &format!("<tr{rid}>");
            for c in row {
                body += &format!("<td>{}</td>", cell(self, c));
            }
            body += "</tr>";
        }
        format!(
            r#"<div class="tw"><table><thead><tr>{th}</tr></thead><tbody>{body}</tbody></table></div>"#
        )
    }

    fn refs<S: AsRef<str>>(&self, names: impl IntoIterator<Item = S>) -> String {
        names
            .into_iter()
            .map(|n| {
                let n = n.as_ref();
                let a = self
                    .api
                    .syms
                    .get(n)
                    .unwrap_or_else(|| panic!("apidoc: nothing documents {n}"));
                format!(r##"<a class="ref" href="#{a}">{}</a>"##, esc(n))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn routine_refs(&self, rs: &[usize]) -> String {
        self.refs(rs.iter().map(|&r| &self.api.routines[r].name))
    }

    #[allow(clippy::too_many_arguments)]
    fn entry(
        &mut self,
        anchor: &str,
        name: &str,
        tag: &str,
        fields: &[(&str, String)],
        source: &str,
        summ: String,
        kind: &str,
    ) -> String {
        self.index
            .push([name.to_string(), kind.to_string(), summ, anchor.to_string()]);
        let rows: String = fields
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| format!("<dt>{k}</dt><dd>{v}</dd>"))
            .collect();
        let class = tag.to_lowercase();
        let class = class.split_whitespace().next().unwrap();
        format!(
            r##"<article class="entry" id="{anchor}"><header><h4><a href="#{anchor}">{}</a></h4><span class="tag t-{class}">{tag}</span>{source}</header><dl>{rows}</dl></article>"##,
            esc(name)
        )
    }

    fn statuses<'s>(&self, codes: impl IntoIterator<Item = &'s String>) -> String {
        let defs = &self.api.macros[self.api.ssdef].defs;
        let mut codes: Vec<&String> = codes.into_iter().collect();
        codes.sort_by_key(|c| (defs.iter().position(|d| &d.0 == *c).unwrap_or(999), *c));
        self.refs(codes)
    }

    fn chapter(&mut self, num: &str, anchor: &str, title: &str, intro: &str, body: &str) -> String {
        self.toc.insert(
            self.toc_mark,
            (1, anchor.to_string(), format!("{num} {title}")),
        );
        self.toc_mark = self.toc.len();
        let num = if num.is_empty() {
            String::new()
        } else {
            format!(r#"<span class="num">{num}</span>"#)
        };
        format!(
            r#"<section class="chapter" id="{anchor}"><h2>{num}{}</h2>{intro}{body}</section>"#,
            esc(title)
        )
    }

    fn section(
        &mut self,
        num: &str,
        anchor: &str,
        title: &str,
        intro: &str,
        body: &str,
        toc_title: Option<&str>,
    ) -> String {
        self.toc.push((
            2,
            anchor.to_string(),
            format!("{num} {}", toc_title.unwrap_or(title)),
        ));
        format!(
            r#"<section id="{anchor}"><h3><span class="num">{num}</span>{title}</h3>{intro}{body}</section>"#
        )
    }

    fn routine_entry(&mut self, ri: usize) -> String {
        let api = self.api;
        let r = &api.routines[ri];
        // How the code reaches it decides its linkage: a call, the SCB, or
        // else branches, its address and the code before it. A global that
        // comes back with RSB is a JSB routine, however it is reached.
        let reached = !r.branched.is_empty() || !r.addressed.is_empty() || r.falls_from.is_some();
        let mut tag = if r.entry {
            "CALL"
        } else if r.scb.is_some() {
            "Handler"
        } else if r.called || (r.global && r.comes_back) || !reached {
            "JSB"
        } else {
            "Label"
        }
        .to_string();
        let n = esc(&r.name);
        let linkage = match tag.as_str() {
            "CALL" => format!(
                "<code>CALLS #n, G^{n}</code> or <code>CALLG</code>; arguments in the AP list"
            ),
            "JSB" if r.global => format!(
                "<code>JSB G^{n}</code>; arguments in registers, returns with <code>RSB</code>"
            ),
            "JSB" => format!(
                "<code>BSBW {n}</code>, from {} only",
                esc(&api.mods[r.module].path)
            ),
            "Handler" => format!(
                "reached through the SCB, <code>{}</code>; returns with <code>REI</code>",
                self.link(r.scb.as_deref().unwrap())
            ),
            _ => {
                let mut how = vec![];
                let list = |rs: &[usize]| {
                    let names: Vec<String> = rs.iter().map(|&r| self.routine_refs(&[r])).collect();
                    names.join(", ")
                };
                if !r.branched.is_empty() {
                    how.push(format!("branched to from {}", list(&r.branched)));
                }
                if let Some(f) = &r.falls_from {
                    how.push(format!("run on into from {}", self.refs([f])));
                }
                if !r.addressed.is_empty() {
                    how.push(format!("its address is used by {}", list(&r.addressed)));
                }
                let ends = if r.comes_back {
                    ""
                } else {
                    " and doesn't return"
                };
                format!("not called{ends}: {}", how.join("; "))
            }
        };
        if !r.global {
            tag += " local";
        }
        let args = if !r.args.is_empty() && r.entry {
            format!("<code>{}</code>", self.link(&r.args))
        } else {
            String::new()
        };
        let desc = self.prose(&r.doc);
        let fields = [
            ("Linkage", linkage),
            ("Arguments", args),
            (
                "Description",
                if desc.is_empty() {
                    "<p><i>Undocumented.</i></p>".into()
                } else {
                    desc
                },
            ),
            ("IPL", self.link(&r.ipl)),
            ("Registers", self.link(&r.regs)),
            ("Status", self.statuses(&r.returns)),
            ("Calls", self.routine_refs(&r.calls)),
            (
                "Called by",
                self.routine_refs(api.called_by.get(&ri).map_or(&[], |v| v)),
            ),
            ("Services", self.refs(&r.svcs)),
            (
                "PAL calls",
                self.refs(r.pal.iter().filter(|p| api.syms.contains_key(*p))),
            ),
            ("Data", self.refs(&r.uses)),
        ];
        let kind = if r.global { "routine" } else { "local routine" };
        self.entry(
            &format!("r-{}", r.name),
            &r.name,
            &tag,
            &fields,
            &src(&api.mods[r.module].path, r.line),
            summary(&r.doc),
            kind,
        )
    }

    fn service_codes(&self, svc: &Service) -> BTreeSet<String> {
        if svc.stub {
            return BTreeSet::from(["SS$_ILLSER".to_string()]);
        }
        let mut codes = self.api.routines[svc.routine.unwrap()].returns.clone();
        if svc.nargs > 0 {
            codes.insert("SS$_INSFARG".into());
        }
        codes
    }

    fn service_entry(&mut self, si: usize) -> String {
        let api = self.api;
        let svc = &api.services[si];
        let (anchor, name) = (format!("svc-{}", svc.name), format!("${}", svc.name));
        if svc.stub {
            let doc = vec![(K::P, "Not implemented yet: returns SS$_ILLSER.".to_string())];
            let fields = [
                (
                    "Format",
                    format!("<code>SYS${}</code>, {} arguments", svc.name, svc.nargs),
                ),
                ("Description", self.prose(&doc)),
                ("Status", self.statuses(&self.service_codes(svc))),
            ];
            return self.entry(
                &anchor,
                &name,
                "Stub",
                &fields,
                "",
                summary(&doc),
                "system service, stub",
            );
        }
        let r = &api.routines[svc
            .routine
            .unwrap_or_else(|| panic!("apidoc: no EXE${}", svc.name))];
        let mac = svc.mac.map(|m| &api.macros[m]);
        let (mode, dispatcher) = if svc.chme {
            ("CHME", "EXE$CMODEXEC")
        } else {
            ("CHMK", "EXE$CMODKRNL")
        };
        let fields = [
            (
                "Format",
                format!(
                    "<code><b>SYS${}</b> {}</code>",
                    svc.name,
                    self.link(&r.args)
                ),
            ),
            (
                "Macro",
                mac.map_or(String::new(), |m| {
                    format!(
                        "<code>{} {}</code>",
                        self.link(&m.name),
                        self.link(&m.params)
                    )
                }),
            ),
            (
                "Dispatch",
                format!(
                    "<code>{mode} #{}</code> to {}, which calls {} with at least {} arguments",
                    svc.code,
                    self.link(dispatcher),
                    esc(&r.name),
                    svc.nargs
                ),
            ),
            ("Description", self.prose(&r.doc)),
            ("IPL", self.link(&r.ipl)),
            ("Status", self.statuses(&self.service_codes(svc))),
            ("Calls", self.routine_refs(&r.calls)),
            (
                "PAL calls",
                self.refs(r.pal.iter().filter(|p| api.syms.contains_key(*p))),
            ),
            ("Data", self.refs(&r.uses)),
        ];
        let source = src(&api.mods[r.module].path, r.line);
        self.entry(
            &anchor,
            &name,
            "Service",
            &fields,
            &source,
            summary(&r.doc),
            "system service",
        )
    }

    fn macro_entry(&mut self, mi: usize) -> String {
        let api = self.api;
        let mac = &api.macros[mi];
        let lib = &api.libs[mac.lib];
        let anchor = format!("m-{}-{}", lib.stem, mac.name);
        let mut doc = mac.doc.clone();
        if doc.is_empty()
            && let Some(svc) = re!(r"^\$(\w+)_S$").captures(&mac.name)
        {
            doc = vec![(
                K::P,
                format!(
                    "Pushes its arguments and calls SYS${0}, the ${0} service.",
                    &svc[1]
                ),
            )];
        }
        if let Some(owner) = mac.shared {
            doc.push((K::P, format!("Documented with {}.", api.macros[owner].name)));
        }
        let desc = self.prose(&doc);
        let mut fields = vec![
            (
                "Format",
                format!(
                    "<code><b>{}</b> {}</code>",
                    esc(&mac.name),
                    self.link(&mac.params)
                ),
            ),
            (
                "Description",
                if desc.is_empty() {
                    "<p><i>Undocumented.</i></p>".into()
                } else {
                    desc
                },
            ),
        ];
        if !mac.defs.is_empty() {
            let mut rows: Table = vec![vec!["Symbol".into(), "Value".into(), "Meaning".into()]];
            rows.extend(
                mac.defs
                    .iter()
                    .map(|d| vec![d.0.clone(), d.1.clone(), d.2.clone()]),
            );
            let ids: Vec<Option<String>> = mac
                .defs
                .iter()
                .map(|d| Some(format!("s-{}", d.0)).filter(|id| api.syms.get(&d.0) == Some(id)))
                .collect();
            fields.push(("Symbols", self.table(&rows, Self::link, Some(&ids))));
            if mi != api.ssdef {
                // chapter 6 indexes those
                for (name, value, note) in &mac.defs {
                    let note = if note.is_empty() {
                        String::new()
                    } else {
                        format!(" — {note}")
                    };
                    self.index.push([
                        name.clone(),
                        format!("{} symbol", mac.name),
                        format!("= {value}{note}"),
                        api.syms[name].clone(),
                    ]);
                }
            }
        } else {
            let body = self.link(&mac.body.join("\n"));
            fields.push((
                "Expansion",
                format!("<details><summary>show</summary><pre>{body}</pre></details>"),
            ));
        }
        let tag = if mac.defs.is_empty() {
            "Macro"
        } else {
            "Definitions"
        };
        self.entry(
            &anchor,
            &mac.name,
            tag,
            &fields,
            &src(&lib.path, mac.line),
            summary(&doc),
            &format!("macro, {}", lib.name),
        )
    }

    fn pal_entry(&mut self, pi: usize) -> String {
        let p = &self.api.pals[pi];
        let col = |k: &str| p.cols.get(k).map_or("", |v| v.as_str());
        let priv_ = if p.code < 0x80 {
            "privileged, kernel mode only"
        } else {
            "any mode"
        };
        let what = col("What the PAL does");
        let pal_sym = format!("PAL$_{}", p.name);
        let or_dash = |s: String| if s.is_empty() { "—".to_string() } else { s };
        let fields = [
            (
                "Format",
                if self.api.paldef.contains(&pal_sym) {
                    format!("<code>CALL_PAL #{}</code>", self.link(&pal_sym))
                } else {
                    format!("<code>CALL_PAL #^X{}</code>", hex(p.code))
                },
            ),
            (
                "VAX",
                p.vax
                    .iter()
                    .map(|v| format!("<code>{}</code>", self.md(v)))
                    .collect::<Vec<_>>()
                    .join("<br>"),
            ),
            (
                "Code",
                format!(
                    "<code>^X{}</code>, {priv_}; {}",
                    hex(p.code),
                    self.md(col("Origin"))
                ),
            ),
            ("In", or_dash(self.md(col("In")))),
            ("Out", or_dash(self.md(col("Out")))),
            (
                "Description",
                if what.is_empty() {
                    String::new()
                } else {
                    format!("<p>{}</p>", self.md(&capitalize(what)))
                },
            ),
            ("Used by", self.routine_refs(&p.users)),
        ];
        let summ = if what.is_empty() {
            format!("PAL call ^X{}", hex(p.code))
        } else {
            what.replace('`', "")
        };
        let source = r#"<a class="src" href="../design/0001-pal-interface.md">DESIGN-0001</a>"#;
        self.entry(
            &format!("pal-{}", p.name),
            &p.name,
            "PAL",
            &fields,
            source,
            summ,
            "PAL call",
        )
    }

    // ------------------------------------------------------------ chapters

    fn ch_overview(&mut self) -> String {
        let rows: Table = [
            ["Tag", "How it is called"],
            [
                "Service",
                "`SYS$name` with `CALLS` or `CALLG`, or the `$name_S` macro; `CHMK`, or for `$CMEXEC` `CHME`, takes it to the executive",
            ],
            ["CALL", "`CALLS` or `CALLG`: arguments in the argument list at AP, status in R0, `RET`"],
            ["JSB", "`JSB` or `BSBW`: arguments and results in registers, `RSB`"],
            ["Handler", "an SCB vector: the PAL delivers an interrupt, an exception or `CHMx` to it; it ends with `REI`"],
            ["Label", "not called: code branches or runs on to it, or uses its address, to `REI` or jump there; its linkage says which"],
            ["PAL", "a privileged VAX instruction, or `CALL_PAL`; vmacro compiles it into `svc #0`"],
            ["Macro", "a macro from a `.LIBRARY`; Definitions macros define symbols"],
        ]
        .iter()
        .map(|r| r.iter().map(|c| c.to_string()).collect())
        .collect();
        let intro = concat!(
            "<p>The interfaces of vaxpunk's executive and the PAL below it, and of the libraries its images ",
            "link, as code written against them sees them. Every entry is read from the sources by <code>cargo run -p apidoc</code>: the comment above ",
            "each routine and macro, the system service vector, and the tables of ",
            r#"<a href="../design/0001-pal-interface.md">DESIGN-0001</a>. How the pieces fit is in "#,
            r#"<a href="../design/0001-pal-interface.md">DESIGN-0001</a> (the PAL) and "#,
            r#"<a href="../design/0002-executive-processes.md">DESIGN-0002</a> (the executive).</p>"#,
            "<p>Press <kbd>/</kbd> to search. Every name in the text links to its entry. ",
            "<i>Status</i> lists the condition values found in a routine's code and in what it calls ",
            "whose failure it passes on. A <i>Limit</i> is a deliberate shortcut, with the way past it.</p>"
        );
        let t = self.table(&rows, Self::md, None);
        self.chapter("", "ch-overview", "Overview", intro, &t)
    }

    fn ch_services(&mut self) -> String {
        let services = &self.api.services;
        let mut groups: Vec<&String> = vec![];
        for svc in services {
            if !groups.contains(&&svc.group) {
                groups.push(&svc.group);
            }
        }
        let mut body = String::new();
        for (i, g) in groups.into_iter().enumerate() {
            let items: Vec<usize> = (0..services.len())
                .filter(|&s| &services[s].group == g)
                .collect();
            let done = items.iter().filter(|&&s| !services[s].stub).count();
            let intro = format!(
                r#"<p class="meta">{done} of {} implemented.</p>"#,
                items.len()
            );
            let entries: String = items.into_iter().map(|s| self.service_entry(s)).collect();
            body += &self.section(
                &format!("1.{}", i + 1),
                &format!("svc-group-{}", i + 1),
                &esc(g),
                &intro,
                &entries,
                Some(g),
            );
        }
        let intro = self.link_intro(concat!(
            "<p>Called from any mode with <code>CALLS</code> or <code>CALLG</code> to <code>SYS$name</code>, ",
            "which is in <code>syssrv.mar</code>, or with the <code>$name_S</code> macros of ",
            r##"<a href="#lib-starlet">starlet.mlb</a>. Programs run in user mode, and <code>SYS$name</code> is "##,
            "in the vector, a page user mode may run. A service called with fewer arguments than it takes ",
            "returns SS$_INSFARG, one with an address its caller's mode can't reach SS$_ACCVIO, and one with ",
            "an argument that isn't a sign-extended longword SS$_ARG_GTR_32_BITS.</p>"
        ));
        self.chapter("1", "ch-services", "System services", &intro, &body)
    }

    fn link_intro(&self, text: &str) -> String {
        re!(r"SS\$_\w+")
            .replace_all(text, |m: &Captures| match self.api.syms.get(&m[0]) {
                Some(a) => format!(r##"<a href="#{a}">{}</a>"##, &m[0]),
                None => m[0].to_string(),
            })
            .into_owned()
    }

    fn ch_routines(&mut self) -> String {
        let body = self.modules("2", false);
        let intro = concat!(
            "<p>Routines and data of <code>EXEC.EXE</code>, by module. Programs link against ",
            "<code>SYS.STB</code>, so kernel-mode code reaches every global here directly. The system ",
            r##"services' own routines, <code>EXE$name</code>, are in <a href="#ch-services">chapter 1</a>; "##,
            "local routines can only be called from their own module.</p>"
        );
        self.chapter("2", "ch-routines", "Executive routines", intro, &body)
    }

    fn ch_images(&mut self) -> String {
        let body = self.modules("3", true);
        let intro = concat!(
            "<p>Routines and data of the libraries in <code>roottask/sysexe/lib</code>, which ",
            "<code>build.rs</code> links into every image on the system disk, and into DCL: ",
            "console output, and the command parser and the <code>CLI$</code> routines ",
            r#"(<a href="../adr/0017-command-tables-from-cld-with-vcdu.md">ADR-0017</a>); "#,
            "and of <code>roottask/sysexe/dcl.mar</code> and <code>dcl/</code>, DCL's own modules, ",
            "whose shared names are <code>DCL$</code>, and the CLD compiler ",
            r#"SET COMMAND uses (<a href="../adr/0018-set-command-and-foreign-commands.md">ADR-0018</a>). "#,
            "Only their global names are here.</p>"
        );
        self.chapter("3", "ch-images", "Routines for images", intro, &body)
    }

    /// The executive's modules, or the images' libraries, as sections of
    /// chapter `num`.
    fn modules(&mut self, num: &str, image: bool) -> String {
        let api = self.api;
        let mut body = String::new();
        for (i, m) in api.mods.iter().filter(|m| m.image == image).enumerate() {
            let mut rs: Vec<usize> = m
                .routines
                .iter()
                .copied()
                .filter(|&r| {
                    !(api.routines[r].entry && api.services.iter().any(|s| s.routine == Some(r)))
                })
                .collect();
            rs.sort_by_key(|&r| !api.routines[r].global);
            let intro = format!(
                r#"<p class="meta">{}</p>{}"#,
                src(&m.path, 1),
                self.prose(&m.intro)
            );
            let mut dl = String::new();
            if !m.data.is_empty() {
                let mut rows: Table = vec![vec!["Name".into(), "Storage".into(), "Holds".into()]];
                let mut ids = vec![];
                for &d in &m.data {
                    let d = &api.data[d];
                    let holds = d
                        .doc
                        .iter()
                        .map(|(_, t)| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    rows.push(vec![d.name.clone(), d.storage.clone(), holds.clone()]);
                    ids.push(Some(format!("d-{}", d.name)));
                    let summ = if holds.is_empty() {
                        d.storage.clone()
                    } else {
                        holds
                    };
                    self.index
                        .push([d.name.clone(), "data".into(), summ, format!("d-{}", d.name)]);
                }
                dl = format!("<h5>Data</h5>{}", self.table(&rows, Self::link, Some(&ids)));
            }
            let entries: String = rs.into_iter().map(|r| self.routine_entry(r)).collect();
            let title = format!(
                "{} <small>{}</small>",
                esc(&m.stem.to_uppercase()),
                esc(&m.title)
            );
            body += &self.section(
                &format!("{num}.{}", i + 1),
                &format!("mod-{}", m.stem),
                &title,
                &intro,
                &(entries + &dl),
                Some(&m.stem.to_uppercase()),
            );
        }
        body
    }

    fn ch_pal(&mut self) -> String {
        let api = self.api;
        let mut order: Vec<usize> = (0..api.pals.len()).collect();
        order.sort_by_key(|&p| api.pals[p].code);
        let entries: String = order.into_iter().map(|p| self.pal_entry(p)).collect();
        let mut body = self.section("4.1", "pal-calls", "Calls", "", &entries, None);
        for (i, (title, heading)) in PAL_REFS.iter().enumerate() {
            let mut t = String::new();
            for (cap, rows) in md_tables(&api.paldoc, heading) {
                if !cap.is_empty() && !cap.ends_with('.') || cap.ends_with(':') {
                    t += &format!(r#"<p class="cap">{}</p>"#, self.md(&cap));
                }
                t += &self.table(&rows, Self::md, None);
            }
            let anchor = format!(
                "pal-ref-{}",
                re!(r"\W+")
                    .replace_all(&title.to_lowercase(), "-")
                    .trim_matches('-')
            );
            let intro = format!(
                r#"<p class="meta">From <a href="../design/0001-pal-interface.md">DESIGN-0001</a>, {}.</p>"#,
                esc(heading.trim_start_matches(['#', ' ']))
            );
            body += &self.section(
                &format!("4.{}", i + 2),
                &anchor,
                &esc(title),
                &intro,
                &t,
                None,
            );
        }
        let intro = concat!(
            "<p>The executive calls the PAL, the root task, with <code>svc #0</code>: the function code in ",
            "x7, arguments in x0-x5, the result in x0. MACRO-32 code doesn't write that: vmacro compiles ",
            "privileged VAX instructions and <code>CALL_PAL</code> into it.</p>"
        );
        self.chapter("4", "ch-pal", "PAL calls", intro, &body)
    }

    fn ch_libs(&mut self) -> String {
        let api = self.api;
        let mut body = String::new();
        for (i, lib) in api.libs.iter().enumerate() {
            let notes: String = lib.notes.iter().map(|n| self.prose(n)).collect();
            let intro = format!(r#"<p class="meta">{}</p>{notes}"#, src(&lib.path, 1));
            let entries: String = lib.macros.iter().map(|&m| self.macro_entry(m)).collect();
            body += &self.section(
                &format!("5.{}", i + 1),
                &format!("lib-{}", lib.stem),
                &esc(&lib.name),
                &intro,
                &entries,
                None,
            );
        }
        let intro = "<p>Macro libraries, for <code>.LIBRARY</code> with <code>vasm -I</code>.</p>";
        self.chapter("5", "ch-libs", "Macros and definitions", intro, &body)
    }

    fn ch_status(&mut self) -> String {
        let api = self.api;
        let mut users: HashMap<String, Vec<String>> = HashMap::new();
        for svc in &api.services {
            for c in self.service_codes(svc) {
                users.entry(c).or_default().push(format!("${}", svc.name));
            }
        }
        let sev = |v: i64| {
            ["Warning", "Success", "Error", "Informational", "Fatal"]
                .get((v & 7) as usize)
                .copied()
                .unwrap_or("?")
        };
        let mut rows: Table = vec![
            ["Code", "Value", "Severity", "Returned by", "Meaning"]
                .map(String::from)
                .to_vec(),
        ];
        let mut ids = vec![];
        for (name, value, note) in &api.macros[api.ssdef].defs {
            let v: i64 = value
                .parse()
                .unwrap_or_else(|_| panic!("apidoc: {name} = {value} isn't decimal"));
            let by = users.get(name).map(|u| u.join(" ")).unwrap_or_default();
            rows.push(vec![
                name.clone(),
                format!("{v} (^X{v:X})"),
                sev(v).into(),
                by,
                note.clone(),
            ]);
            self.index.push([
                name.clone(),
                "condition value".into(),
                format!("{v}, {}", sev(v).to_lowercase()),
                format!("ss-{name}"),
            ]);
            ids.push(Some(format!("ss-{name}")));
        }
        let intro = concat!(
            r##"<p>From <a href="#m-starlet-$SSDEF">$SSDEF</a>, numbered as VMS numbers them. The low three bits "##,
            "are the severity: odd is success.</p>"
        );
        let t = self.table(&rows, Self::link, Some(&ids));
        self.chapter("6", "ch-status", "Condition values", intro, &t)
    }

    fn ch_index(&mut self) -> String {
        let mut entries: Vec<&[String; 4]> = self.index.iter().collect();
        entries.sort_by_key(|e| (e[0].trim_start_matches('$').to_uppercase(), e[0].clone()));
        let (mut seen, mut letters) = (HashSet::new(), Vec::<(String, String)>::new());
        for [name, kind, _, anchor] in entries {
            if !seen.insert((name, anchor)) {
                continue;
            }
            let k: String = name
                .trim_start_matches('$')
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect();
            let li = format!(
                r##"<li><a href="#{anchor}">{}</a> <span>{}</span></li>"##,
                esc(name),
                esc(kind)
            );
            match letters.iter_mut().find(|(l, _)| *l == k) {
                Some((_, v)) => *v += &li,
                None => letters.push((k, li)),
            }
        }
        let body: String = letters
            .iter()
            .map(|(k, v)| format!(r#"<h5 id="ix-{k}">{k}</h5><ul class="ix">{v}</ul>"#))
            .collect();
        let jump = letters
            .iter()
            .map(|(k, _)| format!(r##"<a href="#ix-{k}">{k}</a>"##))
            .collect::<Vec<_>>()
            .join(" ");
        self.chapter(
            "",
            "ch-index",
            "Index",
            &format!(r#"<p class="jump">{jump}</p>"#),
            &body,
        )
    }

    fn toc(&self) -> String {
        let (mut out, mut open) = (String::new(), false);
        for (level, anchor, title) in &self.toc {
            if *level == 1 {
                if open {
                    out += "</ul></details>";
                }
                out += &format!(
                    r##"<details open><summary><a href="#{anchor}">{}</a></summary><ul>"##,
                    esc(title.trim())
                );
                open = true;
            } else {
                out += &format!(r##"<li><a href="#{anchor}">{}</a></li>"##, esc(title));
            }
        }
        out + "</ul></details>"
    }
}

const CSS: &str = include_str!("page.css");
const JS: &str = include_str!("page.js");

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let api = Api::load(root);
    let mut page = Page {
        api: &api,
        index: vec![],
        toc: vec![],
        toc_mark: 0,
    };
    let mut chapters = page.ch_overview();
    chapters += &page.ch_services();
    chapters += &page.ch_routines();
    chapters += &page.ch_images();
    chapters += &page.ch_pal();
    chapters += &page.ch_libs();
    chapters += &page.ch_status();
    chapters += &page.ch_index();
    let idx = page
        .index
        .iter()
        .map(|e| {
            format!(
                "[{}]",
                e.iter()
                    .map(|s| serde_json::to_string(s).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let logo: String = "vaxpunk"
        .chars()
        .map(|c| format!("<span>{c}</span>"))
        .collect();
    let html = format!(
        r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<!-- Generated by cargo run -p apidoc from the sources. Don't edit: run it. -->
<title>vaxpunk Internals — API Reference</title><style>{CSS}</style></head>
<body><header class="top"><a class="logo" href="#ch-overview" aria-label="vaxpunk">{logo}</a>
<div class="ttl">Internals and Data Structures<small>API Reference · executive, PAL and libraries</small></div>
<div class="search"><input id="q" type="search" placeholder="Search routines, services, symbols…   /" autocomplete="off" spellcheck="false" aria-label="Search"><ol id="hits"></ol></div>
<button id="theme" type="button" title="Paper or terminal">VT220</button></header>
<nav id="toc">{}</nav>
<main>{chapters}</main>
<script>const IDX=[{}];{JS}</script>
</body></html>
"##,
        page.toc(),
        idx.replace("</", "<\\/"),
    );
    let out = root.join("docs/api/index.html");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    fs::write(&out, html).unwrap();
    println!("apidoc: {}: {} entries", rel(root, &out), page.index.len());
}
