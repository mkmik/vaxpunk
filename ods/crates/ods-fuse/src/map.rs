//! How VMS appears to POSIX: every rule of docs/fuse.md that is a design
//! choice, in one place.

use std::cmp::Ordering;
use std::time::SystemTime;

use ods_image::{Attributes, DirEntry, NameType, attrs, name, time};

/// A name as the host sees it: ISO Latin-1 read as Unicode, UCS-2 decoded.
pub fn host_name(e: &[u8], t: NameType) -> String {
    name::chars(e, t).iter().map(|&c| char::from_u32(c as u32).unwrap_or('?')).collect()
}

/// Case-blind comparison of a host name with a stored one.
fn same(host: &str, stored: &[u8], t: NameType) -> bool {
    let h: Vec<u16> = host.chars().map(|c| c as u32).map(|c| if c > 0xffff { 0xffff } else { c as u16 }).collect();
    name::cmp(&h, &name::chars(stored, t)) == Ordering::Equal
}

/// macOS metadata, which never exists here.
pub fn is_apple_metadata(host: &str) -> bool {
    host == ".DS_Store" || host.starts_with("._")
}

/// A subdirectory entry: NAME.DIR;1 naming a directory.
fn dir_stem(e: &DirEntry) -> Option<&[u8]> {
    e.is_dir_name().then(|| &e.name[..e.name.len() - 4])
}

/// One name as a directory lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub host: String,
    pub entry: DirEntry,
    pub dir: bool,
}

/// What a directory lists, in directory order: subdirectories by name,
/// each file name once as its highest version, and with `versions` every
/// other version as `name;N`. `is_dir` says whether a NAME.DIR;1 entry
/// really is a directory.
pub fn listing(entries: &[DirEntry], is_dir: impl Fn(&DirEntry) -> bool, versions: bool) -> Vec<Shown> {
    let dirs: Vec<&[u8]> = entries.iter().filter(|e| is_dir(e)).filter_map(dir_stem).collect();
    let mut out = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if let Some(stem) = dir_stem(e).filter(|_| is_dir(e)) {
            out.push(Shown { host: host_name(stem, e.name_type), entry: e.clone(), dir: true });
            continue;
        }
        let highest = i == 0 || entries[i - 1].name != e.name;
        let plain = plain_name(e, &dirs);
        if highest {
            out.push(Shown { host: plain, entry: e.clone(), dir: false });
        } else if versions {
            out.push(Shown { host: format!("{plain};{}", e.version), entry: e.clone(), dir: false });
        }
    }
    // Names a damaged volume could hold that no path can: ".", "..", "a/b".
    out.retain(|s| ods_image::safe_host_name(&s.host));
    out
}

/// A file's name without version: an empty type loses its dot, unless a
/// subdirectory has the same name.
fn plain_name(e: &DirEntry, dirs: &[&[u8]]) -> String {
    match e.name.strip_suffix(b".") {
        Some(stem) if !dirs.contains(&stem) => host_name(stem, e.name_type),
        _ => host_name(&e.name, e.name_type),
    }
}

/// Finds what a host name refers to: `name;N` is that version, anything
/// else a subdirectory of that name, else the highest version of the file
/// (a name without a dot meaning an empty type).
pub fn resolve<'a>(entries: &'a [DirEntry], host: &str, is_dir: impl Fn(&DirEntry) -> bool) -> Option<&'a DirEntry> {
    if is_apple_metadata(host) || !ods_image::safe_host_name(host) {
        return None;
    }
    let (base, version) = match host.rsplit_once(';') {
        Some((b, v)) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => (b, v.parse::<u16>().ok()),
        _ => (host, None),
    };
    if version.is_none()
        && let Some(d) = entries.iter().find(|e| is_dir(e) && dir_stem(e).is_some_and(|s| same(base, s, e.name_type)))
    {
        return Some(d);
    }
    let with_dot = format!("{base}.");
    let want = if base.contains('.') && find_file(entries, base, &is_dir, None).is_some() { base } else { &with_dot };
    find_file(entries, want, &is_dir, version)
}

/// The file (not subdirectory) entry named `want`, highest or given version.
fn find_file<'a>(
    entries: &'a [DirEntry],
    want: &str,
    is_dir: &impl Fn(&DirEntry) -> bool,
    version: Option<u16>,
) -> Option<&'a DirEntry> {
    entries
        .iter()
        .filter(|e| !(is_dir(e) && dir_stem(e).is_some()))
        .filter(|e| same(want, &e.name, e.name_type))
        .find(|e| version.is_none_or(|v| e.version == v))
}

