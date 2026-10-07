//! Data and structures (LRM chapters 9, 11 and 14): the attributes of data
//! declarations, STRUCTURE, FIELD, BIND and MAP declarations, structure
//! references, PLITs, INITIAL and PRESET.
//!
//! A structure's body is parsed once, with its formals as names of their
//! own; a structure reference is a copy of the body with the formals
//! replaced: the segment's address, the access actuals (each evaluated
//! once, in a temporary unless it is a constant or a name), and the
//! allocation actuals, which are constants.

use std::collections::HashMap;
use std::rc::Rc;

use crate::lex::Tok;
use crate::parse::{BOp, Expr, Kind, Parser, R, SelectLabel, Static, Storage, Sym, fold};

/// The predeclared structures (`docs/bliss64.md`), with the dialect's
/// default unit and VECTOR's default extension.
fn predeclared(unit: u8, ext: u8) -> String {
    format!(
        "STRUCTURE
    VECTOR[I; N, UNIT = {unit}, EXT = {ext}] = [N * UNIT] (VECTOR + I * UNIT)<0, 8 * UNIT, EXT>,
    BITVECTOR[I; N] = [(N + 7) / 8] BITVECTOR<I, 1>,
    BLOCK[O, P, S, E; BS, UNIT = {unit}] = [BS * UNIT] (BLOCK + O * UNIT)<P, S, E>,
    BLOCKVECTOR[I, O, P, S, E; N, BS, UNIT = {unit}] =
        [N * BS * UNIT] (BLOCKVECTOR + (I * BS + O) * UNIT)<P, S, E>,
    BLOCK_BYTE[O, P, S, E; BS] = [BS] (BLOCK_BYTE + O)<P, S, E>;"
    )
}

/// A structure declaration.
#[derive(Debug)]
pub struct Structure {
    /// The symbols its body uses for its formals: the structure's own
    /// name, the access formals, then the allocation formals.
    pub formals: Vec<usize>,
    pub access: usize,
    /// The allocation formals' defaults.
    pub defaults: Vec<Option<i64>>,
    /// How many bytes it takes, in its allocation formals.
    pub size: Option<Expr>,
    pub body: Expr,
}

impl PartialEq for Structure {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A data segment's structure attribute: the structure, its allocation
/// actuals (defaults filled in) and whether it is `REF`.
#[derive(Debug, PartialEq)]
pub struct StructAttr {
    pub st: Rc<Structure>,
    pub alloc: Vec<Option<i64>>,
    pub refr: bool,
    /// The field names its FIELD attribute allows in references.
    pub fields: Vec<usize>,
}

/// An item of initial data: a value of n bytes, or bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum Init {
    Val(Expr, u8),
    Bytes(Vec<u8>),
}

impl Init {
    pub fn len(&self) -> u32 {
        match self {
            Init::Val(_, n) => (*n).into(),
            Init::Bytes(b) => b.len() as u32,
        }
    }
}

/// The attributes of a data declaration.
#[derive(Default)]
pub(crate) struct Attrs {
    pub unit: Option<u8>,
    pub signed: Option<bool>,
    pub structure: Option<StructAttr>,
    pub initial: Option<Vec<Init>>,
    pub preset: Option<Vec<(Vec<Option<Expr>>, Expr)>>,
    pub volatile: bool,
}

/// The size in bytes of an allocation unit, if `name` is one.
pub fn unit(name: &str) -> Option<u8> {
    Some(match name {
        "BYTE" => 1,
        "WORD" => 2,
        "LONG" => 4,
        "QUAD" => 8,
        _ => return None,
    })
}

