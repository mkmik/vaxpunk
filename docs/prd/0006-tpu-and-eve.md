# PRD-0006 — TPU, the Text Processing Utility, and EVE on it, in BLISS-64

Oct 6, 2026 · @Marko Mikulicic

## Context and goal

**Status: started** (replanned Oct 9, 2026). The three things this PRD
waited for are in: BLISS-64 is the system's compiler
([PRD-0004](0004-bliss64-compiler.md) steps 11 and 13: `numtim.b64` in the
executive, and VASM, 6,600 lines of BLISS-64, built by stage 0), each
terminal has a UCB with its size and a read that parses escape sequences
([ADR-0027](../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)),
and there are shareable images
([ADR-0028](../adr/0028-shareable-images.md)). Until EVE is done `EDIT`
stays EDT, `edit.mar`, and its gaps are
[PRD-0003](0003-multi-user-vms.md)'s backlog item 29.

TPU is DEC's Text Processing Utility, from VMS V4.2. It is not an editor
but an interpreter for a small block-structured language whose data types
are made for editing: buffers, ranges, markers, windows, patterns, key
maps. An editor is a program in that language. EVE, the Extensible
Versatile Editor, is the one VMS ships: `EDIT/TPU` loads its section file,
EVE's procedures compiled, and from VMS V5 on `EDIT` with no qualifier
runs it instead of EDT. Users change EVE by writing TPU procedures of
their own in an initialization file, or by compiling a section file.

`edit.mar` is about 1,500 lines of MACRO-32 for EDT's line mode and a
keypad mode. An interpreter with a hundred and fifty built-ins, a screen
manager with windows, and an editor on top of them is an order of
magnitude more code, mostly logic: parsing, a symbol table, string and
pattern handling. That is the code [PRD-0004](0004-bliss64-compiler.md)
says MACRO-32 makes expensive, and DEC wrote TPU in BLISS for the same
reason.

Goals:

- **`EDIT/TPU` runs TPU**, the `TPU` image, with the qualifiers EVE's
  users type: `/NODISPLAY`, `/COMMAND`, `/NOCOMMAND`, `/SECTION`,
  `/NOSECTION`, `/READ_ONLY`, `/OUTPUT`, `/CREATE`.
- **The TPU language,** the subset EVE needs (below), as DEC's *Guide to
  the DEC Text Processing Utility* describes it.
- **EVE, written from scratch in TPU,** with EVE's keys, commands and
  look on a VT100 or later terminal of any size, the console's or a
  TELNET terminal's.
- **`EDIT` switches to TPU** once EVE covers what EDT's tests do. EDT stays
  as `EDIT/EDT`.

Decisions this PRD rests on:

| Decision | Choice |
| --- | --- |
| Language | BLISS-64 ([PRD-0004](0004-bliss64-compiler.md)), compiled by stage 0 as the system's other BLISS-64 is; MACRO-32 only where BLISS-64 can't reach |
| Spec | DEC's *Guide to the DEC Text Processing Utility* and *DEC Text Processing Utility Reference Manual*, and the *Guide to the Extensible Versatile Editor* |
| Oracle | `EDIT/TPU` on OpenVMS Alpha V8.4-2L1 in AXPbox: an oracle for behaviour and screens, never for code |
| EVE | Ours, in TPU, from the manuals and the oracle's behaviour |
| Interpreter | A one-pass compiler from TPU to a bytecode of our own, and a loop that runs it (*The interpreter*) |
| Where it runs | TPU's core calls the system only through an I/O module, so the same objects run under `vrun` on the host, for the tests, and on vaxpunk (*Two I/O modules*) |
| Image | One image, `TPU.EXE`, calling `LIBRTL.EXE`. No `TPU$TPUSHR` yet: a shareable image can't call another one (ADR-0028) |
| Section files | Our own format: the bytecode, the constants and the global symbols, saved and read back |

## Non-goals

- **No copying.** DEC's EVE sources (`EVE$*.TPU` in `SYS$EXAMPLES`) are
  proprietary. They, the TPU images and the manuals are read for
  reference only, as with FreeVMS and the VAX/VMS V4.3 sources
  ([AGENTS.md](../../AGENTS.md)).
- **No DEC section files.** A `.TPU$SECTION` built by DEC's TPU doesn't
  load, and ours don't load in DEC's.