/// POSIX permission bits from a VMS protection code: owner from owner,
/// group from group, other from world; R, W and E become r, w and x.
pub fn mode(prot: u16) -> u16 {
    let bits = |cat: u16| {
        let deny = prot >> (4 * cat) & 0xf;
        (deny & 1 == 0) as u16 * 4 + (deny & 2 == 0) as u16 * 2 + (deny & 4 == 0) as u16
    };
    bits(1) << 6 | bits(2) << 3 | bits(3)
}

/// (atime, mtime, ctime, crtime).
pub fn times(a: &Attributes) -> (SystemTime, SystemTime, SystemTime, SystemTime) {
    let t = |v: u64| if v == 0 { SystemTime::UNIX_EPOCH } else { time::to_system(v) };
    let m = if a.revised != 0 { a.revised } else { a.created };
    let or_m = |v: u64| if v != 0 { v } else { m };
    (t(or_m(a.accessed)), t(m), t(or_m(a.attr_changed)), t(a.created))
}

/// The `vms.` extended attributes of a file.
pub fn xattrs(info: &ods_image::FileInfo, version: u16) -> Vec<(&'static str, String)> {
    let a = &info.attrs;
    let r = &a.record;
    vec![
        ("vms.fid", info.fid.to_string()),
        ("vms.version", version.to_string()),
        ("vms.rfm", attrs::rfm_name(r.rtype)),
        ("vms.rat", attrs::rat_names(r.rattrib)),
        ("vms.mrs", r.rsize.to_string()),
        ("vms.lrl", r.maxrec.to_string()),
        ("vms.org", attrs::org_name(r.rtype).to_string()),
        ("vms.fch", attrs::fch_names(a.filechar)),
        ("vms.uic", attrs::uic(a.owner)),
        ("vms.prot", attrs::protection(a.protection)),
        ("vms.eof", r.eof_bytes().to_string()),
        ("vms.created", time::format(a.created)),
        ("vms.revised", time::format(a.revised)),
        ("vms.expires", time::format(a.expires)),
        ("vms.backup", time::format(a.backup)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ods_image::Fid;

    fn e(name: &str, version: u16, num: u32) -> DirEntry {
        DirEntry {
            name: name.as_bytes().to_vec(),
            name_type: NameType::Ods2,
            version,
            fid: Fid::new(num, 1),
            verlimit: 0,
        }
    }

    #[test]
    fn listing_and_lookup() {
        let es = [
            e("A.", 1, 10),
            e("A.DIR", 1, 11),
            e("B.", 1, 12),
            e("LOGIN.COM", 3, 13),
            e("LOGIN.COM", 2, 14),
            e("X.DIR", 2, 15),
        ];
        let is_dir = |d: &DirEntry| d.fid.num == 11;
        let names = |v| listing(&es, is_dir, v).into_iter().map(|s| s.host).collect::<Vec<_>>();
        assert_eq!(names(false), ["A.", "A", "B", "LOGIN.COM", "X.DIR"]);
        assert_eq!(names(true), ["A.", "A", "B", "LOGIN.COM", "LOGIN.COM;2", "X.DIR"]);
        let find = |h: &str| resolve(&es, h, is_dir).map(|d| d.fid.num);
        assert_eq!(find("A"), Some(11));
        assert_eq!(find("a."), Some(10));
        assert_eq!(find("b"), Some(12));
        assert_eq!(find("login.com"), Some(13));
        assert_eq!(find("LOGIN.COM;2"), Some(14));
        assert_eq!(find("LOGIN.COM;9"), None);
        assert_eq!(find("A.DIR"), None);
        assert_eq!(find("X.DIR"), Some(15));
        assert_eq!(find("._LOGIN.COM"), None);
        assert_eq!(find(".DS_Store"), None);
    }

    #[test]
    fn permissions() {
        // S:RWED,O:RWED,G:RE,W: -> rwx r-x ---
        assert_eq!(mode(0xfa00), 0o750);
        // S:RWE,O:RWE,G:RE,W:E -> rwx r-x --x
        assert_eq!(mode(0xba88), 0o751);
        assert_eq!(mode(0), 0o777);
    }
}
