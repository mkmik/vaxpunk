//! Floating point literals (LRM 4.2, RN 2.10.2): `%E`, `%D` and `%G` in
//! VAX F, D and G floating, `%S` and `%T` in IEEE single and double, as
//! the bits a fullword holds. A decimal literal is rounded once, to
//! nearest with ties to even, from its exact value: VAX D's 56 bits are
//! more than an f64's, so the conversion is done in integers.

use std::cmp::Ordering;

/// A float literal's format.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    F,
    D,
    G,
    S,
    T,
}

impl Format {
    /// The format a `%E`, `%D`, `%G`, `%S` or `%T` literal is in.
    pub fn of(name: &str) -> Option<Format> {
        Some(match name {
            "%E" => Format::F,
            "%D" => Format::D,
            "%G" => Format::G,
            "%S" => Format::S,
            "%T" => Format::T,
            _ => return None,
        })
    }

    /// Its size in bytes.
    pub fn bytes(self) -> u8 {
        match self {
            Format::F | Format::S => 4,
            _ => 8,
        }
    }

    /// The fraction's bits with the hidden one, and the exponent's bias
    /// and width: the value is 1.f * 2^(e - bias), e from 1 to 2^width - 1.
    fn layout(self) -> (u32, i32, u32) {
        match self {
            // VAX: 0.1f * 2^(e - 128) is 1.f * 2^(e - 129).
            Format::F => (24, 129, 8),
            Format::D => (56, 129, 8),
            Format::G => (53, 1025, 11),
            Format::S => (24, 127, 8),
            Format::T => (53, 1023, 11),
        }
    }
}

/// What became of a literal's value.
#[derive(Debug, PartialEq)]
pub enum Fit {
    Exact,
    /// Too small: zero, or an IEEE subnormal.
    Underflow,
    /// Too large: the largest magnitude, or IEEE infinity.
    Overflow,
}

/// The bits of literal `text` in `fmt`, as a fullword holds them: VAX
/// formats with their 16-bit words in memory order; and whether it was
/// in range. None if it isn't a number.
pub fn literal(fmt: Format, text: &str) -> Option<(u64, Fit)> {
    let (neg, digits, exp10) = decimal(text)?;
    // A value past 10^400 or below 10^-400 is out of every format's
    // range: 10^400 or 10^-400 stands for it, to overflow or underflow
    // without arithmetic on its own exponent.
    let top = digits
        .iter()
        .position(|&d| d != 0)
        .map(|i| exp10.saturating_add((digits.len() - i) as i64));
    let (digits, exp10) = match top {
        Some(t) if t > 400 => (vec![1], 400),
        Some(t) if t < -400 => (vec![1], -400),
        _ => (digits, exp10 as i32),
    };
    let (bits, bias, width) = fmt.layout();
    // BLISSA64 converts a D literal at G's precision, 53 bits, and widens
    // it (oracle FLTLIT2: %D'0.1' ends in D0, not CD).
    let rounded = if fmt == Format::D { 53 } else { bits };
    let Some((e2, m, lost)) = round(&digits, exp10, rounded) else {
        return Some((sign(fmt, neg, 0), Fit::Exact));
    };
    let m = m << (bits - rounded);
    let biased = e2 + bias;
    let max = (1i32 << width) - 1;
    let ieee = matches!(fmt, Format::S | Format::T);
    let top = if ieee { max - 1 } else { max };
    if biased > top {
        let most = if ieee {
            (max as u64) << (bits - 1)
        } else {
            (1u64 << (width + bits - 1)) - 1
        };
        return Some((sign(fmt, neg, most), Fit::Overflow));
    }
    let fit = if biased >= 1 {
        Fit::Exact
    } else {
        Fit::Underflow
    };
    let frac = m & ((1u128 << (bits - 1)) - 1);
    let packed: u64 = if biased >= 1 {
        ((biased as u64) << (bits - 1)) | frac as u64
    } else if ieee {
        // An IEEE subnormal: rounded again to the bits that are left. A
        // rest of exactly half is half only if the first rounding was
        // exact; otherwise the value is on the side that rounding left.
        let shift = (1 - biased) as u32;
        if shift > bits {
            0
        } else {
            let (q, r) = (m >> shift, m & ((1u128 << shift) - 1));
            let up = match r.cmp(&(1u128 << (shift - 1))).then(lost) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => q & 1 == 1,
            };
            (q + u128::from(up)) as u64
        }
    } else {
        // VAX has no subnormals: zero.
        0
    };
    Some((sign(fmt, neg, packed), fit))
}

