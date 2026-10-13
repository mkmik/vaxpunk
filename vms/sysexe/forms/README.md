# FORMS: DECforms's translator

`FORMS.EXE` is DECforms's development command, `FORMS function file`
([t8y:/layered/decforms](../../../docs/map/layered/decforms.md)). Two of
its functions are here:

| Command | Does |
| --- | --- |
| `FORMS TRANSLATE file` | translates IFDL, `file.IFDL`, into a form file, `file.FORM` |
| `FORMS BACK_TRANSLATE file` | translates a form file, `file.FORM`, back into IFDL, `file.IFDL` |

`TRANSLATE` takes DECforms's `/OUTPUT[=file]`, `/LOG`, `/LIST[=file]`,
`/[NO]WARNINGS`, `/ERROR_LIMIT=n` (30) and `/[NO]COMMENTS`;
`BACK_TRANSLATE` its `/OUTPUT[=file]` and `/LOG`. DCL takes the rest of
DECforms's qualifiers, from [cld/forms.cld](../../cld/forms.cld), and FORMS
does nothing with them. `CONVERT`, `DEVELOP`, `EDIT`, `EXTRACT` and `TEST`
say `%FORMS-E-NOTYETIMP`.

```
$ FORMS TRANSLATE/LOG EMPLOYEE
%FORMS-S-TRANSLATED, file EMPLOYEE.IFDL translated to EMPLOYEE.FORM.
$ FORMS BACK_TRANSLATE/OUTPUT=AGAIN EMPLOYEE
```

The language is the VSI DECforms IFDL Reference Manual's, and the
messages DECforms's own: their idents, text and condition values
(facility 213, `%X00D5xxxx`), as DECforms V2.1's translator has them. A
syntax error says where, with the line and a `^` under the place, and
lists what could have come there:

```
   14                    Line 1 Colum 10
                                ^
%FORMS-E-SYNTAXERR, syntax error; an expected keyword, name or literal is missing or misspelled.
-FORMS-I-EXPECTED, one of the following was expected: LINE, SAME, NEXT, COLUMN, ...
```

## How it works

The grammar of IFDL is data: [gram.b64](gram.b64) has it as rules in a
small notation, from the manual's syntax diagrams, which `GR_INIT`
compiles into tables when FORMS starts. The parser matches the source
against the rule `form`, trying a rule's alternatives in order and keeping
the first that matches, and builds the tree as it goes: an item for each
rule matched, a leaf for each token. The checks then walk the tree, and
the tree, as it is, is the form file. `BACK_TRANSLATE` reads the tree back
and prints it; where lines start comes from the grammar too. Adding a
clause to IFDL is adding it to the grammar.
[ADR-0034](../../../docs/adr/0034-forms-grammar-as-data-and-form-files-as-trees.md)
says why.

| Module | What |
| --- | --- |
| [main.b64](main.b64) | `FORMS$MAIN`: the command line, `TRANSLATE` and `BACK_TRANSLATE` |
| [gram.b64](gram.b64) | The grammar, and its compiler |
| [lex.b64](lex.b64) | The tokens: words, `%` names, numbers, strings, comments; picture strings when the parser asks |
| [parse.b64](parse.b64) | The parser, the tree, and the syntax errors |
| [check.b64](check.b64) | What the grammar can't check: declarations, references, numbers |
| [formfile.b64](formfile.b64) | Writing and reading `.FORM` files |
| [back.b64](back.b64) | The back translator |
| [msg.b64](msg.b64) | Messages, `/LIST`'s listing, memory |
| [forms.r64](forms.r64) | What they share |

The I/O module is TPU's, [../tpu/fio.mar](../tpu/fio.mar), whose
`FIO$K_BINARY` reads and writes a file's bytes, in FIX 512 records; under
vrun, [crosstools/vtools/bliss/fio.mar](../../../crosstools/vtools/bliss/fio.mar).

### The grammar's notation

```
[@]name = elements ;
```

