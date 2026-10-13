# ADR-0034 — FORMS has IFDL's grammar as data, and a .FORM file is the tree it parses, with the names of its rules

Oct 13, 2026 · @Marko Mikulicic

Proposed. `FORMS TRANSLATE`, DECforms's IFDL translator, is a native
vaxpunk program in BLISS-64 whose knowledge of IFDL is a grammar in a
small notation, compiled into tables when it starts; one parser matches
any IFDL against it and builds the parse tree. A `.FORM` file is that
tree, with the names of the rules and keywords it uses, in a layout of
vaxpunk's own. `FORMS BACK_TRANSLATE` prints the tree, and the Form
Manager will walk it.

## Context

DECforms (t8y:/layered/decforms) describes a form in IFDL, which
`FORMS TRANSLATE` turns into a binary `.FORM` file. The Form Manager loads
that file at run time, `FORMS BACK_TRANSLATE` turns it into IFDL again,
and `FORMS EXTRACT OBJECT` puts it in an object module. The user wanted
the translator as a native VMS program, not a cross tool.

IFDL is big. The VSI IFDL Reference Manual has about 120 syntax diagrams,
some 400 keywords and 100 reserved words, and DECforms's own sample forms
use clauses the manual leaves out (`ERASE ON ENTRY`, `FRAME`, `%WINDOWS`).
There is no statement terminator: the language is keywords and `END`
lines, and a name may be any keyword that isn't reserved, so where a
clause ends depends on what can follow it.

DECforms's `.FORM` layout was never published. The kit's files show
variable-length records with DECforms's magic string, names padded to 31
characters, and names the translator invents. Nothing on vaxpunk has to
read a DEC `.FORM` file: forms are translated again from their IFDL.

## Decision

1. **The grammar is data.** `vms/sysexe/forms/gram.b64` has IFDL as
   rules, `[@]name = elements ;`, written from the manual's diagrams:
   keywords, punctuation, other rules, terminals (`$name`, `$str`,
   `$pic`...), optional parts, groups, alternatives and repetition. The
   rule `reserved` lists the reserved words. `GR_INIT` compiles the text
   into tables when FORMS starts, about 2,300 elements, so there is
   nothing to generate at build time and the grammar reads like the
   manual.
2. **One parser for all of it.** The parser matches the source against
   the rule `form`, tries a rule's alternatives in order, takes the first
   that matches and doesn't go back into it. It builds the tree as it
   goes: an item for each rule matched, a leaf for each token, and one
   for each comment, which the next token's rule keeps. Where no
   alternative matches, the error is the furthest token any reached, and
   what they expected there is `SYNTAXERR`'s `EXPECTED` list, as
   DECforms's translator gives it. Picture strings are lexed only where
   the grammar asks for one.
3. **The checks walk the tree.** What the grammar can't say, such as a
   panel field naming a form data item or a viewport inside its layout,
   is checked on the tree, with DECforms's messages and condition values.
4. **A `.FORM` file is the tree.** The file has a header, the names of
   the rules and the text of the keywords the tree uses, then its items
   in preorder: a rule with its number in the file's table and how many
   items are its own, a keyword with its number, a leaf with its text.
   The file is FIX 512 records, as `COPY/BINARY` writes them, read and
   written through RMS. `vms/sysexe/forms/README.md` lays it out.
5. **Back translation prints the tree.** A rule marked `@` in the
   grammar starts a line, indented under the line it is in, so the
   grammar says the layout of `BACK_TRANSLATE`'s output too.

## Alternatives considered

- **A recursive descent parser written for each construct**, as TPU's
  compiler is. A hundred-odd constructs would each need a parsing routine,
  a writer and a printer that agree with each other. With the grammar as
  data, adding a clause is one line, and parsing, writing and printing
  can't disagree.
- **A translator in Rust on the host**, beside vcdu. It would be quicker
  to write, but the user asked for a VMS program, and DECforms users run
  `FORMS TRANSLATE` at the `$` prompt.
- **DECforms's own layout**, worked out from the kit's `.FORM` files. A
  lot of reverse engineering, of a format nothing needs to read, with
  translator-made names in it, and DEC's code to avoid copying.
- **The grammar's own numbers in the file, not names.** Smaller, but any
  change to the grammar would make every `.FORM` file unreadable. With
  the names, a newer FORMS reads an older form, unless a rule it uses
  was renamed.
- **Hexadecimal text, as TPU's section files are.** The I/O module then
  needs no change, but a form file is binary in DECforms, and the Form
  Manager will read it whole. The I/O module gained `FIO$K_BINARY`
  instead, which writes FIX 512 records.

## Consequences

- The Form Manager (t8y:/layered/decforms#form-manager) reads forms as
  trees: it finds rules by name and reads their keywords and leaves. It
  works out record layouts and positions when it loads a form, as
  DECforms on Alpha does. `FORMS EXTRACT OBJECT` can put the same bytes
  in an object module.
- The tree keeps every keyword, so `BACK_TRANSLATE` gives back what was
  written. Only comments may move to the next clause, and continued
  strings are joined. Translated again, its output makes the same tree,
  which the tests and DECforms's 17 sample forms check.
- The parser commits to the first alternative that matches, so the
  grammar has to say where a name may be a keyword that follows it. The
  `stop` rule names keywords that end a list of names, and some rules
  spell out both readings.
- A syntax error ends the translation. DECforms's LALR translator
  recovers and finds more; recovery would mean resynchronizing on `END`
  lines.
- The grammar is part of FORMS.EXE: its tables cost about 190 KB of the
  image, and it is compiled at each start: under vrun, FORMS starts,
  compiles it and exits in under 60 ms, QEMU's own start included.