/// The sign added, and a VAX format's 16-bit words put in memory order.
/// IEEE has a negative zero, VAX doesn't (oracle NZ, NZ2).
fn sign(fmt: Format, neg: bool, packed: u64) -> u64 {
    let total = 8 * u32::from(fmt.bytes());
    let ieee = matches!(fmt, Format::S | Format::T);
    let v = packed | (u64::from(neg && (packed != 0 || ieee)) << (total - 1));
    match fmt {
        Format::S | Format::T => v,
        Format::F => (v >> 16 | v << 16) & 0xFFFF_FFFF,
        Format::D | Format::G => swap_words(v),
    }
}

/// A quadword with its four 16-bit words in reverse order: VAX D and G
/// floating's memory order.
pub fn swap_words(v: u64) -> u64 {
    (v >> 48) | (v >> 16 & 0xFFFF_0000) | (v << 16 & 0xFFFF_0000_0000) | (v << 48)
}

/// `[+-] digits [. digits] [E|D|G|Q|S|T [+-] digits]`: the sign, the
/// digits and the power of ten they are multiplied by.
fn decimal(text: &str) -> Option<(bool, Vec<u8>, i64)> {
    let t = text.trim();
    let (neg, t) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let split = t.find(|c: char| "EDGQSTedgqst".contains(c));
    let (mant, exp) = match split {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    let exp: i64 = match exp {
        Some(e) => e.trim().parse().ok()?,
        None => 0,
    };
    let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
    if int.is_empty() && frac.is_empty()
        || !int.bytes().chain(frac.bytes()).all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|c| c - b'0').collect();
    Some((neg, digits, exp.saturating_sub(frac.len() as i64)))
}

/// digits * 10^exp10 rounded to `bits` bits, as (e, m, lost): m has its
/// top bit at bits - 1, the value rounded is m * 2^(e - bits + 1), and
/// `lost` is how the exact value compares with it. None for zero.
fn round(digits: &[u8], exp10: i32, bits: u32) -> Option<(i32, u128, Ordering)> {
    let mut num = Big::from(0);
    for &d in digits {
        num.mul_add(10, u64::from(d));
    }
    if num.is_zero() {
        return None;
    }
    let mut den = Big::from(1);
    if exp10 >= 0 {
        for _ in 0..exp10 {
            num.mul_add(10, 0);
        }
    } else {
        for _ in 0..-exp10 {
            den.mul_add(10, 0);
        }
    }
    // Scale so that the quotient has bits + 2 bits or one more.
    let shift = i64::from(bits + 2) + den.bits() as i64 - num.bits() as i64;
    if shift > 0 {
        num.shl(shift as u32);
    } else {
        den.shl((-shift) as u32);
    }
    let (q, rest) = num.div(&den);
    let mut q = q;
    let mut lost = !rest;
    let qbits = 128 - q.leading_zeros();
    let extra = qbits - bits;
    lost |= q & ((1u128 << (extra - 1)) - 1) != 0;
    let half = q >> (extra - 1) & 1 == 1;
    q >>= extra;
    let up = half && (lost || q & 1 == 1);
    q += u128::from(up);
    let below = if up {
        Ordering::Less
    } else if half || lost {
        Ordering::Greater
    } else {
        Ordering::Equal
    };
    // The value is q * 2^(extra - shift); its top bit's power is the
    // exponent.
    let mut e = (qbits as i64 - 1 - shift) as i32;
    if q >> bits != 0 {
        q >>= 1;
        e += 1;
    }
    Some((e, q, below))
}

/// A natural number, 32-bit digits, least significant first: enough
/// arithmetic for exact decimal conversion.
struct Big(Vec<u32>);

impl Big {
    fn from(v: u32) -> Big {
        Big(vec![v])
    }

    fn is_zero(&self) -> bool {
        self.0.iter().all(|&d| d == 0)
    }

    fn bits(&self) -> u32 {
        for (i, &d) in self.0.iter().enumerate().rev() {
            if d != 0 {
                return 32 * i as u32 + 32 - d.leading_zeros();
            }
        }
        0
    }

    /// self = self * m + a.
    fn mul_add(&mut self, m: u64, a: u64) {
        let mut carry = a;
        for d in &mut self.0 {
            let v = u64::from(*d) * m + carry;
            *d = v as u32;
            carry = v >> 32;
        }
        while carry != 0 {
            self.0.push(carry as u32);
            carry >>= 32;
        }
    }

    fn shl(&mut self, n: u32) {
        let (words, bits) = ((n / 32) as usize, n % 32);
        let mut out = vec![0u32; words];
        let mut carry = 0u32;
        for &d in &self.0 {
            out.push(d << bits | carry);
            carry = if bits == 0 { 0 } else { d >> (32 - bits) };
        }
        out.push(carry);
        self.0 = out;
    }

    fn bit(&self, i: u32) -> bool {
        self.0
            .get((i / 32) as usize)
            .is_some_and(|d| d >> (i % 32) & 1 == 1)
    }