| Element | Matches |
| --- | --- |
| `KEYWORD`, `%NAME` | that keyword, in any case |
| `"("` | that punctuation; `"~("` an opening parenthesis printed with no blank before it |
| `rule-name` | the rule |
| `$name` | a name that isn't a reserved word |
| `$xname` | a `$name` that isn't one of the rule `stop`'s keywords either, for lists of names that a clause may follow |
| `$qname` | a `$name` or a quoted string |
| `$str` | a quoted string |
| `$int`, `$num` | digits; a number, with a `.` or an exponent |
| `$sint`, `$snum` | the same after a sign, or none |
| `$pic` | a picture string, up to a blank |
| `$imp` | an implementor name, `%NAME` |
| `[ x ]`, `{ x }`, `x...`, `a \| b` | optional; a group; once or more; the first of them that matches |

A rule with `@` starts a line of its own in `BACK_TRANSLATE`'s output,
indented under the line it is in; the rule's text after a line of that
kind starts a line again, at the rule's own indentation. The rule
`reserved` lists the reserved words, the manual's Appendix C.

The parser doesn't go back into an alternative that matched: where a name
may be a keyword that comes next, the rule says both,
`TERMINAL {$name terminal-type | terminal-type}`.

## Form files

A `.FORM` file is vaxpunk's, not DECforms's, whose layout was never
published. It is the tree, with the names of the rules and keywords in it,
so that a newer FORMS reads an older form: little-endian, in FIX 512
records, the last padded with zeros.

| Bytes | What |
| --- | --- |
| 12 | `VAXPUNK FORM` |
| 2 | the format's version, 1 |
| 2 | flags: 1, the form is incomplete (a step names a panel or panel object that isn't there) |
| 4 | how many bytes are the form's, up to the padding |
| counted | the translator, `vaxpunk FORMS V1.0` |
| counted | when it was translated, `dd-mmm-yyyy hh:mm` |
| counted | the form's name, in capitals |
| 2, counted... | how many rules the tree has, then their names |
| 2, counted... | how many keywords, then their text: `FORM`, `(`, `~(`... |
| 4 | how many items |
| items | the tree, in preorder |

A counted string is a byte, its length, then its characters. An item is
a byte, its kind, then:

| Kind | Then |
| --- | --- |
| 1, a rule | 2 bytes, its number in the rules above, from 1; 4 bytes, how many items after it are its own |
| 2, a keyword | 2 bytes, its number in the keywords above, from 1 |
| 3, a name | 2 bytes, a length, and the name in capitals |
| 4, a string | the same: the string's characters, its quotes gone and doubled quotes single |
| 5, a number | the same: as written, its sign too |
| 6, a picture string | the same: as written |
| 7, an implementor name | the same: in capitals |
| 8, a comment | the same: as written, `/* */` or `{ }` and all |

## Tests

`cargo test -p vrun --test forms` builds FORMS.EXE for vrun and translates
each [tests](tests)`/NAME.ifdl` with `/LIST`: what it prints must be
`NAME.stdout`, and its listing `NAME.lis` where there is one. A form it
writes must back translate to `NAME.back`, which must translate and back
translate to itself. There is no DECforms here to compare with, so the
files are FORMS's own: `FORMS_BLESS=1` writes them, and what changed in
them is for a person to read. The boot test translates a form on
`DKB0:` and back again, and checks the `.FORM` file's records.

DECforms's own sample and demo forms are on the March 1997 OpenVMS Alpha
Software Product Library, disc 10 (archive.org's `OVA-SPL-1997`), in
`[FORMSAANMB021.KIT]`: they are DEC's, not to be copied here, but they
are the best check there is. All 17 translate with nothing to say, and
back translate to IFDL that makes the same tree again. They use three
things the manual doesn't have, which the grammar does: `[NO] ERASE ON
ENTRY`, a layout's `FRAME ... END FRAME`, and the device `%WINDOWS`.

## Not yet

- `COPY` of another file or a text library module (t8y:/layered/decforms#copy),
  and `COPY ... FROM DICTIONARY`, which says `NOTYETIMP`: vaxpunk has no CDD.
- Some checks (t8y:/layered/decforms#checks): that fields and literals fit
  their viewport, pictures, `VALUE`s against the data type, key names,
  description entries that conflict.
- The listing's Structure Layout Listing, which needs the records' layout,
  the Form Manager's work.
- `/DIAGNOSTICS` for LSE, `/SHOW`, `/INCLUDE`, `/MEMBER_ALIGNMENT`, `/FLOAT`.
- After a syntax error the translation stops; DECforms's goes on to find more.
