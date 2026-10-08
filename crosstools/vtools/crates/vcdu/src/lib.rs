//! vcdu: the Command Definition Utility. Compiles `.CLD` files into
//! command tables, which CLI$DCL_PARSE parses commands with
//! (`docs/command-tables.md`), as a MACRO-32 module that vmacro compiles.
//! `SET COMMAND/OBJECT` on VMS.

mod cld;

pub use cld::Error;
use cld::{Command, Definitions, Entity, Expr, Loc, Placement, Type};
use std::collections::HashMap;

/// Compiles `.CLD` sources, (file name, text), into one table: MACRO-32
/// source defining the global symbol MODULE names, or `name`. Errors are
/// VMS messages naming the file and line.
pub fn compile(name: &str, sources: &[(String, String)]) -> Result<String, Vec<String>> {
    let mut defs = Definitions::default();
    for (i, (file, text)) in sources.iter().enumerate() {
        cld::parse(text, i, &mut defs).map_err(|e| vec![format!("%CDU-E-SYNTAX, {file}:{e}")])?;
    }
    let module = defs
        .module
        .clone()
        .unwrap_or_else(|| name.to_ascii_uppercase());
    let bytes = Layout::build(&defs).map_err(|errs| {
        errs.into_iter()
            .map(|(loc, msg)| format!("%CDU-E-INVDEF, {}:{}: {msg}", sources[loc.file].0, loc.line))
            .collect::<Vec<_>>()
    })?;
    Ok(macro32(&module, defs.ident.as_deref(), &bytes))
}

/// The table format's version, its first word.
pub const VERSION: u16 = 2;

// An entity's flags, ENT_ in the tables.
const ENT_DEFAULT: u8 = 1;
const ENT_NEGATABLE: u8 = 2;
const ENT_VALUE: u8 = 4;
const ENT_REQUIRED: u8 = 8;
const ENT_LIST: u8 = 16;
const ENT_CONCAT: u8 = 32;
const ENT_BATCH: u8 = 64;

/// The value types, by the code the tables give them.
const TYPES: &[(&str, u8)] = &[
    ("$FILE", 1),
    ("$INFILE", 1),
    ("$OUTFILE", 1),
    ("$OUTLOG", 1),
    ("$NUMBER", 2),
    ("$REST_OF_LINE", 3),
    ("$QUOTED_STRING", 4),
    ("$DATETIME", 6),
    ("$DELTATIME", 6),
    ("$ACL", 6),
    ("$EXPRESSION", 6),
    ("$PARENTHESIZED_VALUE", 6),
    ("$UIC", 7),
];
const TYPE_KEYWORD: u8 = 5;

/// The `DISALLOW` operators.
const OP_NAME: u8 = 1;
const OP_NEG: u8 = 2;
const OP_NOT: u8 = 3;
const OP_AND: u8 = 4;
const OP_OR: u8 = 5;
const OP_ANY2: u8 = 6;

/// An entity's placement, a qualifier's: where it may be given.
const PLACE_LOCAL: u8 = 1;
const PLACE_POSITIONAL: u8 = 2;

/// "Inherit them": a syntax's parameter or qualifier count when it
/// defines none and doesn't say NOPARAMETERS or NOQUALIFIERS.
const INHERIT: u8 = 255;

/// The table's bytes, built block by block, with a comment for each.
struct Layout<'a> {
    defs: &'a Definitions,
    bytes: Vec<u8>,
    /// Where each block starts, and what it is, for the listing.
    marks: Vec<(usize, String)>,
    syntaxes: HashMap<String, usize>,
    types: HashMap<String, usize>,
    /// Places that want a syntax's or a type's offset, filled at the end.
    fixups: Vec<(usize, Ref)>,
    /// The `ROUTINE`s, in the order of their longwords after the table,
    /// and the words that want each one's offset.
    routines: Vec<(String, Vec<usize>)>,
    errors: Vec<(Loc, String)>,
}

/// A block of the table, for the listing: where it starts, its bytes and
/// what it is.
struct Block {
    at: usize,
    bytes: Vec<u8>,
    what: String,
}

/// The table: its blocks, then the routines' addresses, a longword each.
struct Table {
    blocks: Vec<Block>,
    routines: Vec<String>,
}

enum Ref {
    Syntax(String, Loc),
    Type(String, Loc),
}

