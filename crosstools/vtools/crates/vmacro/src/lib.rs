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

use std::collections::HashMap;

use routine::{KEPT, Kind, Params, Regs, Returns, Routine, Survey};
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
/// restore, and for a JSB routine whether it saved x30.
pub(crate) enum Exit {
    None,
    Call(Frame),
    Frameless,
    Jsb(Vec<u8>, bool),
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
    /// Whether x12 may no longer be AP, past a call or a label.
    ap_stale: bool,
    /// `.ENABLE QUADWORD`: address arithmetic in 64 bits; and the command
    /// line's `/ENABLE=QUADWORD`, which each pass starts with.
    quadword: bool,
    quad_default: bool,
    /// `.DISABLE FLAGGING`: no porting message for raw ARM64; and whether
    /// the routine had one.
    flagging: bool,
    warned_raw: bool,
    /// `$SETUP_CALL64`'s count, and how many `$PUSH_ARG64` have pushed.
    call64: Option<(usize, usize)>,
    /// Registers the current routine was warned about writing, and whether
    /// about reading AP.
    warned: Regs,
    warned_ap: bool,
    /// Whether the last instruction may go on to the next.
    falls: bool,
    /// Whether the current routine loaded SP from elsewhere: its frame,
    /// and what it saved, are no longer on the stack. And whether it
    /// loaded FP: its RET returns from that frame, as its descriptor says.
    switched: bool,
    fp_switched: bool,
    /// The psect code goes in, and those `.SAVE_PSECT` saved.
    psect: String,
    saved_psects: Vec<String>,
    /// The bytes the current routine has pushed on the VAX stack, if vmacro
    /// can tell, and at its labels: those it has gone past, and those
    /// branched to ahead.
    depth: Option<i64>,
    depths: HashMap<String, (Option<i64>, bool)>,
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
            ap_stale: true,
            quadword: false,
            quad_default: false,
            flagging: true,
            warned_raw: false,
            call64: None,
            warned: 0,
            warned_ap: false,
            falls: false,
            switched: false,
            fp_switched: false,
            psect: "$CODE$".into(),
            saved_psects: Vec::new(),
            depth: None,
            depths: HashMap::new(),
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
            ".PSECT" => {
                if let Some(name) = operand::split(rest).first() {
                    self.psect = name.to_ascii_uppercase();
                }
                psect(rest).map(|line| Ok(vec![line]))
            }
            ".SAVE_PSECT" => {
                self.saved_psects.push(self.psect.clone());
                None
            }
            ".RESTORE_PSECT" => {
                if let Some(p) = self.saved_psects.pop() {
                    self.psect = p;
                }
                None
            }
            ".ERROR" => Some(Err(format!("%MACRO-E-GENERR, {}", rest.trim()))),
            // Listing control, and what only matters on a VAX.
            ".ENABLE" | ".ENABL" | ".DISABLE" | ".DSABL" => {
                let on = word.starts_with(".EN");
                for a in operand::split(rest) {
                    match a.trim().to_ascii_uppercase().as_str() {
                        "QUADWORD" => self.quadword = on,
                        "FLAGGING" => self.flagging = on,
                        _ => {}
                    }
                }
                Some(Ok(Vec::new()))
            }
            "$SETUP_CALL64" | "$PUSH_ARG64" | "$CALL64" | "$IS_32BITS" | "$IS_DESC64"
            | "$PUSH64" | "$POP64" => Some(self.macro64(word, rest, constant)),
            ".SBTTL" | ".SUBTITLE" | ".PAGE" | ".LIST" | ".NLIST" | ".SHOW" | ".NOSHOW"
            | ".DEFAULT" | ".CROSS" | ".NOCROSS" | ".PRINT" | ".PRESERVE" => Some(Ok(Vec::new())),
            _ if word.starts_with('.') => None,
            _ => match self.instruction(word, rest, &labels, constant) {
                Some(r) => Some(r),
                None => self.native(word, rest),
            },
        }
    }

    fn label(&mut self, name: &str, global: bool) {
        // The labels vmacro makes.
        if name.starts_with("FDSC$$") || name.starts_with("BODY$$") {
            return;
        }
        self.ap_stale = true;
        // vasm starts a local label block at each other label.
        let key = name.split('@').next().unwrap_or(name);
        if !name.contains('@') {
            self.depths.retain(|k, _| !k.ends_with('$'));
        }
        if !self.survey.entries.contains_key(name) {
            let ahead = self.depths.get(key).map(|&(d, _)| d);
            self.depth = match (self.falls, ahead) {
                (false, Some(d)) => d,
                (false, None) => None,
                (true, Some(d)) if d != self.depth => None,
                (true, _) => self.depth,
            };
            self.depths.insert(key.to_string(), (self.depth, true));
        }
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
            quadword: self.quad_default,
            quad_default: self.quad_default,
            ..Macro32::default()
        };
        true
    }

    fn enable(&mut self, what: &str) -> bool {
        let quad = what.eq_ignore_ascii_case("QUADWORD");
        if quad {
            (self.quadword, self.quad_default) = (true, true);
        }
        quad
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
        if op == Op::Evax(insn::Evax::Unsupported) {
            return Some(Err(format!(
                "{mn} isn't a built-in here: byte manipulation, TRAPB, RPCC, the FPCR and PAL calls vaxpunk lacks have no ARM64 meaning (docs/macro32.md)"
            )));
        }
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
            let tail = self.check(op, &ops)?;
            let mut warnings = Vec::new();
            let moves_sp = matches!(
                op,
                Op::Pushl
                    | Op::Pusha
                    | Op::Pushr
                    | Op::Popr
                    | Op::Calls
                    | Op::Callg
                    | Op::Callg64
                    | Op::Jsb
                    | Op::Ret
                    | Op::Rsb
                    | Op::Rei
            ) || ops.iter().enumerate().any(|(i, o)| {
                matches!(
                    o,
                    operand::Opnd::Mem(
                        operand::Mode::Inc(14) | operand::Mode::Dec(14) | operand::Mode::IncDef(14),
                        _
                    )
                ) || (*o == operand::Opnd::Reg(14) && writes(op, i, ops.len()))
            });
            if self.call64.is_some() && moves_sp {
                return Err(BETWEEN.into());
            }
            let stacked = self.stack(op, size, &ops, constant);
            let ap = ap_use(op, size, &ops, constant);
            if self.compiling {
                stacked?;
                let (args, list) = ap?;
                // The caller homes the list, which vmacro sees in the module.
                let r = &self.survey.routines[cur];
                let jsb = matches!(r.kind, Kind::Jsb | Kind::Jsb32);
                if (args.is_some() || list)
                    && jsb
                    && !r.called_here
                    && !r.takes_ap
                    && !self.warned_ap
                {
                    self.warned_ap = true;
                    warnings.push("a JSB routine that reads AP reads its caller's argument list, which its callers must home (HOME_ARGS=TRUE): say so with INPUT=<AP>".into());
                }
            } else if let Ok((args, list)) = ap {
                let r = &mut self.survey.routines[cur];
                r.ap_args = r.ap_args.max(args);
                r.ap_list |= list;
            }
            if (0..ops.len()).any(|i| writes(op, i, ops.len()) && ops[i] == operand::Opnd::Reg(13))
            {
                self.fp_switched = true;
            }
            if switches(op, &ops) {
                self.switched = true;
                if !self.compiling {
                    self.survey.longjumps.extend(labels.iter().cloned());
                }
            }
            // A JSB routine that branches to its own entry goes on past its
            // prologue, with what it saved as it was.
            let r = &self.survey.routines[cur];
            let mut ops = ops;
            if matches!(r.kind, Kind::Jsb | Kind::Jsb32)
                && let Some(t) = branch_target(op, &ops).and_then(target)
                && self.survey.entries.get(t) == Some(&cur)
            {
                *ops.last_mut().unwrap() =
                    operand::Opnd::Mem(operand::Mode::Rel(format!("BODY$${cur}")), None);
            }
            let quad_ap = self.survey.routines[cur].quad_args;
            let mut g = operand::Gen::new(&ops, constant, &mut self.labels);
            g.quadword = self.quadword;
            g.quad_ap = quad_ap;
            let r = &self.survey.routines[cur];
            let exit = match r.kind {
                Kind::Call if r.frameless => Exit::Frameless,
                Kind::Call => Exit::Call(frame(&routine::list(r.saved), r.home, r.quad_args)),
                Kind::Jsb | Kind::Jsb32 => Exit::Jsb(routine::list(r.saved), r.lr),
                Kind::Exception => Exit::None,
            };
            let reads_ap = r.ap;
            let flags = match (&exit, tail) {
                (Exit::Jsb(saved, lr), true) => {
                    tail_call(&mut g, mn, op, size, &ops, &self.flags, &exit, saved, *lr)?
                }
                (Exit::Call(_), _) if op == Op::Ret && self.fp_switched => {
                    any_epilogue(&mut g);
                    None
                }
                _ => insn::compile(&mut g, mn, op, size, &ops, &self.flags, &exit)?,
            };
            let written = g.written;
            let mut lines = g.out;
            // AP is x12 where the routine reads it, which a call changes:
            // it is set again before AP is read after a call or a label,
            // so that what follows a call is still where it returns to.
            let calls = matches!(op, Op::Jsb | Op::Calls | Op::Callg | Op::Callg64);
            if reads_ap && self.ap_stale && ops.iter().any(names_ap) {
                lines.insert(0, "\tadd x12, x29, #32".into());
                self.ap_stale = false;
            }
            self.ap_stale |= calls;
            if self.compiling {
                warnings.extend(self.unmasked(cur, written));
            } else {
                let linkage = self.linkage;
                let r = &mut self.survey.routines[cur];
                r.direct |= written;
                r.calls_out |= calls;
                r.stacked |= ops.iter().any(names_stack)
                    || matches!(op, Op::Pushl | Op::Pusha | Op::Pushr | Op::Popr | Op::Rei);
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
    /// the labels it branches to: errors, and whether it is a tail call,
    /// a branch to a JSB routine's entry, which needs this one's epilogue.
    fn check(&self, op: insn::Op, ops: &[operand::Opnd]) -> Result<bool, String> {
        use insn::Op;
        if !self.compiling {
            return Ok(false);
        }
        let s = &self.survey;
        if op == Op::Jsb && ops[0] == operand::Opnd::Mem(operand::Mode::IncDef(14), None) {
            return Err("JSB @(SP)+, a co-routine call, needs the return address on the VAX stack, where vaxpunk doesn't put it".into());
        }
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
            Op::Calls | Op::Callg | Op::Callg64 => ops.get(1),
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
            return Ok(false);
        };
        let here = self.cur.map(|c| &s.routines[c]);
        if self.switched || s.noreturn.contains(t) || s.longjumps.contains(t) {
            return Ok(false);
        }
        let (there, name) = match s.labels.get(t) {
            Some(r) if *r == self.cur => return Ok(false),
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
            return Ok(false);
        }
        // A JSB routine's entry builds its own save area: going there from
        // a JSB routine is a tail call, which restores what this one saved
        // first; a CALL routine's frame would stay. Code in the middle of
        // another routine ends in that routine's RET or RSB, which must
        // restore what this one saves, from the same frame. A label in
        // another module that isn't known is a JSB routine's that saves
        // nothing.
        let exit = |r: Option<&Routine>| r.map_or(Returns::default(), Routine::returns);
        let (entry, theirs) = match there {
            Some(r) => (s.entries.contains_key(t), exit(Some(r))),
            None => s
                .foreign
                .get(t)
                .copied()
                .unwrap_or((true, Returns::default())),
        };
        let mine = exit(here);
        let jsb = here.is_some_and(|r| r.kind != Kind::Call);
        let plain = mine == Returns::default();
        let tail = entry && !theirs.call && jsb && !plain;
        let ok = if entry {
            !theirs.call && (jsb || plain)
        } else {
            mine == theirs
        };
        if !ok {
            let saves = |r: Returns| {
                let regs = match r.saved {
                    0 => "none".to_string(),
                    s => routine::list(s)
                        .iter()
                        .map(|n| format!("R{n}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                match r.lr {
                    true => format!("{regs} and x30"),
                    false => regs,
                }
            };
            if (mine.call, mine.saved, mine.lr) == (theirs.call, theirs.saved, theirs.lr) {
                let homes = |h: Option<u32>| h.map_or("none".into(), |n| n.to_string());
                return Err(format!(
                    "a branch to {name} which has another argument list in its frame (this routine homes {} arguments; it homes {})",
                    homes(mine.home),
                    homes(theirs.home)
                ));
            }
            return Err(format!(
                "a branch to {name} which doesn't restore the registers this routine saves ({}; it restores {})",
                saves(mine),
                saves(theirs)
            ));
        }
        Ok(tail)
    }

    /// The 64-bit macros AMACRO's library had, which vmacro does itself
    /// since they put arguments where MACRO-32 can't name them ([MCG] App.
    /// D, E): `$SETUP_CALL64 n`, `$PUSH_ARG64 op` for each, the last first,
    /// and `$CALL64 target`; `$IS_32BITS q, leq, gtr`; `$IS_DESC64 desc,
    /// target[, SIZE=LONG|QUAD]`; `$PUSH64 reg` and `$POP64 reg`.
    fn macro64(
        &mut self,
        word: &str,
        rest: &str,
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Result<Vec<String>, String> {
        use operand::{Mode, Opnd};
        let Some(cur) = self.cur else {
            return Err(OUTSIDE.into());
        };
        let p = Params::parse(rest);
        // The positional arguments, empty ones too: `$IS_32BITS q, , gtr`.
        let texts: Vec<String> = operand::split(rest)
            .into_iter()
            .filter(|a| !a.contains('='))
            .collect();
        let label = |i: usize| texts.get(i).map(String::as_str).filter(|t| !t.is_empty());
        // The operand, then the labels branched to.
        let ops = match (word, texts.first()) {
            ("$SETUP_CALL64", _) | (_, None) => Vec::new(),
            (_, Some(t)) => vec![operand::parse(t)?],
        };
        let labels: Vec<Opnd> = match word {
            "$IS_32BITS" => vec![label(1), label(2)],
            "$IS_DESC64" => vec![label(1)],
            _ => Vec::new(),
        }
        .into_iter()
        .flatten()
        .map(|l| Opnd::Mem(Mode::Rel(l.to_string()), None))
        .collect();
        if self.call64.is_some() && matches!(word, "$PUSH64" | "$POP64") {
            return Err(BETWEEN.into());
        }
        // What instruction() does for an instruction's operands: AP, the
        // VAX stack, the branches and the routine called.
        let ap = ap_use(insn::Op::Tst, operand::Size::Q, &ops, constant);
        let mut lines = Vec::new();
        if word != "$PUSH64" && word != "$POP64" {
            let stacked = self.stack(insn::Op::Tst, operand::Size::Q, &ops, constant);
            for l in &labels {
                self.stack(
                    insn::Op::Br,
                    operand::Size::L,
                    std::slice::from_ref(l),
                    constant,
                )?;
            }
            if self.compiling {
                stacked?;
            }
        }
        if self.compiling {
            ap?;
            for l in &labels {
                if self.check(insn::Op::Br, std::slice::from_ref(l))? {
                    return Err(format!(
                        "{word} to another JSB routine: a tail call is a BRB or BRW"
                    ));
                }
            }
            if word == "$CALL64"
                && let Some(t) = ops.first()
            {
                self.check(insn::Op::Callg, &[Opnd::Imm("0".into()), t.clone()])?;
            }
            if self.survey.routines[cur].ap && self.ap_stale && ops.iter().any(names_ap) {
                lines.push("\tadd x12, x29, #32".to_string());
                self.ap_stale = false;
            }
        } else {
            let r = &mut self.survey.routines[cur];
            if let Ok((args, list)) = ap {
                r.ap_args = r.ap_args.max(args);
                r.ap_list |= list;
            }
            r.branches
                .extend(labels.iter().filter_map(target).map(str::to_string));
        }
        let quad_ap = self.survey.routines[cur].quad_args;
        let call64 = self.call64;
        let mut depth = self.depth;
        let mut g = operand::Gen::new(&ops, constant, &mut self.labels);
        g.quadword = self.quadword;
        g.quad_ap = quad_ap;
        let area = |n: usize| (8 * n).next_multiple_of(16);
        let (mut calls, mut falls) = (false, true);
        let mut next = call64;
        match word {
            "$SETUP_CALL64" => {
                p.check(&["INLINE"])?;
                if call64.is_some() {
                    return Err("$SETUP_CALL64 before the last one's $CALL64".into());
                }
                let n = texts
                    .first()
                    .and_then(|t| constant(t.trim()))
                    .filter(|n| (0..=255).contains(n))
                    .ok_or("$SETUP_CALL64 takes the argument count, 0 to 255")?
                    as usize;
                // Below both stacks, sp first: a slot for each argument.
                for line in [
                    "mov x16, sp".to_string(),
                    "cmp x16, x18".into(),
                    "csel x16, x16, x18, lo".into(),
                    "and x16, x16, #0xfffffffffffffff0".into(),
                    format!("sub sp, x16, #{}", area(n)),
                ] {
                    g.emit(line);
                }
                next = Some((n, 0));
            }
            "$PUSH_ARG64" => {
                let Some((n, pushed)) = call64 else {
                    return Err("$PUSH_ARG64 without $SETUP_CALL64".into());
                };
                if pushed == n || ops.len() != 1 {
                    return Err(format!(
                        "$PUSH_ARG64 pushes one of the {n} arguments $SETUP_CALL64 said"
                    ));
                }
                let v = insn::q_read(&mut g, &ops[0])?;
                g.emit(format!("str {v}, [sp, #{}]", 8 * (n - pushed - 1)));
                next = Some((n, pushed + 1));
            }
            "$CALL64" => {
                let Some((n, pushed)) = call64 else {
                    return Err("$CALL64 without $SETUP_CALL64".into());
                };
                self.call64 = None;
                if pushed != n {
                    return Err(format!(
                        "$CALL64 after {pushed} $PUSH_ARG64 of the {n} $SETUP_CALL64 said"
                    ));
                }
                let call = match ops.first() {
                    Some(o) if insn::direct(&g, o).is_some() => {
                        format!("bl {}", insn::direct(&g, o).unwrap())
                    }
                    Some(o) => {
                        let t = g.address(o, operand::Size::B)?;
                        g.emit(format!("mov x13, {t}"));
                        "blr x13".into()
                    }
                    None => return Err("$CALL64 takes the routine to call".into()),
                };
                for i in (0..n.min(8)).step_by(2) {
                    g.emit(if i + 1 < n {
                        format!("ldp x{i}, x{}, [sp, #{}]", i + 1, 8 * i)
                    } else {
                        format!("ldr x{i}, [sp, #{}]", 8 * i)
                    });
                }
                // The rest are at sp, as the calling standard has them.
                let past = if n > 8 { 64 } else { area(n) };
                if past > 0 {
                    g.emit(format!("add sp, sp, #{past}"));
                }
                g.emit(format!("mov x9, #{n}"));
                g.emit(call);
                if n > 8 {
                    g.emit(format!("add sp, sp, #{}", area(n) - 64));
                }
                next = None;
                calls = true;
            }
            "$IS_32BITS" => {
                p.check(&["TEMP_REG"])?;
                let v = insn::q_read(&mut g, ops.first().ok_or("$IS_32BITS takes a quadword")?)?;
                g.emit(format!("cmp {v}, {}, sxtw", operand::w(&v)));
                if let Some(l) = label(1) {
                    g.emit(format!("b.eq {l}"));
                }
                if let Some(l) = label(2) {
                    g.emit(format!("b.ne {l}"));
                    falls = label(1).is_none();
                }
            }
            "$IS_DESC64" => {
                p.check(&["SIZE"])?;
                let (Some(desc), Some(to)) = (ops.first(), label(1)) else {
                    return Err("$IS_DESC64 takes a descriptor's address and a label".into());
                };
                let a = match p
                    .keyed
                    .iter()
                    .find(|(k, _)| k == "SIZE")
                    .map(|(_, v)| v.to_ascii_uppercase())
                {
                    Some(s) if s == "QUAD" => insn::q_read(&mut g, desc)?,
                    None => {
                        let v = g.read(desc, operand::Size::L, operand::Ext::Sext)?;
                        operand::x(&v)
                    }
                    Some(s) if s == "LONG" => {
                        let v = g.read(desc, operand::Size::L, operand::Ext::Sext)?;
                        operand::x(&v)
                    }
                    Some(s) => return Err(format!("SIZE is LONG or QUAD, not {s}")),
                };
                // MBO, 1, where a 32-bit one has its length, and MBMO, -1,
                // where it has its address: both, since a 32-bit one may
                // pass either alone.
                let (t, skip) = (operand::w(&g.tmp()?), g.label());
                g.emit(format!("ldrh {t}, [{a}]"));
                g.emit(format!("cmp {t}, #1"));
                g.emit(format!("b.ne {skip}"));
                g.emit(format!("ldr {t}, [{a}, #4]"));
                g.emit(format!("cmn {t}, #1"));
                g.emit(format!("b.eq {to}"));
                g.place_label(&skip);
            }
            _ => {
                let Some(&Opnd::Reg(n)) = ops.first().filter(|_| ops.len() == 1) else {
                    return Err(format!("{word} takes a register"));
                };
                if n > 11 {
                    return Err(format!("{word} takes one of R0-R11"));
                }
                let r = operand::arm(n)?;
                if word == "$PUSH64" {
                    g.emit(format!("str x{r}, [x18, #-8]!"));
                    depth = depth.map(|d| d + 8);
                } else {
                    g.wrote(n);
                    g.emit(format!("ldr x{r}, [x18], #8"));
                    let jsb_or_call = self.survey.routines[cur].kind != Kind::Exception;
                    match depth {
                        Some(d) if d < 8 && jsb_or_call && self.compiling => {
                            return Err(format!(
                                "$POP64 reaches past what this routine pushed ({d} bytes): the VAX stack has no return address or frame (DESIGN-0004)"
                            ));
                        }
                        d => depth = d.map(|d| d - 8),
                    }
                }
            }
        }
        lines.extend(g.out);
        let written = g.written;
        self.call64 = next;
        self.depth = depth;
        self.falls = falls;
        self.flags = Flags::Live { borrow: false };
        if calls {
            self.ap_stale = true;
        }
        if !self.compiling {
            let r = &mut self.survey.routines[cur];
            r.direct |= written;
            r.calls_out |= calls;
            r.stacked |= matches!(word, "$SETUP_CALL64" | "$CALL64" | "$PUSH64" | "$POP64");
        }
        Ok(lines)
    }

    /// Follows the VAX stack through an instruction: the bytes the routine
    /// has pushed, and those at the labels it branches to. Reading or
    /// popping more than it pushed reaches what the VAX had there, the
    /// return address or the frame, which vaxpunk keeps elsewhere
    /// (DESIGN-0004): an error in a CALL or JSB routine.
    fn stack(
        &mut self,
        op: insn::Op,
        size: operand::Size,
        ops: &[operand::Opnd],
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Result<(), String> {
        use insn::{Alu, Op};
        use operand::{Mode, Opnd};
        let checked = self
            .cur
            .is_some_and(|c| self.survey.routines[c].kind != Kind::Exception);
        let mut err = None;
        let mut past = |d: i64, what: &str| {
            if checked && err.is_none() {
                let theirs = if d == 0 {
                    "it pushed nothing".to_string()
                } else {
                    format!("it pushed {d} bytes")
                };
                err = Some(format!(
                    "{what} reaches past what this routine pushed ({theirs}): the VAX stack has no return address or frame (DESIGN-0004)"
                ));
            }
        };
        let mut depth = self.depth;
        let sp = Some(&Opnd::Reg(14));
        let writes_sp = (0..ops.len()).any(|i| writes(op, i, ops.len()) && Some(&ops[i]) == sp);
        if let (Op::Mova, Some(Opnd::Mem(Mode::Disp(e, 14), None)), true) =
            (op, ops.first(), writes_sp)
        {
            // MOVAx n(SP), SP pops n bytes.
            depth = match (depth, constant(e)) {
                (Some(d), Some(n)) if n <= d => Some(d - n),
                (Some(d), Some(_)) => {
                    past(d, "MOVA to SP");
                    None
                }
                _ => None,
            };
        } else {
            for (i, o) in ops.iter().enumerate() {
                let (Opnd::Mem(mode, index), Some(d)) = (o, depth) else {
                    continue;
                };
                let bytes = match (op, i) {
                    _ if address(op, i) => 0,
                    (Op::Movz(s) | Op::Cvt(s), 0) => s.bytes(),
                    _ => size.bytes(),
                };
                match mode {
                    Mode::Dec(14) => depth = Some(d + bytes),
                    Mode::Inc(14) if bytes > d => past(d, "(SP)+"),
                    Mode::Inc(14) => depth = Some(d - bytes),
                    Mode::IncDef(14) if 4 > d => past(d, "@(SP)+"),
                    Mode::IncDef(14) => depth = Some(d - 4),
                    Mode::Def(14) if index.is_none() && bytes > d => past(d, "(SP)"),
                    Mode::Disp(e, 14) | Mode::DispDef(e, 14) if index.is_none() => {
                        let bytes = if matches!(mode, Mode::DispDef(..)) {
                            4
                        } else {
                            bytes
                        };
                        if let Some(n) = constant(e)
                            && n >= 0
                            && n + bytes > d
                        {
                            past(d, &format!("{n}(SP)"));
                        }
                    }
                    _ => {}
                }
            }
            let mask = |o: &Opnd| match o {
                Opnd::Imm(e) => constant(e).map(|m| 4 * i64::from((m & 0x3fff).count_ones())),
                _ => None,
            };
            depth = match (op, depth) {
                (_, None) => None,
                (Op::Pushl | Op::Pusha, Some(d)) => Some(d + 4),
                (Op::Pushr, Some(d)) => mask(&ops[0]).map(|n| d + n),
                (Op::Popr | Op::Calls, Some(d)) => {
                    let n = match op {
                        Op::Popr => mask(&ops[0]),
                        _ => match &ops[0] {
                            Opnd::Imm(e) => constant(e).map(|n| 4 * n),
                            _ => None,
                        },
                    };
                    match n {
                        Some(n) if n > d => {
                            past(d, if op == Op::Popr { "POPR" } else { "CALLS" });
                            None
                        }
                        Some(n) => Some(d - n),
                        None => None,
                    }
                }
                (Op::Arith(alu @ (Alu::Add | Alu::Sub), false), Some(d)) if writes_sp => {
                    match (&ops[0], alu) {
                        (Opnd::Imm(e), Alu::Sub) => constant(e).map(|n| d + n),
                        (Opnd::Imm(e), _) => match constant(e) {
                            Some(n) if n > d => {
                                past(d, "ADD to SP");
                                None
                            }
                            n => n.map(|n| d - n),
                        },
                        _ => None,
                    }
                }
                (Op::Rsb, Some(d)) if d != 0 => {
                    if checked && err.is_none() {
                        err = Some(format!(
                            "RSB with {d} bytes pushed: the return address is in x30, not on the VAX stack (DESIGN-0004)"
                        ));
                    }
                    None
                }
                _ if writes_sp => None,
                (_, d) => d,
            };
        }
        self.depth = depth;
        // A branch back with another depth than the label had is a loop
        // that pushes or pops: unknown past it.
        if let Some(Opnd::Mem(Mode::Rel(e), None)) = branch_target(op, ops) {
            let key = e.trim().to_string();
            match self.depths.get(&key) {
                Some(&(at, true)) if at != depth => self.depth = None,
                Some(&(at, false)) if at != depth => {
                    self.depths.insert(key, (None, false));
                }
                Some(_) => {}
                None => {
                    self.depths.insert(key, (depth, false));
                }
            }
        }
        err.map_or(Ok(()), Err)
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
        if !self.compiling {
            let r = &mut self.survey.routines[cur];
            if let Some(n) = native_writes(mn, rest) {
                r.direct |= 1 << n;
            }
            r.calls_out |= matches!(mn, "BL" | "BLR");
            r.stacked |= rest.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
                matches!(
                    w.to_ascii_lowercase().as_str(),
                    "sp" | "x18" | "w18" | "x29" | "w29" | "x30" | "w30" | "fp" | "lr"
                )
            });
        }
        self.ap_stale |= matches!(mn, "BL" | "BLR");
        self.falls = !matches!(mn, "B" | "BR" | "RET" | "ERET");
        // x2-x17 and x30 are the translation's, x18-x29 VAX SP, R2-R11 and
        // FP: a built-in says what is meant in MACRO-32 terms.
        let raw = rest.split(|c: char| !c.is_ascii_alphanumeric()).find(|w| {
            w.strip_prefix(['x', 'w', 'X', 'W'])
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (2..=30).contains(&n))
        });
        if self.compiling
            && self.flagging
            && !self.warned_raw
            && let Some(r) = raw
        {
            self.warned_raw = true;
            return Some(Ok(vec![
                format!("\t{mn} {rest}"),
                format!(
                    "\t.WARN ARM64 code naming {r}, which vmacro uses itself or keeps a VAX register in: a built-in says it in MACRO-32 (.DISABLE FLAGGING if meant)"
                ),
            ]));
        }
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
        let max_args = match p.number("MAX_ARGS")? {
            Some(n @ 0..=255) => Some(n as u32),
            Some(_) => return Err("MAX_ARGS is 0 to 255".into()),
            None => None,
        };
        let r = Routine {
            max_args,
            takes_ap: p.regs("INPUT")? & 1 << 12 != 0,
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
        if kind == Kind::Call && r.preserve & 3 != 0 {
            return Err(
                "a CALL routine returns R0 and R1: only a JSB routine can PRESERVE them".into(),
            );
        }
        let i = self.next;
        self.next += 1;
        let mut out = Vec::new();
        if self.compiling
            && self.falls
            && let Some(prev) = self.cur
        {
            // Into its prologue, and out through its RET or RSB, which
            // returns to x30: still the caller's if the code before called
            // nothing.
            let plain = |r: &Routine| r.kind != Kind::Call && r.saved == 0;
            let s = &self.survey.routines;
            if !plain(&s[prev]) || s[prev].lr || !plain(&s[i]) {
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
        self.warned_ap = false;
        self.warned_raw = false;
        self.switched = false;
        self.fp_switched = false;
        self.flags = Flags::Live { borrow: true };
        self.depth = Some(0);
        self.depths.clear();
        let r = &self.survey.routines[i];
        let saved = routine::list(r.saved);
        let reads_ap = r.ap;
        if word == ".ENTRY" {
            out.push(format!("{}::", names[0]));
        }
        match kind {
            Kind::Call if r.frameless => {}
            Kind::Call => {
                let fdsc = format!("FDSC$${i}");
                let f = frame(&saved, r.home, r.quad_args);
                out.extend(descriptor(&fdsc, &names[0], &saved, &f, &self.psect));
                self.labels += 2;
                out.extend(call_prologue(&f, &fdsc, 90000 + self.labels - 1));
            }
            Kind::Jsb | Kind::Jsb32 => {
                out.extend(jsb_prologue(&saved, r.lr));
                out.push(format!("BODY$${i}:"));
            }
            Kind::Exception => {}
        }
        // AP is the list at 32(FP): the caller's, in a routine without a
        // frame.
        if reads_ap {
            out.push("\tadd x12, x29, #32".into());
        }
        self.ap_stale = !reads_ap;
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

const BETWEEN: &str = "between $SETUP_CALL64 and $CALL64 nothing may push, pop, call or return: the arguments wait below VAX SP";

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

/// A branch to a JSB routine's entry from one with a save area: its
/// epilogue, then the branch, out of line if it is conditional.
#[allow(clippy::too_many_arguments)]
fn tail_call(
    g: &mut operand::Gen,
    mn: &str,
    op: insn::Op,
    size: operand::Size,
    ops: &[operand::Opnd],
    flags: &Flags,
    exit: &Exit,
    saved: &[u8],
    lr: bool,
) -> Result<Option<Flags>, String> {
    use operand::{Mode, Opnd};
    let epilogue = jsb_epilogue(saved, lr);
    if matches!(op, insn::Op::Br | insn::Op::Jmp) {
        g.out.extend(epilogue);
        return insn::compile(g, mn, op, size, ops, flags, exit);
    }
    let (out, past) = (g.label(), g.label());
    let mut inner = ops.to_vec();
    let target = inner.pop().unwrap();
    inner.push(Opnd::Mem(Mode::Rel(out.clone()), None));
    let f = insn::compile(g, mn, op, size, &inner, flags, exit)?;
    g.emit(format!("b {past}"));
    g.place_label(&out);
    g.out.extend(epilogue);
    insn::compile(
        g,
        "JMP",
        insn::Op::Jmp,
        operand::Size::B,
        &[target],
        flags,
        exit,
    )?;
    g.place_label(&past);
    Ok(f)
}

/// Whether an operand uses SP or FP.
fn names_stack(o: &operand::Opnd) -> bool {
    use operand::{Mode, Opnd};
    let stack = |n: u8| n == 13 || n == 14;
    match o {
        Opnd::Reg(n) => stack(*n),
        Opnd::Imm(_) => false,
        Opnd::Mem(mode, index) => {
            index.is_some_and(stack)
                || match mode {
                    Mode::Def(n)
                    | Mode::Inc(n)
                    | Mode::Dec(n)
                    | Mode::IncDef(n)
                    | Mode::Disp(_, n)
                    | Mode::DispDef(_, n) => stack(*n),
                    _ => false,
                }
        }
    }
}

/// Whether an operand names AP.
fn names_ap(o: &operand::Opnd) -> bool {
    use operand::{Mode, Opnd};
    match o {
        Opnd::Reg(n) => *n == 12,
        Opnd::Imm(_) => false,
        Opnd::Mem(mode, index) => {
            *index == Some(12)
                || matches!(
                    mode,
                    Mode::Def(12)
                        | Mode::Inc(12)
                        | Mode::Dec(12)
                        | Mode::IncDef(12)
                        | Mode::Disp(_, 12)
                        | Mode::DispDef(_, 12)
                )
        }
    }
}

/// Whether an instruction's operand `i` is an address, not a value it
/// reads or writes there.
fn address(op: insn::Op, i: usize) -> bool {
    use insn::Op;
    matches!(
        (op, i),
        (Op::Mova | Op::Pusha | Op::Jmp | Op::Jsb, 0)
            | (Op::Calls | Op::Callg | Op::Callg64, _)
            | (Op::Evax(insn::Evax::Lda), 1)
            | (Op::Movc3, 1 | 2)
            | (Op::Movc5, 1 | 4)
            | (Op::Cmpc3, 1 | 2)
            | (Op::Cmpc5, 1 | 4)
            | (Op::Locc(_), 2)
            | (Op::Insque | Op::Remque, _)
    )
}

/// Whether an instruction writes its operand `i` of `n`, as a whole.
fn writes(op: insn::Op, i: usize, n: usize) -> bool {
    use insn::{Evax, Op};
    let first = matches!(
        op,
        Op::Ldq | Op::Evax(Evax::Ldu | Evax::Lda | Evax::Ldqu | Evax::Ldl | Evax::Stc)
    );
    (writes_last(op) && i + 1 == n) || (first && i == 0)
}

/// Whether an instruction writes its last operand.
fn writes_last(op: insn::Op) -> bool {
    use insn::Op;
    matches!(
        op,
        Op::Mov
            | Op::Clr
            | Op::Mcom
            | Op::Mneg
            | Op::Movz(_)
            | Op::Cvt(_)
            | Op::Mova
            | Op::Arith(..)
            | Op::Inc
            | Op::Dec
            | Op::Ash
            | Op::Rot
            | Op::Emul
            | Op::Ediv
            | Op::Ext(_)
            | Op::Mfpr
            | Op::Stq
            | Op::Evax(
                insn::Evax::Sext
                    | insn::Evax::Alu(_)
                    | insn::Evax::Zap(_)
                    | insn::Evax::Cmp(_)
                    | insn::Evax::Cmov(_)
                    | insn::Evax::St
                    | insn::Evax::Stqu
                    | insn::Evax::Stc
            )
    )
}

/// How an instruction reads the argument list through AP: the arguments
/// its fixed offsets reach, if any, and whether it uses AP as a list, as
/// an address, indexed, or offset by a variable or unaligned (AMACRO's
/// homing triggers). An error if it writes AP.
fn ap_use(
    op: insn::Op,
    size: operand::Size,
    ops: &[operand::Opnd],
    constant: &dyn Fn(&str) -> Option<i64>,
) -> Result<(Option<u32>, bool), String> {
    use insn::Op;
    use operand::{Mode, Opnd};
    const WRITTEN: &str =
        "AP is the argument list at 32(FP) (DESIGN-0004): vmacro doesn't take code that writes it";
    let reach = |n: i64, bytes: i64| (n >= 0 && n % 4 == 0).then(|| ((n + bytes - 1) / 4) as u32);
    let (mut args, mut list) = (None, false);
    for (i, o) in ops.iter().enumerate() {
        let address = address(op, i);
        let bytes = match (op, i) {
            (Op::Movz(s) | Op::Cvt(s), 0) => s.bytes(),
            _ => size.bytes(),
        }
        .max(4);
        let reached = match o {
            Opnd::Reg(12) if writes(op, i, ops.len()) => return Err(WRITTEN.into()),
            Opnd::Mem(Mode::Inc(12) | Mode::Dec(12) | Mode::IncDef(12), _) => {
                return Err(WRITTEN.into());
            }
            Opnd::Mem(Mode::DispDef(e, 12), _) => constant(e).and_then(|n| reach(n, 4)),
            Opnd::Mem(Mode::Def(12), None) if !address => reach(0, bytes),
            Opnd::Mem(Mode::Disp(e, 12), None) if !address => {
                constant(e).and_then(|n| reach(n, bytes))
            }
            Opnd::Reg(12) | Opnd::Mem(Mode::Def(12) | Mode::Disp(_, 12), _) => None,
            _ => continue,
        };
        match reached {
            Some(a) => args = args.max(Some(a)),
            None => list = true,
        }
    }
    Ok((args, list))
}

/// The destination of a branch or jump.
fn branch_target(op: insn::Op, ops: &[operand::Opnd]) -> Option<&operand::Opnd> {
    use insn::Op;
    match op {
        Op::Jmp => ops.first(),
        Op::Bcc
        | Op::Br
        | Op::Blb(_)
        | Op::Bb(..)
        | Op::Aob(_)
        | Op::Sob(_)
        | Op::Acb
        | Op::Evax(insn::Evax::Branch(_)) => ops.last(),
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

/// A CALL routine's frame (DESIGN-0004): past the frame record at 0, the
/// handler at 16 and the descriptor's address at 24, the argument list it
/// homes at 32, if any, then the registers it saves, x18 first, at `rsa`.
pub(crate) struct Frame {
    regs: Vec<u8>,
    home: Option<u32>,
    /// QUAD_ARGS=TRUE: the list is of quadwords, the count too.
    quad: bool,
    rsa: usize,
    size: usize,
}

impl Frame {
    /// Stores or loads the registers it saves, through x17 if the save
    /// area is past where `stp` reaches from FP.
    fn saves(&self, pair: &str, one: &str) -> Vec<String> {
        if self.rsa + 8 * self.regs.len() <= 512 {
            return saves(&self.regs, "x29", self.rsa, pair, one);
        }
        let mut out = vec![format!("\tadd x17, x29, #{}", self.rsa)];
        out.extend(saves(&self.regs, "x17", 0, pair, one));
        out
    }
}

fn frame(saved: &[u8], home: Option<u32>, quad: bool) -> Frame {
    let mut regs = vec![operand::SP];
    regs.extend(arms(saved));
    let slot = if quad { 8 } else { 4 };
    let rsa = 32 + home.map_or(0, |n| (slot + slot * n as usize).next_multiple_of(8));
    let size = (rsa + 8 * regs.len()).next_multiple_of(16);
    Frame {
        regs,
        home,
        quad,
        rsa,
        size,
    }
}

/// A CALL routine's prologue (docs/macro32.md), with its descriptor at
/// `fdsc`: the frame record, a clear handler and the descriptor's address,
/// then FP, the argument list and the registers. The list is the count,
/// from x9, at most what it has room for, then the arguments: the first
/// eight from x0-x7, the rest from the caller's stack, as many as there
/// are; longwords, or quadwords with QUAD_ARGS. Local labels from `label`
/// on.
fn call_prologue(f: &Frame, fdsc: &str, label: u32) -> Vec<String> {
    let size = f.size;
    let mut out = if size <= 504 {
        vec![format!("\tstp x29, x30, [sp, #-{size}]!")]
    } else {
        vec![
            format!("\tsub sp, sp, #{size}"),
            "\tstp x29, x30, [sp]".into(),
        ]
    };
    out.extend([
        format!("\tadrp x16, {fdsc}"),
        format!("\tadd x16, x16, #:lo12:{fdsc}"),
        "\tstp xzr, x16, [sp, #16]".into(),
        "\tmov x29, sp".into(),
    ]);
    if let Some(n) = f.home {
        let (slot, r) = if f.quad { (8, 'x') } else { (4, 'w') };
        out.extend([
            "\tand x16, x9, #255".into(),
            format!("\tmov x17, #{n}"),
            "\tcmp x16, x17".into(),
            "\tcsel x16, x16, x17, ls".into(),
            format!("\tstr {r}16, [x29, #32]"),
        ]);
        let n = n as usize;
        let at = |i: usize| 32 + slot * (i + 1);
        for i in (0..n.min(8)).step_by(2) {
            out.push(if i + 1 < n {
                format!("\tstp {r}{i}, {r}{}, [x29, #{}]", i + 1, at(i))
            } else {
                format!("\tstr {r}{i}, [x29, #{}]", at(i))
            });
        }
        if n > 8 {
            let (again, done) = (format!("{label}$"), format!("{}$", label + 1));
            out.extend([
                "\tsubs x16, x16, #8".into(),
                format!("\tb.ls {done}"),
                format!("\tadd x13, x29, #{size}"),
                format!("\tadd x17, x29, #{}", at(8)),
                format!("{again}:\tldr x14, [x13], #8"),
                format!("\tstr {r}14, [x17], #{slot}"),
                "\tsubs x16, x16, #1".into(),
                format!("\tb.ne {again}"),
                format!("{done}:"),
            ]);
        }
    }
    out.extend(f.saves("stp", "str"));
    out.push("\tmov x18, x29".into());
    out
}

/// A CALL routine's frame descriptor, `$FDSCDEF`, named `label`, in a psect
/// of its own next to the code's, so that whoever may run the routine may
/// read it: the registers it saves, where, the frame's size, no static
/// handler, and its name, as an offset from the descriptor.
fn descriptor(label: &str, name: &str, saved: &[u8], f: &Frame, psect: &str) -> Vec<String> {
    let bits: u32 = 1 | saved.iter().map(|r| 1 << (r - 1)).sum::<u32>();
    vec![
        "\t.SAVE_PSECT LOCAL_BLOCK".into(),
        format!("\t.PSECT {psect}_FDSC, PIC, SHR, EXE, NOWRT, QUAD"),
        "\t.ALIGN QUAD".into(),
        format!("{label}:\t.LONG 0, {bits}, {}, {}", f.rsa, f.size),
        "\t.QUAD 0".into(),
        format!("\t.QUAD {label}N-{label}"),
        format!("{label}N:\t.ASCIC \"{name}\""),
        "\t.RESTORE_PSECT".into(),
    ]
}

/// Stores or loads ARM64 registers `regs`, in pairs, from `base` + `at`.
fn saves(regs: &[u8], base: &str, at: usize, pair: &str, one: &str) -> Vec<String> {
    regs.chunks(2)
        .enumerate()
        .map(|(i, r)| match r {
            [a, b] => format!("\t{pair} x{a}, x{b}, [{base}, #{}]", at + 16 * i),
            [a] => format!("\t{one} x{a}, [{base}, #{}]", at + 16 * i),
            _ => unreachable!(),
        })
        .collect()
}

/// `RET` from a CALL routine with frame `f`.
pub(crate) fn epilogue(f: &Frame) -> Vec<String> {
    let mut out = f.saves("ldp", "ldr");
    out.push("\tmov sp, x29".into());
    if f.size <= 504 {
        out.push(format!("\tldp x29, x30, [sp], #{}", f.size));
    } else {
        out.extend([
            "\tldp x29, x30, [sp]".into(),
            format!("\tadd sp, sp, #{}", f.size),
        ]);
    }
    out.push("\tret".into());
    out
}

/// `RET` from whatever frame FP is at, as its descriptor says, as a VAX
/// RET reads the frame's register mask: what it saved, x18-x28 by the
/// bits from bit 0, from its save area, then FP and the return address.
fn any_epilogue(g: &mut operand::Gen) {
    g.emit("ldr x16, [x29, #24]");
    g.emit("ldr w15, [x16, #4]");
    g.emit("ldr w17, [x16, #8]");
    g.emit("add x17, x29, x17");
    for n in 0..11 {
        let skip = g.label();
        g.emit(format!("tbz w15, #{n}, {skip}"));
        g.emit(format!("ldr x{}, [x17], #8", 18 + n));
        g.place_label(&skip);
    }
    g.emit("ldr w17, [x16, #12]");
    g.emit("mov sp, x29");
    g.emit("ldp x29, x30, [sp]");
    g.emit("add sp, sp, x17");
    g.emit("ret");
}

/// The ARM64 registers that hold VAX registers `saved`.
fn arms(saved: &[u8]) -> Vec<u8> {
    saved.iter().map(|r| operand::arm(*r).unwrap()).collect()
}

/// What a JSB routine saves: the registers, and x30 if it calls.
fn jsb_saves(saved: &[u8], lr: bool) -> Vec<u8> {
    let mut regs = arms(saved);
    if lr {
        regs.push(30);
    }
    regs
}

/// A JSB routine's prologue, if it saves anything: the registers, all 64
/// bits, and x30 if it calls, below both stacks, with the caller's `sp`
/// and VAX SP, then both stacks below them, so that what it pushes doesn't
/// overwrite them.
fn jsb_prologue(saved: &[u8], lr: bool) -> Vec<String> {
    let regs = jsb_saves(saved, lr);
    if regs.is_empty() {
        return Vec::new();
    }
    let size = (16 + 8 * regs.len()).next_multiple_of(16);
    let mut out = vec![
        "\tmov x16, sp".to_string(),
        "\tcmp x16, x18".into(),
        "\tcsel x17, x16, x18, lo".into(),
        "\tand x17, x17, #0xfffffffffffffff0".into(),
        format!("\tsub x17, x17, #{size}"),
        // sp first: the PAL delivers an interrupt below both stacks.
        "\tmov sp, x17".into(),
        "\tstp x16, x18, [x17]".into(),
    ];
    out.extend(saves(&regs, "x17", 16, "stp", "str"));
    out.push("\tmov x18, x17".into());
    out
}

/// `RSB` from a JSB routine that saves `saved`, and x30 if `lr`, with VAX
/// SP back where the prologue left it: the registers, then both stacks.
pub(crate) fn jsb_epilogue(saved: &[u8], lr: bool) -> Vec<String> {
    let regs = jsb_saves(saved, lr);
    if regs.is_empty() {
        return Vec::new();
    }
    let mut out = saves(&regs, "x18", 16, "ldp", "ldr");
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
