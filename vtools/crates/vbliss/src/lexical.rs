//! The lexical processor (LRM chapters 15 and 16): between the lexer and
//! the parser, it expands macro calls, lexical functions and lexical
//! conditionals, reads require files, and lists the source as it goes.
//!
//! Lexemes come from a stack of streams: the source file at the bottom,
//! require files, and each expansion on top until it is read. A macro
//! body is kept as lexemes and formals (macro-quote level); a call's
//! actuals are read expanded (name-quote level), put in a copy of the
//! body, and the copy is read again. The parser binds names: here a name
//! only matters if it is a macro.

use std::rc::Rc;

use crate::lex::{self, Lexeme, Tok};
use crate::listing::{Entry, NOPOS};
use crate::parse::{Error, Kind, Parser, R, RESERVED};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MacroKind {
    Simple,
    /// `NAME (formals) [] = ...`: may recurse; empty if short of actuals.
    Conditional,
    /// `NAME (fixed) [iterative] = ...`: a copy per group of actuals.
    Iterative,
    Keyword,
}

/// A macro's definition.
#[derive(Debug)]
pub struct Macro {
    pub name: String,
    pub kind: MacroKind,
    /// The formal names: the fixed ones, then the iterative ones.
    pub formals: Vec<String>,
    pub fixed: usize,
    /// A keyword macro's default actuals.
    pub defaults: Vec<Vec<Lexeme>>,
    pub body: Vec<Item>,
}

impl PartialEq for Macro {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// An element of a macro body.
#[derive(Clone, Debug)]
pub enum Item {
    Lex(Lexeme),
    Formal(usize),
}

/// What `%REMAINING`, `%LENGTH` and `%COUNT` mean in a macro's copy.
struct Ctx {
    remaining: Vec<Vec<Lexeme>>,
    length: usize,
    count: usize,
}

struct Stream {
    toks: Vec<Lexeme>,
    pos: usize,
    /// A source file, listed as its lexemes are read.
    file: Option<u16>,
    /// The expansion it belongs to.
    exp: Option<usize>,
    ctx: Option<Rc<Ctx>>,
    /// Where an iteration's separator starts, for %EXITITERATION.
    sep: usize,
}

/// An expansion the listing shows: `[NAME]=` and the lexemes it gave.
struct Expansion {
    id: usize,
    name: String,
    level: usize,
    /// The scan depth whose lexemes it records.
    nest: usize,
    text: Vec<Tok>,
    null: bool,
    /// Its streams still being read.
    open: usize,
    mac: Option<Rc<Macro>>,
}

pub struct SrcFile {
    pub name: String,
    pub lines: Vec<String>,
    pub require: bool,
    listed: u32,
}

/// The lexical processor's state.
#[derive(Default)]
pub struct Lx {
    streams: Vec<Stream>,
    exps: Vec<Expansion>,
    next_exp: usize,
    /// Scans in progress: a lexical function or a macro call reading its
    /// actuals scans again, one deeper.
    nest: usize,
    /// Macro calls reading their actuals.
    collecting: usize,
    /// What the lines read now are inside: 'P' a macro call's actuals,
    /// 'L' a lexical function's.
    reading: Vec<char>,
    /// The last two lexemes each scan depth gave, for an iterative
    /// macro's default punctuation.
    prev: Vec<[Option<Tok>; 2]>,
    pub files: Vec<SrcFile>,
    pub entries: Vec<Entry>,
    listed: u32,
    pub variant: i64,
    /// The bracket that closed the last list of actuals.
    close: Option<Lexeme>,
    /// What %MESSAGE writes.
    pub messages: Vec<String>,
}

impl Lx {
    /// The source file's lexemes: the bottom stream.
    pub fn new(name: &str, text: &str) -> Result<Lx, Error> {
        let mut lx = Lx::default();
        lx.push_file(name, text, false)?;
        Ok(lx)
    }

    fn push_file(&mut self, name: &str, text: &str, require: bool) -> Result<(), Error> {
        let file = self.files.len() as u16;
        let mut toks = lex::lex(text, file).map_err(|(line, msg)| Error {
            file: name.into(),
            line,
            col: 0,
            msg,
        })?;
        if require {
            toks.pop(); // the stream just ends
        }
        self.files.push(SrcFile {
            name: name.into(),
            lines: text
                .lines()
                .map(|l| l.trim_end_matches('\r').to_string())
                .collect(),
            require,
            listed: 0,
        });
        self.streams.push(Stream {
            toks,
            pos: 0,
            file: Some(file),
            exp: None,
            ctx: None,
            sep: usize::MAX,
        });
        Ok(())
    }
}

/// What a `%` name turned out to be.
enum Lexical {
    /// Not a lexical function.
    Not,
    /// Done, its expansion (if any) pushed to be read next.
    Done,
    /// A lexeme as it is, and whether quoted.
    Tok(Lexeme, bool),
}

impl Parser<'_> {
    /// Reads require file `name` next.
    pub(crate) fn require_file(&mut self, name: &str) -> R<()> {
        let text = (self.load)(name).map_err(|e| self.error(e))?;
        self.lx.push_file(name, &text, true)
    }

    /// Reads library `name`'s source next, unlisted.
    pub(crate) fn library_file(&mut self, name: &str) -> R<()> {
        let text = (self.load)(name).map_err(|e| self.error(e))?;
        self.lx.push_file(name, &text, true)?;
        let f = self.lx.files.last_mut().unwrap();
        f.listed = f.lines.len() as u32;
        Ok(())
    }