impl<'a> Layout<'a> {
    fn build(defs: &'a Definitions) -> Result<Table, Vec<(Loc, String)>> {
        let mut l = Layout {
            defs,
            bytes: Vec::new(),
            marks: Vec::new(),
            syntaxes: HashMap::new(),
            types: HashMap::new(),
            fixups: Vec::new(),
            routines: Vec::new(),
            errors: Vec::new(),
        };
        l.check_names();
        // The header: the version, the verbs, and for each name, its own and
        // its synonyms', where the name and the verb's block are.
        let names: Vec<(&str, usize)> = defs
            .verbs
            .iter()
            .enumerate()
            .flat_map(|(i, v)| {
                std::iter::once(v.name.as_str())
                    .chain(v.synonyms.iter().map(|s| s.as_str()))
                    .map(move |n| (n, i))
            })
            .collect();
        // In alphabetical order, as HELP lists them.
        let mut names = names;
        names.sort();
        l.mark("the header: version, verbs, then each one's name and block");
        l.word(VERSION as usize);
        l.word(names.len());
        let slots: Vec<usize> = names
            .iter()
            .map(|_| {
                let at = l.bytes.len();
                l.word(0);
                l.word(0);
                at
            })
            .collect();
        let mut blocks = Vec::new();
        for v in &defs.verbs {
            blocks.push(l.command(v, "verb"));
        }
        for (slot, (name, verb)) in slots.iter().zip(&names) {
            l.mark(&format!("verb name {name}"));
            let at = l.bytes.len();
            l.ascic(name);
            l.put_word(*slot, at);
            l.put_word(slot + 2, blocks[*verb]);
        }
        for s in &defs.syntaxes {
            let at = l.command(s, "syntax");
            l.syntaxes.insert(s.name.clone(), at);
        }
        for t in &defs.types {
            let at = l.type_(t);
            l.types.insert(t.name.clone(), at);
        }
        for (at, r) in std::mem::take(&mut l.fixups) {
            let (map, name, loc, what) = match &r {
                Ref::Syntax(n, loc) => (&l.syntaxes, n, *loc, "syntax"),
                Ref::Type(n, loc) => (&l.types, n, *loc, "type"),
            };
            match map.get(name) {
                Some(&off) => l.put_word(at, off),
                None => l.errors.push((loc, format!("{name}: no such {what}"))),
            }
        }
        // The routines' longwords, after the bytes, aligned.
        while !l.bytes.len().is_multiple_of(4) {
            l.byte(0);
        }
        let vector = l.bytes.len();
        let routines = std::mem::take(&mut l.routines);
        for (i, (_, words)) in routines.iter().enumerate() {
            for at in words {
                l.put_word(*at, vector + 4 * i);
            }
        }
        if vector + 4 * routines.len() > 0xFFFF {
            l.errors
                .push((Loc::default(), "the table is larger than 64 KB".into()));
        }
        if !l.errors.is_empty() {
            l.errors.sort();
            return Err(l.errors);
        }
        // Split the bytes at the marks, for the listing.
        let mut blocks = Vec::new();
        for (i, (at, what)) in l.marks.iter().enumerate() {
            let end = l.marks.get(i + 1).map_or(l.bytes.len(), |m| m.0);
            blocks.push(Block {
                at: *at,
                bytes: l.bytes[*at..end].to_vec(),
                what: what.clone(),
            });
        }
        let routines = routines.into_iter().map(|(name, _)| name).collect();
        Ok(Table { blocks, routines })
    }

    /// Names that must be unique: verbs and their synonyms, syntaxes,
    /// types, and each command's parameters and qualifiers.
    fn check_names(&mut self) {
        let mut seen = HashMap::new();
        let mut once = |what: &str, name: &str, loc: Loc, errors: &mut Vec<(Loc, String)>| {
            if seen.insert(format!("{what} {name}"), loc).is_some() {
                errors.push((loc, format!("{what} {name} is defined twice")));
            }
        };
        let d = self.defs;
        for v in &d.verbs {
            once("verb", &v.name, v.loc, &mut self.errors);
            for s in &v.synonyms {
                once("verb", s, v.loc, &mut self.errors);
            }
        }
        for s in &d.syntaxes {
            once("syntax", &s.name, s.loc, &mut self.errors);
        }
        for t in &d.types {
            once("type", &t.name, t.loc, &mut self.errors);
        }
    }

    fn mark(&mut self, what: &str) {
        self.marks.push((self.bytes.len(), what.into()));
    }

    fn byte(&mut self, b: u8) {
        self.bytes.push(b);
    }

    fn word(&mut self, w: usize) {
        self.bytes.extend_from_slice(&(w as u16).to_le_bytes());
    }

