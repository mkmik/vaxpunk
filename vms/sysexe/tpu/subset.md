# TPU: the subset

[PRD-0006](../../../docs/prd/0006-tpu-and-eve.md)'s step 1: the EVE commands
vaxpunk's EVE has, read down to the TPU built-ins they need. A built-in
outside this list is still a name the compiler knows (all 121 of DEC's
TPU V3.2, `oracle/builtins.txt`), and calling it gives
`%TPU-W-NOTYET, not yet implemented`.

The tables come from DEC's TPU itself, on the oracle
(`crosstools/ods/vms/tpu-oracle.py`): `oracle/keywords.txt` is
`SHOW (KEYWORDS)` without EVE's `EVE$_` messages, `oracle/builtins.txt`
`SHOW (PROCEDURES)`, and `oracle/messages.txt` `MESSAGE_TEXT` of each
`TPU$_` keyword. `mktables.py` makes `tpukw.r64` of them.

## EVE's commands

From the *Guide to the Extensible Versatile Editor*'s command dictionary,
those PRD-0006 keeps:

| Area | Commands |
| --- | --- |
| Moving | MOVE UP, MOVE DOWN, MOVE LEFT, MOVE RIGHT, MOVE BY LINE, MOVE BY WORD, MOVE BY PAGE, TOP, BOTTOM, START OF LINE, END OF LINE, NEXT SCREEN, PREVIOUS SCREEN, LINE, GO TO, MARK, FORWARD, REVERSE, CHANGE DIRECTION |
| Finding | FIND, FIND NEXT, REPLACE, GLOBAL REPLACE, SET FIND CASE EXACT, SET FIND CASE NOEXACT |
| Selecting | SELECT, SELECT ALL, REMOVE, STORE TEXT, INSERT HERE, RESET |
| Erasing | ERASE CHARACTER, ERASE WORD, ERASE PREVIOUS WORD, ERASE LINE, ERASE START OF LINE, DELETE, RESTORE, RESTORE LINE, RESTORE WORD, RESTORE CHARACTER |
| Text | INSERT MODE, OVERSTRIKE MODE, CHANGE MODE, QUOTE, FILL PARAGRAPH, CENTER LINE, CAPITALIZE WORD, LOWERCASE WORD, UPPERCASE WORD, INSERT PAGE BREAK |
| Buffers and files | BUFFER, NEXT BUFFER, PREVIOUS BUFFER, DELETE BUFFER, GET FILE, INCLUDE FILE, WRITE FILE, SAVE FILE, SAVE FILE AS, SET BUFFER, SHOW, SHOW BUFFERS |
| Windows | ONE WINDOW, TWO WINDOWS, OTHER WINDOW, NEXT WINDOW, PREVIOUS WINDOW, ENLARGE WINDOW, SHRINK WINDOW, REFRESH |
| Settings | SET LEFT MARGIN, SET RIGHT MARGIN, SET TABS, SET WRAP, SET NOWRAP, SET CURSOR BOUND, SET CURSOR FREE, SET SCROLL MARGINS |
| Keys and programs | DEFINE KEY, LEARN, REMEMBER, REPEAT, DO, TPU, EXTEND THIS, EXTEND ALL, HELP |
| Ending | EXIT, QUIT |