    /// The next lexeme from the streams, unexpanded, and the stream it came
    /// from. Lists the source lines up to it.
    fn raw(&mut self) -> R<(Lexeme, usize)> {
        loop {
            let i = self.lx.streams.len() - 1;
            let s = &mut self.lx.streams[i];
            if s.pos < s.toks.len() {
                let l = s.toks[s.pos].clone();
                if l.tok != Tok::Eof {
                    s.pos += 1;
                }
                if let Some(f) = s.file {
                    self.list(f, if l.tok == Tok::Eof { u32::MAX } else { l.line });
                }
                return Ok((l, i));
            }
            self.pop_stream();
        }
    }

    fn pop_stream(&mut self) {
        let s = self.lx.streams.pop().unwrap();
        if let Some(f) = s.file {
            self.list(f, u32::MAX);
        }
        if let Some(id) = s.exp {
            let k = self.lx.exps.iter().position(|e| e.id == id).unwrap();
            self.lx.exps[k].open -= 1;
            if self.lx.exps[k].open == 0 {
                let e = self.lx.exps.remove(k);
                self.show(&e);
            }
        }
    }

    /// Lists the lines of `file` up to `upto`.
    fn list(&mut self, file: u16, upto: u32) {
        let flag = self.lx.reading.last().copied().unwrap_or(' ');
        let f = &mut self.lx.files[file as usize];
        while f.listed < upto.min(f.lines.len() as u32) {
            f.listed += 1;
            self.lx.listed += 1;
            self.lx.entries.push(Entry::Line {
                flag,
                require: f.require,
                depth: self.depth,
                number: self.lx.listed,
                file,
                line: f.listed,
                text: f.lines[f.listed as usize - 1].clone(),
            });
        }
    }

    /// Adds an expansion's line to the listing.
    fn show(&mut self, e: &Expansion) {
        let text: Vec<String> = e.text.iter().map(|t| self.shown(t)).collect();
        self.lx.entries.push(Entry::Expansion {
            level: e.level,
            name: e.name.clone(),
            text: (!e.null).then_some(text),
        });
    }

    /// A lexeme as an expansion shows it: a literal's name as its value.
    fn shown(&self, t: &Tok) -> String {
        let value = |id: usize| match self.m.syms[id].kind {
            Kind::Literal(v) | Kind::Compiletime(v) => Some(v),
            _ => None,
        };
        match t {
            Tok::Name(n) => match self.lookup(n).and_then(value) {
                Some(v) => v.to_string(),
                None => n.clone(),
            },
            Tok::Bound(n, id) => value(*id).map_or(n.clone(), |v| v.to_string()),
            Tok::Num(v) => v.to_string(),
            Tok::Str(s) => format!("'{}'", String::from_utf8_lossy(s).replace('\'', "''")),
            Tok::Punct(c) => c.to_string(),
            Tok::Percent => "%".into(),
            Tok::Eof => String::new(),
        }
    }

    /// The listing level of an expansion starting now: the macro calls in
    /// progress.
    fn level(&self) -> usize {
        self.lx.exps.iter().filter(|e| e.mac.is_some()).count() + self.lx.collecting
    }

    /// Pushes `streams`, the parts of one expansion, to be read next (the
    /// first on top), or shows it at once if there are none.
    fn expansion(
        &mut self,
        name: &str,
        level: usize,
        mac: Option<Rc<Macro>>,
        streams: Vec<Stream>,
    ) {
        let id = self.lx.next_exp;
        self.lx.next_exp += 1;
        let e = Expansion {
            id,
            name: name.into(),
            level,
            nest: self.lx.nest - 1,
            text: Vec::new(),
            null: streams.is_empty(),
            open: streams.len(),
            mac,
        };
        if streams.is_empty() {
            return self.show(&e);
        }
        self.lx.exps.push(e);
        for mut s in streams.into_iter().rev() {
            s.exp = Some(id);
            self.lx.streams.push(s);
        }
    }

    /// The next lexeme at name-quote or normal level, expanded, and whether
    /// %QUOTE quoted it.
    pub(crate) fn scan(&mut self) -> R<(Lexeme, bool)> {
        let nest = self.lx.nest;
        self.lx.nest += 1;
        let r = self.scan1();
        self.lx.nest -= 1;
        let (l, quoted) = r?;
        for e in self.lx.exps.iter_mut().filter(|e| e.nest == nest) {
            e.text.push(l.tok.clone());
        }
        if self.lx.prev.len() <= nest {
            self.lx.prev.resize(nest + 1, [None, None]);
        }
        let p = &mut self.lx.prev[nest];
        p[0] = p[1].take();
        p[1] = Some(l.tok.clone());
        Ok((l, quoted))
    }

    fn scan1(&mut self) -> R<(Lexeme, bool)> {
        loop {
            let (l, si) = self.raw()?;
            let id = match &l.tok {
                Tok::Name(n) if n.starts_with('%') => match self.lexical(n.clone(), &l, si)? {
                    Lexical::Not => self.lookup(n),
                    Lexical::Done => continue,
                    Lexical::Tok(t, q) => return Ok((t, q)),
                },
                Tok::Name(n) => self.lookup(n),
                Tok::Bound(_, id) => Some(*id),
                _ => None,
            };
            if let Some(Kind::Macro(m)) = id.map(|id| &self.m.syms[id].kind) {
                let m = m.clone();
                self.call(m, &l)?;
                continue;
            }
            return Ok((l, false));
        }
    }

