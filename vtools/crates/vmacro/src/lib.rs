//! vmacro: a MACRO-32 compiler for ARM64, writing vaxpunk object modules.
//! `docs/macro32.md` describes the language as vmacro takes it.
//!
//! vmacro is a dialect of vasm: vasm reads the source, expands macros and
//! does the directives, and hands vmacro each VAX instruction, which
//! becomes a few lines of ARM64 assembly that vasm then assembles. Unknown
//! mnemonics are left to vasm, so ARM64 instructions can be mixed in.

mod insn;
mod operand;

use vasm::{Diagnostic, Dialect, Object, Options};

/// Compiles MACRO-32 `source` into object records.
pub fn compile(source: &str, opts: &Options) -> Result<Object, Vec<Diagnostic>> {
    let tool = concat!("vmacro ", env!("CARGO_PKG_VERSION"));
    vasm::assemble_with(source, opts, tool, Some(&mut Macro32::default()))
}

/// What ARM64's NZCV say about the last VAX instruction that set the
/// condition codes.
#[derive(Clone, Debug)]
pub(crate) enum Flags {
    /// They hold its N, Z and V, and its C, inverted after a subtraction
    /// (`borrow`): ARM64's carry is VAX's borrow inverted.
    Live { borrow: bool },
    /// Not set yet: these instructions set them from its result, if a
    /// branch needs them. They must come right before the branch.
    Pending(Vec<String>),
}

/// The MACRO-32 dialect: its state between statements.
pub struct Macro32 {
    flags: Flags,
    /// Local labels made so far.
    labels: u32,
    /// The registers the current `.ENTRY` routine saves, for `RET`.
    saved: Option<Vec<u8>>,
}

impl Default for Macro32 {
    fn default() -> Self {
        Macro32 {
            flags: Flags::Live { borrow: true },
            labels: 0,
            saved: None,
        }
    }
}

