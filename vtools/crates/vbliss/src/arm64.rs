//! The back end: IR into vasm assembly, on the calling standard
//! (DESIGN-0004). Instructions are selected one IR instruction at a time.
//! Temporaries live in x19-x28, kept across calls, by a linear scan over
//! their live ranges; those that don't fit are spilled to the frame.
//! x10-x17 are scratch: x10-x13 for operands, x14 for a spilled result,
//! x16 and x17 for addresses.
//!
//! The frame, from FP up: the frame record, the handler at 16, the frame
//! descriptor's address at 24, the saved registers from 32, the IR's
//! slots, then the spills. Every routine has a frame and a descriptor.

use std::fmt::Write;

use crate::ir::{Func, Ins, Item, Module, Op, Term, Un, V};

/// The registers temporaries live in.
const POOL: std::ops::RangeInclusive<u8> = 19..=28;

#[derive(Clone, Copy)]
enum Loc {
    Reg(u8),
    /// A spill, at this offset from FP.
    Spill(u32),
}

/// Writes the module as vasm source.
pub fn module(m: &Module) -> String {
    let mut out = String::new();
    let o = &mut out;
    let _ = writeln!(o, "        .TITLE  {}", m.name);
    if let Some(ident) = &m.ident {
        let _ = writeln!(o, "        .IDENT  \"{ident}\"");
    }
    for e in &m.externals {
        let _ = writeln!(o, "        .EXTERNAL {e}");
    }
    // Data, a psect at a time, in the order they first appear.
    let mut psects: Vec<&str> = Vec::new();
    for d in &m.data {
        if !psects.contains(&d.psect.as_str()) {
            psects.push(&d.psect);
        }
    }
    for p in psects {
        let attrs = if p == "$PLIT$" { ", NOWRT" } else { "" };
        let _ = writeln!(o, "\n        .PSECT  {p}{attrs}");
        for d in m.data.iter().filter(|d| d.psect == p) {
            if d.align > 0 {
                let _ = writeln!(o, "        .ALIGN  {}", d.align);
            }
            let _ = writeln!(o, "{}:{}", d.name, if d.global { ":" } else { "" });
            for item in &d.items {
                data_item(o, item);
            }
        }
    }
    let mut fdscs = String::new();
    if !m.funcs.is_empty() {
        let _ = writeln!(o, "\n        .PSECT  $CODE$");
    }
    for f in &m.funcs {
        Routine::new(f).emit(o, &mut fdscs);
    }
    if !fdscs.is_empty() {
        let _ = writeln!(
            o,
            "\n        .PSECT  $CODE$_FDSC, PIC, SHR, EXE, NOWRT, QUAD"
        );
        o.push_str(&fdscs);
    }
    match &m.main {
        Some(main) => {
            let _ = writeln!(o, "\n        .END    {main}");
        }
        None => o.push_str("\n        .END\n"),
    }
    out
}

fn data_item(o: &mut String, item: &Item) {
    let directive = |size: u8| match size {
        1 => ".BYTE",
        2 => ".WORD",
        4 => ".LONG",
        _ => ".QUAD",
    };
    let _ = match item {
        Item::Val(V::C(c), size) => {
            let v = if *size == 8 {
                *c as u64
            } else {
                (*c as u64) & ((1u64 << (8 * size)) - 1)
            };
            writeln!(o, "        {}  {v}", directive(*size))
        }
        Item::Val(V::Sym(s, off), size) => {
            writeln!(o, "        {}  {}", directive(*size), sym(s, *off))
        }
        Item::Val(v, _) => panic!("static value {v}"),
        Item::Bytes(b) => {
            for chunk in b.chunks(16) {
                let list: Vec<String> = chunk.iter().map(|c| c.to_string()).collect();
                let _ = writeln!(o, "        .BYTE   {}", list.join(", "));
            }
            Ok(())
        }
        Item::Zero(n) => writeln!(o, "        .BLKB   {n}"),
    };
}

fn sym(s: &str, off: i64) -> String {
    match off {
        0 => s.to_string(),
        o if o > 0 => format!("{s}+{o}"),
        o => format!("{s}{o}"),
    }
}

