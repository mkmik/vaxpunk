//! Runs CDU$COMPILE, the CLD compiler that DCL's SET COMMAND uses
//! (roottask/sysexe/dcl/cdu.mar), under vrun, and checks that the tables
//! it makes mean what vcdu's mean: the same verbs, syntaxes, entities and
//! DISALLOWs, wherever in the tables they are. Then the errors it reports.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// CDU$COMPILE's caller: compiles the CLD at CLDDSC and writes the tables,
/// or exits with the error.
const DUMP: &str = "
        .TITLE  DUMP    Compiles CLDDSC's CLD, and writes the tables
        .LIBRARY \"vrun.mlb\"
        .EXTERNAL CDU$COMPILE, CLDDSC
        .PSECT  DUMP_DATA, NOEXE, WRT, LONG
TABDSC: .LONG   65535
        .ADDRESS TABLE
TABLEN: .WORD   0
TABLE:  .BLKB   65535
        .PSECT  DUMP_CODE, EXE, LONG
        .ENTRY  START, ^M<R2>
        PUSHAW  TABLEN
        PUSHAQ  TABDSC
        PUSHAQ  G^CLDDSC
        CALLS   #3, G^CDU$COMPILE
        BLBC    R0, 10$
        MOVAB   TABLE, R1
        MOVZWL  TABLEN, R2
        $WRITE  x1, x19  ; R1, R2
        MOVL    #1, R0
10$:    RET
        .END    START
";

/// PUT_LINE, which writes with vrun's put.
const STUB: &str = "
        .TITLE  STUB    PUT_LINE, under vrun
        .LIBRARY \"vrun.mlb\"
        .PSECT  STUB_DATA, NOEXE, WRT, LONG
NL:     .ASCII  <10>
        .PSECT  STUB_CODE, EXE, LONG
        .ENTRY  PUT_LINE, ^M<R2>
        MOVL    4(AP), R1
        MOVZBL  (R1)+, R2
        $WRITE  x1, x19  ; R1, R2
        MOVAB   NL, R1
        MOVL    #1, R2
        $WRITE  x1, x19  ; R1, R2
        RET
        .END
";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn compile(name: &str, source: &str) -> (String, Vec<u8>) {
    let opts = vasm::Options {
        name: name.into(),
        include: vec![root().join("vtools/lib")],
        ..Default::default()
    };
    let object = vmacro::compile(source, &opts).unwrap_or_else(|d| {
        panic!(
            "{name}: {:?}",
            d.iter()
                .map(|d| format!("{}: {}", d.line, d.msg))
                .collect::<Vec<_>>()
        )
    });
    (name.to_string(), vms_obj::obj::write(&object.records))
}

/// CDU$COMPILE's tables for `cld`, or the messages it wrote.
fn native(cld: &str) -> Result<Vec<u8>, String> {
    let bytes: Vec<String> = cld.bytes().map(|b| b.to_string()).collect();
    let text = format!(
        "\t.TITLE\tCLD\n\t.PSECT\tCLD_DATA, NOEXE, NOWRT, LONG\nCLDDSC::\n\t.LONG\t{}\n\t.ADDRESS CLD\nCLD:\n{}\t.END\n",
        cld.len(),
        bytes
            .chunks(16)
            .map(|c| format!("\t.BYTE\t{}\n", c.join(", ")))
            .collect::<String>()
    );
    let cdu = fs::read_to_string(root().join("roottask/sysexe/dcl/cdu.mar")).unwrap();
    let modules = [
        compile("DUMP", DUMP),
        compile("CDU", &cdu),
        compile("CLD", &text),
        compile("STUB", STUB),
    ];
    let opts = vlink::Options {
        base: vlink::DEFAULT_BASE,
        name: "DUMP".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
    };
    let linked = vlink::link(&modules, &opts).unwrap();
    // The tests run at once: an image each.
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cdudump{n}.exe"));
    fs::write(&exe, linked.image.write()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .arg(&exe)
        .output()
        .unwrap();
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr))
    }
}

/// vcdu's tables for `cld`.
fn vcdu(cld: &str) -> Vec<u8> {
    let source = vcdu::compile("T", &[("t.cld".into(), cld.into())]).unwrap();
    source
        .lines()
        .filter_map(|l| l.trim().strip_prefix(".BYTE"))
        .flat_map(|l| l.split(',').map(|b| b.trim().parse::<u8>().unwrap()))
        .collect()
}

/// What a table means: its verbs and the syntaxes they lead to, by name.
#[derive(Debug, PartialEq)]
struct Meaning {
    verbs: BTreeMap<String, Cmd>,
    syntaxes: BTreeMap<String, Cmd>,
}

