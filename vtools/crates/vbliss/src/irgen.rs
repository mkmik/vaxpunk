//! IR generation: each routine's expression tree into blocks of IR, and
//! OWN and GLOBAL data, PLITs and descriptors into data items.

use std::collections::{BTreeSet, HashMap};

use crate::ir::{self, Block, Func, Ins, Item, Op, Term, Un, V};
use crate::parse::{self, BOp, CaseLabel, Expr, Kind, Rel, SelectLabel, Storage};

pub type Error = String;
type R<T> = Result<T, Error>;

/// A place in memory: a field `<pos, size, signed>` at an address, as a
/// fetch reads it and an assignment writes it.
struct Place {
    addr: V,
    pos: V,
    size: V,
    signed: bool,
}

struct Gen<'a> {
    m: &'a parse::Module,
    f: Func,
    /// The block instructions go into.
    cur: u32,
    /// Enclosing loops: where EXITLOOP goes and the temporary for the value.
    loops: Vec<(u32, u32)>,
    /// Labeled blocks: where LEAVE goes and the temporary for the value.
    labels: HashMap<usize, (u32, u32)>,
    data: Vec<ir::Data>,
    externals: BTreeSet<String>,
}

/// Generates the IR of a parsed module.
pub fn generate(m: &parse::Module) -> R<ir::Module> {
    let mut g = Gen {
        m,
        f: Func::default(),
        cur: 0,
        loops: Vec::new(),
        labels: HashMap::new(),
        data: Vec::new(),
        externals: BTreeSet::new(),
    };
    for s in &m.statics {
        g.statics(s)?;
    }
    let mut funcs = Vec::new();
    for r in &m.routines {
        funcs.push(g.routine(r)?);
    }
    Ok(ir::Module {
        name: m.name.clone(),
        ident: m.ident.clone(),
        main: m.main.clone(),
        externals: g.externals.into_iter().collect(),
        data: g.data,
        funcs,
    })
}

fn rel_op(rel: Rel, unsigned: bool) -> Op {
    match (rel, unsigned) {
        (Rel::Eql, _) => Op::Ceq,
        (Rel::Neq, _) => Op::Cne,
        (Rel::Lss, false) => Op::Clt,
        (Rel::Leq, false) => Op::Cle,
        (Rel::Gtr, false) => Op::Cgt,
        (Rel::Geq, false) => Op::Cge,
        (Rel::Lss, true) => Op::Cltu,
        (Rel::Leq, true) => Op::Cleu,
        (Rel::Gtr, true) => Op::Cgtu,
        (Rel::Geq, true) => Op::Cgeu,
    }
}

/// An address plus a constant.
fn offset(v: &V, k: i64) -> Option<V> {
    Some(match v {
        V::C(c) => V::C(c.wrapping_add(k)),
        V::Sym(s, o) => V::Sym(s.clone(), o + k),
        V::Slot(s, o) => V::Slot(*s, o + k),
        V::T(_) => return None,
    })
}