    /// A macro call, its name read: reads the actuals and pushes the
    /// expansion.
    fn call(&mut self, m: Rc<Macro>, at: &Lexeme) -> R<()> {
        let depth = self
            .lx
            .exps
            .iter()
            .filter(|e| e.mac.as_ref().is_some_and(|x| Rc::ptr_eq(x, &m)))
            .count();
        if depth > 0 && m.kind != MacroKind::Conditional {
            return Err(self.error_at(
                at,
                format!("Recursive invocation of non-recursive macro {}", m.name),
            ));
        }
        if self.lx.streams.len() > 2000 {
            return Err(self.error_at(at, format!("Macro {} nests too deeply", m.name)));
        }
        let level = self.level();
        let nest = self.lx.nest - 1;
        let context = self.lx.prev.get(nest).cloned().unwrap_or_default();
        let actuals = if m.kind == MacroKind::Simple && m.formals.is_empty() {
            Vec::new()
        } else {
            self.lx.collecting += 1;
            self.lx.reading.push('P');
            let r = self.actual_list(&format!("macro {}", m.name), at);
            self.lx.reading.pop();
            self.lx.collecting -= 1;
            r?
        };
        let place = |l: &Lexeme| Lexeme {
            col: NOPOS,
            file: at.file,
            line: at.line,
            ..l.clone()
        };
        let copy = |args: &[Vec<Lexeme>]| -> Vec<Lexeme> {
            let mut out = Vec::new();
            for item in &m.body {
                match item {
                    Item::Lex(l) => out.push(place(l)),
                    Item::Formal(i) => out.extend(args[*i].iter().cloned()),
                }
            }
            out
        };
        let stream = |toks: Vec<Lexeme>, ctx: Option<Ctx>| Stream {
            sep: toks.len(),
            toks,
            pos: 0,
            file: None,
            exp: None,
            ctx: ctx.map(Rc::new),
        };
        let n = m.formals.len();
        let mut streams = Vec::new();
        match m.kind {
            MacroKind::Simple | MacroKind::Conditional | MacroKind::Keyword => {
                let mut args = if m.kind == MacroKind::Keyword {
                    self.keyword_actuals(&m, actuals.clone(), at)?
                } else {
                    actuals.clone()
                };
                let short = args.len() < n || (args.is_empty() && n == 0);
                if m.kind != MacroKind::Conditional || !short {
                    let remaining = if args.len() > n {
                        args.split_off(n)
                    } else {
                        Vec::new()
                    };
                    args.resize(n, Vec::new());
                    let count = if m.kind == MacroKind::Conditional && n > 0 {
                        depth
                    } else {
                        0
                    };
                    streams.push(stream(
                        copy(&args),
                        Some(Ctx {
                            remaining,
                            length: actuals.len(),
                            count,
                        }),
                    ));
                }
            }
            MacroKind::Iterative if actuals.len() > m.fixed => {
                let (left, sep, right) = punctuation(&context);
                let k = n - m.fixed;
                let mut rest = actuals[m.fixed..].to_vec();
                let mut count = 0;
                while !rest.is_empty() {
                    let mut args = actuals[..m.fixed].to_vec();
                    args.extend(rest.drain(..k.min(rest.len())));
                    args.resize(n, Vec::new());
                    let mut s = stream(
                        copy(&args),
                        Some(Ctx {
                            remaining: rest.clone(),
                            length: actuals.len(),
                            count,
                        }),
                    );
                    if !rest.is_empty() {
                        s.toks.push(place(&Lexeme {
                            tok: sep.clone(),
                            ..at.clone()
                        }));
                    }
                    streams.push(s);
                    count += 1;
                }
                let grouper = |t: Option<Tok>| {
                    t.map(|t| {
                        stream(
                            vec![place(&Lexeme {
                                tok: t,
                                ..at.clone()
                            })],
                            None,
                        )
                    })
                };
                if let Some(s) = grouper(left) {
                    streams.insert(0, s);
                }
                streams.extend(grouper(right));
            }
            MacroKind::Iterative => {}
        }
        self.expansion(&m.name, level, Some(m.clone()), streams);
        Ok(())
    }

    /// A keyword macro's actuals, `NAME = lexemes` each, in its formals'
    /// order, with the defaults for those not given.
    fn keyword_actuals(
        &mut self,
        m: &Macro,
        actuals: Vec<Vec<Lexeme>>,
        at: &Lexeme,
    ) -> R<Vec<Vec<Lexeme>>> {
        let mut args: Vec<Option<Vec<Lexeme>>> = vec![None; m.formals.len()];
        for a in actuals {
            let i = match (a.first().map(|l| &l.tok), a.get(1).map(|l| &l.tok)) {
                (Some(Tok::Name(n) | Tok::Bound(n, _)), Some(Tok::Punct('='))) => {
                    m.formals.iter().position(|f| f == n)
                }
                _ => None,
            };
            let Some(i) = i.filter(|&i| args[i].is_none()) else {
                return Err(self.error_at(
                    at,
                    format!("Invalid keyword parameter in call to macro {}", m.name),
                ));
            };
            args[i] = Some(a[2..].to_vec());
        }
        Ok(args
            .into_iter()
            .zip(&m.defaults)
            .map(|(a, d)| a.unwrap_or_else(|| d.clone()))
            .collect())
    }