/// A copy of `e` with `f` applied to each subexpression first: where it
/// gives an expression, that replaces the subexpression.
pub fn rewrite(e: &Expr, f: &mut dyn FnMut(&Expr) -> Option<Expr>) -> Expr {
    if let Some(r) = f(e) {
        return r;
    }
    let mut b = |e: &Expr| Box::new(rewrite(e, f));
    match e {
        Expr::Num(_) | Expr::Name(_) | Expr::Ascid(_) | Expr::Temp(_) | Expr::Jacket => e.clone(),
        Expr::Special(s, args) => Expr::Special(*s, args.iter().map(|a| *b(a)).collect()),
        Expr::Op(op, x, y) => Expr::Op(*op, b(x), b(y)),
        Expr::Fetch(a) => Expr::Fetch(b(a)),
        Expr::Field(a, p, s, x) => Expr::Field(b(a), b(p), b(s), b(x)),
        Expr::Let(t, v, body) => Expr::Let(*t, b(v), b(body)),
        Expr::Plit(n, counted, items) => Expr::Plit(
            *n,
            *counted,
            items
                .iter()
                .map(|i| match i {
                    Init::Val(v, n) => Init::Val(*b(v), *n),
                    i => i.clone(),
                })
                .collect(),
        ),
        Expr::Neg(a) => Expr::Neg(b(a)),
        Expr::Not(a) => Expr::Not(b(a)),
        Expr::Bin(op, x, y) => Expr::Bin(*op, b(x), b(y)),
        Expr::Assign(x, y) => Expr::Assign(b(x), b(y)),
        Expr::Call(t, args) => Expr::Call(b(t), args.iter().map(|a| *b(a)).collect()),
        Expr::Block(es, v) => Expr::Block(es.iter().map(|a| *b(a)).collect(), *v),
        Expr::If(c, t, x) => Expr::If(b(c), b(t), x.as_ref().map(|x| b(x))),
        Expr::Loop {
            until,
            post,
            cond,
            body,
        } => Expr::Loop {
            until: *until,
            post: *post,
            cond: b(cond),
            body: b(body),
        },
        Expr::Incr {
            var,
            down,
            unsigned,
            from,
            to,
            by,
            body,
        } => Expr::Incr {
            var: *var,
            down: *down,
            unsigned: *unsigned,
            from: from.as_ref().map(|x| b(x)),
            to: to.as_ref().map(|x| b(x)),
            by: by.as_ref().map(|x| b(x)),
            body: b(body),
        },
        Expr::Case { sel, lo, hi, arms } => Expr::Case {
            sel: b(sel),
            lo: *lo,
            hi: *hi,
            arms: arms.iter().map(|(l, a)| (l.clone(), *b(a))).collect(),
        },
        Expr::Select {
            sel,
            one,
            unsigned,
            arms,
        } => Expr::Select {
            sel: b(sel),
            one: *one,
            unsigned: *unsigned,
            arms: arms
                .iter()
                .map(|(ls, a)| {
                    let ls = ls
                        .iter()
                        .map(|l| match l {
                            SelectLabel::Range(x, y) => {
                                SelectLabel::Range(*b(x), y.as_ref().map(|y| *b(y)))
                            }
                            l => l.clone(),
                        })
                        .collect();
                    (ls, *b(a))
                })
                .collect(),
        },
        Expr::Labeled(l, a) => Expr::Labeled(*l, b(a)),
        Expr::Leave(l, v) => Expr::Leave(*l, v.as_ref().map(|v| b(v))),
        Expr::Exitloop(v) => Expr::Exitloop(v.as_ref().map(|v| b(v))),
        Expr::Return(v) => Expr::Return(v.as_ref().map(|v| b(v))),
    }
}

/// The data segment an address is in and its byte offset from it, if
/// they are constant.
pub fn base_offset(e: &Expr) -> Option<(usize, i64)> {
    match e {
        Expr::Name(id) => Some((*id, 0)),
        Expr::Block(es, true) if es.len() == 1 => base_offset(&es[0]),
        Expr::Bin(BOp::Add, a, b) => match (base_offset(a), base_offset(b)) {
            (Some((id, x)), _) => Some((id, x.wrapping_add(fold(b)?))),
            (None, Some((id, y))) => Some((id, fold(a)?.wrapping_add(y))),
            _ => None,
        },
        Expr::Bin(BOp::Sub, a, b) => {
            let (id, x) = base_offset(a)?;
            Some((id, x.wrapping_sub(fold(b)?)))
        }
        _ => None,
    }
}

/// The byte offset `e` is from data segment `id`'s address, if it is a
/// constant one.
fn offset(e: &Expr, id: usize) -> Option<i64> {
    match e {
        Expr::Name(n) if *n == id => Some(0),
        Expr::Block(es, true) if es.len() == 1 => offset(&es[0], id),
        Expr::Bin(BOp::Add, a, b) => match (offset(a, id), offset(b, id)) {
            (Some(x), None) => Some(x.wrapping_add(fold(b)?)),
            (None, Some(y)) => Some(fold(a)?.wrapping_add(y)),
            _ => None,
        },
        Expr::Bin(BOp::Sub, a, b) => Some(offset(a, id)?.wrapping_sub(fold(b)?)),
        _ => None,
    }
}

