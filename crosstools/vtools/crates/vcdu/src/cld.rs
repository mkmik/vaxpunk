//! The Command Definition Language: reads `.CLD` text into definitions.

use std::fmt;

/// What a set of `.CLD` files defines.
#[derive(Debug, Default)]
pub struct Definitions {
    /// `MODULE name`: the global symbol the table is at.
    pub module: Option<String>,
    /// `IDENT "string"`.
    pub ident: Option<String>,
    pub verbs: Vec<Command>,
    pub syntaxes: Vec<Command>,
    pub types: Vec<Type>,
}

/// `DEFINE VERB` or `DEFINE SYNTAX`.
#[derive(Clone, Debug, Default)]
pub struct Command {
    pub name: String,
    pub image: Option<String>,
    pub routine: Option<String>,
    /// `CLIROUTINE name`: a verb the command interpreter does itself.
    pub cliroutine: Option<String>,
    pub synonyms: Vec<String>,
    pub params: Vec<Entity>,
    pub quals: Vec<Entity>,
    pub disallows: Vec<Expr>,
    /// `NOPARAMETERS`, `NOQUALIFIERS`, `NODISALLOWS`: a syntax inherits
    /// none of its verb's.
    pub noparams: bool,
    pub noquals: bool,
    pub nodisallows: bool,
    /// Where it was defined, for messages.
    pub loc: Loc,
}

/// `DEFINE TYPE`: keywords.
#[derive(Debug, Default)]
pub struct Type {
    pub name: String,
    pub keywords: Vec<Entity>,
    pub loc: Loc,
}

/// A parameter, a qualifier or a keyword.
#[derive(Clone, Debug, Default)]
pub struct Entity {
    pub name: String,
    pub label: Option<String>,
    pub prompt: Option<String>,
    /// `DEFAULT`: present unless negated.
    pub default: bool,
    /// `NEGATABLE`, `NONNEGATABLE`, or neither said.
    pub negatable: Option<bool>,
    pub batch: bool,
    pub placement: Placement,
    pub syntax: Option<String>,
    pub value: Option<Value>,
    pub loc: Loc,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Placement {
    #[default]
    Global,
    Local,
    Positional,
}

/// `VALUE (clauses)`.
#[derive(Clone, Debug, Default)]
pub struct Value {
    pub required: bool,
    pub list: bool,
    /// `CONCATENATE`, `NOCONCATENATE`, or neither said.
    pub concatenate: Option<bool>,
    pub default: Option<String>,
    /// `TYPE=name`: `$FILE` and the other built-in types, or a `DEFINE TYPE`.
    pub type_: Option<String>,
}

/// A `DISALLOW` expression.
#[derive(Clone, Debug)]
pub enum Expr {
    /// An entity: `P1`, `LOG`, `SELECT.OWNER`, or a label.
    Name(String),
    /// `NEG name`: the entity negated.
    Neg(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// `ANY2 (a, b, ...)`: two or more of them.
    Any2(Vec<Expr>),
}

/// Where a definition is: the index of its file among those compiled
/// together, and its line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Loc {
    pub file: usize,
    pub line: usize,
}

/// An error in the source: its line and what is wrong.
#[derive(Debug, PartialEq)]
pub struct Error {
    pub line: usize,
    pub msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.line, self.msg)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// A name or keyword, in capitals.
    Word(String),
    /// A quoted string, without its quotes.
    Str(String),
    Punct(char),
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    file: usize,
}

/// Reads the text of `.CLD` file number `file` into `defs`.
pub fn parse(text: &str, file: usize, defs: &mut Definitions) -> Result<(), Error> {
    let mut p = Parser {
        toks: lex(text)?,
        pos: 0,
        file,
    };
    while let Some(word) = p.next_word_opt() {
        match word.as_str() {
            "MODULE" => defs.module = Some(p.name()?),
            "IDENT" => defs.ident = Some(p.string_or_name()?),
            "DEFINE" => {
                let what = p.word()?;
                let loc = p.loc();
                let name = p.name()?;
                match what.as_str() {
                    "VERB" => defs.verbs.push(p.command(name, loc)?),
                    "SYNTAX" => defs.syntaxes.push(p.command(name, loc)?),
                    "TYPE" => defs.types.push(p.type_(name, loc)?),
                    _ => return Err(p.error(format!("DEFINE {what}: not VERB, SYNTAX or TYPE"))),
                }
            }
            _ => return Err(p.error_at(p.pos - 1, format!("{word}: not a statement"))),
        }
    }
    Ok(())
}

