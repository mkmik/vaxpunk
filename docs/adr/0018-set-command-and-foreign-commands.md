# ADR-0018 — SET COMMAND compiles CLD in DCL into tables DCL looks in first, and a foreign command's image gets its line as $LINE

Oct 5, 2026 · @Marko Mikulicic

Proposed. DCL's `SET COMMAND file` compiles `file.CLD` with
`CDU$COMPILE`, a CLD compiler in MACRO-32 linked into DCL, into command
tables of vcdu's format in DCL's P1 data. `CLI$$DCL_PARSE` takes a
list of such tables, which it looks for the verb in before
`DCL$TABLES`, the last `SET COMMAND`'s first. A symbol whose value is
`$image` is a foreign command: DCL runs the image with a result block
whose `$LINE` is the command, which `LIB$GET_FOREIGN` reads.

## Context

[ADR-0017](0017-command-tables-from-cld-with-vcdu.md) compiles CLD on
the host with vcdu, and left two follow-ups for commands that aren't
DCL's: a native `SET COMMAND`, and `LIB$GET_FOREIGN` for foreign
commands.

On VMS:

- **`SET COMMAND file`** runs CDU, an image. It reads the `.CLD` file
  and adds its verbs to the process's command tables, which DCL keeps
  in P1, replacing those of the same names. `ROUTINE` is only allowed
  with `/OBJECT`: tables in P1 can't hold an image's addresses.
- **A foreign command** is a symbol, `name :== $image`. DCL runs
  `image`, `SYS$SYSTEM:image.EXE` by default, without parsing the rest
  of the line, beyond upcasing it outside quotes.
  `LIB$GET_FOREIGN(get_str [,prompt [,outlen [,force_prompt]]])` asks
  DCL for the line, skips its first word, the verb, and returns the
  rest. With none, it calls `LIB$GET_INPUT` with the prompt, if there
  is one, and upcases the answer.

On vaxpunk an image runs in user mode, and DCL's P1 pages are
supervisor's: an image can't write DCL's tables, and there is no
service for it to ask DCL to. A result block, the only thing DCL hands
an image, has `$VERB` and `$LINE` for every command.

## Decision

1. **The CDU is a module DCL links**, `vms/sysexe/dcl/cdu.mar`,
   which build.rs links into `DCL.EXE` only. `CDU$COMPILE text, table,
   tablen` compiles CLD text into tables in a buffer.
   - It reads what vcdu reads: the same statements, clauses, defaults
     and checks. `ROUTINE` is `CDU$_INVROUT`, as VMS's `SET COMMAND`
     without `/OBJECT`.
   - Errors are VMS CDU's names, `%CDU-E-DUPDEF, duplicate definition
     of X, line 3`, returned with `STS$M_INHIB_MSG`. ponytail: the
     first error ends the compile.
   - It makes the tables in one pass: blocks go where they are
     defined, references to syntaxes and types are filled at the end,
     and the header has room for 64 verb names.
2. **`SET COMMAND file`** is DCL's `CLIROUTINE`, a syntax of `SET`. It
   reads `file.CLD` with RMS into a 16 KB buffer and compiles it into
   the room left of 16 KB of tables in P1. ponytail: 8 files at most,
   and nothing takes them back out.
3. **DCL looks in the process's tables first.** `CLI$$DCL_PARSE` takes
   a fifth argument, a count and the tables' addresses, the last `SET
   COMMAND`'s first, then `DCL$TABLES`. A verb found exactly in one
   table wins; an abbreviation counts the verbs of all the tables, but
   one that a table before has a verb of the same name of, which hides
   it. So redefining `COPY` keeps `COP` unique.
4. **A foreign command's image gets a result block**, which
   `CLI$$FOREIGN line, verb` makes: `$VERB`, the symbol's first 4
   characters, and `$LINE`, the command, upcased outside quotes, as DCL
   upcases it. DCL runs the image with it as it runs a verb's.
   `LIB$GET_FOREIGN`, in `lib/getforeign.mar`, takes `$LINE` past its
   first word with `CLI$GET_VALUE`; it does for any image a verb ran.
   `LIB$GET_INPUT`, in `lib/getinput.mar`, reads `SYS$INPUT` with
   `IO$_READPROMPT`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| `CDU.EXE`, an image, as on VMS | It runs in user mode and can't write DCL's tables. It would need a service, or a file, to hand them to DCL, where linking the module into DCL needs neither. |
| Merge the new verbs into one table, as VMS does | Our tables hold offsets, not names, and merging two means rewriting every offset of one. A list of tables, each as compiled, is a few lines in `FINDVERB`. |
| Have vcdu write tables to a file, which `SET COMMAND` reads | The file has to be made on the host, and a `.CLD` file on vaxpunk couldn't be used at all. |
| The foreign command's line at `VA$C_FOREIGN`, as before ADR-0017 | It would be a second way to pass a command, where the result block already carries `$LINE`, and an image a verb runs would have no line for `LIB$GET_FOREIGN`. |
| Parse a foreign command with a CLD of `$REST_OF_LINE` | `$VERB` would be the CLD's, not the symbol's, and `$LINE` is there for every command anyway. |

## Consequences

**What gets harder.**
- DCL carries a CLD compiler, about 1400 lines of MACRO-32, and 32 KB
  of P1 data for a file and its tables.
- Two compilers read CLD, which must agree. `vrun`'s `cdu` test
  compiles DCL's own CLD, and one with every clause, with both, and
  compares what the tables mean.
- `CLI$$DCL_PARSE` looks at up to 9 tables for each verb.

**What stays easy.**
- A site adds a verb with a `.CLD` file and `SET COMMAND`, without a
  build: `SYS$MANAGER:DCLTEST.CLD` adds `GREET`, which runs `CLITEST`.
- An image takes a foreign command's words, or its own verb's, with
  `LIB$GET_FOREIGN`, as on VMS.

**Follow-ups:**
- `SET COMMAND/DELETE`, `/TABLE` and `/OBJECT`.
- `SET COMMAND`'s tables for the next login, which VMS keeps in
  `DCLTABLES.EXE`.
