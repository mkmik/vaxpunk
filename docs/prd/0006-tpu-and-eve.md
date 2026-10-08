# PRD-0006 — TPU, the Text Processing Utility, and EVE on it, in BLISS-64

Oct 6, 2026 · @Marko Mikulicic

## Context and goal

**Status: waiting.** Nothing here gets built until the BLISS-64 compiler of
[PRD-0004](0004-bliss64-compiler.md) is the system's compiler (its work
order, step 10). Until then `EDIT` stays EDT, `edit.mar`, and its gaps
are [PRD-0003](0003-multi-user-vms.md)'s backlog item 29.

TPU is DEC's Text Processing Utility, from VMS V4.2. It is not an editor
but an interpreter for a small block-structured language whose data types
are made for editing: buffers, ranges, markers, windows, patterns, key
maps. An editor is a program in that language. EVE, the Extensible
Versatile Editor, is the one VMS ships: `EDIT/TPU` loads its section file,
EVE's procedures compiled, and from VMS V5 on `EDIT` with no qualifier
runs it instead of EDT. Users change EVE by writing TPU procedures of
their own in an initialization file, or by compiling a section file.

`edit.mar` is about 1,500 lines of MACRO-32 for EDT's line mode and a
fixed 24 by 80 keypad mode. An interpreter with a hundred and fifty
built-ins, a screen manager with windows, and an editor on top of them is
an order of magnitude more code, mostly logic: parsing, a symbol table,
string and pattern handling. That is the code
[PRD-0004](0004-bliss64-compiler.md) says MACRO-32 makes expensive, and
DEC wrote TPU in BLISS for the same reason. So TPU waits for BLISS-64.

Goals:

- **`EDIT/TPU` runs TPU.** The `TPU` image, `TPU$TPUSHR` as a shareable
  image, and `TPU$TPU` as the callable interface, as on VMS.
- **The TPU language,** the subset EVE needs (below), as DEC's *Guide to
  the DEC Text Processing Utility* describes it.
- **EVE, written from scratch in TPU,** with EVE's keys, commands and
  look on a VT100 or later terminal of any size.
- **`EDIT` switches to TPU** once EVE covers what EDT's tests do. EDT stays
  as `EDIT/EDT`.

Decisions this PRD rests on:

| Decision | Choice |
| --- | --- |
| Language | BLISS-64 ([PRD-0004](0004-bliss64-compiler.md)); MACRO-32 only where BLISS-64 can't reach |
| Waits for | PRD-0004's pilot (work order step 10), and the per-terminal UCB of PRD-0003 (backlog item 3) for the terminal's size |
| Spec | DEC's *Guide to the DEC Text Processing Utility* and *DEC Text Processing Utility Reference Manual*, and the *Guide to the Extensible Versatile Editor* |
| Oracle | `EDIT/TPU` on OpenVMS Alpha V8.4-2L1 in AXPbox: an oracle for behaviour and screens, never for code |
| EVE | Ours, in TPU, from the manuals and the oracle's behaviour |
| Section files | Our own format: TPU's compiled procedures and variables, saved and mapped back |

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
- **No MACRO-32 interim.** No TPU subset in MACRO-32 before the compiler
  exists: it would be written twice.

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

## The screen

TPU paints a terminal of the size the terminal's characteristics say,
redraws only what changed, and reads keys with the terminal driver's
escape sequence parsing (`IO$M_ESCAPE`) or its own. Windows split the
screen; EVE's are the text window, the status line, the message window
and the command line. VT100 and later, 7-bit and 8-bit controls.

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

## Testing strategy

**Oracle: scripted sessions.** The same keystrokes to `EDIT/TPU` on
AXPbox, on its own disk through `crosstools/ods/vms/run-vms.py`, and to vaxpunk's
TPU, with the file each writes compared, and the final screen compared
as a terminal emulator renders it. The oracle's outputs are committed, so
CI never needs AXPbox.

**Language tests.** TPU programs, run with `EDIT/TPU/NODISPLAY/COMMAND=`
on both sides, that print what they compute: string and pattern
built-ins, ranges after edits, `ON_ERROR`, `CASE` bounds. Agents generate
them by the hundred, as for BLISS-64.

**On vaxpunk.** A boot test edits a file with EVE, as the EDT one does,
and types it.

## Open questions

- **Interpreter shape.** A tree walker over the parsed procedures or a
  bytecode of our own. The section file's format follows from it.
- **Memory.** EVE's procedures, the buffers and the screen image fit in a
  process's pages, but how big a section file gets and whether it is
  mapped or read decides how much of the 4 MB a session costs.
- **Shareable images.** `TPU$TPUSHR` assumes the image activator maps
  shareable images; until it does, TPU links whole into one image.
- **Help.** EVE's `HELP` reads a help library; whether `LBR` and the `HELP`
  utility exist by then.

## Work order

Each step ends with something you can run or look at. None starts before
PRD-0004's pilot.

1. **Subset notes.** EVE's commands read down to the built-ins they need.
   *Visible:* the subset note.
2. **Oracle harness.** Scripted `EDIT/TPU` sessions and `/NODISPLAY` runs
   on AXPbox. *Visible:* the oracle's outputs for a first test.
3. **Interpreter.** Parser, procedures, variables, control, strings and
   integers, buffers, ranges and markers. *Visible:* `EDIT/TPU/NODISPLAY`
   runs a command file that edits a buffer and writes it, matching the
   oracle.
4. **Patterns and search.** *Visible:* the pattern tests pass.
5. **Screen and keys.** Windows, `UPDATE`, key maps, `READ_KEY`.
   *Visible:* a ten-line TPU editor in a command file edits a file on the
   screen.
6. **EVE.** The commands above, in TPU. *Visible:* the scripted sessions
   match the oracle's files and screens.
7. **Section files.** `SAVE` and loading at startup. *Visible:* `EDIT/TPU`
   starts EVE from `TPU$SECTION` without compiling it.
8. **`EDIT` is TPU.** DCL's `EDIT` runs TPU, `EDIT/EDT` runs EDT, and the
   boot test edits with EVE. *Visible:* the boot test.