fn lex(text: &str) -> Result<Vec<(Tok, usize)>, Error> {
    let mut toks = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line_no = n + 1;
        let mut chars = line.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c == '!' {
                break;
            } else if c.is_whitespace() {
                chars.next();
            } else if c == '"' {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                            s.push('"');
                        }
                        Some('"') => break,
                        Some(c) => s.push(c),
                        None => {
                            return Err(Error {
                                line: line_no,
                                msg: "a string has no closing quote".into(),
                            });
                        }
                    }
                }
                toks.push((Tok::Str(s), line_no));
            } else if is_name_char(c) {
                let mut s = String::new();
                while let Some(&c) = chars.peek().filter(|c| is_name_char(**c)) {
                    s.push(c.to_ascii_uppercase());
                    chars.next();
                }
                if s.len() > 31 && !s.contains('.') {
                    return Err(Error {
                        line: line_no,
                        msg: format!("{s}: longer than 31 characters"),
                    });
                }
                toks.push((Tok::Word(s), line_no));
            } else if "(),=".contains(c) {
                chars.next();
                toks.push((Tok::Punct(c), line_no));
            } else {
                return Err(Error {
                    line: line_no,
                    msg: format!("{c}: not a CLD character"),
                });
            }
        }
    }
    Ok(toks)
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '$' || c == '_' || c == '.'
}

/// The words that start a clause of a verb or a syntax, which end the one
/// before.
const COMMAND_CLAUSES: &[&str] = &[
    "IMAGE",
    "ROUTINE",
    "CLIROUTINE",
    "SYNONYM",
    "PARAMETER",
    "QUALIFIER",
    "DISALLOW",
    "NODISALLOWS",
    "NOPARAMETERS",
    "NOQUALIFIERS",
    "CLIFLAGS",
];

impl Parser {
    fn loc(&self) -> Loc {
        Loc {
            file: self.file,
            line: self.line(),
        }
    }

    fn line(&self) -> usize {
        self.toks
            .get(self.pos)
            .or(self.toks.last())
            .map_or(0, |t| t.1)
    }

    fn error(&self, msg: String) -> Error {
        Error {
            line: self.line(),
            msg,
        }
    }

    fn error_at(&self, pos: usize, msg: String) -> Error {
        Error {
            line: self.toks.get(pos).map_or(0, |t| t.1),
            msg,
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.0)
    }