impl Parser<'_> {
    /// Declares the predeclared structures.
    pub(crate) fn predeclare_structures(&mut self) -> R<()> {
        let saved = std::mem::take(&mut self.toks);
        let pos = std::mem::replace(&mut self.pos, 0);
        let d = self.m.dialect;
        let text = predeclared(d.unit(), u8::from(d.signed_long));
        let mut toks = crate::lex::lex(&text, 0).expect("the predeclared structures lex");
        for t in &mut toks {
            t.col = crate::listing::NOPOS;
        }
        let r = self.subparse(toks, |p| {
            p.expect("STRUCTURE")?;
            p.structures()?;
            p.expect_punct(';')
        });
        self.toks = saved;
        self.pos = pos;
        r
    }

    /// `STRUCTURE` definitions, after the word.
    pub(crate) fn structures(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            self.expect_punct('[')?;
            let (mut access, mut alloc) = (Vec::new(), Vec::new());
            if !self.at_punct(';') && !self.at_punct(']') {
                loop {
                    access.push(self.name()?);
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            if self.eat_punct(';') {
                loop {
                    let n = self.name()?;
                    let d = if self.eat_punct('=') {
                        Some(self.ctce()?)
                    } else {
                        None
                    };
                    alloc.push((n, d));
                    if !self.eat_punct(',') {
                        break;
                    }
                }
            }
            self.expect_punct(']')?;
            self.expect_punct('=')?;
            self.scopes.push(HashMap::new());
            let mut formals = Vec::new();
            for n in std::iter::once(&name)
                .chain(&access)
                .chain(alloc.iter().map(|(n, _)| n))
            {
                formals.push(self.declare(n.clone(), Kind::StructFormal)?);
            }
            let size = if self.eat_punct('[') {
                let e = self.expr()?;
                self.expect_punct(']')?;
                Some(e)
            } else {
                None
            };
            let body = self.expr();
            self.scopes.pop();
            let st = Structure {
                formals,
                access: access.len(),
                defaults: alloc.iter().map(|(_, d)| *d).collect(),
                size,
                body: body?,
            };
            self.declare(name, Kind::Structure(Rc::new(st)))?;
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `FIELD` definitions and field sets, after the word.
    pub(crate) fn fields(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            self.expect_punct('=')?;
            if self.eat("SET") {
                let mut set = Vec::new();
                loop {
                    let f = self.name()?;
                    self.expect_punct('=')?;
                    let comps = self.components()?;
                    set.push(self.declare(f, Kind::Field(comps))?);
                    if !self.eat_punct(',') {
                        break;
                    }
                }
                self.expect("TES")?;
                self.declare(name, Kind::FieldSet(set))?;
            } else {
                let comps = self.components()?;
                self.declare(name, Kind::Field(comps))?;
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// A field's components: `[ctce, ...]`.
    fn components(&mut self) -> R<Vec<i64>> {
        self.expect_punct('[')?;
        let mut comps = Vec::new();
        if !self.at_punct(']') {
            loop {
                comps.push(self.ctce()?);
                if !self.eat_punct(',') {
                    break;
                }
            }
        }
        self.expect_punct(']')?;
        Ok(comps)
    }

    /// A structure attribute after its `REF`, if any: the structure's name
    /// and its allocation actuals.
    fn struct_attr(&mut self, refr: bool) -> R<StructAttr> {
        let name = self.name()?;
        let st = match self.lookup(&name).map(|id| &self.m.syms[id].kind) {
            Some(Kind::Structure(st)) => st.clone(),
            _ => return self.err(format!("{name} is not a structure")),
        };
        let mut alloc = vec![None; st.defaults.len()];
        if self.eat_punct('[') {
            for (i, v) in self.alloc_actuals()?.into_iter().enumerate() {
                if i >= alloc.len() {
                    return self.err(format!("too many allocation actuals for {name}"));
                }
                alloc[i] = v;
            }
            self.expect_punct(']')?;
        }
        for (a, d) in alloc.iter_mut().zip(&st.defaults) {
            if a.is_none() {
                *a = *d;
            }
        }
        Ok(StructAttr {
            st,
            alloc,
            refr,
            fields: Vec::new(),
        })
    }

    /// Allocation actuals up to the `]` or `;`: constants, units and
    /// SIGNED or UNSIGNED, or nothing.
    fn alloc_actuals(&mut self) -> R<Vec<Option<i64>>> {
        let mut out = Vec::new();
        loop {
            let v = match self.peek().clone() {
                Tok::Punct(',' | ']' | ';') => None,
                Tok::Name(n) if unit(&n).is_some() || n == "SIGNED" || n == "UNSIGNED" => {
                    self.pos += 1;
                    Some(match n.as_str() {
                        "SIGNED" => 1,
                        "UNSIGNED" => 0,
                        n => unit(n).unwrap().into(),
                    })
                }
                _ => Some(self.ctce()?),
            };
            out.push(v);
            if !self.eat_punct(',') {
                return Ok(out);
            }
        }
    }

    /// How many bytes data with structure attribute `a` takes, if known.
    pub(crate) fn struct_bytes(&self, a: &StructAttr) -> Option<i64> {
        if a.refr {
            let d = self.m.dialect;
            return Some(if d.ref_long || d.signed_long {
                4
            } else {
                d.fullword().into()
            });
        }
        let map: HashMap<usize, Expr> =
            a.st.formals
                .iter()
                .skip(1 + a.st.access)
                .zip(&a.alloc)
                .filter_map(|(f, v)| v.map(|v| (*f, Expr::Num(v))))
                .collect();
        let size = rewrite(a.st.size.as_ref()?, &mut |e| match e {
            Expr::Name(id) => map.get(id).cloned(),
            _ => None,
        });
        fold(&size)
    }

    /// `%SIZE`'s parameter: a structure attribute's size.
    pub(crate) fn size_of_attr(&mut self) -> R<i64> {
        let refr = self.eat("REF");
        let a = self.struct_attr(refr)?;
        match self.struct_bytes(&a) {
            Some(n) => Ok(n),
            None => self.err("the structure's size isn't known"),
        }
    }

    /// The attributes after a data name's `:`.
    pub(crate) fn attributes(&mut self) -> R<Attrs> {
        let mut a = Attrs::default();
        loop {
            let word = match self.peek() {
                Tok::Name(n) | Tok::Bound(n, _) => n.clone(),
                _ => return Ok(a),
            };
            match word.as_str() {
                w if unit(w).is_some() => {
                    self.pos += 1;
                    a.unit = unit(w);
                }
                "SIGNED" | "UNSIGNED" => {
                    self.pos += 1;
                    a.signed = Some(word == "SIGNED");
                }
                "VOLATILE" => {
                    self.pos += 1;
                    a.volatile = true;
                }
                "ALIAS" | "WEAK" | "NOVALUE" => self.pos += 1,
                "FIELD" => {
                    self.pos += 1;
                    self.expect_punct('(')?;
                    let mut fields = Vec::new();
                    loop {
                        let n = self.name()?;
                        match self.lookup(&n).map(|id| (id, &self.m.syms[id].kind)) {
                            Some((id, Kind::Field(_))) => fields.push(id),
                            Some((_, Kind::FieldSet(set))) => fields.extend(set.iter().copied()),
                            _ => return self.err(format!("{n} is not a field name")),
                        }
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                    self.expect_punct(')')?;
                    match &mut a.structure {
                        Some(s) => s.fields = fields,
                        None => return self.err("FIELD without a structure attribute"),
                    }
                }
                "ALIGN" | "PSECT" | "EXTERNAL_NAME" | "ADDRESSING_MODE" => {
                    // ponytail: accepted and ignored; PSECT and
                    // EXTERNAL_NAME come with the pilot if it needs them.
                    self.pos += 1;
                    self.expect_punct('(')?;
                    let mut depth = 1;
                    while depth > 0 {
                        match self.next() {
                            Tok::Punct('(') => depth += 1,
                            Tok::Punct(')') => depth -= 1,
                            Tok::Eof => return self.err("unbalanced parentheses"),
                            _ => {}
                        }
                    }
                }
                "REF" => {
                    self.pos += 1;
                    a.structure = Some(self.struct_attr(true)?);
                }
                "INITIAL" => {
                    self.pos += 1;
                    let unit = match a.structure {
                        Some(_) => self.m.dialect.unit(),
                        None => a.unit.unwrap_or(self.m.dialect.unit()),
                    };
                    a.initial = Some(self.init_list(unit)?);
                }
                "PRESET" => {
                    self.pos += 1;
                    self.expect_punct('(')?;
                    let mut items = Vec::new();
                    loop {
                        self.expect_punct('[')?;
                        let access = self.access_actuals()?;
                        self.expect_punct(']')?;
                        self.expect_punct('=')?;
                        items.push((access, self.expr()?));
                        if !self.eat_punct(',') {
                            break;
                        }
                    }
                    self.expect_punct(')')?;
                    a.preset = Some(items);
                }
                n if matches!(
                    self.lookup(n).map(|id| &self.m.syms[id].kind),
                    Some(Kind::Structure(_))
                ) =>
                {
                    a.structure = Some(self.struct_attr(false)?);
                }
                _ => return Ok(a),
            }
        }
    }

    /// A data name's kind from its attributes: its storage, size and
    /// default field.
    pub(crate) fn data_kind(&mut self, storage: Storage, a: &mut Attrs) -> R<Kind> {
        let structure = a.structure.take().map(Rc::new);
        let d = self.m.dialect;
        let (size, signed) = match &structure {
            // REF_LONG: a REF is a signed longword.
            Some(s) if s.refr && (d.ref_long || d.signed_long) => (4, true),
            Some(_) => (d.fullword(), d.a32),
            None => (
                a.unit.unwrap_or(d.unit()),
                a.signed
                    .unwrap_or(d.signed_long || (d.a32 && a.unit.is_none_or(|u| u == 4))),
            ),
        };
        let bytes = match &structure {
            Some(s) => match (self.struct_bytes(s), storage) {
                (Some(n), _) => n as u32,
                (None, Storage::External | Storage::Bind(_)) => 0,
                (None, _) => return self.err("the structure's size isn't known"),
            },
            None => size.into(),
        };
        let storage = match storage {
            Storage::Local(_) => Storage::Local(self.slot(bytes)),
            s => s,
        };
        Ok(Kind::Data {
            storage,
            bytes,
            size,
            signed,
            structure,
        })
    }

    /// OWN, GLOBAL, EXTERNAL, LOCAL and STACKLOCAL names, after the word.
    pub(crate) fn data(&mut self, storage: Storage) -> R<()> {
        loop {
            let at = self.here();
            let name = self.name()?;
            let mut a = if self.eat_punct(':') {
                self.attributes()?
            } else {
                Attrs::default()
            };
            if !self.at_punct(',') && !self.at_punct(';') {
                let found = self.describe();
                return self.err(format!("attribute {found} is not supported yet"));
            }
            if matches!(storage, Storage::External) && (a.initial.is_some() || a.preset.is_some()) {
                return self.err("INITIAL or PRESET on EXTERNAL data");
            }
            let (initial, preset) = (a.initial.take(), a.preset.take());
            let kind = self.data_kind(storage, &mut a)?;
            let Kind::Data { bytes, storage, .. } = kind else {
                unreachable!()
            };
            let id = self.declare(name, kind)?;
            if a.volatile {
                self.volatile.push(id);
            }
            let mut init = match (initial, preset) {
                (Some(i), _) => i,
                (None, Some(p)) => self.preset(id, p)?,
                (None, None) => Vec::new(),
            };
            let used: u32 = init.iter().map(Init::len).sum();
            if used > bytes {
                self.diag(
                    'W',
                    &at,
                    format!("Size of initial value ({used}) exceeds declared size ({bytes})"),
                );
                init = truncate(init, bytes);
            }
            match storage {
                Storage::Own | Storage::Global => self.m.statics.push(Static { sym: id, init }),
                Storage::Local(_) if !init.is_empty() => {
                    // Assignments at the block's start, the rest zeros.
                    let mut off = 0;
                    for item in init {
                        let n = item.len();
                        match item {
                            Init::Val(v, n) => self.inits.push(byte_store(id, off, n, v)),
                            Init::Bytes(b) => {
                                for (i, c) in b.into_iter().enumerate() {
                                    self.inits.push(byte_store(
                                        id,
                                        off + i as u32,
                                        1,
                                        Expr::Num(c.into()),
                                    ))
                                }
                            }
                        }
                        off += n;
                    }
                    while off < bytes {
                        let n = if (bytes - off) >= 8 { 8 } else { 1 };
                        self.inits.push(byte_store(id, off, n, Expr::Num(0)));
                        off += u32::from(n);
                    }
                }
                _ => {}
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// A PRESET as initial data: the image of the fields it sets, for
    /// static data; for a LOCAL, assignments at the block's start, after
    /// zeros.
    fn preset(&mut self, id: usize, items: Vec<(Vec<Option<Expr>>, Expr)>) -> R<Vec<Init>> {
        let Kind::Data {
            bytes,
            storage,
            structure: Some(a),
            ..
        } = self.m.syms[id].kind.clone()
        else {
            return self.err("PRESET on data without a structure");
        };
        if let Storage::Local(_) = storage {
            let mut off = 0;
            while off < bytes {
                let n = if (bytes - off) >= 8 { 8 } else { 1 };
                self.inits.push(byte_store(id, off, n, Expr::Num(0)));
                off += u32::from(n);
            }
            for (access, v) in items {
                let place = self.instantiate(&a.st, Expr::Name(id), access, &a.alloc);
                self.inits.push(Expr::Assign(Box::new(place), Box::new(v)));
            }
            return Ok(Vec::new());
        }
        let mut image = vec![0u8; bytes as usize];
        let mut relocs: Vec<(u32, Expr, u8)> = Vec::new();
        for (access, v) in items {
            let place = self.instantiate(&a.st, Expr::Name(id), access, &a.alloc);
            let (base, pos, size) = match &place {
                Expr::Field(b, p, s, _) => (&**b, fold(p), fold(s)),
                e => (e, Some(0), Some(64)),
            };
            let (Some(off), Some(pos), Some(size)) = (offset(base, id), pos, size) else {
                return self.err("a PRESET field isn't at a constant place");
            };
            let bit = off * 8 + pos;
            if bit < 0 || !(0..=64).contains(&size) || bit + size > i64::from(bytes) * 8 {
                return self.err("a PRESET field is outside the data");
            }
            match fold(&v) {
                Some(v) => {
                    for i in 0..size {
                        let b = (bit + i) as usize;
                        let on = v >> i & 1 != 0;
                        image[b / 8] = image[b / 8] & !(1 << (b % 8)) | (u8::from(on) << (b % 8));
                    }
                }
                None if bit % 8 == 0 && (size == 32 || size == 64) && self.ltce(&v) => {
                    relocs.push(((bit / 8) as u32, v, (size / 8) as u8));
                }
                None => return self.err("a PRESET value isn't a link-time constant"),
            }
        }
        relocs.sort_by_key(|r| r.0);
        let mut init = Vec::new();
        let mut at = 0;
        for (off, v, n) in relocs {
            if off > at {
                init.push(Init::Bytes(image[at as usize..off as usize].to_vec()));
            }
            init.push(Init::Val(v, n));
            at = off + u32::from(n);
        }
        if (at as usize) < image.len() {
            init.push(Init::Bytes(image[at as usize..].to_vec()));
        }
        Ok(init)
    }

    /// `(items)` of a PLIT or an INITIAL, each `unit` bytes unless it says.
    pub(crate) fn init_list(&mut self, unit: u8) -> R<Vec<Init>> {
        self.expect_punct('(')?;
        let mut out = Vec::new();
        loop {
            self.init_item(unit, &mut out)?;
            if !self.eat_punct(',') {
                break;
            }
        }
        self.expect_punct(')')?;
        Ok(out)
    }

    fn init_item(&mut self, unit: u8, out: &mut Vec<Init>) -> R<()> {
        let word = match self.peek() {
            Tok::Name(n) => n.clone(),
            _ => String::new(),
        };
        if word == "REP" {
            self.pos += 1;
            let n = self.ctce()?;
            self.expect("OF")?;
            let u = self.unit_word().unwrap_or(unit);
            let group = self.init_list(u)?;
            for _ in 0..n.max(0) {
                out.extend(group.iter().cloned());
            }
        } else if let Some(u) =
            crate::data::unit(&word).filter(|_| *self.peek2() == Tok::Punct('('))
        {
            self.pos += 1;
            out.extend(self.init_list(u)?);
        } else if let (Tok::Str(s), Tok::Punct(',' | ')')) =
            (self.peek().clone(), self.peek2().clone())
        {
            self.pos += 1;
            let mut s = s;
            let u = usize::from(unit);
            s.resize(s.len().div_ceil(u).max(1) * u, 0);
            out.push(Init::Bytes(s));
        } else {
            out.push(Init::Val(self.expr()?, unit));
        }
        Ok(())
    }

    /// An allocation unit word, if one is next.
    fn unit_word(&mut self) -> Option<u8> {
        let u = match self.peek() {
            Tok::Name(n) => unit(n),
            _ => None,
        };
        if u.is_some() {
            self.pos += 1;
        }
        u
    }

    /// `PLIT` or `UPLIT`, after the word.
    pub(crate) fn plit(&mut self, counted: bool) -> R<Expr> {
        let mut unit = self.m.dialect.unit();
        loop {
            if let Some(u) = self.unit_word() {
                unit = u;
            } else if self.eat("PSECT") {
                // ponytail: PLITs go in $PLIT$ whatever PSECT says.
                self.expect_punct('(')?;
                self.name()?;
                self.expect_punct(')')?;
            } else {
                break;
            }
        }
        let items = self.init_list(unit)?;
        self.lets += 1;
        Ok(Expr::Plit(self.lets - 1, counted, items))
    }

    /// Access actuals up to the `]` or `;`: expressions, field names
    /// (their components), or nothing.
    pub(crate) fn access_actuals(&mut self) -> R<Vec<Option<Expr>>> {
        self.access_actuals_of(None)
    }

    /// Access actuals of an ordinary reference to data `of`, whose FIELD
    /// attribute says which field names it may use.
    pub(crate) fn access_actuals_of(
        &mut self,
        of: Option<(usize, &[usize])>,
    ) -> R<Vec<Option<Expr>>> {
        let mut out = Vec::new();
        if self.at_punct(']') {
            return Ok(out);
        }
        loop {
            let field = match self.peek().clone() {
                Tok::Name(n) | Tok::Bound(n, _) => {
                    match self.lookup(&n).map(|id| (id, &self.m.syms[id].kind)) {
                        Some((id, Kind::Field(c))) => Some((id, n, c.clone())),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let (Some((id, n, _)), Some((data, allowed))) = (&field, of)
                && !allowed.contains(id)
            {
                let at = self.here();
                let msg = format!(
                    "Field name {n} invalid in structure reference to data segment {}",
                    self.m.syms[data].name
                );
                self.diag('W', &at, msg);
            }
            let field = field.map(|(_, _, c)| c);
            if self.at_punct(',') || self.at_punct(']') || self.at_punct(';') {
                out.push(None);
            } else if let Some(c) =
                field.filter(|_| matches!(self.peek2(), Tok::Punct(',' | ']' | ';')))
            {
                self.pos += 1;
                out.extend(c.into_iter().map(|v| Some(Expr::Num(v))));
            } else {
                out.push(Some(self.expr()?));
            }
            if !self.eat_punct(',') {
                return Ok(out);
            }
        }
    }

    /// A structure reference: the structure's body for segment `seg`.
    pub(crate) fn instantiate(
        &mut self,
        st: &Structure,
        seg: Expr,
        access: Vec<Option<Expr>>,
        alloc: &[Option<i64>],
    ) -> Expr {
        let mut map: HashMap<usize, Expr> = HashMap::new();
        map.insert(st.formals[0], seg);
        let mut lets = Vec::new();
        let mut access = access.into_iter();
        for &f in &st.formals[1..=st.access] {
            let a = access.next().flatten().unwrap_or(Expr::Num(0));
            let a = match fold(&a) {
                Some(v) => Expr::Num(v),
                None if matches!(a, Expr::Name(_)) => a,
                None => {
                    let t = self.lets;
                    self.lets += 1;
                    lets.push((t, a));
                    Expr::Temp(t)
                }
            };
            map.insert(f, a);
        }
        for (&f, v) in st.formals[1 + st.access..].iter().zip(alloc) {
            map.insert(f, Expr::Num(v.unwrap_or(0)));
        }
        let mut body = rewrite(&st.body, &mut |e| match e {
            Expr::Name(id) => map.get(id).cloned(),
            _ => None,
        });
        for (t, a) in lets.into_iter().rev() {
            body = Expr::Let(t, Box::new(a), Box::new(body));
        }
        body
    }

    /// A general structure reference, after the structure's name and `[`.
    pub(crate) fn general_ref(&mut self, st: Rc<Structure>) -> R<Expr> {
        let seg = self.expr()?;
        let access = if self.eat_punct(',') {
            self.access_actuals()?
        } else {
            Vec::new()
        };
        let mut alloc = vec![None; st.defaults.len()];
        if self.eat_punct(';') {
            for (i, v) in self.alloc_actuals()?.into_iter().enumerate() {
                if i < alloc.len() {
                    alloc[i] = v;
                }
            }
        }
        self.expect_punct(']')?;
        for (a, d) in alloc.iter_mut().zip(&st.defaults) {
            if a.is_none() {
                *a = *d;
            }
        }
        Ok(self.instantiate(&st, seg, access, &alloc))
    }

    /// `BIND` and `BIND ROUTINE` names, after the words.
    pub(crate) fn binds(&mut self, routine: bool) -> R<()> {
        loop {
            let name = self.name()?;
            self.expect_punct('=')?;
            let e = self.expr()?;
            if routine {
                self.routine_attributes()?;
                let Expr::Name(id) = e else {
                    return self.err("BIND ROUTINE to anything but a routine's name");
                };
                if !matches!(self.m.syms[id].kind, Kind::Routine { .. }) {
                    return self.err("BIND ROUTINE to anything but a routine's name");
                }
                self.scopes.last_mut().unwrap().insert(name, id);
            } else {
                let mut a = if self.eat_punct(':') {
                    self.attributes()?
                } else {
                    Attrs::default()
                };
                let e = if self.ltce(&e) {
                    e
                } else if self.slots_open() {
                    // Evaluated as the block starts, into a hidden local.
                    let slot = self.slot(8);
                    let h = self.m.syms.len();
                    self.m.syms.push(Sym {
                        name: format!("{name}.BIND"),
                        asm: String::new(),
                        kind: Kind::Data {
                            storage: Storage::Local(slot),
                            bytes: 8,
                            size: 8,
                            signed: false,
                            structure: None,
                        },
                    });
                    self.inits
                        .push(Expr::Assign(Box::new(Expr::Name(h)), Box::new(e)));
                    Expr::Fetch(Box::new(Expr::Name(h)))
                } else {
                    return self.err(format!("BIND {name} to an address not known by link time"));
                };
                let i = self.m.binds.len() as u32;
                self.m.binds.push(e);
                let kind = self.data_kind(Storage::Bind(i), &mut a)?;
                self.declare(name, kind)?;
            }
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }

    /// `MAP` names, after the word: new attributes for data already
    /// declared, in this block.
    pub(crate) fn maps(&mut self) -> R<()> {
        loop {
            let name = self.name()?;
            let Some(id) = self.lookup(&name) else {
                return self.err(format!("Undeclared name:  {name}"));
            };
            let Kind::Data { storage, bytes, .. } = self.m.syms[id].kind else {
                return self.err(format!("MAP of {name}, which isn't data"));
            };
            self.expect_punct(':')?;
            let mut a = self.attributes()?;
            let Kind::Data {
                size,
                signed,
                structure,
                ..
            } = self.data_kind(Storage::External, &mut a)?
            else {
                unreachable!()
            };
            let asm = self.m.syms[id].asm.clone();
            let new = self.m.syms.len();
            self.m.syms.push(Sym {
                name: name.clone(),
                asm,
                kind: Kind::Data {
                    storage,
                    bytes,
                    size,
                    signed,
                    structure,
                },
            });
            self.scopes.last_mut().unwrap().insert(name, new);
            if !self.eat_punct(',') {
                return Ok(());
            }
        }
    }
}

/// An assignment of `v` to the `n` bytes at `off` in data `id`.
fn byte_store(id: usize, off: u32, n: u8, v: Expr) -> Expr {
    let addr = Expr::Bin(
        BOp::Add,
        Box::new(Expr::Name(id)),
        Box::new(Expr::Num(off.into())),
    );
    Expr::Assign(
        Box::new(Expr::Field(
            Box::new(addr),
            Box::new(Expr::Num(0)),
            Box::new(Expr::Num(i64::from(n) * 8)),
            Box::new(Expr::Num(0)),
        )),
        Box::new(v),
    )
}

/// The first `bytes` bytes of initial data.
fn truncate(init: Vec<Init>, bytes: u32) -> Vec<Init> {
    let mut out = Vec::new();
    let mut left = bytes;
    for i in init {
        let n = i.len();
        if n <= left {
            left -= n;
            out.push(i);
        } else if let Init::Bytes(mut b) = i {
            b.truncate(left as usize);
            out.push(Init::Bytes(b));
            break;
        } else {
            break;
        }
    }
    out
}
