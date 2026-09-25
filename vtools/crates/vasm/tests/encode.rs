//! Encodings checked against GNU `as`: the same source goes through both,
//! and every instruction word must match. Needs the cross binutils that
//! vrun's stub build also uses (`CROSS_COMPILE`, default `aarch64-elf-`).

use std::path::Path;
use std::process::Command;

use vms_obj::obj::{self, Record, Tir};

/// Every instruction form vasm supports, in GNU syntax.
const SOURCE: &str = r#"
start:
    add x0, x1, x2
    add w3, w4, #4095
    add x5, sp, #16
    add sp, sp, #32
    add x0, x1, #1, lsl #12
    add x0, x1, #0x5000
    adds x0, x1, x2, lsl #3
    adds w0, w1, w2, asr #31
    add x0, x1, w2, uxtw #2
    add x0, x1, w2, sxtw
    add x0, sp, x1
    add x0, x1, x2, uxtx #4
    add w0, wsp, w1
    sub sp, sp, #32
    sub x0, x1, #-4
    subs x0, x1, #1
    subs w0, w1, w2, lsr #5
    sub x0, x1, x2
    cmp x0, #5
    cmp w1, w2
    cmp x1, x2, lsl #2
    cmp sp, #16
    cmn x0, #1
    cmn w0, w1
    neg x0, x1
    neg w0, w1, lsl #3
    negs x0, x1
    and x0, x1, #0xff
    and w0, w1, #0xfffffffe
    orr w0, w1, #0x80000000
    orr x0, x1, #0x5555555555555555
    eor x0, x1, #0xfffffffffffffff0
    eor x0, x1, x2, ror #7
    ands x0, x1, #1
    ands w0, w1, w2
    and sp, x1, #0xf0
    orr x0, x1, x2
    orn x0, x1, x2, lsl #1
    bic x0, x1, x2
    bics w0, w1, w2, asr #4
    eon x0, x1, x2
    tst x0, #7
    tst w0, w1
    mvn x0, x1
    mvn w0, w1, lsl #2
    mov x0, x1
    mov w0, w1
    mov x0, sp
    mov sp, x0
    mov wsp, w1
    mov w0, #0
    mov x0, #0x10000
    mov x0, #-1
    mov w0, #-1
    mov x0, #0xffffffffffff1234
    mov x0, #0xff00ff00ff00ff00
    mov w0, #0x80000000
    movz x0, #0x1234, lsl #16
    movz w0, #1
    movk x0, #0x5678, lsl #48
    movn w0, #1
    movn x0, #7, lsl #32
    mul x0, x1, x2
    madd x0, x1, x2, x3
    msub w0, w1, w2, w3
    mneg x0, x1, x2
    smull x0, w1, w2
    umull x0, w1, w2
    smaddl x0, w1, w2, x3
    umsubl x0, w1, w2, x3
    smulh x0, x1, x2
    umulh x0, x1, x2
    udiv x0, x1, x2
    sdiv w0, w1, w2
    lsl x0, x1, #3
    lsl w0, w1, #31
    lsr w0, w1, #31
    lsr x0, x1, #1
    asr x0, x1, #63
    asr x0, x1, x2
    lsl w0, w1, w2
    lsr x0, x1, x2
    ror x0, x1, #7
    ror w0, w1, w2
    lslv x0, x1, x2
    rorv w0, w1, w2
    ubfx x0, x1, #4, #8
    sbfx w0, w1, #0, #1
    ubfiz x0, x1, #3, #5
    sbfiz x0, x1, #60, #4
    bfi w0, w1, #8, #8
    bfxil x0, x1, #16, #16
    ubfm x0, x1, #1, #2
    sbfm w0, w1, #3, #4
    bfm x0, x1, #5, #6
    sxtb w0, w1
    sxtb x0, w1
    sxth x0, w1
    sxtw x0, w1
    uxtb w0, w1
    uxth w0, w1
    extr x0, x1, x2, #13
    extr w0, w1, w2, #31
    csel x0, x1, x2, ne
    csel w0, w1, w2, hs
    csinc x0, x1, x2, lt
    csinv x0, x1, x2, cc
    csneg w0, w1, w2, gt
    cset w0, eq
    cset x0, hi
    csetm x0, le
    cinc x0, x1, mi
    cinv w0, w1, pl
    cneg x0, x1, vs
    rbit x0, x1
    rev w0, w1
    rev x0, x1
    rev16 x0, x1
    rev32 x0, x1
    clz w0, w1
    cls x0, x1
back:
    b back
    b fwd
    bl back
    bl fwd
    b.eq back
    b.ne fwd
    b.hs back
    b.cs fwd
    b.lo back
    b.al fwd
    cbz x0, back
    cbnz w1, fwd
    tbz x0, #63, back
    tbnz w1, #3, fwd
    tbz x2, #5, fwd
    adr x0, back
    adr x1, fwd
    adr x2, .
    ldr x0, back
    ldr w1, fwd
    ldrsw x2, back
    ldr d0, fwd
    ldr q0, back