    fn put_word(&mut self, at: usize, w: usize) {
        self.bytes[at..at + 2].copy_from_slice(&(w as u16).to_le_bytes());
    }

    fn ascic(&mut self, s: &str) {
        self.byte(s.len().min(255) as u8);
        self.bytes.extend(s.bytes().take(255));
    }

    /// A verb's or a syntax's block; returns where it starts.
    fn command(&mut self, c: &Command, what: &str) -> usize {
        let doers = [&c.image, &c.routine, &c.cliroutine];
        if doers.iter().filter(|d| d.is_some()).count() > 1 {
            self.errors.push((
                c.loc,
                format!("{}: only one of IMAGE, ROUTINE and CLIROUTINE", c.name),
            ));
        }
        if c.image.as_ref().is_some_and(|i| i.len() > 39) {
            self.errors.push((
                c.loc,
                format!("{}: an image name is at most 39 characters", c.name),
            ));
        }
        if c.params.len() > 8 {
            self.errors
                .push((c.loc, format!("{}: more than 8 parameters", c.name)));
        }
        for (i, p) in c.params.iter().enumerate() {
            if p.name != format!("P{}", i + 1) {
                self.errors.push((
                    p.loc,
                    format!("{}: parameter {} must be P{}", c.name, p.name, i + 1),
                ));
            }
        }
        let required = |p: &Entity| p.value.as_ref().is_some_and(|v| v.required);
        if let Some(p) = c
            .params
            .windows(2)
            .find(|w| !required(&w[0]) && required(&w[1]))
        {
            self.errors.push((
                p[1].loc,
                format!(
                    "{}: required parameter {} after an optional one",
                    c.name, p[1].name
                ),
            ));
        }
        let mut quals = HashMap::new();
        for q in &c.quals {
            if quals.insert(q.name.clone(), ()).is_some() {
                self.errors.push((
                    q.loc,
                    format!("{}: qualifier {} is defined twice", c.name, q.name),
                ));
            }
        }
        let syntax = what == "syntax";
        // Entities first: the block refers to them.
        let params: Vec<usize> = c
            .params
            .iter()
            .map(|p| self.entity(p, "parameter"))
            .collect();
        let quals: Vec<usize> = c
            .quals
            .iter()
            .map(|q| self.entity(q, "qualifier"))
            .collect();
        let disallow = if c.disallows.is_empty() {
            0
        } else {
            self.mark(&format!("{}'s DISALLOW", c.name));
            let at = self.bytes.len();
            // Several DISALLOWs are one, ORed.
            let mut e = c.disallows[0].clone();
            for d in &c.disallows[1..] {
                e = Expr::Or(Box::new(e), Box::new(d.clone()));
            }
            self.expr(&e);
            at
        };
        self.mark(&format!("{what} {}", c.name));
        let at = self.bytes.len();
        self.ascic(&c.name);
        self.ascic(c.image.as_deref().unwrap_or(""));
        self.ascic(c.cliroutine.as_deref().unwrap_or(""));
        let inherit = syntax && !c.noparams && params.is_empty();
        self.byte(if inherit { INHERIT } else { params.len() as u8 });
        for p in params {
            self.word(p);
        }
        let inherit = syntax && !c.noquals && quals.is_empty();
        self.byte(if inherit { INHERIT } else { quals.len() as u8 });
        for q in quals {
            self.word(q);
        }
        // 1 for a syntax's NODISALLOWS, else 0 to inherit its verb's.
        self.word(if disallow == 0 && syntax && c.nodisallows {
            1
        } else {
            disallow
        });
        // Its ROUTINE's longword, after the table, filled at the end.
        if let Some(r) = &c.routine {
            let word = self.bytes.len();
            match self.routines.iter_mut().find(|(n, _)| n == r) {
                Some((_, words)) => words.push(word),
                None => self.routines.push((r.clone(), vec![word])),
            }
        }
        self.word(0);
        at
    }

    fn type_(&mut self, t: &Type) -> usize {
        let keywords: Vec<usize> = t
            .keywords
            .iter()
            .map(|k| self.entity(k, "keyword"))
            .collect();
        self.mark(&format!("type {}", t.name));
        let at = self.bytes.len();
        self.byte(keywords.len() as u8);
        for k in keywords {
            self.word(k);
        }
        at
    }