/// What an instruction defines and uses.
fn operands(ins: &Ins) -> (Option<u32>, Vec<&V>) {
    match ins {
        Ins::Bin(_, d, a, b) => (Some(*d), vec![a, b]),
        Ins::Un(_, d, a) => (Some(*d), vec![a]),
        Ins::Load(d, a, ..) => (Some(*d), vec![a]),
        Ins::Store(v, a, _) => (None, vec![v, a]),
        Ins::Ext(d, v, p, s, _) => (Some(*d), vec![v, p, s]),
        Ins::Insert(d, b, v, p, s) => (Some(*d), vec![b, v, p, s]),
        Ins::Arg(d, _) => (Some(*d), vec![]),
        Ins::Call(d, t, args) => (*d, std::iter::once(t).chain(args).collect()),
    }
}

fn term_uses(t: &Term) -> Vec<&V> {
    match t {
        Term::Jlbs(v, ..) | Term::Ret(v) => vec![v],
        Term::Jmp(_) => vec![],
    }
}

fn successors(t: &Term) -> Vec<u32> {
    match t {
        Term::Jmp(b) => vec![*b],
        Term::Jlbs(_, a, b) => vec![*a, *b],
        Term::Ret(_) => vec![],
    }
}

/// Where each temporary lives: liveness by blocks, then a linear scan.
fn allocate(f: &Func) -> (Vec<Option<Loc>>, u8, u32) {
    let n = f.temps as usize;
    let nb = f.blocks.len();
    // Upward-exposed uses and definitions of each block.
    let mut uses = vec![vec![false; n]; nb];
    let mut defs = vec![vec![false; n]; nb];
    let temps = |vs: Vec<&V>| -> Vec<usize> {
        vs.into_iter()
            .filter_map(|v| {
                if let V::T(t) = v {
                    Some(*t as usize)
                } else {
                    None
                }
            })
            .collect()
    };
    for (b, block) in f.blocks.iter().enumerate() {
        for ins in &block.ins {
            let (d, u) = operands(ins);
            for t in temps(u) {
                if !defs[b][t] {
                    uses[b][t] = true;
                }
            }
            if let Some(d) = d {
                defs[b][d as usize] = true;
            }
        }
        for t in temps(term_uses(&block.term)) {
            if !defs[b][t] {
                uses[b][t] = true;
            }
        }
    }
    let mut live_in = uses.clone();
    let mut live_out = vec![vec![false; n]; nb];
    let mut changed = true;
    while changed {
        changed = false;
        for b in (0..nb).rev() {
            for s in successors(&f.blocks[b].term) {
                for t in 0..n {
                    if live_in[s as usize][t] && !live_out[b][t] {
                        live_out[b][t] = true;
                        changed = true;
                    }
                }
            }
            for t in 0..n {
                if live_out[b][t] && !defs[b][t] && !live_in[b][t] {
                    live_in[b][t] = true;
                    changed = true;
                }
            }
        }
    }
    // Each temporary's range: the first and last position it is live at.
    let mut range: Vec<Option<(u32, u32)>> = vec![None; n];
    let mut extend = |t: usize, p: u32| {
        range[t] = Some(match range[t] {
            None => (p, p),
            Some((a, b)) => (a.min(p), b.max(p)),
        });
    };
    let mut pos = 0u32;
    for (b, block) in f.blocks.iter().enumerate() {
        let first = pos;
        for ins in &block.ins {
            let (d, u) = operands(ins);
            for t in temps(u) {
                extend(t, pos);
            }
            if let Some(d) = d {
                extend(d as usize, pos);
            }
            pos += 1;
        }
        for t in temps(term_uses(&block.term)) {
            extend(t, pos);
        }
        for t in 0..n {
            if live_in[b][t] {
                extend(t, first);
            }
            if live_out[b][t] {
                extend(t, pos);
            }
        }
        pos += 1;
    }
    let mut order: Vec<usize> = (0..n).filter(|&t| range[t].is_some()).collect();
    order.sort_by_key(|&t| (range[t].unwrap().0, t));
    let mut loc: Vec<Option<Loc>> = vec![None; n];
    let mut active: Vec<usize> = Vec::new();
    let mut free: Vec<u8> = POOL.rev().collect();
    let (mut spills, mut top) = (0u32, 0u8);
    let end = |t: usize| range[t].unwrap().1;
    for t in order {
        let start = range[t].unwrap().0;
        active.retain(|&a| {
            let keep = end(a) >= start;
            if !keep && let Some(Loc::Reg(r)) = loc[a] {
                free.push(r);
            }
            keep
        });
        free.sort_by(|a, b| b.cmp(a));
        if let Some(r) = free.pop() {
            loc[t] = Some(Loc::Reg(r));
            top = top.max(r - 18);
            active.push(t);
            continue;
        }
        // No register: spill whichever ends last, this one or an active one.
        let victim = *active.iter().max_by_key(|&&a| (end(a), a)).unwrap();
        if end(victim) > end(t) {
            loc[t] = loc[victim];
            loc[victim] = Some(Loc::Spill(spills));
            active.retain(|&a| a != victim);
            active.push(t);
        } else {
            loc[t] = Some(Loc::Spill(spills));
        }
        spills += 1;
    }
    (loc, top, spills)
}

