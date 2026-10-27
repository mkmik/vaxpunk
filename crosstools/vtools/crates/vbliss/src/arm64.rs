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

use crate::ir::{Func, Ins, Item, LinkCall, Module, Op, Term, Un, V};

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
        let attrs = match m.psects.iter().find(|(n, _)| n == p) {
            Some((_, a)) => format!(", {a}"),
            None if p == "$PLIT$" => ", NOWRT".into(),
            None => String::new(),
        };
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
    // Each routine's frame descriptor goes in a psect named for its code's
    // with _FDSC after, as vmacro puts them.
    let mut fdscs: Vec<(String, String)> = Vec::new();
    let mut code = "";
    for f in &m.funcs {
        let p = if f.psect.is_empty() {
            "$CODE$"
        } else {
            &f.psect
        };
        if p != code {
            let attrs = match m.psects.iter().find(|(n, _)| n == p) {
                Some((_, a)) => format!(", {a}"),
                None => String::new(),
            };
            let _ = writeln!(o, "\n        .PSECT  {p}{attrs}");
            code = p;
        }
        let fdsc = format!("{p}_FDSC");
        let at = match fdscs.iter().position(|(n, _)| *n == fdsc) {
            Some(i) => i,
            None => {
                fdscs.push((fdsc, String::new()));
                fdscs.len() - 1
            }
        };
        Routine::new(f).emit(o, &mut fdscs[at].1);
    }
    for (p, text) in &fdscs {
        let _ = writeln!(o, "\n        .PSECT  {p}, PIC, SHR, EXE, NOWRT, QUAD");
        o.push_str(text);
    }
    // After the labels they may name.
    for (g, v) in &m.globals {
        let v = match v {
            V::Sym(s, off) => sym(s, *off),
            v => v.to_string(),
        };
        let _ = writeln!(o, "{g} == {v}");
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
fn operands(ins: &Ins) -> (Vec<u32>, Vec<&V>) {
    if let Ins::Link(c) = ins {
        let defs = c
            .dst
            .into_iter()
            .chain(c.outs.iter().map(|o| o.1))
            .collect();
        let uses = std::iter::once(&c.target)
            .chain(&c.args)
            .chain(c.ins.iter().map(|(a, _)| a))
            .collect();
        return (defs, uses);
    }
    let (d, u) = match ins {
        Ins::Bin(_, d, a, b) => (Some(*d), vec![a, b]),
        Ins::Un(_, d, a) => (Some(*d), vec![a]),
        Ins::Load(d, a, ..) => (Some(*d), vec![a]),
        Ins::Store(v, a, _) => (None, vec![v, a]),
        Ins::Ext(d, v, p, s, _) => (Some(*d), vec![v, p, s]),
        Ins::Insert(d, b, v, p, s) => (Some(*d), vec![b, v, p, s]),
        Ins::Arg(d, _) | Ins::StackArg(d, _) => (Some(*d), vec![]),
        Ins::Call(d, t, args) => (*d, std::iter::once(t).chain(args).collect()),
        Ins::RegArg(d, _) | Ins::ArgCount(d) | Ins::ArgPtr(d) => (Some(*d), vec![]),
        Ins::ArgN(d, i) => (Some(*d), vec![i]),
        Ins::SetHandler(v) | Ins::SetEnable(v) => (None, vec![v]),
        Ins::Barrier | Ins::Bpt | Ins::Comment(_) => (None, vec![]),
        Ins::Cas(d, a, e, n, _) => (Some(*d), vec![a, e, n]),
        Ins::Pal(d, func, args) => (Some(*d), std::iter::once(func).chain(args).collect()),
        Ins::RetAddr(d) | Ins::Rpcc(d) => (Some(*d), vec![]),
        Ins::SetReg(_, v) => (None, vec![v]),
        Ins::Link(_) => unreachable!(),
    };
    (d.into_iter().collect(), u)
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
            for d in d {
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
            for d in d {
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
    /// The save area's offset from FP: 32, or 40 past the enable vector's
    /// address.
    rsa: u32,
    /// Where the prologue copies the argument list, if the routine reads it.
    args: Option<u32>,
    /// Each slot's offset from FP.
    slots: Vec<u32>,
    size: u32,
    /// Labels past the blocks' and the prologue's, used so far.
    extra: u32,
    out: String,
}

/// The PAL call REI (DESIGN-0001).
const REI: u32 = 0x92;

impl<'a> Routine<'a> {
    fn new(f: &'a Func) -> Self {
        let (mut loc, mut saved, spills) = allocate(f);
        let all = || f.blocks.iter().flat_map(|b| &b.ins);
        // A JSB routine's register parameters are read from the save area,
        // where the prologue put them.
        for ins in all() {
            if let Ins::RegArg(_, r) | Ins::SetReg(r, _) = ins
                && *r >= 2
            {
                saved = saved.max(*r - 1);
            }
        }
        let enable = all().any(|i| matches!(i, Ins::SetEnable(_)));
        let rsa = if enable { 40 } else { 32 };
        let mut off = rsa + 8 * u32::from(saved);
        let args = all()
            .any(|i| matches!(i, Ins::ArgCount(_) | Ins::ArgN(..) | Ins::ArgPtr(_)))
            .then(|| {
                // ponytail: room for the most arguments, 255, in every
                // routine that reads its list.
                let at = off;
                off += 8 * 256;
                at
            });
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
            rsa,
            args,
            slots,
            size: off.next_multiple_of(16),
            extra: 0,
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

    /// A label of its own for a branch inside an instruction's code.
    fn extra_label(&mut self) -> String {
        self.extra += 1;
        self.label(self.f.blocks.len() as u32 + 2 + self.extra)
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
        if let Some(at) = self.args {
            self.home(at);
        }
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
        match f.rei {
            // An INTERRUPT routine pops the PAL's parameters, then REI
            // restores everything from the PAL's frame (DESIGN-0001).
            Some(params) => {
                if params > 0 {
                    self.add_sp(8 * params);
                }
                self.line(format!("mov     x7, #{REI}"));
                self.line("svc     #0");
            }
            None => self.line("ret"),
        }
        o.push_str(&self.out);
        // The descriptor: flags, saved registers, save area, frame size,
        // no static handler, the name.
        let bits: u32 = (1..=u32::from(self.saved)).map(|i| 1 << i).sum();
        let _ = writeln!(fdscs, "        .ALIGN  QUAD");
        let _ = writeln!(fdscs, "{name}.FDSC:");
        let _ = writeln!(fdscs, "        .LONG   0, {bits}, {}, {n}", self.rsa);
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

    /// Copies the argument list to `at` from FP: the count from x9, x0-x7,
    /// and those past them from the caller's stack.
    fn home(&mut self, at: u32) {
        let end = self.f.blocks.len() as u32;
        let (again, done) = (self.label(end + 1), self.label(end + 2));
        // Longwords, or quadwords.
        let (n, r) = if self.f.long_args { (4, "w") } else { (8, "x") };
        self.line("and     x10, x9, #255");
        self.line(format!("str     {r}10, [x29, #{at}]"));
        for i in (0..8).step_by(2) {
            let off = at + n * (1 + i);
            self.line(format!("stp     {r}{i}, {r}{}, [x29, #{off}]", i + 1));
        }
        self.line("subs    x10, x10, #8");
        self.line(format!("b.le    {done}"));
        self.frame_address("x11", self.size.into());
        self.frame_address("x12", i64::from(at + 9 * n));
        self.out.push_str(&format!("{again}:\n"));
        self.line("ldr     x13, [x11], #8");
        self.line(format!("str     {r}13, [x12], #{n}"));
        self.line("subs    x10, x10, #1");
        self.line(format!("b.ne    {again}"));
        self.out.push_str(&format!("{done}:\n"));
    }

    /// Saves or restores x19 up to the last register used, in the save
    /// area.
    fn saves(&mut self, pair: &str, one: &str) {
        let mut r = 19u8;
        while r < 19 + self.saved {
            let off = self.rsa + 8 * u32::from(r - 19);
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
            Ins::StackArg(d, n) => {
                // The stack at entry is above the frame.
                let r = self.dst(*d);
                let off = self.size + 8 * n;
                self.line(format!("ldr     {r}, [x29, #{off}]"));
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
            Ins::Link(c) => self.link(c),
            Ins::SetReg(r, v) => {
                if *r < 2 {
                    self.load_into(v, &format!("x{r}"));
                } else {
                    let v = self.src(v, "x10");
                    let off = self.rsa + 8 * u32::from(r - 2);
                    self.line(format!("str     {v}, [x29, #{off}]"));
                }
            }
            Ins::RegArg(d, r) => {
                let rd = self.dst(*d);
                if *r < 2 {
                    self.line(format!("mov     {rd}, x{r}"));
                } else {
                    let off = self.rsa + 8 * u32::from(r - 2);
                    self.line(format!("ldr     {rd}, [x29, #{off}]"));
                }
                self.done(*d);
            }
            Ins::SetHandler(v) => {
                let r = self.src(v, "x10");
                self.line(format!("str     {r}, [x29, #16]"));
            }
            Ins::SetEnable(v) => {
                let r = self.src(v, "x10");
                self.line(format!("str     {r}, [x29, #32]"));
            }
            Ins::ArgCount(d) => {
                let at = self.args.unwrap();
                let rd = self.dst(*d);
                let ld = if self.f.long_args { "ldrsw" } else { "ldr  " };
                self.line(format!("{ld}   {rd}, [x29, #{at}]"));
                self.done(*d);
            }
            Ins::ArgPtr(d) => {
                let at = self.args.unwrap();
                let rd = self.dst(*d);
                self.frame_address(&rd, at.into());
                self.done(*d);
            }
            Ins::ArgN(d, i) => {
                let at = i64::from(self.args.unwrap());
                let rd = self.dst(*d);
                let (n, ld, sh) = if self.f.long_args {
                    (4, "ldrsw", 2)
                } else {
                    (8, "ldr  ", 3)
                };
                match i {
                    V::C(i) if (0..=255).contains(i) => {
                        self.line(format!("{ld}   {rd}, [x29, #{}]", at + n * i));
                    }
                    i => {
                        let i = self.src(i, "x10");
                        self.frame_address("x16", at);
                        self.line(format!("{ld}   {rd}, [x16, {i}, lsl #{sh}]"));
                    }
                }
                self.done(*d);
            }
            Ins::Barrier => self.line("dmb     ish"),
            Ins::Bpt => self.line("brk     #0"),
            Ins::Comment(s) => self.line(format!("; {s}")),
            Ins::RetAddr(d) => {
                let rd = self.dst(*d);
                self.line(format!("ldr     {rd}, [x29, #8]"));
                self.done(*d);
            }
            Ins::Rpcc(d) => {
                let rd = self.dst(*d);
                self.line(format!("mrs     {rd}, CNTVCT_EL0"));
                self.done(*d);
            }
            Ins::Cas(d, a, e, n, size) => {
                let a = self.src(a, "x16");
                let e = self.src(e, "x11");
                let n = self.src(n, "x12");
                let w = |r: &str| {
                    if *size == 4 {
                        r.replacen('x', "w", 1)
                    } else {
                        r.to_string()
                    }
                };
                let (t, e, n) = (w("x13"), w(&e), w(&n));
                let l = self.extra_label();
                self.line(format!("ldaxr   {t}, [{a}]"));
                self.line(format!("cmp     {t}, {e}"));
                self.line(format!("b.ne    {l}"));
                self.line(format!("stlxr   w15, {n}, [{a}]"));
                self.line("cmp     w15, #0");
                self.out.push_str(&format!("{l}:\n"));
                let rd = self.dst(*d);
                self.line(format!("cset    {rd}, eq"));
                self.done(*d);
            }
            Ins::Pal(d, func, args) => {
                for (i, a) in args.iter().enumerate() {
                    self.load_into(a, &format!("x{i}"));
                }
                self.load_into(func, "x7");
                self.line("svc     #0");
                let rd = self.dst(*d);
                self.line(format!("mov     {rd}, x0"));
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

    /// A call by a linkage with registers: the register arguments and the
    /// results go through the stack, so that no register they share with
    /// a temporary is lost, and the registers the callee may change are
    /// kept around it. A JSB call also keeps x18, which it sets to `sp`
    /// so that the callee's pushes go below this frame; a CALL passes its
    /// standard arguments as `Ins::Call` does.
    fn link(&mut self, c: &LinkCall) {
        let arm = |r: u8| if r < 2 { r } else { r + 17 };
        let mut save: Vec<u8> = c
            .ins
            .iter()
            .map(|(_, r)| *r)
            .chain(c.outs.iter().map(|o| o.0))
            .chain(c.nopreserve.iter().copied())
            .filter(|&r| r >= 2)
            .map(arm)
            .collect();
        if c.jsb {
            save.push(18);
        }
        save.sort_unstable();
        save.dedup();
        let (ns, na, no) = (save.len(), c.ins.len(), c.outs.len());
        // The saves, the register arguments, the result, the outputs.
        let area = (8 * (ns + na + 1 + no) as u32).next_multiple_of(16);
        self.sub_sp(area);
        for (i, r) in save.iter().enumerate() {
            self.line(format!("str     x{r}, [sp, #{}]", 8 * i));
        }
        for (j, (v, _)) in c.ins.iter().enumerate() {
            let r = self.src(v, "x10");
            self.line(format!("str     {r}, [sp, #{}]", 8 * (ns + j)));
        }
        let stack = (c.args.len().saturating_sub(8) as u32 * 8).next_multiple_of(16);
        if stack > 0 {
            self.sub_sp(stack);
            for (i, a) in c.args.iter().enumerate().skip(8) {
                let r = self.src(a, "x10");
                self.line(format!("str     {r}, [sp, #{}]", (i - 8) * 8));
            }
        }
        for (i, a) in c.args.iter().enumerate().take(8) {
            self.load_into(a, &format!("x{i}"));
        }
        if !matches!(c.target, V::Sym(_, 0)) {
            self.load_into(&c.target, "x17");
        }
        for (j, (_, r)) in c.ins.iter().enumerate() {
            let off = stack as usize + 8 * (ns + j);
            self.line(format!("ldr     x{}, [sp, #{off}]", arm(*r)));
        }
        if c.jsb {
            self.line("mov     x18, sp");
        } else if c.count {
            self.line(format!("mov     x9, #{}", c.args.len()));
        }
        match &c.target {
            V::Sym(s, 0) => self.line(format!("bl      {s}")),
            _ => self.line("blr     x17"),
        }
        if stack > 0 {
            self.add_sp(stack);
        }
        let res = 8 * (ns + na);
        self.line(format!("str     x0, [sp, #{res}]"));
        for (k, (r, _)) in c.outs.iter().enumerate() {
            self.line(format!(
                "str     x{}, [sp, #{}]",
                arm(*r),
                res + 8 * (1 + k)
            ));
        }
        for (i, r) in save.iter().enumerate() {
            self.line(format!("ldr     x{r}, [sp, #{}]", 8 * i));
        }
        if let Some(d) = c.dst {
            let r = self.dst(d);
            self.line(format!("ldr     {r}, [sp, #{res}]"));
            self.done(d);
        }
        for (k, (_, t)) in c.outs.iter().enumerate() {
            let r = self.dst(*t);
            self.line(format!("ldr     {r}, [sp, #{}]", res + 8 * (1 + k)));
            self.done(*t);
        }
        self.add_sp(area);
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
                Op::Ror => self.line(three("ror")),
                Op::Umulh => self.line(three("umulh")),
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
