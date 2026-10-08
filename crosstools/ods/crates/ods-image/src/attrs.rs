//! File attributes as text, both ways: protection codes, UICs, record
//! formats and attributes, file characteristics.

use ods_core::{fch, rat, rfm};

const RFM: [(u8, &str); 7] = [
    (rfm::UDF, "UDF"),
    (rfm::FIX, "FIX"),
    (rfm::VAR, "VAR"),
    (rfm::VFC, "VFC"),
    (rfm::STM, "STM"),
    (rfm::STMLF, "STMLF"),
    (rfm::STMCR, "STMCR"),
];

const RAT: [(u8, &str); 5] =
    [(rat::FTN, "FTN"), (rat::CR, "CR"), (rat::PRN, "PRN"), (rat::BLK, "BLK"), (rat::MSBRCW, "MSB")];

const ORG: [&str; 4] = ["SEQ", "REL", "IDX", "DIR"];

const FCH: [(u32, &str); 17] = [
    (fch::WASCONTIG, "WASCONTIG"),
    (fch::NOBACKUP, "NOBACKUP"),
    (fch::WRITEBACK, "WRITEBACK"),
    (fch::READCHECK, "READCHECK"),
    (fch::WRITCHECK, "WRITCHECK"),
    (fch::CONTIGB, "CONTIGB"),
    (fch::LOCKED, "LOCKED"),
    (fch::CONTIG, "CONTIG"),
    (fch::BADACL, "BADACL"),
    (fch::SPOOL, "SPOOL"),
    (fch::DIRECTORY, "DIRECTORY"),
    (fch::BADBLOCK, "BADBLOCK"),
    (fch::MARKDEL, "MARKDEL"),
    (fch::NOCHARGE, "NOCHARGE"),
    (fch::ERASE, "ERASE"),
    (1 << 18, "ALM_AIP"),
    (1 << 19, "SHELVED"),
];

/// Record format name: "VAR", or the number if unknown.
pub fn rfm_name(rtype: u8) -> String {
    let f = rtype & 0xf;
    RFM.iter().find(|r| r.0 == f).map_or(f.to_string(), |r| r.1.to_string())
}

pub fn parse_rfm(s: &str) -> Option<u8> {
    RFM.iter().find(|r| r.1.eq_ignore_ascii_case(s)).map(|r| r.0)
}

/// File organization: "SEQ", "REL", "IDX", "DIR".
pub fn org_name(rtype: u8) -> &'static str {
    ORG[(rtype >> 4 & 3) as usize]
}

/// Record attribute flags joined with commas, "NONE" when empty.
pub fn rat_names(rattrib: u8) -> String {
    let v: Vec<&str> = RAT.iter().filter(|r| rattrib & r.0 != 0).map(|r| r.1).collect();
    if v.is_empty() { "NONE".into() } else { v.join(",") }
}

/// Parses "CR,BLK" or "NONE".
pub fn parse_rat(s: &str) -> Option<u8> {
    let mut v = 0;
    for part in s.split(',').filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("NONE")) {
        v |= RAT.iter().find(|r| r.1.eq_ignore_ascii_case(part))?.0;
    }
    Some(v)
}

/// File characteristics joined with commas.
pub fn fch_names(filechar: u32) -> String {
    let v: Vec<&str> = FCH.iter().filter(|f| filechar & f.0 != 0).map(|f| f.1).collect();
    v.join(",")
}

/// "S:RWED,O:RWED,G:RE,W:" from a protection word, where a set bit denies.
pub fn protection(p: u16) -> String {
    let cats = ["S", "O", "G", "W"];
    let parts: Vec<String> = cats
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let bits = p >> (4 * i) & 0xf;
            let access: String =
                "RWED".chars().enumerate().filter(|(j, _)| bits >> j & 1 == 0).map(|(_, a)| a).collect();
            format!("{c}:{access}")
        })
        .collect();
    parts.join(",")
}

/// Parses "S:RWED,O:RWED,G:RE,W:"; categories not named deny everything,
/// as in DCL's SET PROTECTION=(...) for a full code.
pub fn parse_protection(s: &str) -> Option<u16> {
    let mut p = 0xffffu16;
    for part in s.split(',').filter(|p| !p.is_empty()) {
        let (cat, access) = part.split_once([':', '='])?;
        let shift = match cat.trim().to_ascii_uppercase().as_str() {
            "S" | "SYSTEM" => 0,
            "O" | "OWNER" => 4,
            "G" | "GROUP" => 8,
            "W" | "WORLD" => 12,
            _ => return None,
        };
        let mut deny = 0xfu16;
        for c in access.trim().chars() {
            deny &= !(1 << "RWED".find(c.to_ascii_uppercase())?);
        }
        p = p & !(0xf << shift) | deny << shift;
    }
    Some(p)
}

/// "[g,m]" in octal, as VMS prints UICs.
pub fn uic(u: u32) -> String {
    format!("[{:o},{:o}]", u >> 16, u & 0xffff)
}

/// Parses "[g,m]" (octal) or a plain number.
pub fn parse_uic(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let (g, m) = inner.split_once(',')?;
        let g = u32::from_str_radix(g.trim(), 8).ok().filter(|&g| g <= 0xffff)?;
        let m = u32::from_str_radix(m.trim(), 8).ok().filter(|&m| m <= 0xffff)?;
        return Some(g << 16 | m);
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(protection(0xfa00), "S:RWED,O:RWED,G:RE,W:");
        assert_eq!(parse_protection("S:RWED,O:RWED,G:RE,W:"), Some(0xfa00));
        assert_eq!(parse_protection("s=rwed,o=rwed"), Some(0xff00));
        assert_eq!(parse_protection("X:R"), None);
        assert_eq!(uic(0x0001_0004), "[1,4]");
        assert_eq!(parse_uic("[1,4]"), Some(0x0001_0004));
        assert_eq!(parse_uic("[377,377]"), Some(0x00ff_00ff));
        assert_eq!(rat_names(rat::CR), "CR");
        assert_eq!(parse_rat("cr,blk"), Some(rat::CR | rat::BLK));
        assert_eq!(parse_rat("NONE"), Some(0));
        assert_eq!(rfm_name(0x12), "VAR");
        assert_eq!(org_name(0x12), "REL");
        assert_eq!(parse_rfm("stmlf"), Some(rfm::STMLF));
    }
}
