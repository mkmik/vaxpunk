# ADR-0017 — Commands are defined in CLD files that vcdu compiles at build time, and a parser library linked into each image reads them

Oct 4, 2026 · @Marko Mikulicic

Proposed. Verbs, their parameters and qualifiers are defined in VMS's
Command Definition Language. `vcdu`, a cross tool in vtools, compiles
`.CLD` files into command tables, an object module, as `SET
COMMAND/OBJECT` does. DCL links `DCL$TABLES`, the table of all its
verbs.
`CLI$DCL_PARSE` parses a command against a table, and `CLI$PRESENT`
and `CLI$GET_VALUE` read the result. They are in `CLI`, a library module
that DCL and every image link. DCL passes the image the result of the
parse, which `$IMGACT` copies to the top of its user stack.

## Context

DCL knows each verb by hand. Its loop compares the verb with each name in
turn, and the first that matches wins. A verb that runs an image passes
the rest of the line as the command line at `VA$C_FOREIGN`, in capitals,
a blank between each two words. The image takes words from it with
`GET_PARAM`. Nothing knows a qualifier: `COPY`, `DELETE`, `MOUNT` and
`INIT` each say "no qualifiers", and `DELETE/SYMBOL` works only because
the verb is read up to a blank. [ADR-0006](0006-cli-in-p1-runs-images-in-its-process.md)
left qualifiers as a follow-up.

On VMS, the Command Definition Utility (CDU) compiles `.CLD` files.

- **Definitions.** `DEFINE VERB` gives a verb's `IMAGE` or `ROUTINE`,
  its `PARAMETER`s `P1` to `P8` and its `QUALIFIER`s, with clauses for
  `LABEL`, `PROMPT`, `DEFAULT`, `[NON]NEGATABLE`, `PLACEMENT`, and
  `VALUE(REQUIRED, LIST, DEFAULT=, TYPE=)`.
- **Keywords and variants.** `DEFINE TYPE` gives keywords, which a value
  may be. `DEFINE SYNTAX` defines another form of a verb, which a
  keyword or a qualifier switches to with `SYNTAX=`, as `SHOW DEVICES`
  does. `DISALLOW` rejects combinations.
- **Where the tables go.** `SET COMMAND` adds the tables to the process's
  own copy in P1, or to an image of tables. `SET COMMAND/OBJECT` writes
  an object module whose global symbol is the `MODULE` name.
  `DCLTABLES.EXE`, which `LOGINOUT` maps into P1, holds DCL's tables.
  A utility such as `MAIL` links its own and parses each line it reads
  with `CLI$DCL_PARSE(line, MAIL$COMMAND_TABLE, prompt)`, then calls
  `CLI$DISPATCH`.