struct Routine<'a> {
    f: &'a Func,
    loc: Vec<Option<Loc>>,
    saved: u8,
    /// Each slot's offset from FP.
    slots: Vec<u32>,
    size: u32,
    out: String,
}

impl<'a> Routine<'a> {
    fn new(f: &'a Func) -> Self {
        let (mut loc, saved, spills) = allocate(f);
        let mut off = 32 + 8 * u32::from(saved);
        let mut slots = Vec::new();
        for s in &f.slots {
            slots.push(off);
            off += s.next_multiple_of(8);
        }
        for l in loc.iter_mut() {
            if let Some(Loc::Spill(n)) = l {
                *l = Some(Loc::Spill(off + 8 * *n));
            }
        }
        off += 8 * spills;
        Routine {
            f,
            loc,
            saved,
            slots,
            size: off.next_multiple_of(16),
            out: String::new(),
        }
    }

    fn line(&mut self, s: impl AsRef<str>) {
        self.out.push_str("        ");
        self.out.push_str(s.as_ref());
        self.out.push('\n');
    }

    fn label(&self, b: u32) -> String {
        format!("{}$", b + 1)
    }

    fn mov_imm(&mut self, r: &str, c: i64) {
        if (0..=0xFFFF).contains(&c) {
            self.line(format!("mov     {r}, #{c}"));
        } else if (-0x10000..0).contains(&c) {
            self.line(format!("movn    {r}, #{}", !c));
        } else {
            let u = c as u64;
            let mut first = true;
            for shift in [0, 16, 32, 48] {
                let part = (u >> shift) & 0xFFFF;
                if part == 0 && !(first && shift == 48) {
                    continue;
                }
                let op = if first { "movz" } else { "movk" };
                self.line(format!("{op}    {r}, #{part}, lsl #{shift}"));
                first = false;
            }
        }
    }

    /// `rd = FP + off`.
    fn frame_address(&mut self, r: &str, off: i64) {
        if (0..4096).contains(&off) {
            self.line(format!("add     {r}, x29, #{off}"));
        } else {
            self.mov_imm("x17", off);
            self.line(format!("add     {r}, x29, x17"));
        }
    }

    /// Puts v in register r.
    fn load_into(&mut self, v: &V, r: &str) {
        match v {
            V::T(t) => match self.loc[*t as usize].unwrap() {
                Loc::Reg(x) => self.line(format!("mov     {r}, x{x}")),
                Loc::Spill(off) => self.line(format!("ldr     {r}, [x29, #{off}]")),
            },
            V::C(c) => self.mov_imm(r, *c),
            V::Sym(s, o) => {
                let s = sym(s, *o);
                self.line(format!("adrp    {r}, {s}"));
                self.line(format!("add     {r}, {r}, #:lo12:{s}"));
            }
            V::Slot(n, o) => {
                let off = i64::from(self.slots[*n as usize]) + o;
                self.frame_address(r, off);
            }
        }
    }

    /// A register holding v: its own, or `scratch` loaded with it.
    fn src(&mut self, v: &V, scratch: &str) -> String {
        if let V::T(t) = v
            && let Loc::Reg(x) = self.loc[*t as usize].unwrap()
        {
            return format!("x{x}");
        }
        self.load_into(v, scratch);
        scratch.to_string()
    }