impl Gen<'_> {
    fn temp(&mut self) -> u32 {
        self.f.temps += 1;
        self.f.temps - 1
    }

    fn block(&mut self) -> u32 {
        self.f.blocks.push(Block {
            ins: Vec::new(),
            term: Term::Ret(V::C(0)),
        });
        self.f.blocks.len() as u32 - 1
    }

    fn emit(&mut self, ins: Ins) {
        self.f.blocks[self.cur as usize].ins.push(ins);
    }

    /// Ends the current block with `t`; what follows goes into a new block
    /// nothing jumps to, which `prune` removes.
    fn end(&mut self, t: Term) {
        self.f.blocks[self.cur as usize].term = t;
        self.cur = self.block();
    }

    fn jump(&mut self, to: u32) {
        self.end(Term::Jmp(to));
    }

    fn start(&mut self, b: u32) {
        self.cur = b;
    }

    fn bin(&mut self, op: Op, a: V, b: V) -> V {
        let d = self.temp();
        self.emit(Ins::Bin(op, d, a, b));
        V::T(d)
    }

    fn copy(&mut self, to: u32, v: V) {
        self.emit(Ins::Un(Un::Copy, to, v));
    }

    fn add(&mut self, a: V, k: i64) -> V {
        match offset(&a, k) {
            Some(v) => v,
            None if k == 0 => a,
            None => self.bin(Op::Add, a, V::C(k)),
        }
    }

    fn statics(&mut self, s: &parse::Static) -> R<()> {
        let sym = &self.m.syms[s.sym];
        let Kind::Data { storage, bytes, .. } = sym.kind else {
            unreachable!()
        };
        let mut items = Vec::new();
        let mut used = 0u32;
        for (e, size) in &s.init {
            let v = match e {
                Expr::Name(id) => self.address(*id),
                e => V::C(parse::fold(e).ok_or(format!("INITIAL of {} isn't constant", sym.name))?),
            };
            items.push(Item::Val(v, *size));
            used += u32::from(*size);
        }
        if used < bytes {
            items.push(Item::Zero(bytes - used));
        }
        let global = storage == Storage::Global;
        self.data.push(ir::Data {
            psect: if global { "$GLOBAL$" } else { "$OWN$" }.into(),
            name: sym.asm.clone(),
            global,
            align: 3,
            items,
        });
        Ok(())
    }

    /// The address a data or routine name stands for.
    fn address(&mut self, id: usize) -> V {
        let sym = &self.m.syms[id];
        match sym.kind {
            Kind::Data {
                storage: Storage::Local(slot),
                ..
            } => V::Slot(slot, 0),
            Kind::Data {
                storage: Storage::External,
                ..
            }
            | Kind::Routine { external: true, .. } => {
                self.externals.insert(sym.asm.clone());
                V::Sym(sym.asm.clone(), 0)
            }
            _ => V::Sym(sym.asm.clone(), 0),
        }
    }

    fn routine(&mut self, r: &parse::Routine) -> R<Func> {
        let sym = &self.m.syms[r.sym];
        let Kind::Routine {
            global, novalue, ..
        } = sym.kind
        else {
            unreachable!()
        };
        self.f = Func {
            name: sym.asm.clone(),
            global,
            slots: r.slots.clone(),
            ..Func::default()
        };
        self.cur = self.block();
        for (i, &formal) in r.formals.iter().enumerate() {
            let t = self.temp();
            self.emit(Ins::Arg(t, i as u32));
            let addr = self.address(formal);
            self.emit(Ins::Store(V::T(t), addr, 8));
        }
        let v = self.value(&r.body)?;
        self.end(Term::Ret(if novalue { V::C(0) } else { v }));
        let mut f = std::mem::take(&mut self.f);
        prune(&mut f);
        Ok(f)
    }

    /// The value of an expression.
    fn value(&mut self, e: &Expr) -> R<V> {
        if let Some(c) = parse::fold(e) {
            return Ok(V::C(c));
        }
        Ok(match e {
            Expr::Num(n) => V::C(*n),
            Expr::Name(id) => self.address(*id),
            Expr::Ascid(text) => self.ascid(text),
            Expr::Fetch(a) => {
                let p = self.place(a)?;
                self.load(p)
            }
            Expr::Field(..) | Expr::Index(..) => {
                // A field reference as a value is its address.
                self.place(e)?.addr
            }
            Expr::Neg(a) => {
                let a = self.value(a)?;
                let d = self.temp();
                self.emit(Ins::Un(Un::Neg, d, a));
                V::T(d)
            }
            Expr::Not(a) => {
                let a = self.value(a)?;
                let d = self.temp();
                self.emit(Ins::Un(Un::Not, d, a));
                V::T(d)
            }
            Expr::Bin(op, a, b) => {
                let (a, b) = (self.value(a)?, self.value(b)?);
                self.binary(*op, a, b)
            }
            Expr::Assign(l, r) => {
                let v = self.value(r)?;
                let p = self.place(l)?;
                self.store(p, v.clone());
                v
            }
            Expr::Call(target, args) => {
                let t = match **target {
                    Expr::Name(id) => match self.m.syms[id].kind {
                        Kind::Routine { .. } => self.address(id),
                        _ => return Err(format!("{} is not a routine", self.m.syms[id].name)),
                    },
                    ref t => self.value(t)?,
                };
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.value(a)?);
                }
                if vals.len() > 255 {
                    return Err("more than 255 arguments".into());
                }
                let d = self.temp();
                self.emit(Ins::Call(Some(d), t, vals));
                V::T(d)
            }
            Expr::Block(es, has_value) => {
                let mut v = V::C(0);
                for e in es {
                    v = self.value(e)?;
                }
                if *has_value { v } else { V::C(0) }
            }
            Expr::If(c, t, e) => {
                let r = self.temp();
                let (bt, bf, out) = (self.block(), self.block(), self.block());
                self.cond(c, bt, bf)?;
                self.start(bt);
                let v = self.value(t)?;
                self.copy(r, v);
                self.jump(out);
                self.start(bf);
                let v = match e {
                    Some(e) => self.value(e)?,
                    None => V::C(0),
                };
                self.copy(r, v);
                self.jump(out);
                self.start(out);
                V::T(r)
            }
            Expr::Loop {
                until,
                post,
                cond,
                body,
            } => {
                let r = self.temp();
                let (head, b, done, out) = (self.block(), self.block(), self.block(), self.block());
                self.jump(if *post { b } else { head });
                self.start(head);
                if *until {
                    self.cond(cond, done, b)?;
                } else {
                    self.cond(cond, b, done)?;
                }
                self.start(b);
                self.loops.push((out, r));
                self.value(body)?;
                self.loops.pop();
                self.jump(head);
                self.start(done);
                self.copy(r, V::C(-1));
                self.jump(out);
                self.start(out);
                V::T(r)
            }
            Expr::Incr {
                var,
                down,
                unsigned,
                from,
                to,
                by,
                body,
            } => {
                let opt = |g: &mut Self, e: &Option<Box<Expr>>, d: i64| match e {
                    Some(e) => g.value(e),
                    None => Ok(V::C(d)),
                };
                let (lo, hi) = match (*down, *unsigned) {
                    (false, false) => (0, i64::MAX),
                    (true, false) => (0, i64::MIN),
                    (false, true) => (0, -1),
                    (true, true) => (0, 0),
                };
                let start = opt(self, from, lo)?;
                let i = self.address(*var);
                self.emit(Ins::Store(start, i.clone(), 8));
                let end = opt(self, to, hi)?;
                let end = self.fix(end);
                let step = opt(self, by, 1)?;
                let step = self.fix(step);
                let r = self.temp();
                let (head, b, done, out) = (self.block(), self.block(), self.block(), self.block());
                self.jump(head);
                self.start(head);
                let t = self.temp();
                self.emit(Ins::Load(t, i.clone(), 8, false));
                let past = match (*down, *unsigned) {
                    (false, false) => Op::Cgt,
                    (false, true) => Op::Cgtu,
                    (true, false) => Op::Clt,
                    (true, true) => Op::Cltu,
                };
                let c = self.bin(past, V::T(t), end);
                self.end(Term::Jlbs(c, done, b));
                self.start(b);
                self.loops.push((out, r));
                self.value(body)?;
                self.loops.pop();
                let t = self.temp();
                self.emit(Ins::Load(t, i.clone(), 8, false));
                let n = self.bin(if *down { Op::Sub } else { Op::Add }, V::T(t), step);
                self.emit(Ins::Store(n, i, 8));
                self.jump(head);
                self.start(done);
                self.copy(r, V::C(-1));
                self.jump(out);
                self.start(out);
                V::T(r)
            }
            Expr::Case { sel, lo, hi, arms } => self.case(sel, *lo, *hi, arms)?,
            Expr::Select {
                sel,
                one,
                unsigned,
                arms,
            } => self.select(sel, *one, *unsigned, arms)?,
            Expr::Labeled(label, e) => {
                let r = self.temp();
                let out = self.block();
                self.labels.insert(*label, (out, r));
                let v = self.value(e)?;
                self.copy(r, v);
                self.jump(out);
                self.start(out);
                V::T(r)
            }
            Expr::Leave(label, v) => {
                let &(out, r) = self.labels.get(label).ok_or(format!(
                    "LEAVE {} outside its block",
                    self.m.syms[*label].name
                ))?;
                let v = self.optional(v)?;
                self.copy(r, v);
                self.jump(out);
                V::C(0)
            }
            Expr::Exitloop(v) => {
                let &(out, r) = self.loops.last().ok_or("EXITLOOP outside a loop")?;
                let v = self.optional(v)?;
                self.copy(r, v);
                self.jump(out);
                V::C(0)
            }
            Expr::Return(v) => {
                let v = self.optional(v)?;
                self.end(Term::Ret(v));
                V::C(0)
            }
        })
    }

    /// A temporary holding v, so that a later store can't change it.
    fn fix(&mut self, v: V) -> V {
        match v {
            V::T(_) => {
                let d = self.temp();
                self.copy(d, v);
                V::T(d)
            }
            v => v,
        }
    }

    fn optional(&mut self, v: &Option<Box<Expr>>) -> R<V> {
        match v {
            Some(e) => self.value(e),
            None => Ok(V::C(-1)),
        }
    }

    fn binary(&mut self, op: BOp, a: V, b: V) -> V {
        let op = match op {
            BOp::Add => Op::Add,
            BOp::Sub => Op::Sub,
            BOp::Mul => Op::Mul,
            BOp::Div => Op::Div,
            BOp::Mod => Op::Rem,
            BOp::And => Op::And,
            BOp::Or => Op::Or,
            BOp::Xor => Op::Xor,
            BOp::Eqv => Op::Eqv,
            BOp::Rel(rel, u) => rel_op(rel, u),
            BOp::Shift => match b {
                V::C(n) if (0..64).contains(&n) => Op::Shl,
                V::C(n) if (-63..0).contains(&n) => return self.bin(Op::Sar, a, V::C(-n)),
                V::C(n) if n > 0 => return V::C(0),
                V::C(_) => return self.bin(Op::Sar, a, V::C(63)),
                _ => Op::Ash,
            },
        };
        self.bin(op, a, b)
    }

    /// Branches to t if the low bit of e is set, else to f.
    fn cond(&mut self, e: &Expr, t: u32, f: u32) -> R<()> {
        let v = self.value(e)?;
        match v {
            V::C(c) => self.jump(if c & 1 != 0 { t } else { f }),
            v => self.end(Term::Jlbs(v, t, f)),
        }
        Ok(())
    }

    /// Where a fetch reads and an assignment writes: a name's own field, a
    /// field reference, or a fullword at the address an expression gives.
    fn place(&mut self, e: &Expr) -> R<Place> {
        Ok(match e {
            Expr::Name(id) => {
                let addr = self.address(*id);
                match self.m.syms[*id].kind {
                    Kind::Data { size, signed, .. } => Place {
                        addr,
                        pos: V::C(0),
                        size: V::C(i64::from(size) * 8),
                        signed,
                    },
                    _ => full(addr),
                }
            }
            Expr::Index(base, index) => {
                let Expr::Name(id) = **base else {
                    unreachable!()
                };
                let Kind::Data {
                    vector: Some((unit, signed)),
                    ..
                } = self.m.syms[id].kind
                else {
                    unreachable!()
                };
                let base = self.address(id);
                let addr = match self.value(index)? {
                    V::C(i) => self.add(base, i * i64::from(unit)),
                    i => {
                        let i = self.bin(Op::Mul, i, V::C(unit.into()));
                        self.bin(Op::Add, base, i)
                    }
                };
                Place {
                    addr,
                    pos: V::C(0),
                    size: V::C(i64::from(unit) * 8),
                    signed,
                }
            }
            Expr::Field(base, pos, size, ext) => {
                let addr = match **base {
                    Expr::Name(id) => self.address(id),
                    ref b => self.value(b)?,
                };
                let pos = self.value(pos)?;
                let size = self.value(size)?;
                Place {
                    addr,
                    pos,
                    size,
                    signed: *ext,
                }
            }
            e => {
                let addr = self.value(e)?;
                full(addr)
            }
        })
    }

    /// The width of memory to access for a field, and where: for constant
    /// position and size, the smallest of 1, 2, 4, 8 bytes that holds it.
    fn span(&mut self, p: &Place) -> R<(V, u8, V)> {
        if let (V::C(pos), V::C(size)) = (&p.pos, &p.size) {
            let (pos, size) = (*pos, *size);
            if pos < 0 || !(0..=64).contains(&size) {
                return Err(format!("field <{pos}, {size}> out of range"));
            }
            let bit = pos % 8;
            let width = match bit + size {
                0..=8 => 1,
                9..=16 => 2,
                17..=32 => 4,
                33..=64 => 8,
                _ => return Err(format!("field <{pos}, {size}> spans more than a quadword")),
            };
            let addr = self.add(p.addr.clone(), pos / 8);
            return Ok((addr, width, V::C(bit)));
        }
        // ponytail: a variable field reads a quadword from its byte, which
        // can run past the data at the end of a page.
        let byte = self.bin(Op::Shr, p.pos.clone(), V::C(3));
        let addr = self.bin(Op::Add, p.addr.clone(), byte);
        let bit = self.bin(Op::And, p.pos.clone(), V::C(7));
        Ok((addr, 8, bit))
    }

    fn load(&mut self, p: Place) -> V {
        if p.size == V::C(0) {
            return V::C(0);
        }
        let Ok((addr, width, bit)) = self.span(&p) else {
            return V::C(0);
        };
        let d = self.temp();
        if bit == V::C(0) && p.size == V::C(i64::from(width) * 8) {
            self.emit(Ins::Load(d, addr, width, p.signed));
            return V::T(d);
        }
        self.emit(Ins::Load(d, addr, width, false));
        let e = self.temp();
        self.emit(Ins::Ext(e, V::T(d), bit, p.size, p.signed));
        V::T(e)
    }

    fn store(&mut self, p: Place, v: V) {
        if p.size == V::C(0) {
            return;
        }
        let Ok((addr, width, bit)) = self.span(&p) else {
            return;
        };
        if bit == V::C(0) && p.size == V::C(i64::from(width) * 8) {
            self.emit(Ins::Store(v, addr, width));
            return;
        }
        let d = self.temp();
        self.emit(Ins::Load(d, addr.clone(), width, false));
        let n = self.temp();
        self.emit(Ins::Insert(n, V::T(d), v, bit, p.size));
        self.emit(Ins::Store(V::T(n), addr, width));
    }

    /// `%ASCID`: a static descriptor and its text in $PLIT$.
    fn ascid(&mut self, text: &[u8]) -> V {
        let n = self.data.len();
        let (desc, body) = (format!("P.{n}"), format!("P.{}", n + 1));
        let head = text.len() as i64 | 14 << 16 | 1 << 24;
        self.data.push(ir::Data {
            psect: "$PLIT$".into(),
            name: desc.clone(),
            global: false,
            align: 3,
            items: vec![
                Item::Val(V::C(head), 4),
                Item::Val(V::Sym(body.clone(), 0), 4),
            ],
        });
        self.data.push(ir::Data {
            psect: "$PLIT$".into(),
            name: body,
            global: false,
            align: 0,
            items: vec![Item::Bytes(text.to_vec())],
        });
        V::Sym(desc, 0)
    }

    fn case(&mut self, sel: &Expr, lo: i64, hi: i64, arms: &[(Vec<CaseLabel>, Expr)]) -> R<V> {
        let v = self.value(sel)?;
        let v = self.fix(v);
        let r = self.temp();
        let out = self.block();
        let bodies: Vec<u32> = arms.iter().map(|_| self.block()).collect();
        let find = |l: &CaseLabel| arms.iter().position(|(ls, _)| ls.contains(l));
        let inrange = find(&CaseLabel::Inrange).map_or(out, |i| bodies[i]);
        let outrange = find(&CaseLabel::Outrange).map_or(out, |i| bodies[i]);
        // Out of range first, then each label in order.
        let below = self.bin(Op::Clt, v.clone(), V::C(lo));
        let (b1, b2) = (self.block(), self.block());
        self.end(Term::Jlbs(below, outrange, b1));
        self.start(b1);
        let above = self.bin(Op::Cgt, v.clone(), V::C(hi));
        self.end(Term::Jlbs(above, outrange, b2));
        self.start(b2);
        for ((labels, _), &body) in arms.iter().zip(&bodies) {
            for l in labels {
                if let CaseLabel::Range(a, b) = *l {
                    let c = if a == b {
                        self.bin(Op::Ceq, v.clone(), V::C(a))
                    } else {
                        let ge = self.bin(Op::Cge, v.clone(), V::C(a));
                        let le = self.bin(Op::Cle, v.clone(), V::C(b));
                        self.bin(Op::And, ge, le)
                    };
                    let next = self.block();
                    self.end(Term::Jlbs(c, body, next));
                    self.start(next);
                }
            }
        }
        self.jump(inrange);
        for ((_, e), &body) in arms.iter().zip(&bodies) {
            self.start(body);
            let x = self.value(e)?;
            self.copy(r, x);
            self.jump(out);
        }
        self.start(out);
        Ok(V::T(r))
    }

    fn select(
        &mut self,
        sel: &Expr,
        one: bool,
        unsigned: bool,
        arms: &[(Vec<SelectLabel>, Expr)],
    ) -> R<V> {
        let v = self.value(sel)?;
        let v = self.fix(v);
        let (r, matched) = (self.temp(), self.temp());
        self.copy(r, V::C(-1));
        self.copy(matched, V::C(0));
        let out = self.block();
        for (labels, e) in arms {
            let (body, next) = (self.block(), self.block());
            for l in labels {
                let c = match l {
                    SelectLabel::Always => V::C(1),
                    SelectLabel::Otherwise => self.bin(Op::Ceq, V::T(matched), V::C(0)),
                    SelectLabel::Range(a, None) => {
                        let a = self.value(a)?;
                        self.bin(Op::Ceq, v.clone(), a)
                    }
                    SelectLabel::Range(a, Some(b)) => {
                        let a = self.value(a)?;
                        let b = self.value(b)?;
                        let ge = self.bin(if unsigned { Op::Cgeu } else { Op::Cge }, v.clone(), a);
                        let le = self.bin(if unsigned { Op::Cleu } else { Op::Cle }, v.clone(), b);
                        self.bin(Op::And, ge, le)
                    }
                };
                let other = self.block();
                match c {
                    V::C(c) => self.jump(if c & 1 != 0 { body } else { other }),
                    c => self.end(Term::Jlbs(c, body, other)),
                }
                self.start(other);
            }
            self.jump(next);
            self.start(body);
            let x = self.value(e)?;
            self.copy(r, x);
            self.copy(matched, V::C(1));
            self.jump(if one { out } else { next });
            self.start(next);
        }
        self.jump(out);
        self.start(out);
        Ok(V::T(r))
    }
}