    /// A parenthesized (or bracketed) list of actual parameters: none for
    /// `()`.
    fn actual_list(&mut self, of: &str, at: &Lexeme) -> R<Vec<Vec<Lexeme>>> {
        let (open, _) = self.scan()?;
        let close = match open.tok {
            Tok::Punct('(') => ')',
            Tok::Punct('[') => ']',
            Tok::Punct('<') => '>',
            _ => {
                return Err(self.error_at(at, format!("Missing actual parameter list for {of}")));
            }
        };
        let nest = self.lx.nest;
        self.lx.prev[nest] = [None, Some(open.tok)];
        let (mut out, mut stack) = (vec![Vec::new()], Vec::new());
        loop {
            let (l, quoted) = self.scan()?;
            if !quoted {
                match l.tok {
                    Tok::Eof => {
                        return Err(self.error_at(
                            at,
                            format!("Missing {close} after the actual parameters of {of}"),
                        ));
                    }
                    Tok::Punct(c @ ('(' | '[' | '<')) => stack.push(match c {
                        '(' => ')',
                        '[' => ']',
                        _ => '>',
                    }),
                    Tok::Punct(c) if stack.is_empty() && c == close => {
                        self.lx.close = Some(l);
                        break;
                    }
                    Tok::Punct(',') if stack.is_empty() => {
                        out.push(Vec::new());
                        continue;
                    }
                    Tok::Punct(c) if stack.last() == Some(&c) => {
                        stack.pop();
                    }
                    _ => {}
                }
            }
            out.last_mut().unwrap().push(l);
        }
        if out.len() == 1 && out[0].is_empty() {
            out.clear();
        }
        Ok(out)
    }

    /// A lexical function's parameters.
    fn params(&mut self, name: &str, at: &Lexeme) -> R<Vec<Vec<Lexeme>>> {
        self.lx.reading.push('L');
        let r = self.actual_list(&format!("lexical function {name}"), at);
        self.lx.reading.pop();
        r
    }

    /// A macro body, after its `=`, up to its `%` (macro-quote level):
    /// formals found, quote functions done.
    pub(crate) fn macro_body(&mut self, formals: &[String], end: &[Tok]) -> R<(Vec<Item>, Tok)> {
        let mut body = Vec::new();
        let mut stack = Vec::new();
        loop {
            let (l, _) = self.raw()?;
            // An expansion this body comes from shows it whole.
            let nest = self.lx.nest;
            for e in self.lx.exps.iter_mut().filter(|e| e.nest == nest) {
                e.text.push(l.tok.clone());
            }
            match &l.tok {
                Tok::Percent if end.is_empty() => return Ok((body, Tok::Percent)),
                t if stack.is_empty() && end.contains(t) => return Ok((body, t.clone())),
                Tok::Eof => return Err(self.error_at(&l, "Missing % at the end of a macro body")),
                Tok::Punct(c @ ('(' | '[' | '<')) => {
                    stack.push(match c {
                        '(' => ')',
                        '[' => ']',
                        _ => '>',
                    });
                    body.push(Item::Lex(l));
                }
                Tok::Punct(c) if stack.last() == Some(c) => {
                    stack.pop();
                    body.push(Item::Lex(l));
                }
                Tok::Name(n) if n == "%QUOTE" => body.push(Item::Lex(self.raw()?.0)),
                Tok::Name(n) if n == "%UNQUOTE" => {
                    let (next, _) = self.raw()?;
                    body.push(match &next.tok {
                        Tok::Name(n) => match formals.iter().position(|f| f == n) {
                            Some(i) => Item::Formal(i),
                            None => match self.lookup(n) {
                                Some(id) => Item::Lex(Lexeme {
                                    tok: Tok::Bound(n.clone(), id),
                                    ..next
                                }),
                                None => Item::Lex(next),
                            },
                        },
                        _ => Item::Lex(next),
                    });
                }
                Tok::Name(n) if n == "%EXPAND" => {
                    // What it expands is read as a scan one deeper would.
                    self.lx.nest += 1;
                    let r = self.expand_now(&mut body);
                    self.lx.nest -= 1;
                    r?;
                }
                Tok::Name(n) => match formals.iter().position(|f| f == n) {
                    Some(i) => body.push(Item::Formal(i)),
                    None => body.push(Item::Lex(l)),
                },
                _ => body.push(Item::Lex(l)),
            }
        }
    }

    /// `%EXPAND`'s work in a macro body: the macro call or lexical function
    /// after it, expanded to be read next.
    fn expand_now(&mut self, body: &mut Vec<Item>) -> R<()> {
        let (next, si) = self.raw()?;
        let mac = match &next.tok {
            Tok::Name(n) if n.starts_with('%') => match self.lexical(n.clone(), &next, si)? {
                Lexical::Not => None,
                Lexical::Done => return Ok(()),
                Lexical::Tok(t, _) => {
                    body.push(Item::Lex(t));
                    return Ok(());
                }
            },
            Tok::Name(n) => self.lookup(n),
            _ => None,
        };
        match mac.map(|id| &self.m.syms[id].kind) {
            Some(Kind::Macro(m)) => {
                let m = m.clone();
                self.call(m, &next)
            }
            _ => {
                body.push(Item::Lex(next));
                Ok(())
            }
        }
    }

    /// Skips lexemes up to the `%ELSE` (if `alternative`) or `%FI` that
    /// matches, and past it.
    fn skip(&mut self, alternative: bool) -> R<()> {
        let mut depth = 0;
        loop {
            let (l, _) = self.raw()?;
            match &l.tok {
                Tok::Eof => return Err(self.error_at(&l, "Missing %FI")),
                Tok::Name(n) if n == "%IF" => depth += 1,
                Tok::Name(n) if n == "%FI" && depth == 0 => return Ok(()),
                Tok::Name(n) if n == "%FI" => depth -= 1,
                Tok::Name(n) if n == "%ELSE" && depth == 0 && alternative => return Ok(()),
                _ => {}
            }
        }
    }

    /// The macro context `%REMAINING` and the like in stream `si` refer to:
    /// the stream's index and its context.
    fn ctx(&self, si: usize) -> Option<(usize, Rc<Ctx>)> {
        (0..=si)
            .rev()
            .find_map(|i| self.lx.streams[i].ctx.clone().map(|c| (i, c)))
    }

