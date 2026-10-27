//! The parser: lexemes into routines and data, with names resolved as they
//! are declared, since BLISS declares everything before it is used.

use std::collections::HashMap;
use std::rc::Rc;

use crate::data::{Init, StructAttr, Structure};
use crate::lex::{Lexeme, Tok};
use crate::lexical::{Item, Lx, Macro, MacroKind};
use crate::linkage::Linkage;
use crate::listing::{Diag, NOPOS};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Storage {
    Own,
    Global,
    External,
    /// In the routine's frame slot n.
    Local(u32),
    /// BIND: the address is the module's bind expression n.
    Bind(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// A data segment: where it is, how many bytes it takes, and the field
    /// a fetch or a store of its name uses (size in bytes, signed).
    Data {
        storage: Storage,
        bytes: u32,
        size: u8,
        signed: bool,
        /// Its structure attribute, if it has one.
        structure: Option<Rc<StructAttr>>,
    },
    /// A routine; `defined` once its body is seen, `global` if exported.
    Routine {
        global: bool,
        external: bool,
        novalue: bool,
        linkage: Option<Rc<Linkage>>,
    },
    Linkage(Rc<Linkage>),
    Literal(i64),
    /// A COMPILETIME name and its value now.
    Compiletime(i64),
    Macro(Rc<Macro>),
    Label,
    Structure(Rc<Structure>),
    /// A structure's formal, while its body is parsed.
    StructFormal,
    /// A FIELD name: the access actuals it stands for.
    Field(Vec<i64>),
    /// A FIELD set: its field names.
    FieldSet(Vec<usize>),
    /// A built-in its BUILTIN declaration names.
    Builtin,
    /// An EXTERNAL LITERAL: a value the linker gives, its symbol's.
    ExternalLiteral,
    /// The STRUCTURE switch's default structure attribute, under the name
    /// `%STRUCTURE`, which no source can declare.
    DefaultStructure(Option<Rc<StructAttr>>),
}

#[derive(Clone, Debug)]
pub struct Sym {
    pub name: String,
    /// The name in the assembly: the BLISS name, made unique for OWN.
    pub asm: String,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shift,
    And,
    Or,
    Xor,
    Eqv,
    /// A comparison: its relation, and `unsigned` for the U and A forms.
    Rel(Rel, bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rel {
    Eql,
    Neq,
    Lss,
    Leq,
    Gtr,
    Geq,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseLabel {
    Range(i64, i64),
    Inrange,
    Outrange,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectLabel {
    Range(Expr, Option<Expr>),
    Otherwise,
    Always,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(i64),
    /// A data segment's or a routine's name: its address.
    Name(usize),
    /// `%ASCID 'text'`: the address of a descriptor of the text.
    Ascid(Vec<u8>),
    Fetch(Box<Expr>),
    /// `base<pos, size, ext>`.
    Field(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
    /// The value of temporary n, set by a `Let`.
    Temp(u32),
    /// Temporary n set to a value for an expression: a structure
    /// reference's access actual, evaluated once.
    Let(u32, Box<Expr>, Box<Expr>),
    /// `PLIT` (counted) or `UPLIT`, numbered: the address of its items,
    /// allocated once however often a BIND name or a structure's copy
    /// repeats it.
    Plit(u32, bool, Vec<Init>),
    /// A built-in that is an IR instruction of its own.
    Special(crate::builtin::Special, Vec<Expr>),
    /// An IR operation the operators don't have: SLL, ROT and the like.
    Op(crate::ir::Op, Box<Expr>, Box<Expr>),
    /// The address of the module's ENABLE handler jacket.
    Jacket,
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BOp, Box<Expr>, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    /// A call: the routine, its standard actuals, what goes through
    /// registers, as its linkage says (`linkage.rs`), and the linkage
    /// itself where the routine is an address, not a routine's name: a
    /// general routine call's, or the LINKAGE switch's.
    Call(Box<Expr>, Vec<Expr>, Vec<Link>, Option<Rc<Linkage>>),
    /// A block's expressions; `value` unless it ends with a semicolon.
    Block(Vec<Expr>, bool),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// WHILE/UNTIL c DO b (`post` false), DO b WHILE/UNTIL c (`post`).
    Loop {
        until: bool,
        post: bool,
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    /// INCR and DECR, `unsigned` for the U and A forms.
    Incr {
        var: usize,
        down: bool,
        unsigned: bool,
        from: Option<Box<Expr>>,
        to: Option<Box<Expr>>,
        by: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Case {
        sel: Box<Expr>,
        lo: i64,
        hi: i64,
        arms: Vec<(Vec<CaseLabel>, Expr)>,
    },
    Select {
        sel: Box<Expr>,
        one: bool,
        unsigned: bool,
        arms: Vec<(Vec<SelectLabel>, Expr)>,
    },
    Labeled(usize, Box<Expr>),
    Leave(usize, Option<Box<Expr>>),
    Exitloop(Option<Box<Expr>>),
    Return(Option<Box<Expr>>),
}

/// VAX registers and the data that goes in each.
pub type Registers = Vec<(u8, usize)>;

/// A register a call passes a value in, takes one back from, or both: a
/// register parameter, an output parameter or a global register segment.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub reg: u8,
    pub input: Option<Expr>,
    /// The place the register's value is stored in after the call.
    pub output: Option<Expr>,
}

pub struct Routine {
    pub sym: usize,
    pub formals: Vec<usize>,
    /// EXTERNAL REGISTER data, read from its register at entry, and
    /// output formals; both left in their registers at return.
    pub externals: Registers,
    pub outputs: Registers,
    pub body: Expr,
    /// The sizes of its frame slots.
    pub slots: Vec<u32>,
    /// The psect its code goes in.
    pub psect: String,
}

/// Static data: an OWN or GLOBAL, what it starts as, and its psect.
pub struct Static {
    pub sym: usize,
    pub init: Vec<Init>,
    pub psect: String,
    pub align: u8,
}

/// The psects the storage classes go in, as PSECT declarations set them.
#[derive(Clone, Debug)]
pub struct Psects {
    pub own: String,
    pub global: String,
    pub plit: String,
    pub code: String,
}

impl Default for Psects {
    fn default() -> Self {
        Psects {
            own: "$OWN$".into(),
            global: "$GLOBAL$".into(),
            plit: "$PLIT$".into(),
            code: "$CODE$".into(),
        }
    }
}

#[derive(Default)]
pub struct Module {
    pub name: String,
    pub ident: Option<String>,
    pub main: Option<String>,
    pub syms: Vec<Sym>,
    pub routines: Vec<Routine>,
    pub statics: Vec<Static>,
    /// The addresses BIND names stand for.
    pub binds: Vec<Expr>,
    /// Whether a routine ENABLEs a handler, which the jacket calls.
    pub jacket: bool,
    pub dialect: Dialect,
    /// The psects PSECT declarations name, and their attributes in vasm's
    /// words; and where the storage classes go now.
    pub psect_attrs: Vec<(String, String)>,
    pub psects: Psects,
    /// The psects PLITs with a PSECT attribute go in, by number.
    pub plit_psects: HashMap<u32, String>,
    /// GLOBAL LITERALs and GLOBAL BINDs: names and values, link-time
    /// constants.
    pub global_literals: Vec<(String, Expr)>,
    /// NOCOUNT, the module switch or /NOCOUNT: CALL linkages that say
    /// neither COUNT nor NOCOUNT pass no argument count (RN 2.8.5).
    pub nocount: bool,
    /// CODECOMMENTs' texts, by number.
    pub comments: Vec<String>,
    /// NOCODE: the module is checked, and no code is made of it.
    pub nocode: bool,
    /// ENVIRONMENT(NOFP): no floating point (RN 2.11).
    pub nofp: bool,
}

/// A fatal error: the file, line and column (from 0) it is at.
#[derive(Debug)]
pub struct Error {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub msg: String,
}

pub type R<T> = Result<T, Error>;

/// Reads a require file by the name REQUIRE gives.
pub type Loader<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// What the front end makes of a source: the module or the error that
/// stopped it, the diagnostics, the listing and what %MESSAGE wrote.
pub struct Front {
    pub module: R<Module>,
    pub diags: Vec<(String, Diag)>,
    /// The dot lint's warnings.
    pub lints: Vec<(String, Diag)>,
    /// What the listing shows, but the machine code, which isn't made yet.
    pub listing: Listing,
    pub messages: Vec<String>,
}

/// The listing's makings: its entries, the files they come from by
/// number, the diagnostics by file number, and the libraries' statistics
/// (`listing::Layout`).
#[derive(Default)]
pub struct Listing {
    pub entries: Vec<crate::listing::Entry>,
    pub files: Vec<String>,
    pub diags: Vec<Diag>,
    pub module: String,
    pub libraries: Vec<(String, usize, usize, usize)>,
}

/// A qualifier's values: `(A, B=1)` or `A`, upper case.
fn values(v: Option<&str>) -> Vec<String> {
    v.unwrap_or("")
        .trim_matches(['(', ')'])
        .split(',')
        .map(|a| a.trim().to_ascii_uppercase())
        .filter(|a| !a.is_empty())
        .collect()
}

impl Options {
    fn set(&mut self, switch: &str, on: bool) {
        self.switches.retain(|(s, _)| s != switch);
        self.switches.push((switch.into(), on));
    }

    /// Applies one of the BLISS verb's qualifiers, but /OBJECT, /LIST and
    /// /INCLUDE, which name files; false if it isn't one.
    pub fn qualifier(&mut self, q: &str) -> Result<bool, String> {
        let (name, value) = match q.split_once(['=', ':']) {
            Some((n, v)) => (n, Some(v)),
            None => (q, None),
        };
        let name = name.to_ascii_uppercase();
        // A qualifier may be shortened to four characters, as DCL allows.
        let is = |full: &str| name.len() >= full.len().min(5) && full.starts_with(&name[..]);
        let bad = |v: &str| Err(format!("bad value {v} for {name}"));
        match name.as_str() {
            "/A32" => self.dialect.a32 = true,
            "/A64" => self.dialect.a32 = false,
            "/COUNT" => self.nocount = false,
            "/NOCOUNT" => self.nocount = true,
            _ if is("/SOURCE_LIST") => {
                for v in values(value) {
                    let (base, on) = match v.strip_prefix("NO") {
                        Some(b) => (b, false),
                        None => (v.as_str(), true),
                    };
                    // A keyword shortened, as DCL takes it.
                    let words = [
                        "SOURCE",
                        "REQUIRE",
                        "EXPAND_MACROS",
                        "TRACE_MACROS",
                        "LIBRARY",
                    ];
                    let base = words
                        .iter()
                        .find(|w| base.len() >= 4.min(w.len()) && w.starts_with(base))
                        .copied()
                        .unwrap_or(base);
                    let switch = match base {
                        "SOURCE" => "LIST_SOURCE",
                        "REQUIRE" => "LIST_REQUIRE",
                        "EXPAND_MACROS" => "LIST_EXPAND",
                        "TRACE_MACROS" => "LIST_TRACE",
                        "LIBRARY" => "LIST_LIBRARY",
                        _ => match v.split_once(['=', ':']) {
                            Some(("PAGE_SIZE", n)) => {
                                match n.parse() {
                                    Ok(n @ 20..) => self.page_size = Some(n),
                                    _ => return bad(&v),
                                }
                                continue;
                            }
                            _ => return bad(&v),
                        },
                    };
                    self.set(switch, on);
                }
            }
            _ if is("/MACHINE_CODE") => self.machine_code = true,
            _ if is("/NOMACHINE_CODE") => self.machine_code = false,
            _ if is("/SYNTAX_LEVEL") => {
                self.syntax_level = match value.map(str::parse) {
                    Some(Ok(n @ 1..=3)) => Some(n),
                    _ => return bad(value.unwrap_or("")),
                }
            }
            _ if is("/NAMES") => match values(value).as_slice() {
                [v] if v == "UPPERCASE" => self.lowercase = false,
                [v] if v == "LOWERCASE" => self.lowercase = true,
                _ => return bad(value.unwrap_or("")),
            },
            _ if is("/TERMINAL") => {
                for v in values(value) {
                    match v.as_str() {
                        "ERRORS" => self.set("ERRS", true),
                        "NOERRORS" => self.set("ERRS", false),
                        // BLISS-32's, gone from BLISS-64 (RN 2.19.15).
                        _ => return bad(&v),
                    }
                }
            }
            _ if is("/CODE") => self.set("CODE", true),
            _ if is("/NOCODE") => self.set("CODE", false),
            _ if is("/DEBUG") => self.set("DEBUG", true),
            _ if is("/NODEBUG") => self.set("DEBUG", false),
            _ if is("/TRACEBACK") => self.set("TRACEBACK", true),
            _ if is("/NOTRACEBACK") => self.set("TRACEBACK", false),
            _ if is("/CHECK") => {
                for v in values(value) {
                    match v.as_str() {
                        "ALL" | "NONE" => {
                            for (s, _) in SWITCH_DEFAULTS
                                .iter()
                                .filter(|(s, _)| s.starts_with("CHECK_"))
                            {
                                self.set(s, v == "ALL");
                            }
                        }
                        _ => {
                            let (base, on) = match v.strip_prefix("NO") {
                                Some(b) => (b, false),
                                None => (v.as_str(), true),
                            };
                            let switch = format!("CHECK_{base}");
                            if !SWITCH_DEFAULTS.iter().any(|(s, _)| *s == switch) {
                                return bad(&v);
                            }
                            self.set(&switch, on);
                        }
                    }
                }
            }
            _ if is("/LANGUAGE") => {
                self.language = values(value);
                for l in &self.language {
                    if !LANGUAGES.contains(&l.as_str()) {
                        return bad(l);
                    }
                }
            }
            _ if is("/LIBRARY") => self.library = true,
            _ if is("/CROSS_REFERENCE") => self
                .notes
                .push("CROSS_REFERENCE not implemented in BLISS-64E".into()),
            _ if is("/ERROR_LIMIT") => {
                self.error_limit = match value.map(str::parse) {
                    Some(Ok(n)) => Some(n),
                    None => Some(30),
                    _ => return bad(value.unwrap_or("")),
                }
            }
            _ if is("/NOERROR_LIMIT") => self.error_limit = Some(u32::MAX),
            // Accepted, and nothing to do: /DIAGNOSTICS and /ANALYSIS_DATA,
            // until LSE and SCA read them; /INITIAL_PSECT, as LOCAL initial
            // values are code; and Alpha's code generation.
            _ if [
                "/DIAGNOSTICS",
                "/NODIAGNOSTICS",
                "/ANALYSIS_DATA",
                "/NOANALYSIS_DATA",
                "/INITIAL_PSECT",
                "/NOINITIAL_PSECT",
                "/OPTIMIZE",
                "/NOOPTIMIZE",
                "/ARCHITECTURE",
                "/GRANULARITY",
                "/TIE",
                "/NOTIE",
                "/ALPHA_REGISTER_MAPPING",
                "/NOALPHA_REGISTER_MAPPING",
                "/ANNOTATIONS",
                "/NOANNOTATIONS",
            ]
            .iter()
            .any(|q| is(q)) => {}
            _ if is("/ENVIRONMENT") => {
                for v in values(value) {
                    match v.as_str() {
                        "FP" | "NOFP" => self.set("NOFP", v == "NOFP"),
                        _ => return bad(&v),
                    }
                }
            }
            "/VARIANT" => {
                self.variant = match value {
                    Some(n) => n.parse().map_err(|_| format!("bad /VARIANT value {n}"))?,
                    None => 1,
                }
            }
            "/ASSUME" => {
                for a in value.unwrap_or("").trim_matches(['(', ')']).split(',') {
                    let a = a.trim().to_ascii_uppercase();
                    let on = !a.starts_with("NO");
                    match a.trim_start_matches("NO") {
                        "LONG_DEFAULT" => self.dialect.long_default = on,
                        "REF_LONG" => self.dialect.ref_long = on,
                        "SIGNED_LONG" => self.dialect.signed_long = on,
                        "QUAD_LITERALS" => self.set("QUAD_LITERALS", on),
                        _ => {}
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The qualifiers on a test source's first line, `! BLISS: /A32` and
    /// the like, which the oracle compiles it with too.
    pub fn from_source(text: &str) -> Options {
        Options::default().with_source(text)
    }

    /// These options, then the qualifiers on a test source's first line.
    pub fn with_source(self, text: &str) -> Options {
        let mut opts = self;
        if let Some(rest) = text.lines().next().and_then(|l| {
            l.get(..8)
                .filter(|p| p.eq_ignore_ascii_case("! BLISS:"))
                .map(|_| &l[8..])
        }) {
            for q in rest.split('/').filter(|q| !q.trim().is_empty()) {
                let _ = opts.qualifier(&format!("/{}", q.trim()));
            }
        }
        opts
    }
}

/// The dialect: BLISS-32 (`/A32`), and BLISS-64's switches for moving
/// BLISS-32 code (`docs/bliss64.md`, *Values and sizes*).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dialect {
    pub a32: bool,
    pub long_default: bool,
    pub ref_long: bool,
    pub signed_long: bool,
}

impl Dialect {
    /// The default allocation unit in bytes: of a scalar, a structure's
    /// element, a PLIT item, a PLIT's count.
    pub fn unit(&self) -> u8 {
        if self.a32 || self.long_default || self.signed_long {
            4
        } else {
            8
        }
    }

    /// The fullword in bytes.
    pub fn fullword(&self) -> u8 {
        if self.a32 { 4 } else { 8 }
    }
}

thread_local! {
    /// Whether values are 32 bits, for the constant arithmetic: one
    /// compilation per thread at a time.
    static WIDE32: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A value as the dialect's fullword holds it: sign-extended from 32 bits
/// under BLISS-32.
pub fn wrap(v: i64) -> i64 {
    if WIDE32.get() { v as i32 as i64 } else { v }
}

/// How to compile: the BLISS verb's qualifiers (docs/bliss64.md,
/// *Switches and qualifiers*).
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub dialect: Dialect,
    pub variant: i64,
    /// /NOCOUNT.
    pub nocount: bool,
    /// Where REQUIRE and LIBRARY look for files after the source's own
    /// directory.
    pub include: Vec<std::path::PathBuf>,
    /// The switch settings the qualifiers make the defaults, by the
    /// switches' names (UM table 1-1): ("LIST_EXPAND", true) for
    /// /SOURCE_LIST=EXPAND_MACROS, ("CODE", false) for /NOCODE.
    pub switches: Vec<(String, bool)>,
    /// /SOURCE_LIST=PAGE_SIZE: a listing page's lines, 60 if not given.
    pub page_size: Option<u32>,
    /// /MACHINE_CODE: the assembly after the source in the listing.
    pub machine_code: bool,
    /// /SYNTAX_LEVEL, 2 if not given.
    pub syntax_level: Option<u8>,
    /// /NAMES=LOWERCASE: external names in lower case.
    pub lowercase: bool,
    /// /LIBRARY: the source is a library's, which makes no object.
    pub library: bool,
    /// /ERROR_LIMIT: the errors after which the compiler gives up, 30 if
    /// not given; /NOERROR_LIMIT is no limit.
    pub error_limit: Option<u32>,
    /// /LANGUAGE: the dialects whose subset the source is checked against.
    pub language: Vec<String>,
    /// Informational messages the qualifiers cause.
    pub notes: Vec<String>,
    /// The command line, which the listing ends with.
    pub command: String,
    /// The listing's date, `DD-MMM-YYYY HH:MM:SS`; now if not given.
    pub date: Option<String>,
}

pub struct Parser<'a> {
    pub(crate) load: Loader<'a>,
    pub(crate) opts: Options,
    /// The lexemes the parser has read, and the next one's index: read
    /// one at a time from the lexical processor, as BLISS reads them.
    pub(crate) toks: Vec<Lexeme>,
    pub(crate) pos: usize,
    pub(crate) lx: Lx,
    /// Nonzero while parsing a lexical function's parameter, whose lexemes
    /// are all in `toks`.
    sub: u32,
    /// An error of the lexical processor, reported in place of the parse
    /// error it causes.
    stash: Option<Error>,
    /// The block depth, for the listing.
    pub(crate) depth: u32,
    pub(crate) diags: Vec<Diag>,
    pub(crate) scopes: Vec<HashMap<String, usize>>,
    pub(crate) m: Module,
    /// Frame slots of the routine being parsed.
    slots: Vec<u32>,
    /// Names the assembly already uses.
    asm_names: HashMap<String, u32>,
    /// Temporaries for structure references' actuals.
    pub(crate) lets: u32,
    /// What the block being parsed does first: run-time BINDs and LOCAL
    /// initial values.
    pub(crate) inits: Vec<Expr>,
    /// Routines being parsed: nonzero where a frame exists.
    routines: u32,
    /// Data declared VOLATILE, and ALIAS.
    pub(crate) volatile: Vec<usize>,
    pub(crate) alias: Vec<usize>,
    /// For the checks: the variables CHECK_ADDRESS_TAKEN has told of;
    /// where each name was last read; each routine's formals as its
    /// declaration or prototype gives them; how far each psect's static
    /// data has come; the GLOBAL and EXTERNAL names so far.
    pub(crate) taken: Vec<usize>,
    pub(crate) name_at: HashMap<usize, Lexeme>,
    pub(crate) formals: HashMap<usize, Vec<crate::checks::Formal>>,
    pub(crate) static_offsets: HashMap<String, u32>,
    pub(crate) global_names: Vec<String>,
    /// Where the parser went on after an error.
    recovered: Vec<usize>,
    /// The diagnostics, by index, that a routine with an error doesn't
    /// keep: those BLISSA64 makes once it has parsed it.
    pub(crate) late: Vec<usize>,
    /// Where the last field selector and the last call's actuals began.
    pub(crate) field_at: Option<Lexeme>,
    pub(crate) call_at: Option<Lexeme>,
    /// The dot lint's warnings, and how deep in an address it is.
    pub(crate) lints: Vec<Diag>,
    /// Names used without a declaration.
    pub(crate) undeclared: Vec<usize>,
    pub(crate) address_context: u32,
    /// The system services' and library routines' mechanisms, once read.
    pub(crate) services: Option<HashMap<String, Vec<crate::lint::Mechanism>>>,
    /// Rule 6 of the dot lint: how data is used, and the calls to check.
    pub(crate) uses: HashMap<usize, crate::lint::Use>,
    pub(crate) calls: Vec<(usize, Vec<crate::lint::Passed>, Lexeme)>,
    pub(crate) stores: Vec<(usize, String, Lexeme)>,
    /// What a MAP's name stands for: the data it maps.
    pub(crate) mapped: HashMap<usize, usize>,
    /// Whether the expression about to be parsed is a call's actual.
    whole_actual: bool,
    /// The module's STRUCTURE switch, read once the structures are.
    pending_structure: Option<Vec<Lexeme>>,
    /// The EXTERNAL_NAME and PSECT the last routine attributes gave.
    routine_name: Option<String>,
    routine_psect: Option<String>,
    /// VARIABLE, and the value's unit and sign (RN 2.29), the last routine
    /// attributes gave.
    routine_variable: bool,
    pub(crate) routine_value: Option<(u8, bool)>,
    /// GLOBAL and EXTERNAL REGISTER data: their registers.
    pub(crate) reg_data: HashMap<usize, u8>,
    /// The routines being parsed: their linkages, and their EXTERNAL
    /// REGISTER data and output formals, as `Routine` has them.
    pub(crate) exits: Vec<(Registers, Registers)>,
    linkages_now: Vec<Option<Rc<Linkage>>>,
    /// The routines being parsed, by number, and the routine each LOCAL,
    /// REGISTER and formal belongs to: another routine can't reach them.
    routine_stack: Vec<u32>,
    routine_count: u32,
    pub(crate) local_owner: HashMap<usize, u32>,
    /// OWN and GLOBAL names declared ahead of their declaration: by
    /// FORWARD, or for their own INITIAL (LRM 10.3).
    pub(crate) forward_data: Vec<usize>,
    /// Routines' input formals, where a declaration or prototype says how
    /// many, and whether they are VARIABLE: for the count check.
    pub(crate) formal_counts: HashMap<usize, (usize, bool)>,
    /// The libraries read, and the names each gives; how deep in a
    /// library's declarations the parser is, and the names that
    /// library's own libraries gave it.
    libraries: HashMap<String, Vec<(String, usize)>>,
    pub(crate) library_depth: u32,
    library_imports: Vec<usize>,
    /// The libraries read: each one's name, how many names it gives and
    /// its size; the library each of those names is from; and the names
    /// in the order first looked up, which LIBRARY lists.
    library_stats: Vec<(String, usize, usize)>,
    library_of: HashMap<usize, usize>,
    loaded: std::cell::RefCell<Vec<usize>>,
    loaded_shown: usize,
    /// The routines being parsed, by name, for %ROUTINE.
    pub(crate) routine_names: Vec<String>,
}

/// Parses the module in `text`, from file `name`.
pub fn parse(name: &str, text: &str, load: Loader, opts: &Options) -> Front {
    let lx = match Lx::new(name, text) {
        Ok(lx) => lx,
        Err(e) => {
            return Front {
                module: Err(e),
                diags: Vec::new(),
                lints: Vec::new(),
                listing: Listing::default(),
                messages: Vec::new(),
            };
        }
    };
    let mut p = Parser {
        load,
        opts: opts.clone(),
        toks: Vec::new(),
        pos: 0,
        lx,
        sub: 0,
        stash: None,
        depth: 0,
        diags: Vec::new(),
        scopes: vec![HashMap::new()],
        m: Module::default(),
        slots: Vec::new(),
        asm_names: HashMap::new(),
        lets: 0,
        inits: Vec::new(),
        routines: 0,
        volatile: Vec::new(),
        alias: Vec::new(),
        taken: Vec::new(),
        name_at: HashMap::new(),
        formals: HashMap::new(),
        static_offsets: HashMap::new(),
        global_names: Vec::new(),
        recovered: Vec::new(),
        late: Vec::new(),
        field_at: None,
        call_at: None,
        lints: Vec::new(),
        undeclared: Vec::new(),
        address_context: 0,
        services: None,
        pending_structure: None,
        uses: HashMap::new(),
        calls: Vec::new(),
        stores: Vec::new(),
        mapped: HashMap::new(),
        whole_actual: false,
        routine_name: None,
        routine_psect: None,
        routine_variable: false,
        routine_value: None,
        reg_data: HashMap::new(),
        exits: Vec::new(),
        linkages_now: Vec::new(),
        routine_stack: Vec::new(),
        routine_count: 0,
        local_owner: HashMap::new(),
        forward_data: Vec::new(),
        formal_counts: HashMap::new(),
        libraries: HashMap::new(),
        library_depth: 0,
        library_imports: Vec::new(),
        library_stats: Vec::new(),
        library_of: HashMap::new(),
        loaded: Default::default(),
        loaded_shown: 0,
        routine_names: Vec::new(),
    };
    p.lx.variant = opts.variant;
    p.m.dialect = opts.dialect;
    p.m.nocount = opts.nocount;
    p.m.nofp = opts.switches.contains(&("NOFP".into(), true));
    for msg in &opts.notes {
        p.diags.push(Diag {
            sev: 'I',
            file: 0,
            line: 0,
            col: 0,
            msg: msg.clone(),
        });
    }
    WIDE32.set(opts.dialect.a32);
    p.predeclare();
    let mut module = if opts.library {
        p.library_module()
    } else {
        p.module()
    };
    if module.is_ok() {
        p.lint_formals();
    }
    if let (Ok(_), Some(d)) = (&module, p.diags.iter().find(|d| d.sev == 'E')) {
        module = Err(Error {
            file: p.lx.files[d.file as usize].name.clone(),
            line: d.line,
            col: d.col,
            msg: d.msg.clone(),
        });
    }
    let mut diags = p.diags.clone();
    if let Err(e) = &module
        && !diags.iter().any(|d| d.sev == 'E')
    {
        let file =
            p.lx.files
                .iter()
                .position(|f| f.name == e.file)
                .unwrap_or(0);
        diags.push(Diag {
            sev: 'E',
            file: file as u16,
            line: e.line,
            col: e.col,
            msg: e.msg.clone(),
        });
    }
    p.note_loaded();
    let loaded = p.loaded.borrow().clone();
    let listing = Listing {
        entries: std::mem::take(&mut p.lx.entries),
        files: p.lx.files.iter().map(|f| f.name.clone()).collect(),
        diags: diags.clone(),
        module: p.m.name.clone(),
        libraries: p
            .library_stats
            .iter()
            .enumerate()
            .map(|(i, (name, total, bytes))| {
                let used = loaded.iter().filter(|id| p.library_of[*id] == i).count();
                (name.clone(), *total, used, *bytes)
            })
            .collect(),
    };
    Front {
        diags: diags
            .into_iter()
            .map(|d| (p.lx.files[d.file as usize].name.clone(), d))
            .collect(),
        lints: std::mem::take(&mut p.lints)
            .into_iter()
            .map(|d| (p.lx.files[d.file as usize].name.clone(), d))
            .collect(),
        listing,
        messages: std::mem::take(&mut p.lx.messages),
        module: module.map(|()| p.m),
    }
}

fn is_name(t: &Tok, n: &str) -> bool {
    matches!(t, Tok::Name(s) if s == n)
}

/// Where the parser was, to go back to after an error.
pub(crate) struct Mark {
    pos: usize,
    scopes: usize,
    depth: u32,
    routines: usize,
    names: usize,
    exits: usize,
    linkages: usize,
    address_context: u32,
    sub: u32,
}

impl Parser<'_> {
    pub(crate) fn mark(&self) -> Mark {
        Mark {
            pos: self.pos,
            scopes: self.scopes.len(),
            depth: self.depth,
            routines: self.routine_stack.len(),
            names: self.routine_names.len(),
            exits: self.exits.len(),
            linkages: self.linkages_now.len(),
            address_context: self.address_context,
            sub: self.sub,
        }
    }

    /// After error `e` in what began at `m`, a declaration or a block's
    /// expression: reports it, then skips past the next semicolon at the
    /// nesting `m` had, or up to the close of the block around it, so
    /// that the parser goes on to find the module's other errors, as
    /// BLISS does. Gives `e` back where there's no going on: at the
    /// source's end, again at the same place, or past /ERROR_LIMIT.
    pub(crate) fn recover(&mut self, e: Error, m: &Mark) -> R<()> {
        let e = self.stash.take().unwrap_or(e);
        let file = self
            .lx
            .files
            .iter()
            .position(|f| f.name == e.file)
            .unwrap_or(0);
        self.diags.push(Diag {
            sev: 'E',
            file: file as u16,
            line: e.line,
            col: e.col,
            msg: e.msg.clone(),
        });
        let errors = self.diags.iter().filter(|d| d.sev == 'E').count();
        // The same error again, as the lexical processor's at the end of
        // the source, is no going on.
        let again = self.diags[..self.diags.len() - 1]
            .iter()
            .any(|d| d.sev == 'E' && d.line == e.line && d.col == e.col && d.msg == e.msg);
        if errors >= self.opts.error_limit.unwrap_or(30) as usize
            || self.recovered.contains(&m.pos)
            || again
        {
            return Err(e);
        }
        self.recovered.push(m.pos);
        // The Eofs that stood for lexemes after the lexical processor's
        // error go: it reads on.
        while self.toks.len() > m.pos.max(self.pos)
            && self.toks.last().is_some_and(|l| l.tok == Tok::Eof)
        {
            self.toks.pop();
        }
        self.scopes.truncate(m.scopes);
        self.depth = m.depth;
        self.routine_stack.truncate(m.routines);
        self.routine_names.truncate(m.names);
        self.exits.truncate(m.exits);
        self.linkages_now.truncate(m.linkages);
        self.address_context = m.address_context;
        self.sub = m.sub;
        self.pos = m.pos;
        let mut nest = 0;
        loop {
            match self.peek().clone() {
                Tok::Eof => return Err(e),
                Tok::Punct('(' | '[') => nest += 1,
                Tok::Name(n) if n == "BEGIN" => nest += 1,
                Tok::Punct(')' | ']') if nest == 0 => return Ok(()),
                Tok::Name(n) if (n == "END" || n == "ELUDOM") && nest == 0 => return Ok(()),
                Tok::Punct(')' | ']') => nest -= 1,
                Tok::Name(n) if n == "END" => nest -= 1,
                Tok::Punct(';') if nest == 0 => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.pos += 1;
        }
    }

    /// Reads lexemes until the one `n` ahead of the next is in `toks`.
    pub(crate) fn fill(&mut self, n: usize) {
        while self.toks.len() <= self.pos + n {
            let l = if self.sub > 0 || self.stash.is_some() {
                None
            } else {
                match self.lexeme() {
                    Ok(l) => Some(l),
                    Err(e) => {
                        self.stash = Some(e);
                        None
                    }
                }
            };
            let l = l.unwrap_or_else(|| Lexeme {
                tok: Tok::Eof,
                ..self.toks.last().cloned().unwrap_or(Lexeme {
                    tok: Tok::Eof,
                    file: 0,
                    line: 0,
                    col: 0,
                })
            });
            self.toks.push(l);
        }
    }

    /// The next lexeme for the parser, from the lexical processor.
    pub(crate) fn lexeme(&mut self) -> R<Lexeme> {
        self.scan().map(|(l, _)| l)
    }

    pub(crate) fn peek(&mut self) -> &Tok {
        self.fill(0);
        &self.toks[self.pos].tok
    }

    pub(crate) fn peek2(&mut self) -> &Tok {
        self.fill(1);
        &self.toks[self.pos + 1].tok
    }

    /// The next lexeme's place, or if it has none (a macro body gave it),
    /// the place of the last one read that has.
    pub(crate) fn here(&mut self) -> Lexeme {
        self.fill(0);
        let l = &self.toks[self.pos];
        if l.col != NOPOS {
            return l.clone();
        }
        self.last()
    }

    /// The last lexeme read that has a place.
    pub(crate) fn last(&self) -> Lexeme {
        self.toks[..self.pos.min(self.toks.len())]
            .iter()
            .rev()
            .find(|l| l.col != NOPOS)
            .or(self.toks.first())
            .cloned()
            .unwrap_or(Lexeme {
                tok: Tok::Eof,
                file: 0,
                line: 0,
                col: 0,
            })
    }

    pub(crate) fn next(&mut self) -> Tok {
        self.fill(0);
        let t = self.toks[self.pos].tok.clone();
        if t != Tok::Eof {
            self.pos += 1;
        }
        t
    }

    /// An error at `at`.
    pub(crate) fn error_at(&self, at: &Lexeme, msg: impl Into<String>) -> Error {
        Error {
            file: self.lx.files[at.file as usize].name.clone(),
            line: at.line,
            col: if at.col == NOPOS { 0 } else { at.col },
            msg: msg.into(),
        }
    }

    /// An error at the next lexeme, unless the lexical processor had one.
    pub(crate) fn error(&mut self, msg: impl Into<String>) -> Error {
        if let Some(e) = self.stash.take() {
            return e;
        }
        let at = self.here();
        self.error_at(&at, msg)
    }

    pub(crate) fn err<T>(&mut self, msg: impl Into<String>) -> R<T> {
        Err(self.error(msg))
    }

    /// Reports a diagnostic that doesn't stop the compilation.
    pub(crate) fn diag(&mut self, sev: char, at: &Lexeme, msg: String) {
        self.diags.push(Diag {
            sev,
            file: at.file,
            line: at.line,
            col: if at.col == NOPOS { 0 } else { at.col },
            msg,
        });
    }

    pub(crate) fn at(&mut self, n: &str) -> bool {
        is_name(self.peek(), n)
    }

    pub(crate) fn at_punct(&mut self, c: char) -> bool {
        *self.peek() == Tok::Punct(c)
    }

    pub(crate) fn eat(&mut self, n: &str) -> bool {
        let yes = self.at(n);
        if yes {
            self.pos += 1;
        }
        yes
    }

    pub(crate) fn eat_punct(&mut self, c: char) -> bool {
        let yes = self.at_punct(c);
        if yes {
            self.pos += 1;
        }
        yes
    }

    pub(crate) fn expect(&mut self, n: &str) -> R<()> {
        if self.eat(n) {
            Ok(())
        } else {
            let found = self.describe();
            self.err(format!("expected {n}, found {found}"))
        }
    }

    /// Word `n` next, or BLISS's message `msg` (UM appendix B).
    pub(crate) fn expect_or(&mut self, n: &str, msg: &str) -> R<()> {
        if self.eat(n) { Ok(()) } else { self.err(msg) }
    }

    pub(crate) fn expect_punct(&mut self, c: char) -> R<()> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            let found = self.describe();
            self.err(format!("expected {c}, found {found}"))
        }
    }

    pub(crate) fn describe(&mut self) -> String {
        match self.peek() {
            Tok::Name(n) | Tok::Bound(n, _) => n.clone(),
            Tok::Num(n) => n.to_string(),
            Tok::Str(s) => format!("'{}'", String::from_utf8_lossy(s)),
            Tok::Punct(c) => c.to_string(),
            Tok::Percent => "%".into(),
            Tok::Eof => "the end of the file".into(),
        }
    }

    /// Parses `toks` with `f`, which must read them all: a lexical
    /// function's parameter.
    pub(crate) fn subparse<T>(
        &mut self,
        mut toks: Vec<Lexeme>,
        f: impl FnOnce(&mut Self) -> R<T>,
    ) -> R<T> {
        let end = Lexeme {
            tok: Tok::Eof,
            ..toks.last().cloned().unwrap_or_else(|| self.last())
        };
        toks.push(end);
        let toks = std::mem::replace(&mut self.toks, toks);
        let pos = std::mem::replace(&mut self.pos, 0);
        self.sub += 1;
        let mut r = f(self);
        if r.is_ok() && *self.peek() != Tok::Eof {
            let found = self.describe();
            r = self.err(format!(
                "unexpected {found} in a lexical function's parameter"
            ));
        }
        self.sub -= 1;
        self.toks = toks;
        self.pos = pos;
        r
    }

    /// The linkage of the routine being parsed.
    pub(crate) fn linkage_now(&self) -> Option<Rc<Linkage>> {
        self.linkages_now.last().cloned().flatten()
    }

    /// Whether a routine's frame is open, for data a block computes.
    pub(crate) fn slots_open(&self) -> bool {
        self.routines > 0
    }

    /// Declares the names every module starts with: the predeclared
    /// literals and the dialect macros.
    pub(crate) fn predeclare(&mut self) {
        self.predeclare_linkages();
        let bits = 8 * i64::from(self.m.dialect.fullword());
        for (name, v) in [
            ("%BPVAL", bits),
            ("%BPUNIT", 8),
            ("%BPADDR", bits),
            ("%UPVAL", bits / 8),
        ] {
            self.scopes[0].insert(name.into(), self.m.syms.len());
            self.m.syms.push(Sym {
                name: name.into(),
                asm: name.into(),
                kind: Kind::Literal(v),
            });
        }
        let at = Lexeme {
            tok: Tok::Name("%REMAINING".into()),
            file: 0,
            line: 0,
            col: NOPOS,
        };
        let a32 = self.m.dialect.a32;
        for (name, on) in [
            ("%BLISS16", false),
            ("%BLISS32", a32),
            ("%BLISS32V", false),
            ("%BLISS36", false),
            ("%BLISS32E", a32),
            ("%BLISS64E", !a32),
        ] {
            let m = Macro {
                name: name.into(),
                kind: MacroKind::Conditional,
                formals: Vec::new(),
                fixed: 0,
                defaults: Vec::new(),
                body: if on {
                    vec![Item::Lex(at.clone())]
                } else {
                    Vec::new()
                },
            };
            self.scopes[0].insert(name.into(), self.m.syms.len());
            self.m.syms.push(Sym {
                name: name.into(),
                asm: name.into(),
                kind: Kind::Macro(Rc::new(m)),
            });
        }
    }

    /// Whether on-off switch `name` is on in the block (`NOname` for its
    /// off): as the innermost SWITCHES declaration or module head set it,
    /// else by default (LRM 18.2, 19.2; RN 2.26).
    pub(crate) fn switch_on(&self, name: &str) -> bool {
        let (base, off) = match name.strip_prefix("NO") {
            Some(b) if SWITCH_DEFAULTS.iter().any(|(s, _)| *s == b) => (b, true),
            _ => (name, false),
        };
        let given = self.opts.switches.iter().find(|(s, _)| s == base);
        let on = match self.lookup(&format!("%SWITCH {base}")) {
            Some(id) => self.m.syms[id].kind == Kind::Literal(1),
            None if given.is_some() => given.unwrap().1,
            None => SWITCH_DEFAULTS
                .iter()
                .find(|(s, _)| *s == base)
                .is_some_and(|(_, d)| *d),
        };
        on != off
    }

    /// Sets on-off switch `name` (or turns off `NOname`) in the block;
    /// false if it isn't one.
    fn set_switch(&mut self, name: &str) -> bool {
        let (base, on) = match name.strip_prefix("NO") {
            Some(b) if SWITCH_DEFAULTS.iter().any(|(s, _)| *s == b) => (b, false),
            _ => (name, true),
        };
        if !SWITCH_DEFAULTS.iter().any(|(s, _)| *s == base) {
            return false;
        }
        let id = self.m.syms.len();
        self.m.syms.push(Sym {
            name: format!("%SWITCH {base}"),
            asm: String::new(),
            kind: Kind::Literal(i64::from(on)),
        });
        self.scopes
            .last_mut()
            .unwrap()
            .insert(format!("%SWITCH {base}"), id);
        true
    }

    /// Whether `e` is a link-time constant: an address of static data or a
    /// routine, plus or minus a constant.
    pub(crate) fn ltce(&self, e: &Expr) -> bool {
        match e {
            Expr::Num(_) => true,
            Expr::Name(id) => match self.m.syms[*id].kind {
                Kind::Routine { .. }
                | Kind::Data {
                    storage: Storage::Own | Storage::Global | Storage::External,
                    ..
                } => true,
                Kind::Data {
                    storage: Storage::Bind(i),
                    ..
                } => self.ltce(&self.m.binds[i as usize]),
                _ => false,
            },
            Expr::Bin(BOp::Add, a, b) => {
                (self.ltce(a) && fold(b).is_some()) || (fold(a).is_some() && self.ltce(b))
            }
            Expr::Bin(BOp::Sub, a, b) => self.ltce(a) && fold(b).is_some(),
            Expr::Block(es, true) if es.len() == 1 => self.ltce(&es[0]),
            Expr::Field(b, p, _, _) => self.ltce(b) && fold(p).is_some_and(|p| p % 8 == 0),
            Expr::Plit(..) | Expr::Ascid(_) => true,
            e => fold(e).is_some(),
        }
    }

    pub(crate) fn name(&mut self) -> R<String> {
        match self.peek().clone() {
            Tok::Name(n) | Tok::Bound(n, _) if !RESERVED.contains(&n.as_str()) => {
                self.pos += 1;
                Ok(n)
            }
            _ => {
                let found = self.describe();
                self.err(format!("expected a name, found {found}"))
            }
        }
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<usize> {
        let id = self
            .scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name).copied())?;
        // A library's name is loaded when first looked up, as BLISS loads
        // it from the library file.
        if self.library_of.contains_key(&id) && !self.loaded.borrow().contains(&id) {
            self.loaded.borrow_mut().push(id);
        }
        Some(id)
    }

    /// LIBRARY's notes of the library names loaded since the last.
    pub(crate) fn note_loaded(&mut self) {
        let n = self.loaded.borrow().len();
        if self.loaded_shown < n && self.switch_on("LIST_SOURCE") && self.switch_on("LIST_LIBRARY")
        {
            for k in self.loaded_shown..n {
                let id = self.loaded.borrow()[k];
                let library = &self.library_stats[self.library_of[&id]].0;
                let note = format!(
                    "Loaded symbol {} from library {library}",
                    self.m.syms[id].name
                );
                self.lx.entries.push(crate::listing::Entry::Note(note));
            }
        }
        self.loaded_shown = n;
    }

    /// Declares `name` in the innermost scope; a forward routine
    /// declaration is completed rather than redeclared.
    pub(crate) fn declare(&mut self, name: String, kind: Kind) -> R<usize> {
        // The words each syntax level reserves (docs/bliss64.md, *Syntax
        // levels*): level 2, the default, has QUAD and the like.
        let level = self.opts.syntax_level.unwrap_or(2);
        if level >= 2 && LEVEL2.contains(&name.as_str()) || level >= 3 && name == "VARIABLE" {
            let at = self.last();
            return Err(self.error_at(
                &at,
                format!("Name to be declared is a reserved word:  {name}"),
            ));
        }
        if let Some(&id) = self.scopes.last().unwrap().get(&name)
            && let Some(i) = self.forward_data.iter().position(|&f| f == id)
            && let Kind::Data {
                storage: storage @ (Storage::Own | Storage::Global),
                ..
            } = kind
        {
            // A FORWARD name's OWN or GLOBAL declaration.
            self.forward_data.remove(i);
            self.m.syms[id].kind = kind;
            if storage == Storage::Global {
                self.m.syms[id].asm = if self.opts.lowercase {
                    name.to_lowercase()
                } else {
                    name
                };
            }
            return Ok(id);
        }
        if let Some(&id) = self.scopes.last().unwrap().get(&name) {
            let old = &self.m.syms[id].kind;
            let forward = matches!(old, Kind::Routine { external: true, .. })
                && matches!(
                    kind,
                    Kind::Routine {
                        external: false,
                        ..
                    }
                );
            // An external declared again is the same name: BLISSA64 takes
            // it, as in PRINT.R64 and a module both declaring
            // LIB$PUT_OUTPUT (oracle RATTR).
            let again = matches!(
                (old, &kind),
                (
                    Kind::Routine { external: true, .. },
                    Kind::Routine { external: true, .. }
                ) | (
                    Kind::Data {
                        storage: Storage::External,
                        ..
                    },
                    Kind::Data {
                        storage: Storage::External,
                        ..
                    }
                ) | (Kind::ExternalLiteral, Kind::ExternalLiteral)
            );
            if again {
                return Ok(id);
            }
            if !forward {
                return self.err(format!("Name already declared in this block:  {name}"));
            }
            self.m.syms[id].kind = kind;
            return Ok(id);
        }
        if name.chars().count() > 64 {
            let at = self.last();
            let short: String = name.chars().take(64).collect();
            self.diag(
                'W',
                &at,
                format!("Name \"{short}\" longer than 64 characters"),
            );
        }
        self.check_redeclare(&name);
        // Statics and routines are named in the assembly; locals aren't.
        let asm = match kind {
            Kind::Data {
                storage: Storage::Own,
                ..
            } => {
                let n = self.asm_names.entry(name.clone()).or_insert(0);
                *n += 1;
                if *n == 1 {
                    name.clone()
                } else {
                    format!("{name}.{}", *n - 1)
                }
            }
            _ => {
                self.asm_names.insert(name.clone(), 1);
                name.clone()
            }
        };
        // /NAMES=LOWERCASE: the linker's names in lower case, C's.
        let asm = if self.opts.lowercase {
            asm.to_lowercase()
        } else {
            asm
        };
        let id = self.m.syms.len();
        if let (
            Kind::Data {
                storage: Storage::Local(_),
                ..
            },
            Some(&r),
        ) = (&kind, self.routine_stack.last())
        {
            self.local_owner.insert(id, r);
        }
        self.m.syms.push(Sym {
            name: name.clone(),
            asm,
            kind,
        });
        self.scopes.last_mut().unwrap().insert(name, id);
        Ok(id)
    }

    pub(crate) fn slot(&mut self, bytes: u32) -> u32 {
        self.slots.push(bytes);
        self.slots.len() as u32 - 1
    }

    // Modules and declarations.

    pub(crate) fn module(&mut self) -> R<()> {
        self.expect("MODULE")?;
        self.m.name = self.name()?;
        if self.eat_punct('(') {
            loop {
                self.switch()?;
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
        }
        self.expect_punct('=')?;
        // The switches set, the structures they change are declared.
        self.predeclare_structures()?;
        if let Some(toks) = self.pending_structure.take() {
            self.subparse(toks, |p| p.structure_switch())?;
        }
        let paren = self.eat_punct('(');
        if !paren {
            self.expect("BEGIN")?;
        }
        // The module's names are in a block inside the predeclared ones,
        // which it may declare again (LRM 19.3).
        self.scopes.push(HashMap::new());
        // The module's block stays open in the listing to ELUDOM.
        self.depth += 1;
        self.declarations()?;
        if paren {
            self.expect_punct(')')?;
        } else {
            self.expect("END")?;
        }
        self.expect_or("ELUDOM", "Missing ELUDOM following module")?;
        if *self.peek() != Tok::Eof {
            return self.err("text after ELUDOM");
        }
        self.m.nocode = !self.switch_on("CODE");
        if let Some(&id) = self.forward_data.first() {
            let n = self.m.syms[id].name.clone();
            return self.err(format!("Name declared FORWARD is not defined:  {n}"));
        }
        if let Some(main) = &self.m.main {
            let main = main.clone();
            match self.lookup(&main).map(|id| &self.m.syms[id].kind) {
                Some(Kind::Routine {
                    external: false, ..
                }) => {}
                _ => return self.err("Routine specified as MAIN is not defined"),
            }
        }
        Ok(())
    }

    /// A module switch: MAIN and IDENT matter; the rest is accepted.
    pub(crate) fn switch(&mut self) -> R<()> {
        let name = self.name().or_else(|_| match self.next() {
            Tok::Name(n) => Ok(n),
            _ => self.err("expected a module switch"),
        })?;
        if name == "STRUCTURE" && self.at_punct('(') {
            return self.structure_switch();
        }
        if name == "ENVIRONMENT" && self.eat_punct('(') {
            // FP or NOFP (RN 2.11); vaxpunk's integer division never needs
            // floating point, so only the floating built-ins care.
            loop {
                match self.next() {
                    Tok::Name(n) if n == "NOFP" => self.m.nofp = true,
                    Tok::Name(n) if n == "FP" => self.m.nofp = false,
                    Tok::Name(_) => {}
                    _ => return self.err("expected an ENVIRONMENT option"),
                }
                if !self.eat_punct(',') {
                    break;
                }
            }
            return self.expect_punct(')');
        }
        if name == "COUNT" || name == "NOCOUNT" {
            self.m.nocount = name == "NOCOUNT";
        }
        if !self.at_punct('(') && !self.at_punct('=') && self.set_switch(&name) {
            return Ok(());
        }
        if name == "LIST" && self.eat_punct('(') {
            // Each option a LIST_ switch: LIST(NOEXPAND) is NOLIST_EXPAND.
            loop {
                let Tok::Name(o) = self.next() else {
                    return self.err("expected a LIST option");
                };
                let (no, base) = match o.strip_prefix("NO") {
                    Some(b) => ("NO", b),
                    None => ("", o.as_str()),
                };
                if !self.set_switch(&format!("{no}LIST_{base}")) {
                    return self.err(format!("{o} is not a LIST option"));
                }
                if !self.eat_punct(',') {
                    break;
                }
            }
            return self.expect_punct(')');
        }
        if name == "LANGUAGE" && self.at_punct('(') {
            let at = self.last();
            self.pos += 1;
            let mut list = Vec::new();
            while let Tok::Name(n) = self.peek().clone() {
                self.pos += 1;
                list.push(n);
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(')')?;
            self.set_language(&list, &at);
            return Ok(());
        }
        if name == "LINKAGE" && self.eat_punct('(') {
            // The default linkage for the block (LRM 18.2): `%LINKAGE`,
            // which no source can declare.
            let l = self.name()?;
            let kind = match self.lookup(&l).map(|id| &self.m.syms[id].kind) {
                Some(Kind::Linkage(k)) => Kind::Linkage(k.clone()),
                _ => {
                    return self.err(format!(
                        "Name in attribute list not declared as a structure or linkage name:  {l}"
                    ));
                }
            };
            self.expect_punct(')')?;
            let id = self.m.syms.len();
            self.m.syms.push(Sym {
                name: "%LINKAGE".into(),
                asm: String::new(),
                kind,
            });
            self.scopes
                .last_mut()
                .unwrap()
                .insert("%LINKAGE".into(), id);
            return Ok(());
        }
        if self.eat_punct('=') {
            match (name.as_str(), self.next()) {
                ("MAIN", Tok::Name(n)) => self.m.main = Some(n),
                ("IDENT", Tok::Str(s)) => {
                    self.m.ident = Some(String::from_utf8_lossy(&s).into_owned())
                }
                (_, Tok::Name(_) | Tok::Str(_) | Tok::Num(_)) => {}
                _ => return self.err(format!("bad value for {name}")),
            }
        } else if self.at_punct('(') {
            let mut depth = 0;
            loop {
                match self.next() {
                    Tok::Punct('(') => depth += 1,
                    Tok::Punct(')') => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Tok::Eof => return self.err("unbalanced parentheses"),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// `STRUCTURE(attribute)`, after the switch's name: the default
    /// structure for default structure references in the block (LRM 11.8,
    /// 18.2). A module switch's is read once the structures it names are
    /// declared.
    fn structure_switch(&mut self) -> R<()> {
        if self.lookup("VECTOR").is_none() {
            let start = self.pos;
            let mut depth = 0;
            loop {
                match self.next() {
                    Tok::Punct('(') => depth += 1,
                    Tok::Punct(')') => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Tok::Eof => return self.err("unbalanced parentheses"),
                    _ => {}
                }
            }
            self.pending_structure = Some(self.toks[start..self.pos].to_vec());
            return Ok(());
        }
        self.expect_punct('(')?;
        let attr = if self.at_punct(')') {
            None
        } else {
            let refr = self.eat("REF");
            Some(Rc::new(self.struct_attr(refr)?))
        };
        self.expect_punct(')')?;
        let id = self.m.syms.len();
        self.m.syms.push(Sym {
            name: "%STRUCTURE".into(),
            asm: String::new(),
            kind: Kind::DefaultStructure(attr),
        });
        self.scopes
            .last_mut()
            .unwrap()
            .insert("%STRUCTURE".into(), id);
        Ok(())
    }

    /// Declarations, each ending with a semicolon, as long as there are.
    pub(crate) fn declarations(&mut self) -> R<()> {
        // ponytail: module-level declarations can't need code (run-time
        // BINDs, LOCALs), so only blocks collect `inits`.
        loop {
            let m = self.mark();
            let r = if self.eat("REQUIRE") {
                self.require()
            } else if self.eat("LIBRARY") {
                self.library()
            } else if self.eat_punct(';') {
                // A stray semicolon between declarations, as TPAMAC.REQ's
                // $INIT_STATE leaves one, which BLISSA64 takes (oracle
                // STRAY, BLKSEMI).
                Ok(())
            } else {
                match self.declaration() {
                    Ok(true) => self.expect_punct(';'),
                    Ok(false) => return Ok(()),
                    Err(e) => Err(e),
                }
            };
            if let Err(e) = r {
                self.recover(e, &m)?;
            }
        }
    }

    /// `REQUIRE 'file';`: the file's lexemes take the declaration's place.
    pub(crate) fn require(&mut self) -> R<()> {
        let Tok::Str(name) = self.next() else {
            return self.err("Missing quoted string following REQUIRE or LIBRARY");
        };
        self.expect_punct(';')?;
        self.require_file(&String::from_utf8_lossy(&name))
    }

    /// `LIBRARY 'file';`, after the word: the declarations of the
    /// library's source (docs/bliss64.md *Libraries*), read as LRM 16.6
    /// has a precompiled library: in a context of its own, which sees the
    /// predeclared names and its own, so that its lexical processing is
    /// its own; with declarations only; and giving the block the names it
    /// declares, not those its own libraries brought it. A library read
    /// again gives the same names.
    pub(crate) fn library(&mut self) -> R<()> {
        let at = self.here();
        let Tok::Str(name) = self.next() else {
            return self.err("Missing quoted string following REQUIRE or LIBRARY");
        };
        self.expect_punct(';')?;
        let name = String::from_utf8_lossy(&name).into_owned();
        let names = match self.libraries.get(&name) {
            Some(names) => names.clone(),
            None => {
                let (names, bytes) = self.read_library(&name)?;
                let i = self.library_stats.len();
                let given: Vec<usize> = names
                    .iter()
                    .filter(|(n, _)| !n.starts_with('%'))
                    .map(|&(_, id)| id)
                    .collect();
                self.library_stats.push((name.clone(), given.len(), bytes));
                for id in given {
                    self.library_of.entry(id).or_insert(i);
                }
                if self.switch_on("LIST_SOURCE") && self.switch_on("LIST_LIBRARY") {
                    let date = self.opts.date.clone().unwrap_or_default();
                    self.lx.entries.push(crate::listing::Entry::Note(format!(
                        "Library file {name} produced by {} on {date}",
                        crate::TOOL
                    )));
                }
                self.libraries.insert(name, names.clone());
                names
            }
        };
        for (n, id) in names {
            match self.scopes.last().unwrap().get(&n) {
                Some(&old) if old != id => {
                    return self.err(format!("Name already declared in this block:  {n}"));
                }
                // The same library again: BLISSA64 warns of each name
                // (oracle LIBS2).
                Some(_) => {
                    let msg = format!("Name already declared in this block:  {n}");
                    self.diag('W', &at, msg);
                }
                None => {
                    self.scopes.last_mut().unwrap().insert(n, id);
                }
            }
        }
        Ok(())
    }

    /// Parses library `name`'s declarations in a context of their own;
    /// returns the names they declare.
    fn read_library(&mut self, name: &str) -> R<(Vec<(String, usize)>, usize)> {
        // The lexemes already read ahead are the declaration's: none.
        self.toks.truncate(self.pos);
        let bytes = self.library_file(name)?;
        let predeclared = self.scopes[0].clone();
        let outer = std::mem::replace(&mut self.scopes, vec![predeclared, HashMap::new()]);
        let forward = std::mem::take(&mut self.forward_data);
        let outer_imports = std::mem::take(&mut self.library_imports);
        self.library_depth += 1;
        let r = self.library_declarations();
        self.library_depth -= 1;
        self.forward_data = forward;
        let imported = std::mem::replace(&mut self.library_imports, outer_imports);
        let mut scopes = std::mem::replace(&mut self.scopes, outer);
        r?;
        // Names its own libraries gave it stay its own.
        let mut names: Vec<(String, usize)> = scopes
            .pop()
            .unwrap()
            .into_iter()
            .filter(|(_, id)| !imported.contains(id))
            .collect();
        names.sort();
        Ok((names, bytes))
    }

    /// The declarations of a library's source, up to its end.
    fn library_declarations(&mut self) -> R<()> {
        self.library_body()?;
        // The library's Eof, in `toks` at `pos`, goes with its stream.
        self.toks.truncate(self.pos);
        self.end_library();
        Ok(())
    }

    /// A library's source compiled on its own, as /LIBRARY does: its
    /// declarations, checked as LIBRARY reads them, and no code.
    pub(crate) fn library_module(&mut self) -> R<()> {
        self.predeclare_structures()?;
        self.scopes.push(HashMap::new());
        self.library_depth += 1;
        let r = self.library_body();
        self.library_depth -= 1;
        self.m.nocode = true;
        r
    }

    fn library_body(&mut self) -> R<()> {
        loop {
            if self.eat("REQUIRE") {
                self.require()?;
            } else if self.eat("LIBRARY") {
                // Its names are the library's to use, not to give.
                let before: Vec<usize> = self.scopes.last().unwrap().values().copied().collect();
                self.library()?;
                let new: Vec<usize> = self
                    .scopes
                    .last()
                    .unwrap()
                    .values()
                    .copied()
                    .filter(|id| !before.contains(id))
                    .collect();
                self.library_imports.extend(new);
            } else if self.at_punct(';') {
                self.pos += 1;
            } else if self.declaration()? {
                self.expect_punct(';')?;
            } else {
                break;
            }
        }
        if *self.peek() != Tok::Eof {
            return self.err("Library source module must contain only declarations");
        }
        Ok(())
    }

    /// MACRO or KEYWORDMACRO definitions, after the word.
    pub(crate) fn macros(&mut self, keyword: bool) -> R<()> {
        loop {
            let name = self.name()?;
            let (mut formals, mut defaults) = (Vec::new(), Vec::new());
            let mut kind = if keyword {
                MacroKind::Keyword
            } else {
                MacroKind::Simple
            };
            if self.eat_punct('(') {
                loop {
                    formals.push(self.name()?);
                    if keyword {
                        if self.eat_punct('=') {
                            // The default, kept as a macro body is.
                            let ends = [Tok::Punct(','), Tok::Punct(')')];
                            let (body, end) = self.macro_body(&[], &ends)?;
                            defaults.push(
                                body.into_iter()
                                    .filter_map(|i| match i {
                                        Item::Lex(l) => Some(l),
                                        Item::Formal(_) => None,
                                    })
                                    .collect(),
                            );
                            if end == Tok::Punct(')') {
                                break;
                            }
                            continue;
                        }
                        defaults.push(Vec::new());
                    }
                    if !self.eat_punct(',') {
                        self.expect_punct(')')?;
                        break;
                    }
                }
            } else if keyword {
                // Without formals it is a simple macro, as BLISSA64 takes
                // DEC's STARLET.R64's $CREATE_MAPPING and the like.
                kind = MacroKind::Simple;
            }
            let fixed = formals.len();
            if !keyword && self.eat_punct('[') {
                kind = MacroKind::Conditional;
                if !self.eat_punct(']') {
                    kind = MacroKind::Iterative;
                    loop {
                        formals.push(self.name()?);
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                    self.expect_punct(']')?;
                }
            }
            self.expect_punct('=')?;
            let (body, _) = self.macro_body(&formals, &[])?;
            let m = Macro {
                name: name.clone(),
                kind,
                formals,
                fixed,
                defaults,
                body,
            };
            self.declare(name, Kind::Macro(Rc::new(m)))?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// One declaration, if one is next.
    pub(crate) fn declaration(&mut self) -> R<bool> {
        let Tok::Name(word) = self.peek().clone() else {
            return Ok(false);
        };
        // A library declares names only (LRM 16.6.2).
        let external = matches!(self.peek2(), Tok::Name(n) if n == "ROUTINE" || n == "LITERAL");
        if self.library_depth > 0
            && (matches!(
                word.as_str(),
                "OWN"
                    | "GLOBAL"
                    | "LOCAL"
                    | "STACKLOCAL"
                    | "REGISTER"
                    | "ROUTINE"
                    | "MAP"
                    | "LABEL"
                    | "ENABLE"
                    | "PSECT"
            ) || word == "FORWARD" && !external)
        {
            return self.err("Illegal declaration type in library source module");
        }
        let global = word == "GLOBAL";
        let external = word == "EXTERNAL";
        let forward = word == "FORWARD";
        if global || external || forward {
            match self.peek2().clone() {
                Tok::Name(n) if n == "ROUTINE" => {
                    self.pos += 2;
                    return if global {
                        self.routine(true).map(|_| true)
                    } else {
                        self.routine_names().map(|_| true)
                    };
                }
                Tok::Name(n) if n == "LITERAL" => {
                    self.pos += 2;
                    if external {
                        return self.external_literals().map(|_| true);
                    }
                    return self.literals(global).map(|_| true);
                }
                Tok::Name(n) if n == "REGISTER" && !forward => {
                    self.pos += 2;
                    return self.registers(global, external).map(|_| true);
                }
                Tok::Name(n) if n == "BIND" && global => {
                    self.pos += 2;
                    let routine = self.eat("ROUTINE");
                    return self.binds(routine, true).map(|_| true);
                }
                _ if forward => {
                    self.pos += 1;
                    return self.forward_data_names().map(|_| true);
                }
                _ => {}
            }
        }
        match word.as_str() {
            "OWN" | "GLOBAL" | "EXTERNAL" | "LOCAL" | "STACKLOCAL" => {
                self.pos += 1;
                let storage = match word.as_str() {
                    "OWN" => Storage::Own,
                    "GLOBAL" => Storage::Global,
                    "EXTERNAL" => Storage::External,
                    _ => Storage::Local(0),
                };
                self.data(storage)?;
            }
            "ROUTINE" => {
                self.pos += 1;
                self.routine(false)?;
            }
            "LITERAL" => {
                self.pos += 1;
                self.literals(false)?;
            }
            "MACRO" | "KEYWORDMACRO" => {
                self.pos += 1;
                self.macros(word == "KEYWORDMACRO")?;
            }
            "STRUCTURE" => {
                self.pos += 1;
                self.structures()?;
            }
            "FIELD" => {
                self.pos += 1;
                self.fields()?;
            }
            "BIND" => {
                self.pos += 1;
                let routine = self.eat("ROUTINE");
                self.binds(routine, false)?;
            }
            "MAP" => {
                self.pos += 1;
                self.maps()?;
            }
            "COMPILETIME" => {
                self.pos += 1;
                loop {
                    let name = self.name()?;
                    self.expect_punct('=')?;
                    let v = self.ctce()?;
                    self.declare(name, Kind::Compiletime(v))?;
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            "LABEL" => {
                self.pos += 1;
                loop {
                    let name = self.name()?;
                    self.declare(name, Kind::Label)?;
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            "LINKAGE" => {
                let at = self.here();
                self.pos += 1;
                self.transport("LINKAGE", &at, true);
                self.linkages()?;
            }
            "PSECT" => {
                self.pos += 1;
                let at = self.here();
                self.transport("PSECT", &at, true);
                self.psect_declaration()?;
            }
            "SWITCHES" => {
                self.pos += 1;
                loop {
                    self.switch()?;
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            "UNDECLARE" => {
                self.pos += 1;
                loop {
                    let name = self.name()?;
                    for scope in self.scopes.iter_mut().rev() {
                        if scope.remove(&name).is_some() {
                            break;
                        }
                    }
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            "BUILTIN" => {
                self.pos += 1;
                self.builtins()?;
            }
            "ENABLE" => {
                let at = self.here();
                self.pos += 1;
                self.transport("ENABLE", &at, true);
                self.enable()?;
            }
            "REGISTER" => {
                self.pos += 1;
                self.registers(false, false)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn literals(&mut self, global: bool) -> R<()> {
        loop {
            let name = self.name()?;
            self.expect_punct('=')?;
            let v = self.ctce()?;
            // A unit, a range (LRM 9.12) and EXTERNAL_NAME (RN 2.14).
            let a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                crate::data::Attrs::default()
            };
            if let Some((signed, bits)) = a.range {
                let fits = match (signed, bits) {
                    (_, 64..) => true,
                    (true, 1..) => (-(1i64 << (bits - 1))..1i64 << (bits - 1)).contains(&v),
                    (false, 0..) => (0..1i64 << bits).contains(&v),
                    _ => false,
                };
                if !fits {
                    let at = self.last();
                    self.diag(
                        'W',
                        &at,
                        "Literal value cannot be represented in the declared number of bits".into(),
                    );
                }
            }
            if global {
                let symbol = a.external_name.unwrap_or_else(|| name.clone());
                self.m.global_literals.push((symbol, Expr::Num(v)));
            }
            self.declare(name, Kind::Literal(v))?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// EXTERNAL LITERAL names, after the words: values the linker gives.
    pub(crate) fn external_literals(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            let a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                crate::data::Attrs::default()
            };
            let id = self.declare(name, Kind::ExternalLiteral)?;
            if let Some(n) = a.external_name {
                self.m.syms[id].asm = n;
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `PSECT class = name (attributes), ...`, after the word.
    pub(crate) fn psect_declaration(&mut self) -> R<()> {
        loop {
            let class = self.name().or_else(|_| match self.next() {
                Tok::Name(n) => Ok(n),
                _ => self.err("expected OWN, GLOBAL, PLIT or CODE"),
            })?;
            self.expect_punct('=')?;
            let name = self.name()?;
            let mut attrs = Vec::new();
            if self.eat_punct('(') {
                loop {
                    let a = match self.next() {
                        Tok::Name(a) => a,
                        _ => return self.err("expected a psect attribute"),
                    };
                    let word = match a.as_str() {
                        "WRITE" => "WRT",
                        "NOWRITE" => "NOWRT",
                        "EXECUTE" => "EXE",
                        "NOEXECUTE" => "NOEXE",
                        "READ" => "RD",
                        "NOREAD" => "NORD",
                        "SHARE" => "SHR",
                        "NOSHARE" => "NOSHR",
                        "PIC" => "PIC",
                        "NOPIC" => "NOPIC",
                        "LOCAL" => "LCL",
                        "GLOBAL" => "GBL",
                        "OVERLAY" => "OVR",
                        "CONCATENATE" => "CON",
                        "VECTOR" => "VEC",
                        _ => "",
                    };
                    if !word.is_empty() {
                        attrs.push(word.to_string());
                    }
                    if a == "ALIGN" || a == "ADDRESSING_MODE" {
                        self.expect_punct('(')?;
                        if a == "ALIGN" {
                            attrs.push(self.ctce()?.to_string());
                        } else {
                            self.name()?;
                        }
                        self.expect_punct(')')?;
                    }
                    if !self.eat_punct(',') {
                        break;
                    }
                }
                self.expect_punct(')')?;
            }
            if !attrs.is_empty() {
                self.m.psect_attrs.retain(|(n, _)| *n != name);
                self.m.psect_attrs.push((name.clone(), attrs.join(", ")));
            }
            let p = &mut self.m.psects;
            match class.as_str() {
                "OWN" => p.own = name,
                "GLOBAL" => p.global = name,
                "PLIT" => p.plit = name,
                "CODE" => p.code = name,
                // ARM64 code reaches its data PC-relative, with no linkage
                // section, and a LOCAL's INITIAL is code, with no constant
                // data behind it (docs/bliss64.md *Psects*): nothing goes
                // in either.
                "NODEFAULT" | "LINK" | "INITIAL" => {}
                _ => return self.err(format!("{class} is not a storage class for PSECT")),
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// FORWARD names, after the word: OWN or GLOBAL data declared later
    /// in the block, its attributes given here too (LRM 10.3).
    fn forward_data_names(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            let mut a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                crate::data::Attrs::default()
            };
            let kind = self.data_kind(Storage::Own, &mut a)?;
            let id = self.declare(name, kind)?;
            self.forward_data.push(id);
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// EXTERNAL ROUTINE and FORWARD ROUTINE: names with attributes.
    pub(crate) fn routine_names(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            // A prototype: the formals, with their attributes (RN 2.19.8).
            let formals = if self.at_punct('(') {
                let list = self.formal_list()?.0;
                Some(
                    list.iter()
                        .map(|(_, a)| crate::checks::Formal {
                            unit: a.unit.filter(|_| a.structure.is_none()),
                            ref_bytes: a
                                .structure
                                .as_ref()
                                .filter(|s| s.refr)
                                .and_then(|s| {
                                    // The structure's own size, not the
                                    // REF's.
                                    self.struct_bytes(&crate::data::StructAttr {
                                        st: s.st.clone(),
                                        alloc: s.alloc.clone(),
                                        refr: false,
                                        fields: Vec::new(),
                                    })
                                })
                                .map(|n| n as u32),
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let (novalue, linkage) = self.routine_attributes()?;
            let linkage = linkage.or_else(|| self.default_linkage());
            let id = self.declare(
                name,
                Kind::Routine {
                    global: false,
                    external: true,
                    novalue,
                    linkage,
                },
            )?;
            if let Some(n) = self.routine_name.take() {
                self.m.syms[id].asm = n;
            }
            if let Some(f) = formals {
                self.formal_counts
                    .insert(id, (f.len(), self.routine_variable));
                self.formals.insert(id, f);
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `: NOVALUE`, a linkage name and the like; returns whether NOVALUE
    /// and the linkage. EXTERNAL_NAME and PSECT go in `routine_name` and
    /// `routine_psect` for the routine being declared.
    pub(crate) fn routine_attributes(&mut self) -> R<(bool, Option<Rc<Linkage>>)> {
        let (mut novalue, mut linkage) = (false, None);
        self.routine_name = None;
        self.routine_psect = None;
        self.routine_variable = false;
        self.routine_value = None;
        let (mut unit, mut signed) = (None, None);
        if self.eat_punct(':') {
            loop {
                if self.eat("NOVALUE") {
                    novalue = true;
                } else if self.eat("EXTERNAL_NAME") {
                    self.routine_name = Some(self.external_name()?);
                } else if self.eat("PSECT") {
                    self.expect_punct('(')?;
                    self.routine_psect = Some(self.name()?);
                    self.expect_punct(')')?;
                } else if self.eat("VARIABLE") {
                    self.routine_variable = true;
                } else if self.eat("WEAK") {
                } else if self.eat("ADDRESSING_MODE") {
                    // Meaningless on ARM64, as on Alpha (RN 2.21).
                    self.skip_parenthesized()?;
                } else if let Tok::Name(n) = self.peek().clone()
                    && let Some(u) = crate::data::unit(&n)
                {
                    // The value's unit and sign (RN 2.29).
                    self.pos += 1;
                    unit = Some(u);
                } else if self.eat("SIGNED") {
                    signed = Some(true);
                } else if self.eat("UNSIGNED") {
                    signed = Some(false);
                } else if self.eat("REF") {
                    // The value is a pointer to such a structure.
                    self.struct_attr(true)?;
                } else if self.eat("FIELD") {
                    self.skip_parenthesized()?;
                } else if let Tok::Name(n) = self.peek().clone()
                    && matches!(
                        self.lookup(&n).map(|id| &self.m.syms[id].kind),
                        Some(Kind::Structure(_))
                    )
                {
                    self.struct_attr(false)?;
                } else if let Tok::Name(n) = self.peek().clone()
                    && let Some(Kind::Linkage(l)) = self.lookup(&n).map(|id| &self.m.syms[id].kind)
                {
                    linkage = Some(l.clone());
                    let at = self.here();
                    self.pos += 1;
                    // LRM C: the FORTRAN ones are common, BLISS and
                    // FORTRAN in the BLISS-16/BLISS-32 subset.
                    match n.as_str() {
                        "FORTRAN_FUNC" | "FORTRAN_SUB" => {}
                        "BLISS" | "FORTRAN" => self.transport(&n, &at, false),
                        _ => self.transport(&n, &at, true),
                    }
                } else {
                    break;
                }
            }
        }
        if unit.is_some() || signed.is_some() {
            let u = unit.unwrap_or(self.m.dialect.fullword());
            self.routine_value = Some((u, signed.unwrap_or(false)));
        }
        Ok((novalue, linkage))
    }

    /// `(formal : attributes, ...; output, ...)`: a routine's formals
    /// and its output formals.
    #[allow(clippy::type_complexity)]
    fn formal_list(
        &mut self,
    ) -> R<(
        Vec<(String, crate::data::Attrs)>,
        Vec<(String, crate::data::Attrs)>,
    )> {
        let (mut names, mut out_names) = (Vec::new(), Vec::new());
        self.expect_punct('(')?;
        let mut list = &mut names;
        loop {
            if self.eat_punct(';') {
                list = &mut out_names;
            }
            let n = self.name()?;
            let a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                crate::data::Attrs::default()
            };
            list.push((n, a));
            if !self.eat_punct(',') && !self.at_punct(';') {
                break;
            }
        }
        self.expect_punct(')')?;
        Ok((names, out_names))
    }

    pub(crate) fn routine(&mut self, global: bool) -> R<()> {
        let first_diag = self.diags.len();
        let name = self.name()?;
        let (names, out_names) = if self.at_punct('(') {
            self.formal_list()?
        } else {
            (Vec::new(), Vec::new())
        };
        let (mut novalue, mut linkage) = self.routine_attributes()?;
        let (asm, psect) = (self.routine_name.take(), self.routine_psect.take());
        let variable = self.routine_variable;
        if linkage.is_none()
            && let Some(Kind::Routine { linkage: l, .. }) = self
                .scopes
                .last()
                .unwrap()
                .get(&name)
                .map(|&id| &self.m.syms[id].kind)
        {
            // A FORWARD declaration's.
            linkage = l.clone();
        }
        if linkage.is_none() {
            linkage = self.default_linkage();
        }
        if let Some(l) = &linkage
            && l.interrupt
        {
            // Its formals are the stack at entry, the PC and the PSL last
            // (LRM 13.3.4.2), and it returns nothing.
            if !out_names.is_empty() {
                return self.err(format!("{name} can't have output formals"));
            }
            novalue = true;
        }
        if let Some(l) = &linkage
            && l.jsb
            && names.len() > l.params.len()
        {
            return self.err(format!(
                "{name} has more formals than its JSB linkage has registers"
            ));
        }
        let out_regs = linkage.as_ref().map_or(&[][..], |l| &l.outs[..]).to_vec();
        if out_names.len() > out_regs.len() {
            return self.err(format!(
                "{name} has more output formals than its linkage has output parameters"
            ));
        }
        let sym = self.declare(
            name,
            Kind::Routine {
                global,
                external: false,
                novalue,
                linkage,
            },
        )?;
        if let Some(Kind::Routine { global: g, .. }) = self.m.syms.get_mut(sym).map(|s| &mut s.kind)
        {
            *g = global;
        }
        if let Some(n) = asm {
            self.m.syms[sym].asm = n;
        }
        self.formal_counts.insert(sym, (names.len(), variable));
        let units = names
            .iter()
            .map(|(_, a)| crate::checks::Formal {
                unit: a.unit.filter(|_| a.structure.is_none()),
                ref_bytes: None,
            })
            .collect();
        self.formals.insert(sym, units);
        self.expect_punct('=')?;
        let outer = std::mem::take(&mut self.slots);
        self.scopes.push(HashMap::new());
        self.routine_count += 1;
        self.routine_stack.push(self.routine_count);
        self.routine_names.push(self.m.syms[sym].name.clone());
        let mut formals = Vec::new();
        for (n, mut a) in names {
            let refr = a.structure.as_ref().is_some_and(|s| s.refr);
            let kind = self.data_kind(Storage::Local(0), &mut a)?;
            if let Kind::Data {
                storage: Storage::Local(slot),
                ..
            } = kind
            {
                // The argument is stored whole.
                let s = &mut self.slots[slot as usize];
                *s = (*s).max(8);
            }
            let id = self.declare(n, kind)?;
            if refr {
                self.uses.entry(id).or_default().address = true;
            }
            formals.push(id);
        }
        let mut outputs = Vec::new();
        for ((n, mut a), r) in out_names.into_iter().zip(out_regs) {
            let kind = self.data_kind(Storage::Local(0), &mut a)?;
            outputs.push((r, self.declare(n, kind)?));
        }
        let linkage = match &self.m.syms[sym].kind {
            Kind::Routine { linkage, .. } => linkage.clone(),
            _ => None,
        };
        self.linkages_now.push(linkage);
        self.exits.push((Vec::new(), outputs));
        self.routines += 1;
        let body = self.expr();
        self.routines -= 1;
        self.routine_stack.pop();
        self.routine_names.pop();
        self.linkages_now.pop();
        let (externals, outputs) = self.exits.pop().unwrap();
        let body = body?;
        self.scopes.pop();
        let slots = std::mem::replace(&mut self.slots, outer);
        let psect = psect.unwrap_or_else(|| self.m.psects.code.clone());
        // BLISSA64 looks at a routine's values only if it has no error.
        if self.diags[first_diag..].iter().any(|d| d.sev == 'E') {
            let late = std::mem::take(&mut self.late);
            let mut i = 0;
            self.diags.retain(|_| {
                i += 1;
                !late.contains(&(i - 1))
            });
        }
        self.late.retain(|&i| i < first_diag);
        self.m.routines.push(Routine {
            sym,
            formals,
            externals,
            outputs,
            body,
            slots,
            psect,
        });
        Ok(())
    }

    // Expressions, from the loosest operator to the tightest.

    pub fn expr(&mut self) -> R<Expr> {
        let actual = std::mem::take(&mut self.whole_actual);
        let left = self.binary(0)?;
        if self.at_punct('=') {
            let at = self.here();
            self.pos += 1;
            let right = if self.operand_next() || self.stash.is_some() {
                self.value()?
            } else {
                let at = self.here();
                let msg = "Missing operand following \"=\".  A literal zero has been inserted";
                self.diag('W', &at, msg.into());
                Expr::Num(0)
            };
            // An actual `K = X` is a keyword macro's, if one undeclared.
            if !actual {
                self.lint_store(&left, &right, &at);
                self.check_store(&left, &right, &at);
            }
            return Ok(Expr::Assign(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    /// An expression whose value is used: an IF without ELSE gets the
    /// informational BLISSA64 gives.
    pub(crate) fn value(&mut self) -> R<Expr> {
        let e = self.expr()?;
        if matches!(e, Expr::If(_, _, None)) {
            let at = self.last();
            self.diag(
                'I',
                &at,
                "Null expression appears in value-required context".into(),
            );
        }
        Ok(e)
    }

    /// Warns, as BLISSA64 does, of a fetch or store at a constant place
    /// that isn't all inside its data segment.
    pub(crate) fn check_inside(&mut self, place: &Expr, at: &Lexeme) {
        let (addr, pos, size) = match place {
            Expr::Field(b, p, s, _) => match (fold(p), fold(s)) {
                (Some(p), Some(s)) => (&**b, p, s),
                _ => return,
            },
            Expr::Name(_) => return,
            e => (e, 0, 8 * i64::from(self.m.dialect.fullword())),
        };
        let Some((id, off)) = crate::data::base_offset(addr) else {
            return;
        };
        let Kind::Data { storage, bytes, .. } = self.m.syms[id].kind else {
            return;
        };
        if !matches!(storage, Storage::Own | Storage::Global | Storage::Local(_)) {
            return;
        }
        let first = off * 8 + pos;
        // At the field selector, if there is one.
        let at = match (place, &self.field_at) {
            (Expr::Field(..), Some(f)) => &f.clone(),
            _ => at,
        };
        if first < 0 || first + size > i64::from(bytes) * 8 {
            let msg = format!(
                "Reference outside of data segment {}, possible optimizations lost",
                self.m.syms[id].name
            );
            self.diag('W', at, msg);
        }
    }

    /// Whether a test is decided by its operand's extension, as BLISSA64
    /// sees it: an unsigned field narrower than a fullword compared
    /// signed with 0.
    fn constant_test(&self, c: &Expr) -> Option<&'static str> {
        let Expr::Bin(BOp::Rel(rel, false), a, b) = c else {
            return None;
        };
        if fold(b) != Some(0) {
            return None;
        }
        let Expr::Fetch(x) = &**a else {
            return None;
        };
        let unsigned_narrow = match &**x {
            Expr::Name(id) => matches!(
                self.m.syms[*id].kind,
                Kind::Data { size, signed: false, .. } if size < self.m.dialect.fullword()
            ),
            Expr::Field(_, _, s, e) => {
                fold(e) == Some(0)
                    && fold(s).is_some_and(|s| s < 8 * i64::from(self.m.dialect.fullword()))
            }
            _ => false,
        };
        match rel {
            Rel::Lss if unsigned_narrow => Some("false"),
            Rel::Geq if unsigned_narrow => Some("true"),
            _ => None,
        }
    }

    /// Whether the next lexeme can start an operand.
    pub(crate) fn operand_next(&mut self) -> bool {
        match self.peek() {
            Tok::Num(_) | Tok::Str(_) | Tok::Bound(..) => true,
            Tok::Punct(c) => "(.+-".contains(*c),
            Tok::Name(n) => {
                !RESERVED.contains(&n.as_str())
                    || [
                        "BEGIN",
                        "IF",
                        "WHILE",
                        "UNTIL",
                        "DO",
                        "INCR",
                        "INCRA",
                        "INCRU",
                        "DECR",
                        "DECRA",
                        "DECRU",
                        "CASE",
                        "SELECT",
                        "SELECTA",
                        "SELECTU",
                        "SELECTONE",
                        "SELECTONEA",
                        "SELECTONEU",
                        "LEAVE",
                        "EXITLOOP",
                        "RETURN",
                        "NOT",
                        "PLIT",
                        "UPLIT",
                    ]
                    .contains(&n.as_str())
            }
            Tok::Percent | Tok::Eof => false,
        }
    }

    /// Binary operators of `level` and tighter: EQV and XOR, OR, AND, NOT,
    /// the relations, + and -, * / MOD, ^.
    pub(crate) fn binary(&mut self, level: u8) -> R<Expr> {
        if level == 3 {
            if self.eat("NOT") {
                return Ok(Expr::Not(Box::new(self.binary(3)?)));
            }
            return self.binary(4);
        }
        if level == 8 {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        while let Some(op) = self.operator(level) {
            let op_at = self.here();
            self.pos += 1;
            let right = if self.operand_next() || self.stash.is_some() {
                self.binary(level + 1)?
            } else {
                let op = self.toks[self.pos - 1].tok.clone();
                let at = self.here();
                let op = match op {
                    Tok::Punct(c) => c.to_string(),
                    Tok::Name(n) => n,
                    _ => String::new(),
                };
                self.diag(
                    'W',
                    &at,
                    format!(
                        "Missing operand following \"{op}\".  A literal zero has been inserted"
                    ),
                );
                Expr::Num(0)
            };
            self.lint_operands(op, &left, &right, &op_at);
            let checked = match op {
                BOp::Add => Some(crate::ir::Op::Add),
                BOp::Sub => Some(crate::ir::Op::Sub),
                BOp::Mul => Some(crate::ir::Op::Mul),
                _ => None,
            };
            left = match checked {
                // OVERFLOW: arithmetic that signals (RN 2.26).
                Some(o)
                    if self.slots_open()
                        && self.switch_on("OVERFLOW")
                        && (fold(&left).is_none() || fold(&right).is_none()) =>
                {
                    Expr::Special(crate::builtin::Special::Checked(o), vec![left, right])
                }
                _ => Expr::Bin(op, Box::new(left), Box::new(right)),
            };
        }
        Ok(left)
    }

    pub(crate) fn operator(&mut self, level: u8) -> Option<BOp> {
        let t = self.peek();
        let op = match t {
            Tok::Punct('+') => BOp::Add,
            Tok::Punct('-') => BOp::Sub,
            Tok::Punct('*') => BOp::Mul,
            Tok::Punct('/') => BOp::Div,
            Tok::Punct('^') => BOp::Shift,
            Tok::Name(n) => match n.as_str() {
                "MOD" => BOp::Mod,
                "AND" => BOp::And,
                "OR" => BOp::Or,
                "XOR" => BOp::Xor,
                "EQV" => BOp::Eqv,
                n => {
                    let (rel, rest) = n.split_at(n.len().min(3));
                    let rel = match rel {
                        "EQL" => Rel::Eql,
                        "NEQ" => Rel::Neq,
                        "LSS" => Rel::Lss,
                        "LEQ" => Rel::Leq,
                        "GTR" => Rel::Gtr,
                        "GEQ" => Rel::Geq,
                        _ => return None,
                    };
                    match rest {
                        "" => BOp::Rel(rel, false),
                        "U" | "A" => BOp::Rel(rel, true),
                        _ => return None,
                    }
                }
            },
            _ => return None,
        };
        let op_level = match op {
            BOp::Eqv | BOp::Xor => 0,
            BOp::Or => 1,
            BOp::And => 2,
            BOp::Rel(..) => 4,
            BOp::Add | BOp::Sub => 5,
            BOp::Mul | BOp::Div | BOp::Mod => 6,
            BOp::Shift => 7,
        };
        (op_level == level).then_some(op)
    }

    /// Fetch and the signs, then a primary with its calls and fields.
    pub(crate) fn unary(&mut self) -> R<Expr> {
        if self.at_punct('.') {
            let at = self.here();
            self.pos += 1;
            let start = match self.peek().clone() {
                Tok::Name(n) => self.lookup(&n),
                Tok::Bound(_, id) => Some(id),
                _ => None,
            };
            self.address_context += 1;
            let e = self.unary();
            self.address_context -= 1;
            let e = e?;
            self.check_inside(&e, &at);
            self.check_fetch(&e, &at);
            self.lint_fetch(&e, start, &at);
            self.note_use(&e, true);
            return Ok(Expr::Fetch(Box::new(e)));
        }
        if self.eat_punct('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_punct('+') {
            return self.unary();
        }
        let mut e = self.primary()?;
        loop {
            let paren = self.here();
            if self.eat_punct('(') {
                self.call_at = Some(paren);
                let (mut args, mut outs) = (Vec::new(), Vec::new());
                let first = self.here();
                let mut starts = Vec::new();
                if !self.eat_punct(')') {
                    if !self.at_punct(';') {
                        loop {
                            starts.push(self.here());
                            // An actual left out passes 0 (LRM 12.1.1).
                            if self.at_punct(',') || self.at_punct(';') || self.at_punct(')') {
                                self.diag(
                                    'I',
                                    &first,
                                    "Null expression appears in value-required context".into(),
                                );
                                args.push(Expr::Num(0));
                            } else if let Some(f) = self.float_actual()? {
                                args.push(Expr::Special(
                                    crate::builtin::Special::FloatIn(f.0),
                                    vec![f.1],
                                ));
                            } else {
                                self.whole_actual = true;
                                args.push(self.value()?);
                            }
                            if !self.eat_punct(',') {
                                break;
                            }
                        }
                    }
                    if self.eat_punct(';') {
                        loop {
                            outs.push(if self.at_punct(',') || self.at_punct(')') {
                                None
                            } else if let Some(f) = self.float_actual()? {
                                Some(Expr::Special(
                                    crate::builtin::Special::FloatOut(f.0),
                                    vec![f.1],
                                ))
                            } else {
                                Some(self.expr()?)
                            });
                            if !self.eat_punct(',') {
                                break;
                            }
                        }
                    }
                    self.expect_punct(')')?;
                }
                let at = self.last();
                // LINKAGE(address, actuals): a general routine call (LRM 12.2).
                let general = match &e {
                    Expr::Name(id) => match &self.m.syms[*id].kind {
                        Kind::Linkage(l) => Some(l.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if general.is_some() {
                    if args.is_empty() {
                        return self.err("a general routine call needs the routine's address");
                    }
                    e = args.remove(0);
                }
                self.lint_call(&e, &args, &at);
                self.note_call(&e, &args, &at);
                self.check_call(&e, &args, &starts);
                e = self.link_call(e, args, outs, general)?;
            } else if self.at_punct('<') {
                if matches!(e, Expr::Field(..) | Expr::Let(..)) {
                    let at = self.here();
                    self.diag('W', &at, "Two consecutive field selectors".into());
                }
                self.note_use(&e, true);
                let at = self.here();
                self.pos += 1;
                let pos = self.expr()?;
                self.expect_punct(',')?;
                let size = self.expr()?;
                let ext = if self.eat_punct(',') {
                    self.expr()?
                } else {
                    Expr::Num(0)
                };
                self.expect_punct('>')?;
                self.field_at = Some(at);
                e = Expr::Field(Box::new(e), Box::new(pos), Box::new(size), Box::new(ext));
            } else if self.at_punct('[') {
                let attr = match e {
                    Expr::Name(id) => match &self.m.syms[id].kind {
                        Kind::Data {
                            structure: Some(a), ..
                        } => Some((Some(id), a.clone())),
                        _ => None,
                    },
                    _ => None,
                };
                // Else a default structure reference (LRM 11.8).
                let attr = attr.or_else(|| match self.lookup("%STRUCTURE") {
                    Some(d) => match &self.m.syms[d].kind {
                        Kind::DefaultStructure(Some(a)) => Some((None, a.clone())),
                        _ => None,
                    },
                    None => None,
                });
                let Some((id, a)) = attr else {
                    return self.err(
                        "a structure reference needs a name with a structure or a STRUCTURE switch",
                    );
                };
                self.pos += 1;
                let access = self.access_actuals_of(id.map(|id| (id, &a.fields[..])))?;
                let at = self.last();
                self.expect_punct(']')?;
                let seg = if a.refr { Expr::Fetch(Box::new(e)) } else { e };
                e = self.instantiate(&a.st, seg, access, &a.alloc);
                self.check_field_align(&e, &at);
            } else {
                return Ok(e);
            }
        }
    }

    pub(crate) fn primary(&mut self) -> R<Expr> {
        let at = self.here();
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => {
                self.transport("", &at, true);
                string_value(&s)
                    .map(Expr::Num)
                    .ok_or_else(|| self.error_at(&at, "string too long for a value"))
            }
            Tok::Bound(n, id) => self.named(&n, id),
            Tok::Punct('(') => self.block(')'),
            Tok::Name(n) => match n.as_str() {
                "BEGIN" => self.block('E'),
                "CODECOMMENT" => {
                    // CODECOMMENT 'text', ... : block (LRM 4.10): the texts
                    // go in the code as comments, before the block's.
                    let mut texts = Vec::new();
                    loop {
                        let Tok::Str(s) = self.next() else {
                            return self.err("Missing quoted string following CODECOMMENT");
                        };
                        texts.push(String::from_utf8_lossy(&s).into_owned());
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                    self.expect_punct(':')?;
                    let mut es: Vec<Expr> = texts
                        .into_iter()
                        .map(|t| {
                            self.m.comments.push(t);
                            let n = self.m.comments.len() as u32 - 1;
                            Expr::Special(crate::builtin::Special::Comment(n), Vec::new())
                        })
                        .collect();
                    let at = self.here();
                    let block = self.primary()?;
                    if !matches!(block, Expr::Block(..)) {
                        return Err(self.error_at(&at, "CODECOMMENT's text is followed by a block"));
                    }
                    es.push(block);
                    Ok(Expr::Block(es, true))
                }
                "IF" => {
                    let c = self.expr()?;
                    self.lint_test(&c, &at);
                    if let Some(always) = self.constant_test(&c) {
                        let msg = format!("Test expression is always {always}");
                        self.diag('I', &at, msg);
                    }
                    self.expect_or("THEN", "Missing THEN following IF")?;
                    let t = self.expr()?;
                    let e = if self.eat("ELSE") {
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    Ok(Expr::If(Box::new(c), Box::new(t), e))
                }
                "WHILE" | "UNTIL" => {
                    let cond = self.expr()?;
                    self.lint_test(&cond, &at);
                    self.expect_or("DO", "Missing DO following WHILE or UNTIL")?;
                    let body = self.expr()?;
                    Ok(Expr::Loop {
                        until: n == "UNTIL",
                        post: false,
                        cond: Box::new(cond),
                        body: Box::new(body),
                    })
                }
                "DO" => {
                    let body = self.expr()?;
                    let until = if self.eat("UNTIL") {
                        true
                    } else {
                        self.expect_or("WHILE", "Missing WHILE or UNTIL following DO")?;
                        false
                    };
                    let cond_at = self.here();
                    let cond = self.expr()?;
                    self.lint_test(&cond, &cond_at);
                    Ok(Expr::Loop {
                        until,
                        post: true,
                        cond: Box::new(cond),
                        body: Box::new(body),
                    })
                }
                "INCR" | "INCRU" | "INCRA" | "DECR" | "DECRU" | "DECRA" => self.incr(&n),
                "CASE" => self.case(),
                "SELECT" | "SELECTU" | "SELECTA" | "SELECTONE" | "SELECTONEU" | "SELECTONEA" => {
                    self.select(&n)
                }
                "LEAVE" => {
                    let name = self.name()?;
                    let label = match self.lookup(&name) {
                        Some(id) if self.m.syms[id].kind == Kind::Label => id,
                        _ => return self.err(format!("{name} is not a label")),
                    };
                    let v = if self.eat("WITH") {
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    Ok(Expr::Leave(label, v))
                }
                "EXITLOOP" => Ok(Expr::Exitloop(self.optional_value()?)),
                "RETURN" => Ok(Expr::Return(self.optional_value()?)),
                "PLIT" | "UPLIT" => self.plit(n == "PLIT"),
                "%ASCID" => {
                    let at = self.here();
                    self.transport("%ASCID", &at, true);
                    match self.next() {
                        Tok::Str(s) => Ok(Expr::Ascid(s)),
                        _ => self.err("expected a string after %ASCID"),
                    }
                }
                _ if RESERVED.contains(&n.as_str()) => {
                    Err(self.error_at(&at, format!("{n} can't start an expression")))
                }
                _ => match self.lookup(&n) {
                    Some(id) => self.named(&n, id),
                    None if self.is_builtin(&n) => self.builtin(&n),
                    None if n.starts_with('%') => {
                        Err(self.error_at(&at, format!("{n} is not supported yet")))
                    }
                    None => {
                        // As BLISSA64 does: a warning, and an external name,
                        // a routine if it is called.
                        self.diag('W', &at, format!("Undeclared name:  {n}"));
                        let kind = if self.at_punct('(') {
                            Kind::Routine {
                                global: false,
                                external: true,
                                novalue: false,
                                linkage: None,
                            }
                        } else {
                            Kind::Data {
                                storage: Storage::External,
                                bytes: 0,
                                size: self.m.dialect.fullword(),
                                signed: false,
                                structure: None,
                            }
                        };
                        let id = self.m.syms.len();
                        self.m.syms.push(Sym {
                            name: n.clone(),
                            asm: n.clone(),
                            kind,
                        });
                        self.scopes[0].insert(n.clone(), id);
                        // Nothing is known of it: the lint leaves it alone.
                        self.undeclared.push(id);
                        Ok(Expr::Name(id))
                    }
                },
            },
            _ => {
                self.pos -= 1;
                let found = self.describe();
                self.err(format!("expected an expression, found {found}"))
            }
        }
    }

    /// A use of name `n`, declared as `id`.
    pub(crate) fn named(&mut self, n: &str, id: usize) -> R<Expr> {
        match self.m.syms[id].kind.clone() {
            Kind::Literal(v) | Kind::Compiletime(v) => Ok(Expr::Num(v)),
            Kind::Label if self.eat_punct(':') => Ok(Expr::Labeled(id, Box::new(self.expr()?))),
            Kind::Label | Kind::Macro(_) => {
                self.err(format!("Name used in an expression has no value:  {n}"))
            }
            Kind::Structure(st) if self.eat_punct('[') => self.general_ref(st),
            Kind::Builtin => match crate::builtin::register(n) {
                Some(r @ 0..2) => Ok(Expr::Special(crate::builtin::Special::Reg(r), Vec::new())),
                // Declared outside the routine: BLISSA64 warns, and its code
                // faults.
                Some(_) => self.err(format!("Illegal up-level reference to {n}")),
                None => self.builtin(n),
            },
            Kind::Structure(_) | Kind::Field(_) | Kind::FieldSet(_) => {
                self.err(format!("Name used in an expression has no value:  {n}"))
            }
            _ if self
                .local_owner
                .get(&id)
                .is_some_and(|r| Some(r) != self.routine_stack.last()) =>
            {
                self.err(format!("Illegal up-level reference to {n}"))
            }
            _ => {
                let at = self.last();
                self.name_at.insert(id, at);
                Ok(Expr::Name(id))
            }
        }
    }

    /// `%FFLOAT(address)` to `%TFLOAT(address)`, if one is next: an
    /// actual's floating format and the address (RN 2.10.4).
    fn float_actual(&mut self) -> R<Option<(crate::ir::Fl, Expr)>> {
        let Tok::Name(n) = self.peek().clone() else {
            return Ok(None);
        };
        let Some(f) = crate::builtin::float_actual(&n) else {
            return Ok(None);
        };
        self.pos += 1;
        self.expect_punct('(')?;
        let e = self.expr()?;
        self.expect_punct(')')?;
        Ok(Some((f, e)))
    }

    /// An expression, unless the next lexeme ends one.
    pub(crate) fn optional_value(&mut self) -> R<Option<Box<Expr>>> {
        let ends = matches!(self.peek(), Tok::Punct(';' | ')' | ',') | Tok::Eof)
            || ["END", "ELSE", "TES", "THEN", "DO", "WHILE", "UNTIL"]
                .iter()
                .any(|w| self.at(w));
        Ok(if ends {
            None
        } else {
            Some(Box::new(self.expr()?))
        })
    }

    /// A block, after its BEGIN or `(`: declarations, then expressions
    /// separated by semicolons, up to END (`close` 'E') or `)`.
    pub(crate) fn block(&mut self, close: char) -> R<Expr> {
        self.scopes.push(HashMap::new());
        self.depth += 1;
        let outer = std::mem::take(&mut self.inits);
        let r = self.declarations();
        let mut exprs = std::mem::replace(&mut self.inits, outer);
        r?;
        let mut value = false;
        let at_close = |p: &mut Self| {
            if close == 'E' {
                p.at("END")
            } else {
                p.at_punct(')')
            }
        };
        while !at_close(self) {
            // A null expression, between semicolons: a macro that
            // expanded to nothing, as DEC's $RMS_VALFLD_INI may.
            if self.eat_punct(';') {
                value = false;
                continue;
            }
            let m = self.mark();
            match self.expr() {
                Ok(e) => exprs.push(e),
                Err(e) => {
                    self.recover(e, &m)?;
                    value = false;
                    continue;
                }
            }
            value = true;
            if !self.eat_punct(';') {
                break;
            }
            value = false;
        }
        let msg =
            "Missing semicolon, right parenthesis or END following a subexpression of a block";
        if close == 'E' {
            if self.at_punct(')') {
                return self.err("BEGIN paired with right parenthesis");
            }
            self.expect_or("END", msg)?;
        } else if !self.eat_punct(')') {
            if self.at("END") {
                return self.err("Left parenthesis paired with END");
            }
            return self.err(msg);
        }
        // BLISSA64 closes the block in the listing once it has read the
        // lexeme after it.
        self.peek();
        self.depth -= 1;
        self.scopes.pop();
        Ok(Expr::Block(exprs, value))
    }

    pub(crate) fn incr(&mut self, word: &str) -> R<Expr> {
        let name = self.name()?;
        self.scopes.push(HashMap::new());
        let slot = self.slot(8);
        let var = self.declare(
            name,
            Kind::Data {
                storage: Storage::Local(slot),
                bytes: 8,
                size: 8,
                signed: false,
                structure: None,
            },
        )?;
        let part = |p: &mut Self, w: &str| -> R<Option<Box<Expr>>> {
            Ok(if p.eat(w) {
                Some(Box::new(p.expr()?))
            } else {
                None
            })
        };
        let from = part(self, "FROM")?;
        let to = part(self, "TO")?;
        let by = part(self, "BY")?;
        self.expect_or("DO", "Missing DO following INCR or DECR")?;
        let body = Box::new(self.expr()?);
        self.scopes.pop();
        Ok(Expr::Incr {
            var,
            down: word.starts_with("DECR"),
            unsigned: word.len() == 5,
            from,
            to,
            by,
            body,
        })
    }

    pub(crate) fn case(&mut self) -> R<Expr> {
        let sel = self.expr()?;
        self.expect_or("FROM", "Missing FROM following CASE")?;
        let lo = self.ctce()?;
        self.expect_or("TO", "Missing TO following FROM in CASE expression")?;
        let hi = self.ctce()?;
        self.expect_or("OF", "Missing OF following TO in CASE expression")?;
        self.expect_or("SET", "Missing SET following OF in CASE expression")?;
        let mut arms = Vec::new();
        while self.eat_punct('[') {
            let mut labels = Vec::new();
            loop {
                labels.push(if self.eat("INRANGE") {
                    CaseLabel::Inrange
                } else if self.eat("OUTRANGE") {
                    CaseLabel::Outrange
                } else {
                    let a = self.ctce()?;
                    let b = if self.eat("TO") { self.ctce()? } else { a };
                    if a > b {
                        return self.err("Lower bound of a range exceeds upper bound");
                    }
                    if a < lo || b > hi {
                        return self
                            .err("Value in CASE-label outside the range given by FROM and TO");
                    }
                    CaseLabel::Range(a, b)
                });
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(']')?;
            self.expect_punct(':')?;
            arms.push((labels, self.expr()?));
            if !self.eat_punct(';') {
                break;
            }
        }
        self.expect("TES")?;
        Ok(Expr::Case {
            sel: Box::new(sel),
            lo,
            hi,
            arms,
        })
    }

    pub(crate) fn select(&mut self, word: &str) -> R<Expr> {
        let sel = self.expr()?;
        self.expect_or("OF", "Missing OF following SELECT")?;
        self.expect_or("SET", "Missing SET following OF in SELECT expression")?;
        let mut arms = Vec::new();
        while self.eat_punct('[') {
            let mut labels = Vec::new();
            loop {
                labels.push(if self.eat("OTHERWISE") {
                    SelectLabel::Otherwise
                } else if self.eat("ALWAYS") {
                    SelectLabel::Always
                } else {
                    let a = self.expr()?;
                    let b = if self.eat("TO") {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    SelectLabel::Range(a, b)
                });
                if !self.eat_punct(',') {
                    break;
                }
            }
            self.expect_punct(']')?;
            self.expect_punct(':')?;
            arms.push((labels, self.expr()?));
            if !self.eat_punct(';') {
                break;
            }
        }
        self.expect("TES")?;
        Ok(Expr::Select {
            sel: Box::new(sel),
            one: word.starts_with("SELECTONE"),
            unsigned: word.ends_with('U') || word.ends_with('A'),
            arms,
        })
    }

    /// A compile-time constant expression.
    pub(crate) fn ctce(&mut self) -> R<i64> {
        let at = self.here();
        let e = self.expr()?;
        fold(&e).ok_or_else(|| self.error_at(&at, "expected a compile-time constant"))
    }
}

/// A quoted string as a value: its characters from the low byte up, at
/// most a fullword's worth.
fn string_value(s: &[u8]) -> Option<i64> {
    (s.len() <= 8).then(|| s.iter().rev().fold(0i64, |v, &c| (v << 8) | i64::from(c)))
}

/// The value of a constant expression, as the fullword arithmetic gives
/// it, or None.
/// /LANGUAGE's and LANGUAGE's dialects (LRM 18.2, RN 2.26).
pub const LANGUAGES: &[&str] = &[
    "COMMON", "BLISS16", "BLISS32", "BLISS36", "BLISS32M", "BLISS32E", "BLISS64E",
];

/// The on-off switches and whether each is on by default: the LRM's
/// (18.2, 19.2) and the Alpha compilers' (RN 2.8.5, 2.26), with the
/// `/CHECK` options' defaults (RN 2.19.2).
const SWITCH_DEFAULTS: &[(&str, bool)] = &[
    ("ERRS", true),
    ("OPTIMIZE", true),
    ("SAFE", true),
    ("ZIP", false),
    ("CODE", true),
    ("DEBUG", false),
    ("TRACEBACK", true),
    ("UNAMES", false),
    ("OVERFLOW", false),
    ("COUNT", true),
    ("QUAD_LITERALS", true),
    ("ALPHA_REGISTER_MAPPING", false),
    ("CHECK_ADDRESS_TAKEN", false),
    ("CHECK_ALIGNMENT", true),
    ("CHECK_FIELD", true),
    ("CHECK_LONGWORD", false),
    ("CHECK_OPTIMIZE", true),
    ("CHECK_PARAMETERS", false),
    ("CHECK_REDECLARE", false),
    ("CHECK_SHARE", true),
    ("CHECK_SHORT_ADDRESS", false),
    // LIST's options (LRM 18.2): the source part's, as /SOURCE_LIST sets
    // them, and the machine code's, which vbliss's listing doesn't vary.
    ("LIST_SOURCE", true),
    ("LIST_REQUIRE", false),
    ("LIST_EXPAND", false),
    ("LIST_TRACE", false),
    ("LIST_LIBRARY", false),
    ("LIST_OBJECT", true),
    ("LIST_ASSEMBLY", false),
    ("LIST_SYMBOLIC", true),
    ("LIST_BINARY", true),
    ("LIST_COMMENTARY", true),
];

pub fn fold(e: &Expr) -> Option<i64> {
    Some(wrap(match e {
        Expr::Num(n) => *n,
        Expr::Neg(a) => fold(a)?.wrapping_neg(),
        Expr::Not(a) => !fold(a)?,
        Expr::Block(es, true) if es.len() == 1 => fold(&es[0])?,
        Expr::Bin(op, a, b) => binop(*op, fold(a)?, fold(b)?)?,
        Expr::Special(crate::builtin::Special::Checked(o), args) => {
            let op = match o {
                crate::ir::Op::Add => BOp::Add,
                crate::ir::Op::Sub => BOp::Sub,
                _ => BOp::Mul,
            };
            binop(op, fold(&args[0])?, fold(&args[1])?)?
        }
        _ => return None,
    }))
}

/// A binary operation on constants; None for a division by zero.
pub fn binop(op: BOp, a: i64, b: i64) -> Option<i64> {
    Some(match op {
        BOp::Add => a.wrapping_add(b),
        BOp::Sub => a.wrapping_sub(b),
        BOp::Mul => a.wrapping_mul(b),
        BOp::Div => a.checked_div(b).or((b == -1).then(|| a.wrapping_neg()))?,
        BOp::Mod => a.checked_rem(b).or((b == -1).then_some(0))?,
        BOp::Shift => shift(a, b),
        BOp::And => a & b,
        BOp::Or => a | b,
        BOp::Xor => a ^ b,
        BOp::Eqv => !(a ^ b),
        BOp::Rel(rel, unsigned) => {
            let ord = if unsigned {
                (a as u64).cmp(&(b as u64))
            } else {
                a.cmp(&b)
            };
            let yes = match rel {
                Rel::Eql => ord.is_eq(),
                Rel::Neq => ord.is_ne(),
                Rel::Lss => ord.is_lt(),
                Rel::Leq => ord.is_le(),
                Rel::Gtr => ord.is_gt(),
                Rel::Geq => ord.is_ge(),
            };
            yes as i64
        }
    })
}

/// `a ^ n`: left for a positive n, right (arithmetic) for a negative one;
/// 0 or the sign past 63, which BLISS leaves undefined.
pub fn shift(a: i64, n: i64) -> i64 {
    match n {
        0..=63 => a << n,
        -63..=-1 => a >> -n,
        n if n > 0 => 0,
        _ => a >> 63,
    }
}

/// The words syntax level 2 reserves besides the manual's (RN 2.19.11).
const LEVEL2: &[&str] = &[
    "QUAD",
    "SHARED",
    "ALIAS",
    "EXTERNAL_NAME",
    "NOCHECK_ALIGNMENT",
];

/// Words that can't name anything.
pub const RESERVED: &[&str] = &[
    "LINKAGE",
    "ENABLE",
    "BUILTIN",
    "BIND",
    "FIELD",
    "MAP",
    "PLIT",
    "PRESET",
    "INITIAL",
    "REF",
    "REP",
    "STRUCTURE",
    "UPLIT",
    "ALWAYS",
    "AND",
    "BEGIN",
    "BY",
    "CASE",
    "COMPILETIME",
    "DECR",
    "DECRA",
    "DECRU",
    "DO",
    "ELSE",
    "ELUDOM",
    "END",
    "EQL",
    "EQLA",
    "EQLU",
    "EQV",
    "EXITLOOP",
    "EXTERNAL",
    "FORWARD",
    "FROM",
    "GEQ",
    "GEQA",
    "GEQU",
    "GLOBAL",
    "GTR",
    "GTRA",
    "GTRU",
    "IF",
    "INCR",
    "INCRA",
    "INCRU",
    "INRANGE",
    "LABEL",
    "LEAVE",
    "LEQ",
    "LEQA",
    "LEQU",
    "LITERAL",
    "LOCAL",
    "LSS",
    "LSSA",
    "LSSU",
    "MACRO",
    "KEYWORDMACRO",
    "MOD",
    "MODULE",
    "NEQ",
    "NEQA",
    "NEQU",
    "NOT",
    "OF",
    "OR",
    "OTHERWISE",
    "OUTRANGE",
    "OWN",
    "RETURN",
    "ROUTINE",
    "SELECT",
    "SELECTA",
    "SELECTONE",
    "SELECTONEA",
    "SELECTONEU",
    "SELECTU",
    "SET",
    "TES",
    "THEN",
    "TO",
    "UNTIL",
    "WHILE",
    "WITH",
    "XOR",
];
