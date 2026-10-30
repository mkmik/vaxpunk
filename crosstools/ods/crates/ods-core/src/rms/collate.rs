//! Collated keys: the collating sequences an indexed file keeps after its
//! prologue, NCS's tables, and the order they give (`indexed.md`,
//! *Collated keys*).

use alloc::vec::Vec;
use core::cmp::Ordering;

use super::{Result, RmsError};
use crate::layout::BLOCK;

/// A collated key's collating sequence.
#[derive(Clone, Debug)]
pub struct Collation {
    /// Its name, as NCS calls it.
    pub name: Vec<u8>,
    prim: [u8; 256],
    sec: [u8; 256],
    /// Characters weighed otherwise: the character, the next or `FF`
    /// for itself alone, and its primary weights, the second 0 for one.
    list: Vec<[u8; 4]>,
}

impl Collation {
    /// Orders two keys as NCS does: by their characters' primary weights,
    /// then if those are the same by their secondary ones, then if they
    /// still are by their bytes, the greater first, so that lowercase
    /// comes before uppercase.
    pub fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        for secondary in [false, true] {
            let o = self.weights(a, secondary).cmp(&self.weights(b, secondary));
            if o != Ordering::Equal {
                return o;
            }
        }
        b.cmp(a)
    }

    fn weights(&self, s: &[u8], secondary: bool) -> Vec<u8> {
        let table = if secondary { &self.sec } else { &self.prim };
        let mut w = Vec::with_capacity(s.len() * 2);
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            let pair = s.get(i + 1).and_then(|&n| self.list.iter().find(|e| e[0] == c && e[1] == n));
            let entry =
                pair.map(|e| (e, 2)).or_else(|| self.list.iter().find(|e| e[0] == c && e[1] == 0xff).map(|e| (e, 1)));
            match entry {
                None => {
                    w.push(table[c as usize]);
                    i += 1;
                }
                Some((e, n)) => {
                    let two = e[3] != 0;
                    if secondary {
                        w.extend(core::iter::repeat_n(table[c as usize], 1 + two as usize));
                    } else {
                        w.push(e[2]);
                        if two {
                            w.push(e[3]);
                        }
                    }
                    i += n;
                }
            }
        }
        // Shorter, with the same weights, comes first.
        w
    }
}

/// Reads the collating sequences of a file's collated keys, `keys` of
/// them by key number, from the tables at VBN `colvbn`: 510 bytes a
/// block, each table a header, 0, 5, the key, its kind, its size, and a
/// longword, then NCS's table, of that size.
pub(crate) fn read(data: &[u8], colvbn: u32, keys: &[usize]) -> Result<Vec<(usize, Collation)>> {
    let bad = || RmsError::Corrupt { what: "collating table", vbn: colvbn };
    let byte = |off: usize| -> Result<u8> {
        let vbn = colvbn as usize + off / (BLOCK - 2);
        data.get((vbn - 1) * BLOCK + off % (BLOCK - 2)).copied().ok_or_else(bad)
    };
    let bytes = |off: usize, n: usize| (off..off + n).map(byte).collect::<Result<Vec<u8>>>();
    let mut out = Vec::new();
    let mut off = 0;
    for &key in keys {
        let h = bytes(off, 12)?;
        if h[0] != 0 || h[1] != 5 || h[2] as usize != key {
            return Err(bad());
        }
        let size = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
        if !(600..=4096).contains(&size) {
            return Err(bad());
        }
        let t = bytes(off + 12, size)?;
        let n = (t[20] as usize).min(31);
        let mut c = Collation { name: t[21..21 + n].to_vec(), prim: [0; 256], sec: [0; 256], list: Vec::new() };
        c.prim.copy_from_slice(&t[88..344]);
        c.sec.copy_from_slice(&t[344..600]);
        for e in t[600..].as_chunks::<4>().0 {
            if e[0] == 0xff && e[1] == 0xff {
                break;
            }
            c.list.push(*e);
        }
        out.push((key, c));
        off += 12 + size;
    }
    Ok(out)
}
