# vcdu: command tables from CLD

`vcdu` compiles VMS's Command Definition Language into command tables. It
does what VMS's `SET COMMAND/OBJECT` does, as a cross tool
([ADR-0017](../../docs/adr/0017-command-tables-from-cld-with-vcdu.md)).
`CLI$DCL_PARSE`, in `roottask/sysexe/lib/cli.mar`, parses commands with
the tables, and DCL parses every command with `DCL$TABLES`, which
`roottask/build.rs` compiles from `roottask/cld/*.cld`. `HELP.EXE`
(`roottask/sysexe/help.mar`) describes DCL's verbs from the same table.

```
vcdu [/OBJECT=file | -o file] [/MACRO | --macro] FILE.CLD...
```

The files are compiled together into one table, an object module whose
global symbol is the table's start: the `MODULE` name, or the first file's
name. With `/MACRO`, vcdu writes the MACRO-32 source instead, a `.BYTE`
line per 12 bytes under a comment naming each block. The object is that
source compiled by vmacro. The library, `vcdu::compile`, returns the
source; build.rs compiles it with vmacro.

```
vcdu -o copy.obj copy.cld
vcdu --macro -o /dev/stdout roottask/cld/*.cld
```

## The language

VMS's CLD, as CDU reads it. Comments start with `!`. Line ends are
blanks, and a clause's subclauses follow it after commas. Names and
keywords may be given in either case. Strings are in double quotes,
`""` a quote. `=` before a clause's value may be left out. A name is
at most 31 characters, an image's at most 39.

```
MODULE name
IDENT "string"
DEFINE VERB name     clauses
DEFINE SYNTAX name   clauses
DEFINE TYPE name     KEYWORD name [, entity clause]...
```

Verb and syntax clauses:

| Clause | Means |
| --- | --- |
| `IMAGE name` | the image the verb runs, `SYS$SYSTEM:name.EXE` |
| `CLIROUTINE name` | the command interpreter does the verb itself, with its code of that name |
| `SYNONYM name` | another name for the verb |
| `PARAMETER Pn [, clause]...` | a parameter: `P1`, then `P2`..., up to `P8`, required ones first |
| `QUALIFIER name [, clause]...` | a qualifier |
| `DISALLOW expression` | the combinations that are errors: entity names or paths (`SELECT.SIZE`), `NEG name` (negated), `NOT`, `AND`, `OR`, `ANY2(a, b, ...)`, parentheses |
| `NOPARAMETERS`, `NOQUALIFIERS`, `NODISALLOWS` | a syntax inherits none of the verb's |

Entity clauses, for parameters, qualifiers and keywords:

| Clause | Means |
| --- | --- |
| `LABEL=name` | the name `CLI$PRESENT` and `CLI$GET_VALUE` know it by; the entity's name by default |
| `PROMPT="text"` | a parameter's prompt, `_text: `; the label by default |
| `DEFAULT` | present unless negated: `CLI$_DEFAULTED` |
| `NEGATABLE`, `NONNEGATABLE` | `/NOname` allowed or not. Qualifiers are negatable, keywords not, unless they say otherwise |
| `SYNTAX=name` | given (not negated), the command is parsed again with that syntax |
| `VALUE [(clauses)]` | it takes a value: `REQUIRED`, `LIST`, `[NO]CONCATENATE` (by default as `LIST`), `DEFAULT="text"`, `TYPE=type` |

A `TYPE` is a `DEFINE TYPE`, whose keywords the value must be, or one
of VMS's built-in types:

| Type | Value |
| --- | --- |
| `$FILE`, `$INFILE`, `$OUTFILE`, `$OUTLOG` | a file specification |
| `$NUMBER` | an integer: digits, after `-`, or `%X`, `%O`, `%D` |
| `$REST_OF_LINE` | the rest of the line, as it is |
| `$QUOTED_STRING` | a string, its quotes, and `""` in it, kept |
| `$DATETIME`, `$DELTATIME`, `$ACL`, `$EXPRESSION`, `$PARENTHESIZED_VALUE` | taken as a word, unchecked |

A syntax takes from the command that switched to it whatever it lacks:
the image or routine (if it has neither), the parameters and the
qualifiers (if it has none, and doesn't say `NO`), and `DISALLOW`.

vcdu reports an error with the file and line, as `%CDU-E-SYNTAX` or
`%CDU-E-INVDEF`, and makes no table. Such errors include an undefined
type or syntax, a name defined twice, parameters out of order, or a
required parameter after an optional one. Clauses it doesn't implement
are errors, not ignored: `ROUTINE`, `PLACEMENT=LOCAL` and `POSITIONAL`,
`CLIFLAGS`, `OUTPUTS`, `PREFIX`, `IMPCAT`, a list of default values.
`BATCH` goes in the tables, but nothing runs in batch yet.

## The tables

Bytes, with no addresses in them: the tables may be moved, or read from a
file. A string is `.ASCIC`. An offset is a little-endian word from the
table's first byte. The blocks are in the order the listing shows; only
the header's place is fixed.

The header:

| Offset | Size | What |
| --- | --- | --- |
| 0 | word | the format's version, 1 |
| 2 | word | n, how many verb names: verbs and synonyms |
| 4 | 4n | for each, in alphabetical order, the offset of its name and of its command block |

A command block, a verb's or a syntax's:

| Field | What |
| --- | --- |
| `.ASCIC` | its name |
| `.ASCIC` | its `IMAGE`, or empty |
| `.ASCIC` | its `CLIROUTINE`, or empty |
| byte, words | how many parameters, then each one's entity; 255 and no words: inherited |
| byte, words | the qualifiers, the same way |
| word | its `DISALLOW` expression; 0 inherits, 1 is none |

An entity, a parameter, qualifier or keyword:

| Offset | Size | What |
| --- | --- | --- |
| 0 | byte | flags: 1 `DEFAULT`, 2 negatable, 4 takes a value, 8 the value is required, 16 `LIST`, 32 concatenates, 64 `BATCH` |
| 1 | byte | the value's type: 0 a word, 1 a file, 2 `$NUMBER`, 3 `$REST_OF_LINE`, 4 `$QUOTED_STRING`, 5 keywords, 6 the other built-in types |
| 2 | word | for type 5, the type's block |
| 4 | word | its `SYNTAX=`'s command block, or 0 |
| 6 | `.ASCIC` ×4 | its name, label, prompt and default value |

A type's block: a byte, how many keywords, then a word for each, its
entity.

A `DISALLOW` expression, in prefix form: 1 and an `.ASCIC` path, an
entity present; 2 and a path, negated; 3 and an expression, `NOT`; 4 or 5
and two expressions, `AND` or `OR`; 6, a byte n and n expressions, `ANY2`.
Several `DISALLOW` clauses are one, ORed.