    fn entity(&mut self, e: &Entity, what: &str) -> usize {
        let placement = match e.placement {
            Placement::Global => 0,
            Placement::Local => PLACE_LOCAL,
            Placement::Positional => PLACE_POSITIONAL,
        };
        if placement != 0 && what != "qualifier" {
            self.errors.push((
                e.loc,
                format!("{}: only a qualifier has a PLACEMENT", e.name),
            ));
        }
        let value = e.value.clone();
        let mut flags = 0;
        if e.default {
            flags |= ENT_DEFAULT;
        }
        // Qualifiers are negatable unless they say not, keywords only if
        // they say so.
        if e.negatable.unwrap_or(what == "qualifier") {
            flags |= ENT_NEGATABLE;
        }
        if e.batch {
            flags |= ENT_BATCH;
        }
        let mut code = 0;
        if let Some(v) = &value {
            flags |= ENT_VALUE;
            if v.required {
                flags |= ENT_REQUIRED;
            }
            if v.list {
                flags |= ENT_LIST;
            }
            // A list concatenates with + unless it says not.
            if v.concatenate.unwrap_or(v.list) {
                flags |= ENT_CONCAT;
            }
            if let Some(t) = &v.type_ {
                // ponytail: a qualifier given after a value has values,
                // but no keywords of its own.
                if placement != 0 && !t.starts_with('$') {
                    self.errors.push((
                        e.loc,
                        format!(
                            "{}: PLACEMENT=LOCAL or POSITIONAL takes no keywords",
                            e.name
                        ),
                    ));
                }
                code = match TYPES.iter().find(|(n, _)| n == t) {
                    Some((_, c)) => *c,
                    None if t.starts_with('$') => {
                        self.errors
                            .push((e.loc, format!("{}: {t} is not a type", e.name)));
                        0
                    }
                    None => TYPE_KEYWORD,
                };
            }
        }
        self.mark(&format!("{what} {}", e.name));
        let at = self.bytes.len();
        self.byte(flags);
        self.byte(code);
        if code == TYPE_KEYWORD {
            let t = value.as_ref().and_then(|v| v.type_.clone()).unwrap();
            self.fixups.push((self.bytes.len(), Ref::Type(t, e.loc)));
        }
        self.word(0);
        if let Some(s) = &e.syntax {
            self.fixups
                .push((self.bytes.len(), Ref::Syntax(s.clone(), e.loc)));
        }
        self.word(0);
        self.byte(placement);
        self.ascic(&e.name);
        let label = e.label.as_deref().unwrap_or(&e.name);
        self.ascic(label);
        self.ascic(e.prompt.as_deref().unwrap_or(label));
        self.ascic(
            value
                .as_ref()
                .and_then(|v| v.default.as_deref())
                .unwrap_or(""),
        );
        at
    }

    /// A `DISALLOW` expression, in prefix form.
    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Name(n) => {
                self.byte(OP_NAME);
                self.ascic(n);
            }
            Expr::Neg(n) => {
                self.byte(OP_NEG);
                self.ascic(n);
            }
            Expr::Not(a) => {
                self.byte(OP_NOT);
                self.expr(a);
            }
            Expr::And(a, b) | Expr::Or(a, b) => {
                self.byte(if matches!(e, Expr::And(..)) {
                    OP_AND
                } else {
                    OP_OR
                });
                self.expr(a);
                self.expr(b);
            }
            Expr::Any2(list) => {
                self.byte(OP_ANY2);
                self.byte(list.len() as u8);
                for a in list {
                    self.expr(a);
                }
            }
        }
    }
}

/// The table as MACRO-32: a read-only psect with the module's name a
/// global label on its first byte, then the blocks, each a comment and its
/// bytes.
fn macro32(module: &str, ident: Option<&str>, table: &Table) -> String {
    let mut s = format!("\t.TITLE\t{module}\tCommand tables, from vcdu\n");
    if let Some(ident) = ident {
        s += &format!("\t.IDENT\t/{ident}/\n");
    }
    s += "\t.PSECT\tCLI$TABLES, NOEXE, NOWRT, LONG\n";
    s += &format!("{module}::\n");
    for Block { at, bytes, what } in &table.blocks {
        s += &format!("; {at}: {what}\n");
        for chunk in bytes.chunks(12) {
            let list: Vec<String> = chunk.iter().map(|b| b.to_string()).collect();
            s += &format!("\t.BYTE\t{}\n", list.join(", "));
        }
    }
    if !table.routines.is_empty() {
        s += "; the routines\n";
    }
    for r in &table.routines {
        s += &format!("\t.EXTERNAL {r}\n\t.ADDRESS {r}\n");
    }
    s += "\t.END\n";
    s
}