fn full(addr: V) -> Place {
    Place {
        addr,
        pos: V::C(0),
        size: V::C(64),
        signed: false,
    }
}

/// Removes the blocks nothing reaches from the entry, and numbers the rest
/// in their order.
fn prune(f: &mut Func) {
    let n = f.blocks.len();
    let mut seen = vec![false; n];
    let mut work = vec![0usize];
    while let Some(b) = work.pop() {
        if std::mem::replace(&mut seen[b], true) {
            continue;
        }
        match f.blocks[b].term {
            Term::Jmp(t) => work.push(t as usize),
            Term::Jlbs(_, t, e) => {
                work.push(t as usize);
                work.push(e as usize);
            }
            Term::Ret(_) => {}
        }
    }
    let mut map = vec![u32::MAX; n];
    let mut next = 0;
    for b in 0..n {
        if seen[b] {
            map[b] = next;
            next += 1;
        }
    }
    let blocks = std::mem::take(&mut f.blocks);
    for (b, mut block) in blocks.into_iter().enumerate() {
        if !seen[b] {
            continue;
        }
        match &mut block.term {
            Term::Jmp(t) => *t = map[*t as usize],
            Term::Jlbs(_, t, e) => {
                *t = map[*t as usize];
                *e = map[*e as usize];
            }
            Term::Ret(_) => {}
        }
        f.blocks.push(block);
    }
}