impl Dialect for Macro32 {
    fn statement(
        &mut self,
        word: &str,
        rest: &str,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Option<Result<Vec<String>, String>> {
        let one = |line: String| Some(Ok(vec![line]));
        match word {
            ".ENTRY" => Some(self.entry(rest, constant)),
            ".ADDRESS" => one(format!(".LONG {rest}")),
            ".BLKA" => one(format!(".BLKL {rest}")),
            ".EXTRN" => one(format!(".EXTERNAL {rest}")),
            ".SIGNED_BYTE" => one(format!(".BYTE {rest}")),
            ".SIGNED_WORD" => one(format!(".WORD {rest}")),
            ".PSECT" => psect(rest).map(|line| Ok(vec![line])),
            ".ERROR" => Some(Err(format!("%MACRO-E-GENERR, {}", rest.trim()))),
            // Listing control, and what only matters on a VAX.
            ".JSB_ENTRY" | ".JSB32_ENTRY" | ".SBTTL" | ".SUBTITLE" | ".PAGE" | ".LIST"
            | ".NLIST" | ".SHOW" | ".NOSHOW" | ".ENABLE" | ".ENABL" | ".DISABLE" | ".DSABL"
            | ".DEFAULT" | ".CROSS" | ".NOCROSS" | ".PRINT" | ".WARN" | ".PRESERVE" => {
                Some(Ok(Vec::new()))
            }
            _ => self.instruction(word, rest, constant),
        }
    }
}

impl Macro32 {
    fn instruction(
        &mut self,
        mn: &str,
        rest: &str,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Option<Result<Vec<String>, String>> {
        let (op, size) = insn::kind(mn)?;
        let mut run = || {
            let texts = operand::split(rest);
            let n = insn::arity(op);
            if texts.len() != n {
                return Err(format!("{mn} takes {n} operands"));
            }
            let ops = texts
                .iter()
                .map(|t| operand::parse(t))
                .collect::<Result<Vec<_>, _>>()?;
            let mut g = operand::Gen::new(&ops, constant, &mut self.labels);
            let saved = self.saved.as_deref();
            let flags = insn::compile(&mut g, mn, op, size, &ops, &self.flags, saved)?;
            Ok((g.out, flags))
        };
        Some(run().map(|(mut lines, flags)| {
            match (flags, &self.flags) {
                (Some(f), _) => self.flags = f,
                // It leaves the codes alone but may reuse the registers a
                // pending test reads: set them first, it keeps NZCV.
                (None, Flags::Pending(test)) => {
                    let test = test.iter().map(|l| format!("\t{l}"));
                    lines.splice(0..0, test);
                    self.flags = Flags::Live { borrow: false };
                }
                (None, Flags::Live { .. }) => {}
            }
            lines
        }))
    }

    /// `.ENTRY name, mask`: a global label and a call frame (docs/macro32.md).
    fn entry(
        &mut self,
        rest: &str,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Result<Vec<String>, String> {
        let args = operand::split(rest);
        let [name, mask] = match args.as_slice() {
            [n] => [n.as_str(), "0"],
            [n, m] => [n.as_str(), m.as_str()],
            _ => return Err(".ENTRY takes a name and a register mask".into()),
        };
        let mask = constant(mask).ok_or("the entry mask must be a constant")?;
        // Bits 12 and up enable arithmetic traps, which ARM64 lacks.
        let saved: Vec<u8> = (0..12).filter(|r| mask & 1 << r != 0).collect();
        let size = frame_size(&saved);
        // The mask of what it saved, for $UNWIND, which RETs from any frame.
        let saved_mask = mask & 0xfff;
        let mut out = vec![
            format!("{name}::"),
            format!("\tsub sp, sp, #{size}"),
            "\tstp xzr, x12, [sp]".into(),
            "\tstp x29, x30, [sp, #16]".into(),
            format!("\tmov x14, #{saved_mask}"),
            "\tstp x18, x14, [sp, #32]".into(),
        ];
        out.extend(saves(&saved, "stp", "str"));
        out.extend([
            "\tmov x29, sp".into(),
            "\tmov x12, x13".into(),
            "\tmov x18, sp".into(),
        ]);
        self.saved = Some(saved);
        self.flags = Flags::Live { borrow: true };
        Ok(out)
    }
}

/// The call frame: condition handler, AP, FP, LR, the caller's SP, the
/// entry mask, then the saved registers, 16-byte aligned.
fn frame_size(saved: &[u8]) -> usize {
    (48 + 8 * saved.len()).next_multiple_of(16)
}

/// Stores or loads the saved registers, in pairs, from offset 48.
fn saves(saved: &[u8], pair: &str, one: &str) -> Vec<String> {
    let arm = |r: &u8| operand::arm(*r).unwrap();
    saved
        .chunks(2)
        .enumerate()
        .map(|(i, regs)| match regs {
            [a, b] => format!("\t{pair} x{}, x{}, [sp, #{}]", arm(a), arm(b), 48 + 16 * i),
            [a] => format!("\t{one} x{}, [sp, #{}]", arm(a), 48 + 16 * i),
            _ => unreachable!(),
        })
        .collect()
}

/// `RET` from a routine that saves `saved`.
pub(crate) fn epilogue(saved: &[u8]) -> Vec<String> {
    let mut out = vec!["\tmov sp, x29".to_string()];
    out.extend(saves(saved, "ldp", "ldr"));
    out.extend([
        "\tldr x18, [sp, #32]".into(),
        "\tldp x29, x30, [sp, #16]".into(),
        "\tldr x12, [sp, #8]".into(),
        format!("\tadd sp, sp, #{}", frame_size(saved)),
        "\tret".into(),
    ]);
    out
}

/// `.PSECT name, attributes`: VAX code psects are writable, ARM64 ones
/// can't be, so an `EXE` psect becomes `NOWRT`. `USR` and `LIB` go.
fn psect(rest: &str) -> Option<String> {
    let args = operand::split(rest);
    let (name, attrs) = args.split_first()?;
    let upper: Vec<String> = attrs.iter().map(|a| a.to_ascii_uppercase()).collect();
    let exe = upper.iter().any(|a| a == "EXE");
    let drop = |a: &str| matches!(a, "USR" | "LIB") || exe && a == "WRT";
    if !exe && !upper.iter().any(|a| drop(a)) {
        return None;
    }
    let mut kept: Vec<&str> = upper
        .iter()
        .map(String::as_str)
        .filter(|a| !drop(a))
        .collect();
    if exe {
        kept.push("NOWRT");
    }
    Some(format!(".PSECT {name}, {}", kept.join(", ")))
}