    /// Pushes lexemes to be read next.
    fn push(&mut self, toks: Vec<Lexeme>) {
        self.lx.streams.push(Stream {
            toks,
            pos: 0,
            file: None,
            exp: None,
            ctx: None,
            sep: usize::MAX,
        });
    }

    /// The lexical function `name`, read at `at` from stream `si`.
    fn lexical(&mut self, name: String, at: &Lexeme, si: usize) -> R<Lexical> {
        let lexeme = |tok| Lexeme { tok, ..at.clone() };
        let num = |n: i64| Lexical::Tok(lexeme(Tok::Num(n)), false);
        let flag = |b: bool| num(b as i64);
        Ok(match name.as_str() {
            "%QUOTE" => {
                let (l, _) = self.raw()?;
                Lexical::Tok(l, true)
            }
            "%UNQUOTE" => {
                let (l, _) = self.raw()?;
                match &l.tok {
                    Tok::Name(n) => match self.lookup(n) {
                        Some(id) => Lexical::Tok(
                            Lexeme {
                                tok: Tok::Bound(n.clone(), id),
                                ..l.clone()
                            },
                            true,
                        ),
                        None => Lexical::Tok(l, true),
                    },
                    _ => Lexical::Tok(l, true),
                }
            }
            "%EXPAND" => Lexical::Done,
            "%REMAINING" | "%LENGTH" | "%COUNT" | "%EXITITERATION" | "%EXITMACRO" => {
                let Some((k, ctx)) = self.ctx(si) else {
                    return Err(self.error_at(at, format!("{name} outside a macro")));
                };
                match name.as_str() {
                    "%LENGTH" => num(ctx.length as i64),
                    "%COUNT" => num(ctx.count as i64),
                    "%REMAINING" => {
                        let mut toks = Vec::new();
                        for (i, a) in ctx.remaining.iter().enumerate() {
                            if i > 0 {
                                toks.push(Lexeme {
                                    tok: Tok::Punct(','),
                                    col: NOPOS,
                                    ..at.clone()
                                });
                            }
                            toks.extend(a.iter().cloned());
                        }
                        let level = self.level();
                        let streams = if toks.is_empty() {
                            Vec::new()
                        } else {
                            vec![Stream {
                                toks,
                                pos: 0,
                                file: None,
                                exp: None,
                                ctx: None,
                                sep: usize::MAX,
                            }]
                        };
                        self.expansion("%REMAINING", level, None, streams);
                        Lexical::Done
                    }
                    _ => {
                        while self.lx.streams.len() > k + 1 {
                            self.pop_stream();
                        }
                        let s = &mut self.lx.streams[k];
                        if name == "%EXITITERATION" && s.sep != usize::MAX {
                            s.pos = s.pos.max(s.sep);
                        } else {
                            let exp = s.exp;
                            for s in self.lx.streams[..=k].iter_mut().rev() {
                                if s.exp != exp || s.ctx.is_none() {
                                    break;
                                }
                                s.pos = s.toks.len();
                            }
                        }
                        Lexical::Done
                    }
                }
            }
            "%IF" => {
                let mut test = Vec::new();
                loop {
                    let (l, quoted) = self.scan()?;
                    match &l.tok {
                        Tok::Name(n) if n == "%THEN" && !quoted => break,
                        Tok::Eof => return Err(self.error_at(at, "Missing %THEN")),
                        _ => test.push(l),
                    }
                }
                if self.subparse(test, |p| p.ctce())? & 1 == 0 {
                    self.skip(true)?;
                }
                Lexical::Done
            }
            "%ELSE" => {
                self.skip(false)?;
                Lexical::Done
            }
            "%FI" => Lexical::Done,
            "%B" | "%O" | "%X" | "%DECIMAL" | "%C" | "%ASCII" | "%ASCIZ" | "%ASCIC" => {
                let (l, _) = self.scan()?;
                let Tok::Str(s) = l.tok else {
                    return Err(
                        self.error_at(at, format!("Missing quoted string following {name}"))
                    );
                };
                let tok = match name.as_str() {
                    "%ASCII" => Tok::Str(s),
                    "%ASCIZ" => Tok::Str([&s[..], &[0]].concat()),
                    "%ASCIC" if s.len() < 256 => Tok::Str([&[s.len() as u8], &s[..]].concat()),
                    "%C" if s.len() == 1 => Tok::Num(s[0].into()),
                    "%ASCIC" | "%C" => {
                        return Err(self.error_at(at, format!("String too long for {name}")));
                    }
                    _ => {
                        let radix = match name.as_str() {
                            "%B" => 2,
                            "%O" => 8,
                            "%X" => 16,
                            _ => 10,
                        };
                        let text = String::from_utf8_lossy(&s).trim().to_string();
                        let (neg, digits) = match text.strip_prefix('-') {
                            Some(d) => (true, d),
                            None => (false, text.strip_prefix('+').unwrap_or(&text)),
                        };
                        let n = lex::number(digits.trim(), radix).ok_or_else(|| {
                            self.error_at(at, format!("Illegal character in {name} literal"))
                        })?;
                        Tok::Num(if neg { n.wrapping_neg() } else { n })
                    }
                };
                Lexical::Tok(lexeme(tok), false)
            }
            "%STRING" | "%CHARCOUNT" | "%EXPLODE" | "%NAME" | "%QUOTENAME" | "%ERROR" | "%WARN"
            | "%INFORM" | "%PRINT" | "%MESSAGE" | "%ERRORMACRO" | "%REQUIRE" => {
                let ps = self.params(&name, at)?;
                let mut s = Vec::new();
                for p in &ps {
                    s.extend(self.string_param(&name, p, at)?);
                }
                let first = ps
                    .iter()
                    .flatten()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| at.clone());
                match name.as_str() {
                    "%STRING" => Lexical::Tok(lexeme(Tok::Str(s)), false),
                    "%CHARCOUNT" => num(s.len() as i64),
                    "%EXPLODE" => {
                        let mut toks = Vec::new();
                        for (i, c) in s.iter().enumerate() {
                            if i > 0 {
                                toks.push(lexeme(Tok::Punct(',')));
                            }
                            toks.push(lexeme(Tok::Str(vec![*c])));
                        }
                        if toks.is_empty() {
                            toks.push(lexeme(Tok::Str(Vec::new())));
                        }
                        self.push(toks);
                        Lexical::Done
                    }
                    "%NAME" => {
                        self.push(vec![lexeme(Tok::Name(String::from_utf8_lossy(&s).into()))]);
                        Lexical::Done
                    }
                    "%QUOTENAME" => {
                        Lexical::Tok(lexeme(Tok::Name(String::from_utf8_lossy(&s).into())), true)
                    }
                    "%PRINT" => {
                        self.lx
                            .entries
                            .push(Entry::Print(String::from_utf8_lossy(&s).into()));
                        Lexical::Done
                    }
                    "%MESSAGE" => {
                        self.lx.messages.push(String::from_utf8_lossy(&s).into());
                        Lexical::Done
                    }
                    "%REQUIRE" => {
                        self.require_file(&String::from_utf8_lossy(&s))?;
                        Lexical::Done
                    }
                    _ => {
                        let (sev, label) = match name.as_str() {
                            "%INFORM" => ('I', "%INFORM:\t"),
                            "%WARN" => ('W', "%WARN:\t"),
                            "%ERROR" => ('E', "%ERROR:\t"),
                            _ => ('E', "%ERRORMACRO:\t"),
                        };
                        self.diag(
                            sev,
                            &first,
                            format!("{label}{}", String::from_utf8_lossy(&s)),
                        );
                        if name == "%ERRORMACRO" {
                            while self.lx.streams.last().is_some_and(|s| s.file.is_none()) {
                                self.pop_stream();
                            }
                        }
                        Lexical::Done
                    }
                }
            }
            "%EXACTSTRING" => {
                let mut ps = self.params(&name, at)?.into_iter();
                let (Some(n), Some(fill)) = (ps.next(), ps.next()) else {
                    return Err(
                        self.error_at(at, "%EXACTSTRING takes a length and a fill character")
                    );
                };
                let (n, fill) = (self.ctce_param(n)?, self.ctce_param(fill)?);
                let mut s = Vec::new();
                for p in ps {
                    s.extend(self.string_param(&name, &p, at)?);
                }
                if !(0..=65535).contains(&n) {
                    return Err(self.error_at(at, "Illegal length for %EXACTSTRING"));
                }
                s.resize(n as usize, fill as u8);
                Lexical::Tok(lexeme(Tok::Str(s)), false)
            }
            "%CHAR" => {
                let mut s = Vec::new();
                for p in self.params(&name, at)? {
                    s.push(self.ctce_param(p)? as u8);
                }
                Lexical::Tok(lexeme(Tok::Str(s)), false)
            }
            "%REMOVE" => {
                let mut ps = self.params(&name, at)?;
                let mut p = ps.pop().unwrap_or_default();
                if !ps.is_empty() {
                    return Err(self.error_at(at, "%REMOVE takes one parameter"));
                }
                if enclosed(&p) {
                    p.pop();
                    p.remove(0);
                }
                self.push(p);
                Lexical::Done
            }
            "%NULL" => flag(self.params(&name, at)?.iter().all(|p| p.is_empty())),
            "%IDENTICAL" => {
                let ps = self.params(&name, at)?;
                let [a, b] = &ps[..] else {
                    return Err(self.error_at(at, "%IDENTICAL takes two parameters"));
                };
                flag(a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.tok == y.tok))
            }
            "%ISSTRING" => flag(self.params(&name, at)?.iter().all(|p| {
                matches!(
                    &p[..],
                    [Lexeme {
                        tok: Tok::Str(_),
                        ..
                    }]
                )
            })),
            "%CTCE" | "%LTCE" => {
                let mut all = true;
                for p in self.params(&name, at)? {
                    let e = self.subparse(p, |p| p.expr())?;
                    all &= if name == "%CTCE" {
                        crate::parse::fold(&e).is_some()
                    } else {
                        self.ltce(&e)
                    };
                }
                flag(all)
            }
            "%NBITS" | "%NBITSU" => {
                let mut bits = 0;
                for p in self.params(&name, at)? {
                    let v = self.ctce_param(p)?;
                    bits = bits.max(if name == "%NBITSU" {
                        if v < 0 {
                            64
                        } else {
                            (64 - v.leading_zeros() as i64).max(1)
                        }
                    } else if v < 0 {
                        65 - v.leading_ones() as i64
                    } else {
                        65 - v.leading_zeros() as i64
                    });
                }
                num(bits)
            }
            "%ASSIGN" => {
                let mut ps = self.params(&name, at)?.into_iter();
                let (Some(n), Some(v), None) = (ps.next(), ps.next(), ps.next()) else {
                    return Err(self.error_at(at, "%ASSIGN takes a name and a value"));
                };
                let id = match &n[..] {
                    [
                        Lexeme {
                            tok: Tok::Name(n) | Tok::Bound(n, _),
                            ..
                        },
                    ] => self.lookup(n),
                    _ => None,
                };
                let v = self.ctce_param(v)?;
                match id.map(|id| &mut self.m.syms[id].kind) {
                    Some(Kind::Compiletime(x)) => *x = v,
                    _ => {
                        return Err(self
                            .error_at(at, "%ASSIGN's first parameter must be a COMPILETIME name"));
                    }
                }
                Lexical::Done
            }
            "%NUMBER" => {
                let ps = self.params(&name, at)?;
                let v = match &ps[..] {
                    [p] => match &p[..] {
                        [
                            Lexeme {
                                tok: Tok::Num(n), ..
                            },
                        ] => Some(*n),
                        [
                            Lexeme {
                                tok: Tok::Str(s), ..
                            },
                        ] => {
                            let t = String::from_utf8_lossy(s);
                            let t = t.trim();
                            let (neg, d) = match t.strip_prefix('-') {
                                Some(d) => (true, d),
                                None => (false, t.strip_prefix('+').unwrap_or(t)),
                            };
                            lex::number(d, 10).map(|n| if neg { n.wrapping_neg() } else { n })
                        }
                        [
                            Lexeme {
                                tok: Tok::Name(n), ..
                            },
                        ] => self.lookup(n).and_then(|id| match self.m.syms[id].kind {
                            Kind::Literal(v) | Kind::Compiletime(v) => Some(v),
                            _ => None,
                        }),
                        [
                            Lexeme {
                                tok: Tok::Bound(_, id),
                                ..
                            },
                        ] => match self.m.syms[*id].kind {
                            Kind::Literal(v) | Kind::Compiletime(v) => Some(v),
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                };
                match v {
                    Some(v) => num(v),
                    None => {
                        return Err(self.error_at(
                            at,
                            "Illegal parameter in call to lexical function %NUMBER",
                        ));
                    }
                }
            }
            "%DECLARED" => {
                let ps = self.params(&name, at)?;
                match &ps[..] {
                    [p] if matches!(
                        &p[..],
                        [Lexeme {
                            tok: Tok::Name(_) | Tok::Bound(..),
                            ..
                        }]
                    ) =>
                    {
                        let (Tok::Name(n) | Tok::Bound(n, _)) = &p[0].tok else {
                            unreachable!()
                        };
                        flag(self.lookup(n).is_some())
                    }
                    _ => {
                        let at = self.lx.close.clone().unwrap_or_else(|| at.clone());
                        self.diag(
                            'W',
                            &at,
                            "Illegal parameter in call to lexical function %DECLARED".into(),
                        );
                        num(0)
                    }
                }
            }
            "%BLISS" => {
                let ps = self.params(&name, at)?;
                flag(matches!(&ps[..], [p] if matches!(&p[..],
                    [Lexeme { tok: Tok::Name(n), .. }] if n == "BLISS64E")))
            }
            "%SWITCHES" => {
                let mut all = true;
                for p in self.params(&name, at)? {
                    let [
                        Lexeme {
                            tok: Tok::Name(s), ..
                        },
                    ] = &p[..]
                    else {
                        return Err(self.error_at(
                            at,
                            "Illegal parameter in call to lexical function %SWITCHES",
                        ));
                    };
                    all &= self.switch_on(s);
                }
                flag(all)
            }
            "%VARIANT" => num(self.lx.variant),
            "%MODULE" => Lexical::Tok(lexeme(Tok::Name(self.m.name.clone())), false),
            "%ALLOCATION" => {
                let ps = self.params(&name, at)?;
                let id = match &ps[..] {
                    [p] => match &p[..] {
                        [
                            Lexeme {
                                tok: Tok::Name(n), ..
                            },
                        ] => self.lookup(n),
                        [
                            Lexeme {
                                tok: Tok::Bound(_, id),
                                ..
                            },
                        ] => Some(*id),
                        _ => None,
                    },
                    _ => None,
                };
                match id.map(|id| &self.m.syms[id].kind) {
                    Some(Kind::Data { bytes, .. }) => num(*bytes as i64),
                    _ => {
                        return Err(self.error_at(
                            at,
                            "Illegal parameter in call to lexical function %ALLOCATION",
                        ));
                    }
                }
            }
            "%SIZE" => {
                let mut ps = self.params(&name, at)?;
                let (Some(p), true) = (ps.pop(), ps.is_empty()) else {
                    return Err(self.error_at(at, "%SIZE takes a structure attribute"));
                };
                num(self.subparse(p, |p| p.size_of_attr())?)
            }
            "%FIELDEXPAND" => {
                let mut ps = self.params(&name, at)?.into_iter();
                let comps = match ps.next().as_deref() {
                    Some(
                        [
                            Lexeme {
                                tok: Tok::Name(n) | Tok::Bound(n, _),
                                ..
                            },
                        ],
                    ) => match self.lookup(n).map(|id| &self.m.syms[id].kind) {
                        Some(Kind::Field(c)) => c.clone(),
                        _ => return Err(self.error_at(at, format!("{n} is not a field name"))),
                    },
                    _ => return Err(self.error_at(at, "%FIELDEXPAND takes a field name")),
                };
                let comps = match ps.next() {
                    Some(n) => {
                        let n = self.ctce_param(n)?;
                        match comps.get(n as usize) {
                            Some(&c) if n >= 0 => vec![c],
                            _ => {
                                return Err(self.error_at(
                                    at,
                                    "%FIELDEXPAND's component number is out of range",
                                ));
                            }
                        }
                    }
                    None => comps,
                };
                let mut toks = Vec::new();
                for (i, c) in comps.into_iter().enumerate() {
                    if i > 0 {
                        toks.push(lexeme(Tok::Punct(',')));
                    }
                    toks.push(lexeme(Tok::Num(c)));
                }
                self.push(toks);
                Lexical::Done
            }
            "%TITLE" | "%SBTTL" => {
                let (l, _) = self.scan()?;
                if !matches!(l.tok, Tok::Str(_)) {
                    return Err(
                        self.error_at(at, format!("Missing quoted string following {name}"))
                    );
                }
                Lexical::Done
            }
            _ => Lexical::Not,
        })
    }