fwd:
    ret
    ret x1
    br x16
    blr x8
    eret
    svc #0
    svc #0x8000
    hvc #1
    brk #0xf000
    hlt #0xf000
    udf #0x42
    nop
    yield
    wfi
    wfe
    sev
    sevl
    hint #34
    dmb ish
    dmb ishld
    dsb sy
    dsb oshst
    dmb #5
    isb
    isb sy
    mrs x0, tpidr_el0
    msr tpidr_el0, x1
    mrs x0, ctr_el0
    mrs x3, nzcv
    msr fpcr, x2
    mrs x1, cntvct_el0
    mrs x0, sctlr_el1
    msr vbar_el1, x0
    mrs x5, s3_3_c13_c0_2
    msr daifset, #0xf
    msr daifclr, #2
    msr spsel, #1
    ldr x0, [x1]
    ldr x0, [x1, #8]
    ldr x0, [x1, #32760]
    ldr w0, [x1, #4092]
    ldr x0, [x1, #-8]
    ldr x0, [x1, #3]
    ldrb w0, [x1, #1]
    ldrb w0, [x1, #4095]
    ldrh w0, [x1, #2]
    ldrsb w0, [x1]
    ldrsb x0, [x1, #7]
    ldrsh x0, [x1, #2]
    ldrsh w0, [x1, #6]
    ldrsw x0, [x1, #4]
    ldr b0, [x1, #1]
    ldr h0, [x1, #2]
    ldr s0, [x1, #4]
    ldr d0, [x1, #8]
    ldr q0, [x1, #16]
    str x0, [sp, #-16]!
    str x0, [sp, #8]!
    ldr x0, [sp], #16
    ldr w0, [x1], #-4
    strb w0, [x1], #1
    str q0, [x1, #32]
    str d1, [sp, #-16]!
    ldur x0, [x1, #-3]
    stur w0, [x1, #1]
    ldurb w0, [x1, #-1]
    ldursw x0, [x1, #-4]
    ldr x0, [x1, x2]
    ldr x0, [x1, x2, lsl #3]
    ldr w0, [x1, w2, uxtw]
    ldr w0, [x1, w2, sxtw #2]
    ldr x0, [x1, x2, sxtx]
    ldrb w0, [x1, x2]
    ldrb w0, [x1, x2, lsl #0]
    ldrh w0, [x1, w2, uxtw #1]
    strb w3, [x4, w5, sxtw]
    str x0, [x1, x2, lsl #3]
    ldr q1, [x2, x3, lsl #4]
    ldp x29, x30, [sp], #16
    stp x29, x30, [sp, #-16]!
    stp w0, w1, [x2, #8]
    ldp x0, x1, [x2]
    ldp w0, w1, [x2, #-256]
    ldpsw x0, x1, [x2, #4]
    stp q0, q1, [sp, #-32]!
    ldp d0, d1, [x0], #16
    stp s0, s1, [x0, #4]
    ldxr x0, [x1]
    ldxr w0, [x1]
    ldaxr x0, [x1]
    stxr w2, x0, [x1]
    stlxr w2, w0, [x1]
    ldar x0, [x1]
    stlr w0, [x1]
"#;

fn cross() -> String {
    std::env::var("CROSS_COMPILE").unwrap_or_else(|_| {
        let elf = Command::new("aarch64-elf-as").arg("--version").output();
        if elf.is_ok() {
            "aarch64-elf-"
        } else {
            "aarch64-linux-gnu-"
        }
        .into()
    })
}

/// The code GNU as produces for `source`.
fn gnu(source: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let (s, o, bin) = (
        dir.join("oracle.s"),
        dir.join("oracle.o"),
        dir.join("oracle.bin"),
    );
    std::fs::write(&s, source).unwrap();
    let cross = cross();
    let run = |cmd: &mut Command| {
        let out = cmd.output().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
        assert!(
            out.status.success(),
            "{cmd:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(Command::new(format!("{cross}as")).arg("-o").arg(&o).arg(&s));
    run(Command::new(format!("{cross}objcopy"))
        .args(["-O", "binary", "-j", ".text"])
        .arg(&o)
        .arg(&bin));
    std::fs::read(&bin).unwrap()
}

/// The code vasm produces for `source`: all of it goes into STO_IMM
/// commands, because every branch target is in the same psect.
fn vasm(source: &str) -> Vec<u8> {
    let opts = vasm::Options {
        name: "ORACLE".into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    let records = vasm::assemble(source, &opts).unwrap_or_else(|diags| {
        let msgs: Vec<String> = diags
            .iter()
            .map(|d| format!("line {}: {}: {}", d.line, d.text.trim(), d.msg))
            .collect();
        panic!("vasm errors:\n{}", msgs.join("\n"))
    });
    let bytes = obj::write(&records);
    let mut code = Vec::new();
    for record in obj::parse(&bytes).unwrap() {
        if let Record::Tir(cmds) = record {
            for cmd in cmds {
                match cmd {
                    Tir::StoImm { data } => code.extend(data),
                    Tir::StaPq { .. } | Tir::CtlSetrb {} => {}
                    other => panic!("unexpected {}", other.name()),
                }
            }
        }
    }
    code
}

#[test]
fn matches_gnu_as() {
    let (ours, theirs) = (vasm(SOURCE), gnu(SOURCE));
    let lines: Vec<&str> = SOURCE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.ends_with(':'))
        .collect();
    assert_eq!(ours.len(), theirs.len(), "different code sizes");
    let mut bad = Vec::new();
    for (i, (a, b)) in ours.chunks(4).zip(theirs.chunks(4)).enumerate() {
        if a != b {
            let (a, b) = (
                u32::from_le_bytes(a.try_into().unwrap()),
                u32::from_le_bytes(b.try_into().unwrap()),
            );
            bad.push(format!("{:<32} vasm {a:08x}  gnu {b:08x}", lines[i]));
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
