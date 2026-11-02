//! /LIST: the source with its location counter, then the machine code each
//! line became, laid out as AMACRO's /LIST/MACHINE_CODE.

#[test]
fn listing() {
    let source = "\
\t.TITLE\tLISTT\tListing probe
\t.MACRO\tTWICE\tREG
\tINCL\tREG
\tINCL\tREG
\t.ENDM\tTWICE
\t.PSECT\tDATA,WRT,NOEXE,LONG
COUNT:\t.LONG\t5, 7
\t.PSECT\tCODE,EXE,NOWRT,QUAD
\t.ENTRY\tFOO,^M<R2>
\tTWICE\tR2
10$:\tMOVL\t#1, R0\t; success
\tRET
\t.END
";
    let opts = vasm::Options::default();
    let mut dialect = vmacro::Macro32::default();
    let (_, l) = vasm::assemble_listed(source, &opts, "vmacro", Some(&mut dialect))
        .unwrap_or_else(|d| panic!("{d:?}"));
    let has = |s: &str| assert!(l.contains(s), "{s:?} isn't in:\n{l}");
    has("LISTT                           Listing probe                   ");
    // Every line, with the location counter; a macro's body keeps the one
    // before it.
    has("                      00000000\t      4 \tINCL\tREG\n");
    has("                      00000000\t      7 COUNT:\t.LONG\t5, 7\n");
    has("\t     13 \t.END\n");
    has("LISTT                           Machine Code Listing            ");
    has(
        "\n\t\t\t\t.PSECT\tDATA, LONG, NOPIC, CON, REL, LCL, NOSHR, NOEXE, RD, WRT\n\
         \t     0000\tCOUNT:\n\
         00000005     0000\t\t.LONG\t5, 7\t\t\t\t\t\t\t\t\t\t    ; 000007\n\
         00000007     0004\n",
    );
    has("\t     0000\tFOO::\n");
    // A macro call's expansion is the call's line; a label is on its own.
    has("31000673     001C\t\tadds\tw19, w19, #1\t\t\t\t\t\t\t\t\t    ; 000010\n");
    has("\t     002C\t10$:\nD2800020     002C\t\tmovz\tx0, #1\t\t\t\t\t\t\t\t\t\t    ; 000011\n");
}
