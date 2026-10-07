//! vdefs: the `$xxxDEF` macros of a MACRO-32 macro library (`lib.mlb`,
//! `starlet.mlb`) as a BLISS require file, so BLISS code and MACRO-32 code
//! share one source of the definitions (PRD-0004 *Definitions*).
//!
//! Each symbol's kind comes from its name, as VMS names encode it: the
//! letters between `$` and `_`. A data field (`PCB$L_STS`) becomes a
//! macro of its four access actuals for a `BLOCK[, BYTE]` reference:
//! `PCB$L_STS = 20, 0, 32, 0 %`. Everything else (constants `$K_` and
//! `$C_`, masks `$M_`, bit numbers `$V_`, sizes `$S_`, and names with no
//! letters, as `SS$_NORMAL`) becomes a LITERAL. A letter the rule doesn't
//! know is an error that names the symbol, to be fixed in the library.
//!
//! DEC's STARLET.R64 has the same field macros, with `$A_`, `$IS_` and
//! `$IH_` fields signed, but makes `$V_` names field macros too, since its
//! sources know which field a bit is in; the macro libraries give only the
//! bit number, so here they are literals: `.PCB[PCB$L_STS]<PCB$V_WALL, 1>`.

use std::collections::HashMap;

/// The dialect a require file is for: `.R64` for BLISS-64, `.REQ` for
/// BLISS-32, whose fields are at most 32 bits, so a quadword is an
/// address with no size, as DEC's STARLET.REQ has it.
#[derive(Clone, Copy, PartialEq)]
pub enum Dialect {
    Bliss64,
    Bliss32,
}

/// What a symbol's type letters make it: a field's size in bits and
/// whether it is signed, or a literal.
fn kind(letters: &str, dialect: Dialect) -> Option<Option<(u32, bool)>> {
    let quad = if dialect == Dialect::Bliss64 { 64 } else { 0 };
    Some(match letters {
        "" | "C" | "K" | "M" | "S" | "V" => None,
        "B" => Some((8, false)),
        "W" => Some((16, false)),
        "L" => Some((32, false)),
        // Addresses and signed integers sign-extend, as in DEC's STARLET.
        "A" | "IS" | "PS" => Some((32, true)),
        "Q" => Some((quad, false)),
        "IH" | "PH" | "PQ" => Some((quad, true)),
        // Text, and arrays: their address, no size.
        "T" | "AB" | "AW" | "AL" | "AQ" => Some((0, false)),
        _ => return None,
    })
}

/// A definition macro: its name, and its symbols with their values in
/// order, with the comment on each line.
struct Def {
    name: String,
    syms: Vec<(String, String, String)>,
}

