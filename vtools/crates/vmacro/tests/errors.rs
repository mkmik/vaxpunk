//! What vmacro rejects, and where it says so.

fn errors(source: &str) -> Vec<String> {
    let opts = vasm::Options::default();
    match vmacro::compile(source, &opts) {
        Ok(_) => Vec::new(),
        Err(diags) => diags
            .iter()
            .map(|d| format!("{}:{}: {}", d.line, d.col, d.msg))
            .collect(),
    }
}

#[test]
fn diagnostics() {
    let source = "\
        .PSECT  CODE, EXE
        RET
        .ENTRY  START, ^M<R2>
        MOVL    R1
        MOVL    PC, R0
        CASEL   R0, #0, R1
        EXTZV   #0, R2, R1, R0
        MOVQ    R11, R0
        PUSHR   #^M<R2, SP>
        MOVL    R0, #1
        RET
        .END    START
";
    assert_eq!(
        errors(source),
        [
            "2:9: RET outside a .ENTRY routine",
            "4:9: MOVL takes 2 operands",
            "5:9: PC can't be used as a register",
            "6:9: the CASE limit must be an immediate, #n",
            "7:9: vmacro needs the field size as a constant, #n",
            "8:9: a quadword needs two registers, up to R10 and R11",
            "9:9: PUSHR and POPR can't save SP or PC",
            "10:9: can't write to an immediate",
        ]
    );
}

#[test]
fn compiles() {
    let source = "\
        .PSECT  CODE, EXE
        .ENTRY  START, ^M<>
        MOVL    #1, R0
        RET
        .END    START
";
    assert_eq!(errors(source), Vec::<String>::new());
}