- **No DECwindows.** No `/DISPLAY=DECWINDOWS`, no motif widgets, no mouse.
- **No EDT or WPS keypad emulation** in EVE (`SET KEYPAD EDT`) at first.
  EDT itself is still there.
- **No LSE, no `SPAWN` from the editor, no subprocesses** (`CREATE_PROCESS`)
  until DCL can run one for it.
- **No journaling** (`/JOURNAL`, `/RECOVER`) at first.
- **No callable interface** (`TPU$TPU`, `TPU$INITIALIZE`, ...) and no
  `TPU$TPUSHR` until a shareable image may call another one.
- **No MACRO-32 interim.** No TPU subset in MACRO-32: it would be written
  twice.

## The language

The subset is what EVE uses and a user's initialization file is likely to:

- Procedures with parameters and local variables, `RETURN`, global
  variables and constants, `IF`, `LOOP` with `EXITIF`, `CASE`,
  `ON_ERROR`, and the error and message codes TPU signals.
- Data types: integer, string, buffer, range, marker, window, pattern,
  keyword, key map and key map list, array, learn sequence, program,
  unspecified.
- Built-ins, grouped as the reference manual groups them: buffers
  (`CREATE_BUFFER`, `POSITION`, `COPY_TEXT`, `ERASE`, `SPLIT_LINE`, ...),
  ranges and markers (`CREATE_RANGE`, `MARK`, `BEGINNING_OF`, ...), search
  and patterns (`SEARCH`, `SEARCH_QUIETLY`, `ANCHOR`, `ANY`, `SPAN`,
  `NOTANY`, `LINE_BEGIN`, ...), windows and the screen (`CREATE_WINDOW`,
  `MAP`, `UPDATE`, `REFRESH`, `SET`), keys (`DEFINE_KEY`, `READ_KEY`,
  `LOOKUP_KEY`, `CREATE_KEY_MAP`), files (`READ_FILE`, `WRITE_FILE`,
  `FILE_PARSE`, `FILE_SEARCH`), and the rest (`COMPILE`, `EXECUTE`,
  `MESSAGE`, `GET_INFO`, `SAVE`, `EXIT`, `QUIT`).
- `COMPILE` at run time, so a user's procedures and EVE's `TPU` command
  work.

Which built-ins and which `GET_INFO` and `SET` keywords make the subset
is decided by reading EVE's documented commands down to what they need,
and recorded in a subset note next to TPU's sources.

## The interpreter

TPU compiles what it is given, a command file, a buffer or a string, in
one pass, as a recursive descent parser that emits code as it goes: a
bytecode for a stack machine, with the procedure's constants after it.
A loop runs it. Compared with a tree that an evaluator walks, the
bytecode is three or four times smaller, which counts with EVE's
thousands of lines in 4 MB of memory, and a section file is the bytecode
written out. Values are tagged: a type and an integer or a pointer to a
heap object (a string, a buffer, a range...). Objects are counted
references, freed when the last goes, which is how TPU itself behaves: a
buffer stays until `DELETE` or the last variable naming it.

The compiler and the run-time live in `vms/sysexe/tpu/`, one BLISS-64
module per area (lexer and compiler, run-time and values, strings,
buffers, patterns, screen, keys), linked into `TPU.EXE`.

## Two I/O modules

TPU's core reaches the system only through the routines `fio.r64`
declares ([PRD-0004](0004-bliss64-compiler.md)'s I/O module: files,
memory, the terminal, the command line, exit) and a few for the screen.
Under `vrun`, `crosstools/vtools/bliss/fio.mar` serves them with vrun's
file monitor calls, so `cargo test` runs TPU programs on the host in
seconds, with no boot. On vaxpunk an RMS and `$QIO` module serves them,
which [PRD-0004](0004-bliss64-compiler.md)'s step 16 needs for
`BLISS.EXE` too. The screen's tests feed keys from a file and render the
escape sequences TPU writes with a small VT100 model in the test.

## The screen

TPU paints a terminal of the size its UCB says (`IO$_SENSEMODE`),
redraws only what changed, and reads keys with the terminal driver's
escape sequence parsing (`IO$M_ESCAPE`, `IO$M_NOFILTR`). Windows split
the screen; EVE's are the text window, the status line, the message
window and the command line. VT100 and later, 7-bit controls.