#[derive(Debug, PartialEq)]
struct Cmd {
    image: String,
    cliroutine: String,
    params: Option<Vec<Ent>>,
    quals: Option<Vec<Ent>>,
    /// 0 inherits, 1 none, else the expression.
    disallow: Result<Vec<u8>, u16>,
    routine: u16,
}

#[derive(Debug, PartialEq)]
struct Ent {
    flags: u8,
    ty: u8,
    keywords: Option<Vec<Ent>>,
    syntax: Option<String>,
    placement: u8,
    /// Name, label, prompt and default value.
    strings: Vec<String>,
}

struct Reader<'a> {
    t: &'a [u8],
    syntaxes: BTreeMap<String, Cmd>,
}

impl Reader<'_> {
    fn word(&self, at: usize) -> usize {
        u16::from_le_bytes([self.t[at], self.t[at + 1]]) as usize
    }

    fn ascic(&self, at: usize) -> (String, usize) {
        let n = self.t[at] as usize;
        (
            String::from_utf8_lossy(&self.t[at + 1..at + 1 + n]).into_owned(),
            at + 1 + n,
        )
    }

    fn name(&self, block: usize) -> String {
        self.ascic(block).0
    }

    fn command(&mut self, at: usize) -> Cmd {
        let (_, at) = self.ascic(at);
        let (image, at) = self.ascic(at);
        let (cliroutine, at) = self.ascic(at);
        let (params, at) = self.list(at);
        let (quals, at) = self.list(at);
        let d = self.word(at);
        let disallow = if d < 2 {
            Err(d as u16)
        } else {
            Ok(self.t[d..d + self.expr(d)].to_vec())
        };
        Cmd {
            image,
            cliroutine,
            params,
            quals,
            disallow,
            routine: self.word(at + 2) as u16,
        }
    }

    fn list(&mut self, at: usize) -> (Option<Vec<Ent>>, usize) {
        let n = self.t[at] as usize;
        if n == 255 {
            return (None, at + 1);
        }
        let list = (0..n)
            .map(|i| {
                let e = self.word(at + 1 + 2 * i);
                self.entity(e, 0)
            })
            .collect();
        (Some(list), at + 1 + 2 * n)
    }

    fn entity(&mut self, at: usize, depth: usize) -> Ent {
        assert!(depth < 8, "types too deep");
        let ty = self.t[at + 1];
        let keywords = (ty == 5).then(|| {
            let k = self.word(at + 2);
            (0..self.t[k] as usize)
                .map(|i| {
                    let e = self.word(k + 1 + 2 * i);
                    self.entity(e, depth + 1)
                })
                .collect()
        });
        let s = self.word(at + 4);
        let syntax = (s != 0).then(|| {
            let name = self.name(s);
            if !self.syntaxes.contains_key(&name) {
                // Mark it, then read it: a syntax may lead back to itself.
                self.syntaxes.insert(name.clone(), Cmd::placeholder());
                let cmd = self.command(s);
                self.syntaxes.insert(name.clone(), cmd);
            }
            name
        });
        let mut strings = Vec::new();
        let mut p = at + 7;
        for _ in 0..4 {
            let (s, next) = self.ascic(p);
            strings.push(s);
            p = next;
        }
        Ent {
            flags: self.t[at],
            ty,
            keywords,
            syntax,
            placement: self.t[at + 6],
            strings,
        }
    }

    /// The length of the DISALLOW expression at `at`.
    fn expr(&self, at: usize) -> usize {
        match self.t[at] {
            1 | 2 => 2 + self.t[at + 1] as usize,
            3 => 1 + self.expr(at + 1),
            4 | 5 => {
                let a = self.expr(at + 1);
                1 + a + self.expr(at + 1 + a)
            }
            6 => {
                let mut len = 2;
                for _ in 0..self.t[at + 1] {
                    len += self.expr(at + len);
                }
                len
            }
            op => panic!("operator {op} at {at}"),
        }
    }
}

impl Cmd {
    fn placeholder() -> Cmd {
        Cmd {
            image: String::new(),
            cliroutine: String::new(),
            params: None,
            quals: None,
            disallow: Err(0),
            routine: 0,
        }
    }
}

fn meaning(t: &[u8]) -> Meaning {
    let mut r = Reader {
        t,
        syntaxes: BTreeMap::new(),
    };
    assert_eq!(r.word(0), 2, "the version");
    let mut verbs = BTreeMap::new();
    for i in 0..r.word(2) {
        let name = r.ascic(r.word(4 + 4 * i)).0;
        let cmd = r.command(r.word(6 + 4 * i));
        verbs.insert(name, cmd);
    }
    Meaning {
        verbs,
        syntaxes: r.syntaxes,
    }
}