    /// self >= other.
    fn geq(&self, other: &Big) -> bool {
        let n = self.0.len().max(other.0.len());
        for i in (0..n).rev() {
            let (a, b) = (
                self.0.get(i).copied().unwrap_or(0),
                other.0.get(i).copied().unwrap_or(0),
            );
            if a != b {
                return a > b;
            }
        }
        true
    }

    fn sub(&mut self, other: &Big) {
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let v = i64::from(self.0[i]) - i64::from(other.0.get(i).copied().unwrap_or(0)) - borrow;
            self.0[i] = v as u32;
            borrow = i64::from(v < 0);
        }
    }

    /// The quotient, at most 127 bits here, and whether the remainder is
    /// zero: binary long division.
    fn div(&self, den: &Big) -> (u128, bool) {
        let mut rem = Big::from(0);
        let mut q = 0u128;
        for i in (0..self.bits()).rev() {
            rem.shl(1);
            if self.bit(i) {
                rem.0[0] |= 1;
            }
            q <<= 1;
            if rem.geq(den) {
                rem.sub(den);
                q |= 1;
            }
        }
        (q, rem.is_zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee() {
        let lit = |f, t| literal(f, t).map(|(b, _)| b);
        assert_eq!(lit(Format::T, "1.5"), Some(1.5f64.to_bits()));
        assert_eq!(lit(Format::T, "-2.25E3"), Some((-2250.0f64).to_bits()));
        assert_eq!(lit(Format::T, "0.1"), Some(0.1f64.to_bits()));
        assert_eq!(lit(Format::S, "0.1"), Some(u64::from(0.1f32.to_bits())));
        assert_eq!(lit(Format::T, "1E-320"), Some(1e-320f64.to_bits()));
        assert_eq!(lit(Format::T, "0"), Some(0));
        // Negative zero, and a negative value too small for the format.
        assert_eq!(lit(Format::T, "-0.0"), Some(1 << 63));
        assert_eq!(lit(Format::S, "-0.0"), Some(1 << 31));
        assert_eq!(lit(Format::T, "-1E-400"), Some(1 << 63));
        assert_eq!(lit(Format::S, "-1E-50"), Some(1 << 31));
        // Above half the smallest subnormal: it, not zero. At half, ties to
        // even; just above a half the first rounding made exact, it.
        assert_eq!(lit(Format::S, "1E-45"), Some(1));
        assert_eq!(lit(Format::T, "4E-324"), Some(1));
        assert_eq!(lit(Format::T, "2.4703282292062327E-324"), Some(0));
        assert_eq!(lit(Format::T, "2.4703282292062328E-324"), Some(1));
        assert_eq!(lit(Format::T, "1E-325"), Some(0));
        assert_eq!(literal(Format::T, "1E400").unwrap().1, Fit::Overflow);
        assert_eq!(literal(Format::T, "1E-320").unwrap().1, Fit::Underflow);
        // Exponents far out of range, without the arithmetic they'd take.
        assert_eq!(
            literal(Format::T, "1E-2147483648"),
            Some((0, Fit::Underflow))
        );
        assert_eq!(
            literal(Format::T, "-1E-9223372036854775808"),
            Some((1 << 63, Fit::Underflow))
        );
        assert_eq!(
            literal(Format::T, "1E2147483647"),
            Some((f64::INFINITY.to_bits(), Fit::Overflow))
        );
        assert_eq!(
            lit(Format::S, "3.4028235E38"),
            Some(u64::from(f32::MAX.to_bits()))
        );
    }

    #[test]
    fn vax() {
        // 1.5 is 0.11 * 2^1: F exponent 129, fraction 0.1 after the hidden
        // bit; the words swapped.
        let lit = |f, t| literal(f, t).map(|(b, _)| b);
        assert_eq!(lit(Format::F, "1.5"), Some(0x0000_40C0));
        assert_eq!(lit(Format::F, "-1.5"), Some(0x0000_C0C0));
        // No negative zero: the sign with a zero exponent is a reserved
        // operand.
        for f in [Format::F, Format::D, Format::G] {
            assert_eq!(lit(f, "-0.0"), Some(0));
        }
        assert_eq!(lit(Format::G, "1.5"), Some(0x0000_0000_0000_4018));
        assert_eq!(lit(Format::D, "1.5"), Some(0x0000_0000_0000_40C0));
        // 0.1: D at G's 53 bits, as BLISSA64 has it, widened.
        assert_eq!(
            lit(Format::D, "0.1"),
            Some(swap_words(0x3ECC_CCCC_CCCC_CCD0))
        );
        assert_eq!(lit(Format::D, "1.0D-3"), Some(0x4FE0_978D_126E_3B83));
        assert_eq!(lit(Format::G, "2.5Q2"), Some(0x4000_408F));
        assert_eq!(
            lit(Format::G, "0.1"),
            Some(swap_words(0x3FD9_9999_9999_999A))
        );
        assert_eq!(literal(Format::F, "1E39").unwrap().1, Fit::Overflow);
    }
}