## EVE

The commands of the *Guide to the Extensible Versatile Editor*'s
quick reference that don't need a non-goal: moving, finding and
replacing, `SELECT`, `CUT`, `PASTE`, `INSERT HERE`, `REMOVE`, buffers
(`BUFFER`, `GET FILE`, `INCLUDE FILE`, `WRITE FILE`, `SAVE FILE`),
windows (`TWO WINDOWS`, `ONE WINDOW`, `OTHER WINDOW`), `SET`
commands for margins, tabs, wrap and search case, `LEARN`, `REPEAT`,
`DEFINE KEY`, `TPU`, `HELP`, `EXIT` and `QUIT`. The keypad, the editing
keypad and `Do` are mapped as EVE maps them, with F-key alternatives for
keyboards that lack them.

## Memory

At about 25 bytes of code per line of BLISS-64 (NSLOOKUP's 400 lines are
10 KB of image), TPU's core comes to some 250 KB. The image activator
reads an image whole into nonpaged pool, 512 KB shared with everything
else ([PRD-0003](0003-multi-user-vms.md) backlog item 21), so `TPU.EXE`
needs the activator to read each section straight into its pages
first. EVE's bytecode, its buffers and the screen image go in the
process's `$EXPREG` pages.

## Testing strategy

**Oracle: scripted sessions.** The same keystrokes to `EDIT/TPU` on
AXPbox, on its own disk through `crosstools/ods/vms/run-vms.py`, and to
vaxpunk's TPU, with the file each writes compared, and the final screen
compared as a terminal emulator renders it. The oracle's outputs are
committed, so CI never needs AXPbox.

**Language tests.** TPU programs, run with
`EDIT/TPU/NODISPLAY/NOSECTION/COMMAND=` on the oracle and under `vrun` on
the host, that print what they compute: string and pattern built-ins,
ranges after edits, `ON_ERROR`, `CASE` bounds. Agents generate them by
the hundred, as for BLISS-64.

**On vaxpunk.** A boot test runs a command file with
`EDIT/TPU/NODISPLAY`, and later edits a file with EVE, as the EDT one
does, and types it.

## Open questions

- **Help.** EVE's `HELP` reads a help library; whether `LBR` and the `HELP`
  utility have text by then (PRD-0003 backlog item 27), or EVE's help is
  a buffer of its own.
- **Section file size.** Whether `EDIT/TPU` reads EVE's section file
  into `$EXPREG` pages at each start or maps it, once global sections
  exist.

## Work order

Each step ends with something you can run or look at.

1. **Subset notes.** EVE's commands read down to the built-ins they need.
   *Visible:* the subset note.
2. **Oracle harness.** `tpu-oracle.py`, which runs TPU command files with
   `EDIT/TPU/NODISPLAY` on AXPbox and writes back what they printed and
   the files they wrote. *Visible:* the oracle's outputs for a first test.
3. **Interpreter.** Lexer, compiler, bytecode, procedures, variables,
   control, `ON_ERROR`, integers and strings, `MESSAGE`, under `vrun`.
   *Visible:* the language tests pass on the host against the oracle.
4. **Buffers, ranges and markers.** With `READ_FILE` and `WRITE_FILE`.
   *Visible:* a command file that edits a buffer and writes it matches
   the oracle, under `vrun`.
5. **Patterns and search.** *Visible:* the pattern tests pass.
6. **On vaxpunk.** The image activator reads sections into their pages
   (PRD-0003 item 21), the I/O module's RMS implementation, `TPU.EXE`,
   and `EDIT/TPU` in `edit.cld`. *Visible:* a boot test runs a command
   file with `EDIT/TPU/NODISPLAY`.
7. **Screen and keys.** Windows, `UPDATE`, key maps, `READ_KEY`.
   *Visible:* a ten-line TPU editor in a command file edits a file on the
   screen.
8. **EVE.** The commands above, in TPU. *Visible:* the scripted sessions
   match the oracle's files and screens.
9. **Section files.** `SAVE` and loading at startup. *Visible:* `EDIT/TPU`
   starts EVE from `TPU$SECTION` without compiling it.
10. **`EDIT` is TPU.** DCL's `EDIT` runs TPU, `EDIT/EDT` runs EDT, and the
   boot test edits with EVE. *Visible:* the boot test.