    /// The register to compute temporary t in; `done` stores it if spilled.
    fn dst(&self, t: u32) -> String {
        match self.loc[t as usize].unwrap() {
            Loc::Reg(x) => format!("x{x}"),
            Loc::Spill(_) => "x14".into(),
        }
    }

    fn done(&mut self, t: u32) {
        if let Loc::Spill(off) = self.loc[t as usize].unwrap() {
            self.line(format!("str     x14, [x29, #{off}]"));
        }
    }

    /// The memory operand for an access of `size` bytes at the address v.
    fn mem(&mut self, v: &V, size: u8) -> String {
        if let V::Slot(n, o) = v {
            let off = i64::from(self.slots[*n as usize]) + o;
            let size = i64::from(size);
            if off % size == 0 && (0..4096 * size).contains(&off) {
                return format!("[x29, #{off}]");
            }
        }
        let r = self.src(v, "x16");
        format!("[{r}]")
    }

    fn emit(mut self, o: &mut String, fdscs: &mut String) {
        let f = self.f;
        let name = &f.name;
        let n = self.size;
        let _ = writeln!(o, "\n{name}:{}", if f.global { ":" } else { "" });
        if n <= 504 {
            self.line(format!("stp     x29, x30, [sp, #-{n}]!"));
        } else {
            self.sub_sp(n);
            self.line("stp     x29, x30, [sp]");
        }
        self.line(format!("adrp    x16, {name}.FDSC"));
        self.line(format!("add     x16, x16, #:lo12:{name}.FDSC"));
        self.line("stp     xzr, x16, [sp, #16]");
        self.line("mov     x29, sp");
        self.saves("stp", "str");
        for (b, block) in f.blocks.iter().enumerate() {
            if b > 0 {
                let l = self.label(b as u32);
                self.out.push_str(&format!("{l}:\n"));
            }
            for ins in &block.ins {
                self.ins(ins);
            }
            self.term(&block.term, b as u32);
        }
        let ret = self.label(f.blocks.len() as u32);
        self.out.push_str(&format!("{ret}:\n"));
        self.saves("ldp", "ldr");
        self.line("mov     sp, x29");
        if n <= 504 {
            self.line(format!("ldp     x29, x30, [sp], #{n}"));
        } else {
            self.line("ldp     x29, x30, [sp]");
            self.add_sp(n);
        }
        self.line("ret");
        o.push_str(&self.out);
        // The descriptor: flags, saved registers, save area, frame size,
        // no static handler, the name.
        let bits: u32 = (1..=u32::from(self.saved)).map(|i| 1 << i).sum();
        let _ = writeln!(fdscs, "        .ALIGN  QUAD");
        let _ = writeln!(fdscs, "{name}.FDSC:");
        let _ = writeln!(fdscs, "        .LONG   0, {bits}, 32, {n}");
        let _ = writeln!(fdscs, "        .QUAD   0, {name}.NAME - {name}.FDSC");
        let _ = writeln!(fdscs, "{name}.NAME:");
        let _ = writeln!(fdscs, "        .ASCIC  \"{name}\"");
    }

    fn sub_sp(&mut self, n: u32) {
        if n >= 4096 {
            self.line(format!("sub     sp, sp, #{}, lsl #12", n >> 12));
        }
        if n & 0xFFF != 0 {
            self.line(format!("sub     sp, sp, #{}", n & 0xFFF));
        }
    }

    fn add_sp(&mut self, n: u32) {
        if n >= 4096 {
            self.line(format!("add     sp, sp, #{}, lsl #12", n >> 12));
        }
        if n & 0xFFF != 0 {
            self.line(format!("add     sp, sp, #{}", n & 0xFFF));
        }
    }

    /// Saves or restores x19 up to the last register used, from 32(FP).
    fn saves(&mut self, pair: &str, one: &str) {
        let mut r = 19u8;
        while r < 19 + self.saved {
            let off = 32 + 8 * u32::from(r - 19);
            if r + 1 < 19 + self.saved {
                self.line(format!("{pair}     x{r}, x{}, [x29, #{off}]", r + 1));
                r += 2;
            } else {
                self.line(format!("{one}     x{r}, [x29, #{off}]"));
                r += 1;
            }
        }
    }