    /// A %STRING parameter's characters.
    fn string_param(&self, name: &str, p: &[Lexeme], at: &Lexeme) -> R<Vec<u8>> {
        Ok(match p {
            [] => Vec::new(),
            [l] => match &l.tok {
                Tok::Str(s) => s.clone(),
                Tok::Num(n) => n.to_string().into_bytes(),
                Tok::Name(n) | Tok::Bound(n, _) if !RESERVED.contains(&n.as_str()) => {
                    n.clone().into_bytes()
                }
                _ => {
                    return Err(self.error_at(
                        l,
                        format!("Illegal parameter in call to lexical function {name}"),
                    ));
                }
            },
            _ => {
                return Err(self.error_at(
                    p.iter().find(|l| l.col != NOPOS).unwrap_or(at),
                    format!("Illegal parameter in call to lexical function {name}"),
                ));
            }
        })
    }

    /// A compile-time constant expression parameter's value.
    fn ctce_param(&mut self, p: Vec<Lexeme>) -> R<i64> {
        self.subparse(p, |p| p.ctce())
    }
}

/// Whether `p` is enclosed in a matched pair of brackets.
fn enclosed(p: &[Lexeme]) -> bool {
    let close = match p.first().map(|l| &l.tok) {
        Some(Tok::Punct('(')) => ')',
        Some(Tok::Punct('[')) => ']',
        Some(Tok::Punct('<')) => '>',
        _ => return false,
    };
    let open = match close {
        ')' => '(',
        ']' => '[',
        _ => '<',
    };
    let mut depth = 0;
    for (i, l) in p.iter().enumerate() {
        match l.tok {
            Tok::Punct(c) if c == open => depth += 1,
            Tok::Punct(c) if c == close => {
                depth -= 1;
                if depth == 0 {
                    return i == p.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

/// An iterative macro's default punctuation from the two lexemes before
/// its call (LRM 16.3.3.4): the left grouper, the separator and the right
/// grouper.
fn punctuation(prev: &[Option<Tok>; 2]) -> (Option<Tok>, Tok, Option<Tok>) {
    let comma = (None, Tok::Punct(','), None);
    let semi = (None, Tok::Punct(';'), None);
    let parens = (
        Some(Tok::Punct('(')),
        Tok::Punct(','),
        Some(Tok::Punct(')')),
    );
    let name = |t: &Option<Tok>, words: &[&str]| matches!(t, Some(Tok::Name(n)) if words.contains(&n.as_str()));
    match &prev[1] {
        None => comma,
        Some(Tok::Punct('(')) => {
            let list = match &prev[0] {
                None => true,
                Some(Tok::Name(n)) => {
                    !RESERVED.contains(&n.as_str())
                        || ["PLIT", "UPLIT", "INITIAL", "PRESET"].contains(&n.as_str())
                }
                Some(Tok::Bound(..) | Tok::Punct(')' | ']' | '>')) => true,
                _ => false,
            };
            if list { comma } else { semi }
        }
        Some(Tok::Punct('[' | '<' | ',')) => comma,
        Some(Tok::Punct(';')) => semi,
        Some(Tok::Punct(c @ ('+' | '-' | '*' | '/' | '^' | '='))) => (None, Tok::Punct(*c), None),
        Some(Tok::Name(n)) if OPERATORS.contains(&n.as_str()) => (None, Tok::Name(n.clone()), None),
        t if name(t, &["OF"]) => (
            Some(Tok::Name("SET".into())),
            Tok::Punct(';'),
            Some(Tok::Name("TES".into())),
        ),
        t if name(
            t,
            &[
                "BEGIN",
                "SET",
                "IF",
                "THEN",
                "ELSE",
                "DO",
                "WHILE",
                "UNTIL",
                "CASE",
                "SELECT",
                "SELECTU",
                "SELECTA",
                "SELECTONE",
                "SELECTONEU",
                "SELECTONEA",
                "INCR",
                "INCRA",
                "INCRU",
                "DECR",
                "DECRA",
                "DECRU",
                "RETURN",
                "CODECOMMENT",
            ],
        ) =>
        {
            semi
        }
        t if name(t, DECLARATIONS) => comma,
        _ => parens,
    }
}

/// The operators named by words, which an iterative macro after them
/// repeats between its copies.
const OPERATORS: &[&str] = &[
    "MOD", "AND", "OR", "XOR", "EQV", "NOT", "EQL", "EQLA", "EQLU", "NEQ", "NEQA", "NEQU", "LSS",
    "LSSA", "LSSU", "LEQ", "LEQA", "LEQU", "GTR", "GTRA", "GTRU", "GEQ", "GEQA", "GEQU",
];

/// The words that begin declarations.
const DECLARATIONS: &[&str] = &[
    "OWN",
    "GLOBAL",
    "EXTERNAL",
    "LOCAL",
    "STACKLOCAL",
    "REGISTER",
    "LITERAL",
    "BIND",
    "MACRO",
    "KEYWORDMACRO",
    "ROUTINE",
    "FORWARD",
    "COMPILETIME",
    "FIELD",
    "STRUCTURE",
    "LINKAGE",
    "LABEL",
    "BUILTIN",
    "UNDECLARE",
    "PSECT",
    "SWITCHES",
];
