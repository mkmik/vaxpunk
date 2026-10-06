//! vmacro: a MACRO-32 compiler for ARM64, writing vaxpunk object modules.
//! `docs/macro32.md` describes the language as vmacro takes it.
//!
//! vmacro is a dialect of vasm: vasm reads the source, expands macros and
//! does the directives, and hands vmacro each VAX instruction, which
//! becomes a few lines of ARM64 assembly that vasm then assembles. Unknown
//! mnemonics are left to vasm, so ARM64 instructions can be mixed in.
//!
//! vasm reads the source twice. The first pass surveys the routines: what
//! each writes and which JSB routines it calls (`routine.rs`). The second
//! compiles them, each saving the registers the survey says it must.

mod insn;
mod operand;
mod routine;

use routine::{KEPT, Kind, Params, Regs, Routine, Survey};
use vasm::{Diagnostic, Dialect, Object, Options};

const TOOL: &str = concat!("vmacro ", env!("CARGO_PKG_VERSION"));

/// Compiles MACRO-32 `source` into object records.
pub fn compile(source: &str, opts: &Options) -> Result<Object, Vec<Diagnostic>> {
    vasm::assemble_with(source, opts, TOOL, Some(&mut Macro32::default()))
}

/// Compiles modules that are linked together, each `(source, options)`. A
/// JSB from one to a routine another exports (`NAME::`) knows what that
/// routine modifies from its declaration, as if the caller had a
/// `.CALL_LINKAGE` for it, rather than counting all of R2-R11.
pub fn compile_modules(modules: &[(&str, &Options)]) -> Vec<Result<Object, Vec<Diagnostic>>> {
    let mut surveys: Vec<Survey> = modules
        .iter()
        .map(|(source, opts)| {
            let mut m = Macro32 {
                alone: false,
                ..Macro32::default()
            };
            let _ = vasm::assemble_with(source, opts, TOOL, Some(&mut m));
            m.survey
        })
        .collect();
    routine::solve_together(&mut surveys);
    modules
        .iter()
        .zip(surveys)
        .map(|((source, opts), survey)| {
            let mut m = Macro32 {
                compiling: true,
                survey,
                ..Macro32::default()
            };
            vasm::assemble_with(source, opts, TOOL, Some(&mut m))
        })
        .collect()
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

/// How the routine an instruction is in returns: what `RET` and `RSB`
/// restore.
pub(crate) enum Exit {
    None,
    Call(Vec<u8>),
    Jsb(Vec<u8>),
}

/// The MACRO-32 dialect: its state between statements.
pub struct Macro32 {
    flags: Flags,
    /// Local labels made so far.
    labels: u32,
    /// The first pass surveys the module; the second compiles it. A module
    /// compiled `alone` goes straight on to the second.
    compiling: bool,
    alone: bool,
    survey: Survey,
    /// Routines declared so far in this pass, and the one being compiled.
    next: usize,
    cur: Option<usize>,
    /// Labels defined since the last statement: a declaration's names.
    pending: Vec<String>,
    /// `.USE_LINKAGE`'s, for the next JSB.
    linkage: Option<Regs>,
    /// Registers the current routine was warned about writing.
    warned: Regs,
    /// Whether the last instruction may go on to the next.
    falls: bool,
    /// Whether the current routine loaded SP from elsewhere: its frame,
    /// and what it saved, are no longer on the stack.
    switched: bool,
}

impl Default for Macro32 {
    fn default() -> Self {
        Macro32 {
            flags: Flags::Live { borrow: true },
            labels: 0,
            compiling: false,
            alone: true,
            survey: Survey::default(),
            next: 0,
            cur: None,
            pending: Vec::new(),
            linkage: None,
            warned: 0,
            falls: false,
            switched: false,
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
        let labels = std::mem::take(&mut self.pending);
        let one = |line: String| Some(Ok(vec![line]));
        match word {
            ".ENTRY" => Some(self.declare(Kind::Call, word, rest, labels, constant)),
            ".CALL_ENTRY" => Some(self.declare(Kind::Call, word, rest, labels, constant)),
            ".JSB_ENTRY" => Some(self.declare(Kind::Jsb, word, rest, labels, constant)),
            ".JSB32_ENTRY" => Some(self.declare(Kind::Jsb32, word, rest, labels, constant)),
            ".EXCEPTION_ENTRY" => Some(self.declare(Kind::Exception, word, rest, labels, constant)),
            ".GLOBAL_LABEL" => {
                if labels.is_empty() {
                    return Some(Err(".GLOBAL_LABEL follows the label it declares".into()));
                }
                self.survey.globals.extend(labels);
                Some(Ok(Vec::new()))
            }
            ".CALL_LINKAGE" | ".DEFINE_LINKAGE" | ".USE_LINKAGE" => Some(self.linkage(word, rest)),
            ".ADDRESS" => one(format!(".LONG {rest}")),
            ".BLKA" => one(format!(".BLKL {rest}")),
            ".EXTRN" => one(format!(".EXTERNAL {rest}")),
            ".SIGNED_BYTE" => one(format!(".BYTE {rest}")),
            ".SIGNED_WORD" => one(format!(".WORD {rest}")),
            ".PSECT" => psect(rest).map(|line| Ok(vec![line])),
            ".ERROR" => Some(Err(format!("%MACRO-E-GENERR, {}", rest.trim()))),
            // Listing control, and what only matters on a VAX.
            ".SBTTL" | ".SUBTITLE" | ".PAGE" | ".LIST" | ".NLIST" | ".SHOW" | ".NOSHOW"
            | ".ENABLE" | ".ENABL" | ".DISABLE" | ".DSABL" | ".DEFAULT" | ".CROSS" | ".NOCROSS"
            | ".PRINT" | ".PRESERVE" => Some(Ok(Vec::new())),
            _ if word.starts_with('.') => None,
            _ => match self.instruction(word, rest, &labels, constant) {
                Some(r) => Some(r),
                None => self.native(word, rest),
            },
        }
    }

    fn label(&mut self, name: &str, global: bool) {
        if name.contains('@') {
            return;
        }
        if !self.compiling {
            self.survey.labels.insert(name.to_string(), self.cur);
            if global {
                self.survey.exports.insert(name.to_string());
            }
        }
        self.pending.push(name.to_string());
    }

    fn again(&mut self) -> bool {
        if self.compiling || !self.alone {
            return false;
        }
        self.survey.solve();
        *self = Macro32 {
            compiling: true,
            survey: std::mem::take(&mut self.survey),
            ..Macro32::default()
        };
        true
    }
}

impl Macro32 {
    fn instruction(
        &mut self,
        mn: &str,
        rest: &str,
        labels: &[String],
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Option<Result<Vec<String>, String>> {
        let (op, size) = insn::kind(mn)?;
        use insn::Op;
        self.falls = !matches!(
            op,
            Op::Br | Op::Jmp | Op::Rsb | Op::Ret | Op::Rei | Op::Halt
        );
        let mut run = || {
            let Some(cur) = self.cur else {
                return Err(OUTSIDE.into());
            };
            let texts = operand::split(rest);
            let n = insn::arity(op);
            if texts.len() != n {
                return Err(format!("{mn} takes {n} operands"));
            }
            let ops = texts
                .iter()
                .map(|t| operand::parse(t))
                .collect::<Result<Vec<_>, _>>()?;
            let mut warnings = self.check(op, &ops)?;
            if switches(op, &ops) {
                self.switched = true;
                if !self.compiling {
                    self.survey.longjumps.extend(labels.iter().cloned());
                }
            }
            let mut g = operand::Gen::new(&ops, constant, &mut self.labels);
            let r = &self.survey.routines[cur];
            let exit = match r.kind {
                Kind::Call => Exit::Call(routine::list(r.saved)),
                Kind::Jsb | Kind::Jsb32 => Exit::Jsb(routine::list(r.saved)),
                Kind::Exception => Exit::None,
            };
            let flags = insn::compile(&mut g, mn, op, size, &ops, &self.flags, &exit)?;
            let written = g.written;
            let mut lines = g.out;
            if self.compiling {
                warnings.extend(self.unmasked(cur, written));
            } else {
                let linkage = self.linkage;
                let r = &mut self.survey.routines[cur];
                r.direct |= written;
                if op == insn::Op::Jsb {
                    match target(&ops[0]) {
                        Some(t) => r.calls.push((t.to_string(), linkage)),
                        None => r.elsewhere |= linkage.unwrap_or(KEPT),
                    }
                }
                if let Some(t) = branch_target(op, &ops).and_then(target) {
                    r.branches.push(t.to_string());
                }
            }
            if op == insn::Op::Jsb {
                self.linkage = None;
            }
            lines.extend(warnings.iter().map(|w| format!("\t.WARN {w}")));
            Ok((lines, flags))
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

    /// The second pass's checks of the routines an instruction calls and
    /// the labels it branches to: errors, and warnings it returns.
    fn check(&self, op: insn::Op, ops: &[operand::Opnd]) -> Result<Vec<String>, String> {
        use insn::Op;
        if !self.compiling {
            return Ok(Vec::new());
        }
        let s = &self.survey;
        if op == Op::Jsb
            && let operand::Opnd::Mem(operand::Mode::Rel(e), None) = &ops[0]
            && e.trim().starts_with(|c: char| c.is_ascii_digit())
        {
            return Err(format!(
                "JSB to {}, a local label: a JSB routine is declared, with .JSB_ENTRY",
                e.trim()
            ));
        }
        let called = match op {
            Op::Jsb => ops.first(),
            Op::Calls | Op::Callg => ops.get(1),
            _ => None,
        };
        let branched = branch_target(op, ops);
        if let Some(t) = called.and_then(target)
            && s.labels.contains_key(t)
            && !s.entries.contains_key(t)
        {
            let how = if op == Op::Jsb {
                ".JSB_ENTRY"
            } else {
                ".CALL_ENTRY or .ENTRY"
            };
            return Err(format!(
                "{t} isn't a declared routine: declare it with {how}"
            ));
        }
        let Some(t) = branched.and_then(target) else {
            return Ok(Vec::new());
        };
        let here = self.cur.map(|c| &s.routines[c]);
        if self.switched || s.noreturn.contains(t) || s.longjumps.contains(t) {
            return Ok(Vec::new());
        }
        let (there, name) = match s.labels.get(t) {
            Some(r) if *r == self.cur => return Ok(Vec::new()),
            Some(r) => {
                if !s.entries.contains_key(t) && !s.globals.contains(t) {
                    return Err(format!(
                        "{t} is in another routine: declare it with .GLOBAL_LABEL"
                    ));
                }
                (
                    r.map(|r| &s.routines[r]),
                    format!("{t}, in another routine,"),
                )
            }
            None => (None, format!("{t}, in another module,")),
        };
        // An .EXCEPTION_ENTRY routine has no frame and saves nothing, and
        // one never returns: nothing is lost going from or to one.
        if here.is_some_and(|r| r.kind == Kind::Exception)
            || there.is_some_and(|r| r.kind == Kind::Exception)
        {
            return Ok(Vec::new());
        }
        // A JSB routine's entry builds its own save area: going there is a
        // tail call, for a routine with nothing of its own to restore. Code
        // in the middle of another routine ends in that routine's RET or
        // RSB, which must restore what this one saves. A label in another
        // module that isn't known is a JSB routine's that saves nothing.
        let exit = |r: Option<&Routine>| r.map_or((false, 0), |r| (r.kind == Kind::Call, r.saved));
        let (entry, theirs) = match there {
            Some(r) => (s.entries.contains_key(t), exit(Some(r))),
            None => s
                .foreign
                .get(t)
                .map_or((true, (false, 0)), |&(e, call, saved)| (e, (call, saved))),
        };
        let ok = if entry {
            exit(here) == (false, 0) && !theirs.0
        } else {
            exit(here) == theirs
        };
        if !ok {
            let saves = |s: Regs| match s {
                0 => "none".to_string(),
                s => routine::list(s)
                    .iter()
                    .map(|n| format!("R{n}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            return Err(format!(
                "a branch to {name} which doesn't restore the registers this routine saves ({}; it restores {})",
                saves(exit(here).1),
                saves(theirs.1)
            ));
        }
        Ok(Vec::new())
    }

    /// Warnings for registers a `.ENTRY` routine writes but its mask
    /// leaves out, once each: vmacro saves them anyway.
    fn unmasked(&mut self, cur: usize, written: Regs) -> Vec<String> {
        let r = &self.survey.routines[cur];
        let Some(mask) = r.mask else {
            return Vec::new();
        };
        let new = written & KEPT & !mask & !r.output & !r.scratch & !self.warned;
        self.warned |= new;
        routine::list(new)
            .iter()
            .map(|n| format!("R{n} is written but isn't in the entry mask"))
            .collect()
    }

    /// An ARM64 instruction: vasm's, but only in a routine, and the first
    /// pass notes the VAX registers it writes.
    fn native(&mut self, mn: &str, rest: &str) -> Option<Result<Vec<String>, String>> {
        let Some(cur) = self.cur else {
            return Some(Err(OUTSIDE.into()));
        };
        if !self.compiling
            && let Some(n) = native_writes(mn, rest)
        {
            self.survey.routines[cur].direct |= 1 << n;
        }
        self.falls = !matches!(mn, "B" | "BR" | "RET" | "ERET");
        None
    }

    /// A routine's declaration: `.ENTRY name, mask`, or a label and
    /// `.CALL_ENTRY`, `.JSB_ENTRY`, `.JSB32_ENTRY` or `.EXCEPTION_ENTRY`
    /// with AMACRO's parameters.
    fn declare(
        &mut self,
        kind: Kind,
        word: &str,
        rest: &str,
        labels: Vec<String>,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Result<Vec<String>, String> {
        let p = Params::parse(rest);
        let (names, mask) = if word == ".ENTRY" {
            let (name, mask) = match p.positional.as_slice() {
                [n] if p.keyed.is_empty() => (n, "0"),
                [n, m] if p.keyed.is_empty() => (n, m.as_str()),
                _ => return Err(".ENTRY takes a name and a register mask".into()),
            };
            let mask = constant(mask).ok_or("the entry mask must be a constant")?;
            // Bits 12 and up enable arithmetic traps, which ARM64 lacks.
            (vec![name.clone()], Some(mask as Regs & 0xfff))
        } else {
            if !p.positional.is_empty() {
                return Err(format!("{word} takes only parameters, NAME=value"));
            }
            if labels.is_empty() {
                return Err(format!("{word} follows the label that names the routine"));
            }
            (labels, None)
        };
        let known: &[&str] = match kind {
            Kind::Call => &[
                "MAX_ARGS",
                "HOME_ARGS",
                "QUAD_ARGS",
                "INPUT",
                "OUTPUT",
                "SCRATCH",
                "PRESERVE",
                "LABEL",
            ],
            Kind::Jsb | Kind::Jsb32 => &["INPUT", "OUTPUT", "SCRATCH", "PRESERVE"],
            Kind::Exception => &["INPUT", "OUTPUT", "SCRATCH", "PRESERVE", "STACK_BASE"],
        };
        p.check(known)?;
        p.number("MAX_ARGS")?;
        let r = Routine {
            kind,
            mask,
            output: p.regs("OUTPUT")?,
            scratch: p.regs("SCRATCH")?,
            preserve: p.regs("PRESERVE")?,
            home_args: p.flag("HOME_ARGS")?,
            quad_args: p.flag("QUAD_ARGS")?.unwrap_or(false),
            ..Routine::default()
        };
        if r.home_args == Some(true) && r.quad_args {
            return Err("HOME_ARGS and QUAD_ARGS exclude each other".into());
        }
        let i = self.next;
        self.next += 1;
        let mut out = Vec::new();
        if self.compiling
            && self.falls
            && let Some(prev) = self.cur
        {
            // Into its prologue, and out through its RET or RSB.
            let plain = |r: &Routine| r.kind != Kind::Call && r.saved == 0;
            let s = &self.survey.routines;
            if !plain(&s[prev]) || !plain(&s[i]) {
                out.push(format!(
                    "\t.ERROR {} comes right after code that goes on into it: end that with a branch",
                    names[0]
                ));
            }
        }
        self.falls = false;
        if !self.compiling {
            for n in &names {
                self.survey.entries.insert(n.clone(), i);
                self.survey.labels.insert(n.clone(), Some(i));
            }
            self.survey.routines.push(r);
        }
        self.cur = Some(i);
        self.warned = 0;
        self.switched = false;
        self.flags = Flags::Live { borrow: true };
        let saved = routine::list(self.survey.routines[i].saved);
        if word == ".ENTRY" {
            out.push(format!("{}::", names[0]));
        }
        // ponytail: a debugging aid while declarations are reviewed.
        match kind {
            Kind::Call => out.extend(call_prologue(&saved)),
            Kind::Jsb | Kind::Jsb32 => out.extend(jsb_prologue(&saved)),
            Kind::Exception => {}
        }
        Ok(out)
    }

    /// `.CALL_LINKAGE routine, ...`, `.DEFINE_LINKAGE name, ...` and
    /// `.USE_LINKAGE ...`: what JSB routines in other modules modify.
    fn linkage(&mut self, word: &str, rest: &str) -> Result<Vec<String>, String> {
        let p = Params::parse(rest);
        p.check(&["LINKAGE_NAME", "INPUT", "OUTPUT", "SCRATCH", "PRESERVE"])?;
        let named = p.keyed.iter().find(|(k, _)| k == "LINKAGE_NAME");
        let effect = match named {
            Some((_, n)) => *self
                .survey
                .linkages
                .get(&n.to_ascii_uppercase())
                .ok_or_else(|| format!("no linkage {n}: define it with .DEFINE_LINKAGE"))?,
            None => {
                routine::linkage_effect(p.regs("OUTPUT")?, p.regs("SCRATCH")?, p.regs("PRESERVE")?)
            }
        };
        let name = |what| match p.positional.as_slice() {
            [n] => Ok(n.to_ascii_uppercase()),
            _ => Err(format!("{word} takes {what} first")),
        };
        match word {
            ".CALL_LINKAGE" => {
                let n = name("the routine's name")?;
                self.survey.called.insert(n, effect);
            }
            ".DEFINE_LINKAGE" => {
                let n = name("the linkage's name")?;
                self.survey.linkages.insert(n, effect);
            }
            _ => self.linkage = Some(effect),
        }
        Ok(Vec::new())
    }
}

const OUTSIDE: &str =
    "code outside a routine: declare it with .CALL_ENTRY, .JSB_ENTRY or .EXCEPTION_ENTRY";

/// A call or branch's destination, if it is a name: `NAME` or `G^NAME`.
fn target(op: &operand::Opnd) -> Option<&str> {
    use operand::{Mode, Opnd};
    let (Opnd::Mem(Mode::Rel(e), None) | Opnd::Mem(Mode::Gen(e), None)) = op else {
        return None;
    };
    let name = e.trim();
    let symbol = name.chars().next().is_some_and(|c| !c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "$_.".contains(c));
    symbol.then_some(name)
}

/// Whether an instruction loads SP from something other than SP: a switch
/// to another stack.
fn switches(op: insn::Op, ops: &[operand::Opnd]) -> bool {
    use operand::{Mode, Opnd};
    if ops.last() != Some(&Opnd::Reg(14)) {
        return false;
    }
    match op {
        insn::Op::Mov => ops[0] != Opnd::Reg(14),
        insn::Op::Mova => !matches!(ops[0], Opnd::Mem(Mode::Def(14) | Mode::Disp(_, 14), None)),
        _ => false,
    }
}

/// The destination of a branch or jump.
fn branch_target(op: insn::Op, ops: &[operand::Opnd]) -> Option<&operand::Opnd> {
    use insn::Op;
    match op {
        Op::Jmp => ops.first(),
        Op::Bcc | Op::Br | Op::Blb(_) | Op::Bb(..) | Op::Aob(_) | Op::Sob(_) | Op::Acb => {
            ops.last()
        }
        _ => None,
    }
}

/// The VAX register among R2-R11 an ARM64 instruction writes, if its first
/// operand is one of x19-x28 and it writes it.
fn native_writes(mn: &str, rest: &str) -> Option<u8> {
    let reads = [
        "ST", "CMP", "CMN", "TST", "B", "CB", "TB", "RET", "SVC", "NOP", "PRFM",
    ];
    if reads.iter().any(|r| mn.starts_with(r)) {
        return None;
    }
    let first = rest.split(',').next()?.trim().to_ascii_lowercase();
    let n: u8 = first.strip_prefix(['x', 'w'])?.parse().ok()?;
    (19..=28).contains(&n).then(|| n - 17)
}

/// A CALL routine's prologue: the frame (docs/macro32.md), saving `saved`.
fn call_prologue(saved: &[u8]) -> Vec<String> {
    let size = frame_size(saved);
    // What it saved, for $UNWIND, which RETs from any frame.
    let mask: u32 = saved.iter().map(|r| 1 << r).sum();
    let mut out = vec![
        format!("\tsub sp, sp, #{size}"),
        "\tstp xzr, x12, [sp]".into(),
        "\tstp x29, x30, [sp, #16]".into(),
        format!("\tmov x14, #{mask}"),
        "\tstp x18, x14, [sp, #32]".into(),
    ];
    out.extend(saves(saved, "sp", 48, "stp", "str"));
    out.extend([
        "\tmov x29, sp".into(),
        "\tmov x12, x13".into(),
        "\tmov x18, sp".into(),
    ]);
    out
}

/// The call frame: condition handler, AP, FP, LR, the caller's SP, the
/// mask of what it saves, then the saved registers, 16-byte aligned.
fn frame_size(saved: &[u8]) -> usize {
    (48 + 8 * saved.len()).next_multiple_of(16)
}

/// Stores or loads the saved registers, in pairs, from `base` + `at`.
fn saves(saved: &[u8], base: &str, at: usize, pair: &str, one: &str) -> Vec<String> {
    let arm = |r: &u8| operand::arm(*r).unwrap();
    saved
        .chunks(2)
        .enumerate()
        .map(|(i, regs)| match regs {
            [a, b] => format!(
                "\t{pair} x{}, x{}, [{base}, #{}]",
                arm(a),
                arm(b),
                at + 16 * i
            ),
            [a] => format!("\t{one} x{}, [{base}, #{}]", arm(a), at + 16 * i),
            _ => unreachable!(),
        })
        .collect()
}

/// `RET` from a CALL routine that saves `saved`.
pub(crate) fn epilogue(saved: &[u8]) -> Vec<String> {
    let mut out = vec!["\tmov sp, x29".to_string()];
    out.extend(saves(saved, "sp", 48, "ldp", "ldr"));
    out.extend([
        "\tldr x18, [sp, #32]".into(),
        "\tldp x29, x30, [sp, #16]".into(),
        "\tldr x12, [sp, #8]".into(),
        format!("\tadd sp, sp, #{}", frame_size(saved)),
        "\tret".into(),
    ]);
    out
}

/// A JSB routine's prologue, if it saves anything: the registers, all 64
/// bits, below both stacks, with the caller's `sp` and VAX SP, then both
/// stacks below them, so that what it pushes doesn't overwrite them.
fn jsb_prologue(saved: &[u8]) -> Vec<String> {
    if saved.is_empty() {
        return Vec::new();
    }
    let size = (16 + 8 * saved.len()).next_multiple_of(16);
    let mut out = vec![
        "\tmov x16, sp".to_string(),
        "\tcmp x16, x18".into(),
        "\tcsel x17, x16, x18, lo".into(),
        "\tand x17, x17, #0xfffffffffffffff0".into(),
        format!("\tsub x17, x17, #{size}"),
        "\tstp x16, x18, [x17]".into(),
    ];
    out.extend(saves(saved, "x17", 16, "stp", "str"));
    out.extend(["\tmov x18, x17".into(), "\tmov sp, x17".into()]);
    out
}

/// `RSB` from a JSB routine that saves `saved`, with VAX SP back where the
/// prologue left it: the registers, then both stacks.
pub(crate) fn jsb_epilogue(saved: &[u8]) -> Vec<String> {
    if saved.is_empty() {
        return Vec::new();
    }
    let mut out = saves(saved, "x18", 16, "ldp", "ldr");
    out.extend([
        "\tldp x16, x17, [x18]".into(),
        "\tmov sp, x16".into(),
        "\tmov x18, x17".into(),
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