/// CDU$COMPILE and vcdu must agree on what `cld` means.
fn same(cld: &str) {
    let ours = native(cld).unwrap_or_else(|e| panic!("CDU$COMPILE failed: {e}"));
    assert_eq!(meaning(&ours), meaning(&vcdu(cld)));
}

#[test]
fn dcl_tables() {
    let dir = root().join("roottask/cld");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    let all: String = paths
        .iter()
        .map(|p| fs::read_to_string(p).unwrap() + "\n")
        .collect();
    same(&all);
}

#[test]
fn every_clause() {
    same(
        "module MYTAB ident \"V1\" ! a comment
         define verb COPY
             image \"COPY\"
             synonym DUPLICATE, synonym CP
             parameter P1, label=FROM, prompt=\"From\",
                 value(required, list, type=$infile)
             parameter P2, value(default=\"x\"\"y\", noconcatenate, list)
             qualifier LOG, default, batch
             qualifier BY, nonnegatable, value(type=BY_TYPE)
             qualifier COPIES, placement=local, value(type=$number)
             qualifier CONFIRM, placement=positional, negatable
             qualifier OTHER, syntax=COPY_OTHER, label=ELSE
             disallow LOG and not (BY.SIZE or NEG CONFIRM)
             disallow any2(LOG, BY, OTHER) and P2
         define type BY_TYPE
             keyword NAME, default
             keyword SIZE, value(required, type=$number), negatable,
             keyword WHAT, syntax=COPY_OTHER
         define syntax COPY_OTHER
             cliroutine OTHER
             noparameters
             nodisallows
         define syntax COPY_MORE
             noqualifiers
             parameter P1, value(concatenate)
         define verb SHOW
             cliroutine SHOW
             parameter P1, value(type=$rest_of_line)",
    );
}

#[test]
fn errors() {
    let fails = |cld: &str, msg: &str| {
        let e = native(cld).expect_err(cld);
        assert!(e.contains(msg), "{cld}: {e}");
    };
    fails(
        "define verb A\n  image A\n  parameter P2",
        "%CDU-E-INVPARM, parameters must be P1 to P8, in order P2, line 3",
    );
    fails(
        "define verb A\n  qualifier X, value(type=NOSUCH)",
        "%CDU-E-UNDEFTYPE, type is not defined NOSUCH, line 2",
    );
    fails(
        "define verb A\n  qualifier X, syntax=NOSUCH",
        "%CDU-E-UNDEFSYNTAX",
    );
    fails("define verb A\ndefine verb A", "%CDU-E-DUPDEF");
    fails("define verb A, synonym A", "%CDU-E-DUPDEF");
    fails("define type T\ndefine type T", "%CDU-E-DUPDEF");
    fails(
        "define verb A\n qualifier X\n qualifier X",
        "%CDU-E-DUPQUAL",
    );
    fails(
        "define verb A\n parameter P1\n parameter P2, value(required)",
        "%CDU-E-INVREQPARM",
    );
    fails("define verb A\n  routine A_ROUTINE", "%CDU-E-INVROUT");
    fails(
        "define verb A\n  image A\n  cliroutine A",
        "%CDU-E-CONFROUTIMG",
    );
    fails(
        "define verb A\n  qualifier X, placement=local, value(type=T)\ndefine type T",
        "%CDU-E-INVPLACE",
    );
    fails(
        "define verb A\n  parameter P1, placement=local",
        "%CDU-E-INVITEM",
    );
    fails(
        "define verb A\n  qualifier X, value(type=$NOSUCH)",
        "%CDU-E-INVTYPE",
    );
    fails(
        "define verb A\n  image \"A_VERY_LONG_IMAGE_NAME_THAT_GOES_ON_AND_ON\"",
        "%CDU-E-IMAGELEN",
    );
    fails(
        "define verb A_NAME_THAT_IS_LONGER_THAN_31_CHARS",
        "%CDU-E-SYMTOOLONG",
    );
    fails(
        "define verb A\n  frobnicate",
        "%CDU-E-INVITEM, invalid item FROBNICATE, line 2",
    );
    fails("define verb A\n  image \"x", "%CDU-E-MISSQUOTE");
    fails("define verb A\n  image #", "%CDU-E-INVCHAR");
    fails("define thing A", "%CDU-E-INVDEFINE");
}