- **How DCL parses.**
  - A verb or a qualifier is matched on its first 4 characters, and a
    shorter abbreviation must be unique.
  - `/NOname` negates a negatable qualifier. A value follows `=` or `:`,
    and a list is in parentheses.
  - `,` separates a parameter's values when it is a `LIST`, and `+` when
    it concatenates.
  - A missing required parameter is prompted for (`_From: `), but only at
    the terminal.
  - Each error is a CLI status: `CLI$_IVQUAL`, `CLI$_NOTNEG`,
    `CLI$_VALREQ`, `CLI$_MAXPARM`, `CLI$_INSFPRM`, `CLI$_CONFLICT`.
    DCL reports it as `%DCL-W-IVQUAL, unrecognized qualifier - check
    validity, spelling, and placement`, followed by ` \NAME\`.
- **How an image asks.** `CLI$PRESENT(name)` returns `CLI$_PRESENT`,
  `CLI$_DEFAULTED`, `CLI$_NEGATED` or `CLI$_ABSENT`.
  `CLI$GET_VALUE(name, buffer)` returns the values one at a time:
  `CLI$_COMMA` or `CLI$_CONCAT` while more follow, `SS$_NORMAL` for the
  last, then `CLI$_ABSENT`.
  - A name is the entity's label, which defaults to its name (`P1`,
    `LOG`). A keyword is reached by a path, `SELECT.SIZE`. `$VERB` and
    `$LINE` are reserved.
  - A name the tables don't define is a fatal error, `CLI$_ENTNF`.
- **Where the parse lives.** The `CLI$` routines run in user mode. DCL's
  P1 pages are supervisor owned but user readable, and the routines read
  the parse straight from them; only DCL's own services, such as symbols
  and `SPAWN`, change mode with `CHMS`. Standalone images that run
  without DCL link DCL's parser instead.

DEC built `DCLTABLES` on VMS, with CDU running as `SET COMMAND`. FreeVMS
compiles its CLDs on the host with a flex and bison CDU, and links the
tables into DCL as an object.

vaxpunk has no compiler or linker of its own yet. `roottask/build.rs`
assembles and links every image on the host, and writes them to the
system disk.

## Decision

1. **`vcdu`, a vtools crate, compiles CLD.** Its library turns `.CLD`
   sources into MACRO-32 source for one table; build.rs compiles that
   with vmacro, as it does `SYS.STB`. The `vcdu` command writes the
   object module (or, with `/MACRO`, the source), as `SET
   COMMAND/OBJECT` does. The table's global symbol is the `MODULE` name,
   or the first file's name. vcdu checks what it can at build time:
   unknown syntaxes and types, names defined twice, parameters not named
   `P1` to `Pn` in order. It refuses a clause it doesn't implement yet,
   rather than ignoring it.
2. **Table format.** The tables are bytes with no addresses in them:
   strings are `.ASCIC`, and blocks point to each other with word
   offsets from the table's start. They are documented in
   `vtools/docs/command-tables.md`. A `ROUTINE`, which needs an
   address, is a longword after the table's bytes, which the linker
   fills, and which `CLI$DISPATCH` calls.
3. **Every DCL verb is CLD.** `roottask/cld/*.cld` compile into one
   table, `DCL$TABLES`, linked into `DCL.EXE`.
   - DCL parses every command with it, after symbols and labels, so one
     set of rules matches each verb: 4 characters, or fewer if unique.
   - A verb DCL does itself names its code with `CLIROUTINE`, as on VMS.
     DCL dispatches on that name.
   - A verb with an `IMAGE` runs it with the parse.
   - Variants are `SYNTAX=`: `DELETE/SYMBOL` is a syntax of `DELETE`
     with its own `CLIROUTINE`, and `SHOW PROCESS` a keyword whose
     syntax has `IMAGE SHOW`.
   - An image's own CLD, `sysexe/NAME.cld`, is linked into `NAME.EXE`
     for `CLI$DCL_PARSE`.
   - `HELP` is an image, as on VMS, linked with `DCL$TABLES`, which
     describes the verbs from them: their parameters, keywords,
     qualifiers and syntaxes.
4. **`CLI`, a library module** in `roottask/sysexe/lib/cli.mar`, is
   linked into every image on the system disk, as `PUT_LINE` is, and
   into DCL, in supervisor mode. It has:
   - **`CLI$DCL_PARSE(line, table [,param_routine])`.** It parses a
     command against a table, with VMS's rules above. It prompts for a
     missing required parameter only when given `param_routine`, a
     `LIB$GET_INPUT`. DCL passes one at the console, not in command
     procedures, as VMS does. It reports an error as DCL does, with the
     ` \segment\` line, and returns the status with `STS$M_INHIB_MSG`.
   - **`CLI$PRESENT` and `CLI$GET_VALUE`,** VMS's, over the result of
     the last parse in this image or, if there was none, DCL's.
   - **`CLI$$RESULT`, `CLI$$IMAGE` and `CLI$$ROUTINE`,** for DCL: the
     result block, and the image or routine of the verb or syntax that
     was parsed.
5. **The result is one block** of bytes with no addresses in it. For
   each entity of the verb, given or not, it holds the label path, the
   state (present, negated, defaulted or absent) and the values, each
   with the separator that followed it. `$VERB` and `$LINE` come first.
   - A name not in the block is `CLI$_ENTNF`. The routine reports it as
     VMS's `%CLI-F-SYNTAX` would, and the image exits with it, since
     there are no condition handlers to signal to.
6. **DCL hands the block to the image.** `$IMGACT image, cmdlin` takes
   the block as `cmdlin`, up to `VA$C_CLI_RESMAX`, 2 KB, and copies it
   to `VA$C_CLI_RESULT`, at the top of the user stack, where
   `VA$C_FOREIGN`'s command line was. There the image's `CLI` finds it.
   `GET_PARAM` goes.

## Alternatives considered

| Option | Why not |
| --- | --- |
| CDU on vaxpunk, as `SET COMMAND`, compiling DCL's tables at first boot or on the target | Nothing on vaxpunk can build anything yet, and DCL needs its tables before anything runs. The tables' format allows a native `SET COMMAND` later, writing or loading the same bytes. |
| Keep the verbs and their words in DCL's code, adding qualifiers by hand | Each verb would grow its own parser, and images would still guess at their command line. Every later utility (`MAIL`-like prompts, `SET`, `SHOW`) would repeat it. |
| `CLI$PRESENT` and `CLI$GET_VALUE` call DCL in supervisor mode, with a `$CLI` service | VMS doesn't for these; they read DCL's pages in user mode. A mode change per call needs a per-process `CHMS` handler that supervisor mode can read, which there is no P1 control region for, and an image that parses its own commands would need DCL. |
| Map DCL's result pages user readable, as VMS does, and let the image read them in place | The image activator maps a command interpreter supervisor-only, and there is no per-psect protection in the image format. A copy of at most 2 KB per image is cheap, and keeps DCL's data out of the image's sight. |
| The tables as a separate `DCLTABLES.EXE` that DCL reads at startup | Nothing changes the tables without a build yet. Linking them into DCL is one less file and one less failure at login. The offsets-only format keeps this open. |
| A parser written in Rust on the host, passing images a pre-parsed form | Commands are typed on vaxpunk, after symbol substitution; the parse has to run there. |
| vcdu emitting VMS's own table format (CLITABDEF) | More than we need (MCR flags, 4-character verb vectors, block headers for in-place editing), and not documented outside DEC's sources, which we must not copy. Ours is smaller and documented. |

## Consequences

**What gets harder.**
- Every image that takes parameters needs a CLD definition, and
  `CLI$GET_VALUE` calls instead of reading a string.
- The parser is a large MACRO-32 module that every image links. ponytail:
  it is linked in, not shared: there are no shareable images yet.
- `$IMGACT`'s command line is no longer text: anything that called it
  with text, other than DCL, must build a block (`RUN` passes none).
- Abbreviations follow VMS: `D` is ambiguous now, where DCL took the
  first verb that matched.
- The verbs DCL does itself take their values from the parse, but an
  expression, `IF`'s or `WRITE`'s, is one `$REST_OF_LINE` value that
  DCL's own code reads: expressions aren't CLD.
- The parse result is limited to 2 KB, and a command whose result
  doesn't fit is `CLI$_BUFOVF`.

**What stays easy.**
- Adding a verb, or a qualifier to one, is a CLD change and the image's
  `CLI$PRESENT`. DCL's code doesn't change.
- An image that parses its own command lines (an editor's, a utility's
  `SUBCOMMAND>` prompt) uses the same parser with its own table.
- Error messages are VMS's, from one place.
- `CLITEST` checks the parser on the system, and under vrun in CI, which
  can't build the system.

**Follow-ups:**
- Done: `PLACEMENT=LOCAL` and `POSITIONAL`, with `CLI$_LOCPRES` and
  `CLI$_LOCNEG`; `ROUTINE` and `CLI$DISPATCH`, in the tables' version
  2. A native `SET COMMAND`, and `LIB$GET_FOREIGN` for foreign
  commands, `name := $image`, are
  [ADR-0018](0018-set-command-and-foreign-commands.md).
- Text for `HELP`, from a help library, as VMS's `HELPLIB.HLB`.