    fn ins(&mut self, ins: &Ins) {
        match ins {
            Ins::Bin(op, d, a, b) => self.binary(*op, *d, a, b),
            Ins::Un(op, d, a) => {
                let a = self.src(a, "x10");
                let r = self.dst(*d);
                let m = match op {
                    Un::Copy => "mov",
                    Un::Neg => "neg",
                    Un::Not => "mvn",
                };
                self.line(format!("{m:<8}{r}, {a}"));
                self.done(*d);
            }
            Ins::Load(d, a, size, signed) => {
                let mem = self.mem(a, *size);
                let r = self.dst(*d);
                let w = r.replacen('x', "w", 1);
                let line = match (size, signed) {
                    (8, _) => format!("ldr     {r}, {mem}"),
                    (4, true) => format!("ldrsw   {r}, {mem}"),
                    (4, false) => format!("ldr     {w}, {mem}"),
                    (2, true) => format!("ldrsh   {r}, {mem}"),
                    (2, false) => format!("ldrh    {w}, {mem}"),
                    (_, true) => format!("ldrsb   {r}, {mem}"),
                    (_, false) => format!("ldrb    {w}, {mem}"),
                };
                self.line(line);
                self.done(*d);
            }
            Ins::Store(v, a, size) => {
                let v = self.src(v, "x10");
                let mem = self.mem(a, *size);
                let w = v.replacen('x', "w", 1);
                let line = match size {
                    8 => format!("str     {v}, {mem}"),
                    4 => format!("str     {w}, {mem}"),
                    2 => format!("strh    {w}, {mem}"),
                    _ => format!("strb    {w}, {mem}"),
                };
                self.line(line);
            }
            Ins::Ext(d, v, p, s, signed) => {
                let v = self.src(v, "x10");
                let r = self.dst(*d);
                if let (V::C(p), V::C(s)) = (p, s) {
                    let m = if *signed { "sbfx" } else { "ubfx" };
                    self.line(format!("{m}    {r}, {v}, #{p}, #{s}"));
                } else {
                    // (v >> p) << (64 - s) >> (64 - s)
                    let p = self.src(p, "x11");
                    let s = self.src(s, "x12");
                    self.line(format!("lsr     x13, {v}, {p}"));
                    self.line(format!("neg     x12, {s}"));
                    self.line("lsl     x13, x13, x12");
                    let m = if *signed { "asr" } else { "lsr" };
                    self.line(format!("{m}     {r}, x13, x12"));
                }
                self.done(*d);
            }
            Ins::Insert(d, base, v, p, s) => {
                let base = self.src(base, "x10");
                let v = self.src(v, "x11");
                if let (V::C(p), V::C(s)) = (p, s) {
                    self.line(format!("mov     x16, {base}"));
                    self.line(format!("bfi     x16, {v}, #{p}, #{s}"));
                } else {
                    // mask = ~(-1 << s) << p; (base & ~mask) | (v << p & mask)
                    let p = self.src(p, "x12");
                    let s = self.src(s, "x13");
                    self.line("mov     x16, #-1");
                    self.line(format!("lsl     x16, x16, {s}"));
                    self.line("mvn     x16, x16");
                    self.line(format!("lsl     x16, x16, {p}"));
                    self.line(format!("lsl     x17, {v}, {p}"));
                    self.line("and     x17, x17, x16");
                    self.line(format!("bic     x16, {base}, x16"));
                    self.line("orr     x16, x16, x17");
                }
                let r = self.dst(*d);
                self.line(format!("mov     {r}, x16"));
                self.done(*d);
            }
            Ins::Arg(d, n) => {
                let r = self.dst(*d);
                if *n < 8 {
                    self.line(format!("mov     {r}, x{n}"));
                } else {
                    let off = self.size + 8 * (n - 8);
                    self.line(format!("ldr     {r}, [x29, #{off}]"));
                }
                self.done(*d);
            }
            Ins::Call(d, target, args) => {
                let stack = (args.len().saturating_sub(8) as u32 * 8).next_multiple_of(16);
                if stack > 0 {
                    self.sub_sp(stack);
                    for (i, a) in args.iter().enumerate().skip(8) {
                        let r = self.src(a, "x10");
                        self.line(format!("str     {r}, [sp, #{}]", (i - 8) * 8));
                    }
                }
                for (i, a) in args.iter().enumerate().take(8) {
                    self.load_into(a, &format!("x{i}"));
                }
                self.line(format!("mov     x9, #{}", args.len()));
                match target {
                    V::Sym(s, 0) => self.line(format!("bl      {s}")),
                    t => {
                        self.load_into(t, "x17");
                        self.line("blr     x17");
                    }
                }
                if stack > 0 {
                    self.add_sp(stack);
                }
                if let Some(d) = d {
                    let r = self.dst(*d);
                    self.line(format!("mov     {r}, x0"));
                    self.done(*d);
                }
            }
        }
    }

