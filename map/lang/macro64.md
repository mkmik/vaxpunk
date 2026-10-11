+++
title = "MACRO-64"
summary = "The Alpha's native assembler, sold as a layered product"
aka = ["MACRO-64", "MACRO-64 Assembler for OpenVMS AXP", "MACRO/ALPHA", ".M64"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "dropped"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/docs/assembler.md"]
+++

MACRO-64 was DEC's assembler for Alpha machine code, a layered product
apart from the MACRO-32 compiler. It had MACRO-32's directives and macro
language over Alpha instructions, linkage sections and procedure
descriptors, and macros (`$ROUTINE`, `$CALL`, `$RETURN`) that built
standard frames. DEC wrote little in it: the executive stayed MACRO-32
and BLISS, and MACRO-64 covered the few places that needed Alpha
instructions by hand.

Dropped: in vaxpunk's history Alpha went to ARM, and nothing on Alpha
needed a MACRO-64 tradition, so hand-written ARM64 is kept to the same
slivers. Some of its ideas live on in the toolchain: vasm's directives
are modelled on MACRO-64's and MACRO-32's
([PRD-0001](../../docs/prd/0001-vtools.md)), and `call.mlb`'s
`$ROUTINE`, `$CALL` and `$RETURN` build standard frames in vasm as
MACRO-64's macros did (PRD-0005).