Left out, as PRD-0006's non-goals say: the EDT and WPS keypads (`SET
KEYPAD EDT`), box selection, the clipboard, DECwindows, journaling and
recovery (`RECOVER`, `SET JOURNALING`), `SPAWN`, `DCL`, `ATTACH`, spelling,
`SAVE EXTENDED EVE` until section files (step 9), and wildcard find.

## The built-ins they need

| Area | Built-ins | Step |
| --- | --- | --- |
| Language | `ASCII`, `COMPILE`, `EXECUTE`, `FAO`, `INDEX`, `INT`, `LENGTH`, `MESSAGE`, `MESSAGE_TEXT`, `STR`, `SUBSTR`, `CREATE_ARRAY`, `GET_INFO` | 3, done |
| Strings | `CHANGE_CASE`, `EDIT` (`TRIM`, `COLLAPSE`, `COMPRESS`, `UPPER`, `LOWER`) | 3, `CHANGE_CASE` of a string done |
| Buffers | `APPEND_LINE`, `BEGINNING_OF`, `COPY_TEXT`, `CREATE_BUFFER`, `CREATE_RANGE`, `CURRENT_BUFFER`, `CURRENT_CHARACTER`, `CURRENT_LINE`, `CURRENT_OFFSET`, `CURRENT_DIRECTION`, `DELETE`, `END_OF`, `ERASE`, `ERASE_CHARACTER`, `ERASE_LINE`, `MARK`, `MOVE_HORIZONTAL`, `MOVE_TEXT`, `MOVE_VERTICAL`, `POSITION`, `SPLIT_LINE`, `READ_FILE`, `WRITE_FILE`, `CHANGE_CASE` of a range, `FILL` | 4, done but the last two |
| Patterns | `ANY`, `ARB`, `MATCH`, `NOTANY`, `SCAN`, `SCANL`, `SEARCH`, `SEARCH_QUIETLY`, `SPAN`, `SPANL` | 5, done |
| Files | `FILE_PARSE`, `FILE_SEARCH` | `FILE_SEARCH` done for one name, no wildcards |
| Screen | `ADJUST_WINDOW`, `CREATE_WINDOW`, `CURRENT_COLUMN`, `CURRENT_ROW`, `CURRENT_WINDOW`, `CURSOR_HORIZONTAL`, `CURSOR_VERTICAL`, `MAP`, `REFRESH`, `SCROLL`, `SELECT`, `SELECT_RANGE`, `SHIFT`, `UNMAP`, `UPDATE`, and `SET`'s `PROMPT_AREA`, `SCROLLING`, `STATUS_LINE`, `VIDEO`, `WIDTH`, `TEXT`, `PAD`, `CROSS_WINDOW_BOUNDS`, `EOB_TEXT`, `MESSAGE_FLAGS` | 7, done; the cursor is free (`CURSOR_VERTICAL` keeps its column past a line's end, `GET_INFO (window, "beyond_eol")`); `SCROLLING`, `PAD` and `CROSS_WINDOW_BOUNDS` taken and not acted on |
| Keys | `ADD_KEY_MAP`, `CREATE_KEY_MAP`, `CREATE_KEY_MAP_LIST`, `DEFINE_KEY`, `KEY_NAME`, `LAST_KEY`, `LEARN_ABORT`, `LEARN_BEGIN`, `LEARN_END`, `LOOKUP_KEY`, `READ_CHAR`, `READ_KEY`, `READ_LINE`, `REMOVE_KEY_MAP`, `UNDEFINE_KEY`, and `SET`'s `SELF_INSERT`, `UNDEFINED_KEY`, `SHIFT_KEY`, `PRE_KEY_PROCEDURE`, `POST_KEY_PROCEDURE`, `KEY_MAP_LIST` | 7, done |
| The session | `EXIT`, `QUIT`, `SAVE`, `SHOW`, `SLEEP` | done but `SHOW`; `SAVE` writes our own section file (`section.b64`) |

`GET_INFO` and `SET` take a keyword or string naming what they read or
set; each step adds those its built-ins' objects have, as EVE's code
asks for them.

## Not in the subset

`CALL_USER`, `CREATE_PROCESS`, `SEND`, `SEND_EOF`, `SPAWN`, `ATTACH`
(subprocesses); `JOURNAL_OPEN`, `JOURNAL_CLOSE`, `RECOVER_BUFFER`;
`CREATE_WIDGET`, `DEFINE_WIDGET_CLASS`, `MANAGE_WIDGET`, `UNMANAGE_WIDGET`,
`REALIZE_WIDGET`, `RAISE_WIDGET`, `LOWER_WIDGET`, `SET (WIDGET...)`,
`LOCATE_MOUSE`, `GET_CLIPBOARD`, `READ_CLIPBOARD`, `WRITE_CLIPBOARD`,
`GET_GLOBAL_SELECT`, `READ_GLOBAL_SELECT`, `WRITE_GLOBAL_SELECT`,
`SEND_CLIENT_MESSAGE` (DECwindows); `CONVERT`, `DEBUG_LINE`, `HELP_TEXT`
(no help libraries), `GET_DEFAULT`, `EXPAND_NAME`, `TRANSLATE`, `MODIFY_RANGE`.