    fn binary(&mut self, op: Op, d: u32, a: &V, b: &V) {
        let a = self.src(a, "x10");
        // An immediate where ARM64 has one: add, sub, compare, shifts.
        let imm = match (op, b) {
            (
                Op::Add
                | Op::Sub
                | Op::Ceq
                | Op::Cne
                | Op::Clt
                | Op::Cle
                | Op::Cgt
                | Op::Cge
                | Op::Cltu
                | Op::Cleu
                | Op::Cgtu
                | Op::Cgeu,
                V::C(c),
            ) if (0..4096).contains(c) => Some(format!("#{c}")),
            (Op::Shl | Op::Shr | Op::Sar, V::C(c)) if (0..64).contains(c) => Some(format!("#{c}")),
            _ => None,
        };
        let b = match imm {
            Some(i) => i,
            None => self.src(b, "x11"),
        };
        let r = self.dst(d);
        let three = |m: &str| format!("{m:<8}{r}, {a}, {b}");
        let cond = match op {
            Op::Ceq => Some("eq"),
            Op::Cne => Some("ne"),
            Op::Clt => Some("lt"),
            Op::Cle => Some("le"),
            Op::Cgt => Some("gt"),
            Op::Cge => Some("ge"),
            Op::Cltu => Some("lo"),
            Op::Cleu => Some("ls"),
            Op::Cgtu => Some("hi"),
            Op::Cgeu => Some("hs"),
            _ => None,
        };
        if let Some(c) = cond {
            self.line(format!("cmp     {a}, {b}"));
            self.line(format!("cset    {r}, {c}"));
        } else {
            match op {
                Op::Add => self.line(three("add")),
                Op::Sub => self.line(three("sub")),
                Op::Mul => self.line(three("mul")),
                Op::Div => self.line(three("sdiv")),
                Op::Rem => {
                    self.line(format!("sdiv    x12, {a}, {b}"));
                    self.line(format!("msub    {r}, x12, {b}, {a}"));
                }
                Op::And => self.line(three("and")),
                Op::Or => self.line(three("orr")),
                Op::Xor => self.line(three("eor")),
                Op::Eqv => self.line(three("eon")),
                Op::Shl => self.line(three("lsl")),
                Op::Shr => self.line(three("lsr")),
                Op::Sar => self.line(three("asr")),
                Op::Ash => {
                    self.line(format!("neg     x12, {b}"));
                    self.line(format!("lsl     x13, {a}, {b}"));
                    self.line(format!("asr     x12, {a}, x12"));
                    self.line(format!("cmp     {b}, #0"));
                    self.line(format!("csel    {r}, x13, x12, ge"));
                }
                _ => unreachable!(),
            }
        }
        self.done(d);
    }

    fn term(&mut self, t: &Term, b: u32) {
        let next = b + 1;
        match t {
            Term::Jmp(to) => {
                if *to != next {
                    let l = self.label(*to);
                    self.line(format!("b       {l}"));
                }
            }
            Term::Jlbs(v, yes, no) => {
                let r = self.src(v, "x10");
                let (ly, ln) = (self.label(*yes), self.label(*no));
                if *no == next {
                    self.line(format!("tbnz    {r}, #0, {ly}"));
                } else if *yes == next {
                    self.line(format!("tbz     {r}, #0, {ln}"));
                } else {
                    self.line(format!("tbnz    {r}, #0, {ly}"));
                    self.line(format!("b       {ln}"));
                }
            }
            Term::Ret(v) => {
                self.load_into(v, "x0");
                let end = self.f.blocks.len() as u32;
                if next != end {
                    let l = self.label(end);
                    self.line(format!("b       {l}"));
                }
            }
        }
    }
}