    fn peek_word(&self) -> Option<&str> {
        match self.peek() {
            Some(Tok::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn next_word_opt(&mut self) -> Option<String> {
        let w = self.peek_word()?.to_string();
        self.pos += 1;
        Some(w)
    }

    fn word(&mut self) -> Result<String, Error> {
        self.next_word_opt()
            .ok_or_else(|| self.error("a keyword is missing".into()))
    }

    fn name(&mut self) -> Result<String, Error> {
        self.next_word_opt()
            .ok_or_else(|| self.error("a name is missing".into()))
    }

    fn string_or_name(&mut self) -> Result<String, Error> {
        match self.peek().cloned() {
            Some(Tok::Str(s) | Tok::Word(s)) => {
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.error("a string or a name is missing".into())),
        }
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(&Tok::Punct(c)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, c: char) -> Result<(), Error> {
        if self.eat(c) {
            Ok(())
        } else {
            Err(self.error(format!("{c} is missing")))
        }
    }

    /// `[=] x`, after a clause that takes a value.
    fn equals(&mut self) -> Result<String, Error> {
        self.eat('=');
        self.string_or_name()
    }

    fn command(&mut self, name: String, loc: Loc) -> Result<Command, Error> {
        let mut cmd = Command {
            name,
            loc,
            ..Default::default()
        };
        // Clauses are separated by commas or just follow each other.
        loop {
            self.eat(',');
            let Some(word) = self.peek_word().filter(|w| COMMAND_CLAUSES.contains(w)) else {
                break;
            };
            let word = word.to_string();
            self.pos += 1;
            match word.as_str() {
                "IMAGE" => cmd.image = Some(self.string_or_name()?),
                "ROUTINE" => cmd.routine = Some(self.name()?),
                "CLIROUTINE" => cmd.cliroutine = Some(self.name()?),
                "SYNONYM" => cmd.synonyms.push(self.name()?),
                "PARAMETER" => {
                    let loc = self.loc();
                    let name = self.name()?;
                    cmd.params.push(self.entity(name, loc, false)?);
                }
                "QUALIFIER" => {
                    let loc = self.loc();
                    let name = self.name()?;
                    cmd.quals.push(self.entity(name, loc, true)?);
                }
                "DISALLOW" => cmd.disallows.push(self.expr()?),
                "NODISALLOWS" => cmd.nodisallows = true,
                "NOPARAMETERS" => cmd.noparams = true,
                "NOQUALIFIERS" => cmd.noquals = true,
                _ => return Err(self.error_at(self.pos - 1, format!("{word}: not supported yet"))),
            }
        }
        Ok(cmd)
    }

    fn type_(&mut self, name: String, loc: Loc) -> Result<Type, Error> {
        let mut t = Type {
            name,
            loc,
            ..Default::default()
        };
        loop {
            self.eat(',');
            if self.peek_word() != Some("KEYWORD") {
                break;
            }
            self.pos += 1;
            let loc = self.loc();
            let name = self.name()?;
            t.keywords.push(self.entity(name, loc, true)?);
        }
        Ok(t)
    }

    /// The clauses after a parameter's, a qualifier's or a keyword's
    /// name, each after a comma.
    fn entity(&mut self, name: String, loc: Loc, negatable: bool) -> Result<Entity, Error> {
        let mut e = Entity {
            name,
            loc,
            ..Default::default()
        };
        while self.peek() == Some(&Tok::Punct(',')) {
            let Some(Tok::Word(word)) = self.toks.get(self.pos + 1).map(|t| t.0.clone()) else {
                break;
            };
            if COMMAND_CLAUSES.contains(&word.as_str()) || word == "KEYWORD" {
                break;
            }
            self.pos += 2;
            match word.as_str() {
                "DEFAULT" => e.default = true,
                "LABEL" => e.label = Some(self.equals()?.to_ascii_uppercase()),
                "PROMPT" => e.prompt = Some(self.equals()?),
                "VALUE" => e.value = Some(self.value()?),
                "SYNTAX" => e.syntax = Some(self.equals()?.to_ascii_uppercase()),
                "NEGATABLE" if negatable => e.negatable = Some(true),
                "NONNEGATABLE" if negatable => e.negatable = Some(false),
                "BATCH" if negatable => e.batch = true,
                "PLACEMENT" if negatable => {
                    e.placement = match self.equals()?.to_ascii_uppercase().as_str() {
                        "GLOBAL" => Placement::Global,
                        "LOCAL" => Placement::Local,
                        "POSITIONAL" => Placement::Positional,
                        p => return Err(self.error(format!("PLACEMENT={p}: not a placement"))),
                    }
                }
                _ => return Err(self.error_at(self.pos - 1, format!("{word}: not a clause here"))),
            }
        }
        Ok(e)
    }

    fn value(&mut self) -> Result<Value, Error> {
        let mut v = Value::default();
        if !self.eat('(') {
            return Ok(v);
        }
        loop {
            match self.word()?.as_str() {
                "REQUIRED" => v.required = true,
                "LIST" => v.list = true,
                "CONCATENATE" => v.concatenate = Some(true),
                "NOCONCATENATE" => v.concatenate = Some(false),
                "DEFAULT" => v.default = Some(self.equals()?),
                "TYPE" => v.type_ = Some(self.equals()?.to_ascii_uppercase()),
                w => return Err(self.error_at(self.pos - 1, format!("{w}: not a VALUE clause"))),
            }
            if !self.eat(',') {
                break;
            }
        }
        self.expect(')')?;
        Ok(v)
    }

    /// A `DISALLOW` expression: `OR` binds least, then `AND`, then `NOT`.
    fn expr(&mut self) -> Result<Expr, Error> {
        let mut left = self.and_expr()?;
        while self.peek_word() == Some("OR") {
            self.pos += 1;
            left = Expr::Or(Box::new(left), Box::new(self.and_expr()?));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        while self.peek_word() == Some("AND") {
            self.pos += 1;
            left = Expr::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.eat('(') {
            let e = self.expr()?;
            self.expect(')')?;
            return Ok(e);
        }
        let word = self.name()?;
        Ok(match word.as_str() {
            "NOT" => Expr::Not(Box::new(self.unary()?)),
            "NEG" => Expr::Neg(self.name()?),
            "ANY2" => {
                self.expect('(')?;
                let mut list = vec![self.expr()?];
                while self.eat(',') {
                    list.push(self.expr()?);
                }
                self.expect(')')?;
                Expr::Any2(list)
            }
            _ => Expr::Name(word),
        })
    }
}