/// The `$xxxDEF` macros in a macro library's text.
fn defs(text: &str) -> Vec<Def> {
    let mut out: Vec<Def> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let (code, comment) = line.split_once(';').unwrap_or((line, ""));
        let words: Vec<&str> = code.split_whitespace().collect();
        match words.as_slice() {
            [w, name, ..] if w.eq_ignore_ascii_case(".MACRO") => {
                inside = name.starts_with('$') && name.ends_with("DEF");
                if inside {
                    out.push(Def {
                        name: name.to_string(),
                        syms: Vec::new(),
                    });
                }
            }
            [w, ..] if w.eq_ignore_ascii_case(".ENDM") => inside = false,
            _ if inside => {
                if let Some((name, value)) = code.split_once('=') {
                    let value = value.trim_start_matches('=').trim();
                    out.last_mut().unwrap().syms.push((
                        name.trim().to_string(),
                        value.to_string(),
                        comment.trim().to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
    out
}

/// The value of a MACRO-32 expression: numbers (`^X` and the like), names
/// defined before, `+`, `-` and `*`, and `<>` around a part, done first.
fn eval(e: &str, values: &HashMap<String, i64>) -> Result<i64, String> {
    let mut e = e.to_string();
    while let Some(close) = e.find('>') {
        let open = e[..close]
            .rfind('<')
            .ok_or_else(|| format!("can't evaluate {e:?}"))?;
        let v = eval(&e[open + 1..close], values)?;
        e.replace_range(open..=close, &v.to_string());
    }
    let mut total = 0i64;
    let mut sign = 1i64;
    let mut product: Option<i64> = None;
    let mut term = String::new();
    let flush = |term: &str| -> Result<i64, String> {
        let t = term.trim();
        let (radix, digits) = match t.get(..2).map(|p| p.to_ascii_uppercase()) {
            Some(p) if p == "^X" => (16, &t[2..]),
            Some(p) if p == "^O" => (8, &t[2..]),
            Some(p) if p == "^B" => (2, &t[2..]),
            Some(p) if p == "^D" => (10, &t[2..]),
            _ => (10, t),
        };
        if let Ok(v) = u64::from_str_radix(digits, radix) {
            return Ok(v as i64);
        }
        values
            .get(t)
            .copied()
            .ok_or_else(|| format!("can't evaluate {t:?}"))
    };
    for c in e.chars().chain(std::iter::once('\0')) {
        match c {
            '+' | '-' | '*' | '\0' => {
                if term.trim().is_empty() && c == '-' && product.is_none() {
                    sign = -sign;
                    continue;
                }
                let v = flush(&term)?;
                term.clear();
                let v = product.take().map_or(v, |p| p.wrapping_mul(v));
                if c == '*' {
                    product = Some(v);
                    continue;
                }
                total = total.wrapping_add(sign.wrapping_mul(v));
                sign = if c == '-' { -1 } else { 1 };
            }
            c => term.push(c),
        }
    }
    Ok(total)
}

/// The require file for the `$xxxDEF` macros in `text`, macro library
/// `source`.
pub fn generate(source: &str, text: &str, dialect: Dialect) -> Result<String, String> {
    let mut out = format!(
        "! Generated by vdefs from {source}: edit that, then run vdefs.\n\
         ! Fields are BLOCK[, BYTE] access actuals: offset, position, size, extension.\n"
    );
    let mut values = HashMap::new();
    for def in defs(text) {
        let (mut literals, mut fields) = (Vec::new(), Vec::new());
        for (name, value, comment) in &def.syms {
            let v = eval(value, &values).map_err(|e| format!("{name} in {}: {e}", def.name))?;
            // MACRO-32 takes a name again with the same value; BLISS once.
            match values.insert(name.clone(), v) {
                Some(old) if old == v => continue,
                Some(old) => {
                    return Err(format!("{name} in {}: {v}, but {old} before", def.name));
                }
                None => {}
            }
            // A name without a $, as $SIOCDEF's BSD ones, is a literal.
            let letters = match name.split_once('$') {
                Some((_, rest)) => rest.split_once('_').map_or("", |(l, _)| l),
                None => "",
            };
            let comment = if comment.is_empty() {
                String::new()
            } else {
                format!("  ! {comment}")
            };
            match kind(letters, dialect) {
                None => {
                    return Err(format!(
                        "{name} in {}: the naming rule doesn't know ${letters}_",
                        def.name
                    ));
                }
                Some(None) => literals.push((format!("    {name} = {v}"), comment)),
                Some(Some((size, signed))) => fields.push((
                    format!("    {name} = {v}, 0, {size}, {} %", u8::from(signed)),
                    comment,
                )),
            }
        }
        out.push_str(&format!("\n! {}\n", def.name));
        for (word, items, end) in [("LITERAL", &literals, ";"), ("MACRO", &fields, ";")] {
            if items.is_empty() {
                continue;
            }
            out.push_str(word);
            out.push('\n');
            for (i, (item, comment)) in items.iter().enumerate() {
                let sep = if i + 1 == items.len() { end } else { "," };
                out.push_str(&format!("{item}{sep}{comment}\n"));
            }
        }
    }
    Ok(out)
}

/// The run-time library routines the lint knows, which no macro defines:
/// their mechanisms, `V` by value, `A` by reference or descriptor.
const LIBRARY: &[(&str, &str)] = &[
    ("LIB$PUT_OUTPUT", "A"),
    ("LIB$SIGNAL", "V"),
    ("LIB$STOP", "V"),
];

/// The table of mechanisms vbliss's dot lint reads (`services.txt`): for
/// each system service a `$name_S` macro in `text` calls, the service's
/// name and `V` or `A` for each argument, as the macro pushes it with
/// `PUSHL` or `$PUSHADR`; then the library routines above.
pub fn services(source: &str, text: &str) -> String {
    let mut out = format!(
        "! Generated by vdefs from {source}: each routine's arguments, V by value,\n\
         ! A by reference or descriptor, for vbliss's dot lint.\n"
    );
    let mut pushes: Vec<&str> = Vec::new();
    for line in text.lines() {
        let code = line.split(';').next().unwrap_or("");
        let words: Vec<&str> = code.split_whitespace().collect();
        match words.as_slice() {
            [w, name, ..] if w.eq_ignore_ascii_case(".MACRO") && name.ends_with("_S") => {
                pushes.clear();
            }
            [w, ..] if w.eq_ignore_ascii_case("PUSHL") => pushes.push("V"),
            [w, ..] if w.eq_ignore_ascii_case("$PUSHADR") => pushes.push("A"),
            // A macro's own argument (SYS$'NAME) names no routine.
            [w, _, target] if w.eq_ignore_ascii_case("CALLS") && !target.contains('\'') => {
                let routine = target.trim_start_matches("G^");
                // Pushed last argument first.
                let args: Vec<&str> = pushes.iter().rev().copied().collect();
                out.push_str(&format!("{routine} {}\n", args.join(" ")));
                pushes.clear();
            }
            _ => {}
        }
    }
    for (name, args) in LIBRARY {
        out.push_str(&format!("{name} {args}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_literals() {
        let text = "
        .MACRO  $XDEF
X$L_A = 4                               ; a longword
X$Q_B = X$L_A+4
X$K_LENGTH = ^X10
X$V_FLAG = 3
        .ENDM   $XDEF
        .MACRO  SETX A
        .ENDM   SETX
";
        assert_eq!(
            generate("x.mlb", text, Dialect::Bliss64).unwrap(),
            "! Generated by vdefs from x.mlb: edit that, then run vdefs.\n\
             ! Fields are BLOCK[, BYTE] access actuals: offset, position, size, extension.\n\
             \n! $XDEF\n\
             LITERAL\n    X$K_LENGTH = 16,\n    X$V_FLAG = 3;\n\
             MACRO\n    X$L_A = 4, 0, 32, 0 %,  ! a longword\n    X$Q_B = 8, 0, 64, 0 %;\n"
        );
        assert!(
            generate("x.mlb", text, Dialect::Bliss32)
                .unwrap()
                .contains("X$Q_B = 8, 0, 0, 0 %;")
        );
        let bad = ".MACRO $YDEF\nY$Z_FOO = 1\n.ENDM\n";
        assert!(
            generate("y.mlb", bad, Dialect::Bliss64)
                .unwrap_err()
                .contains("Y$Z_FOO")
        );
    }
}
