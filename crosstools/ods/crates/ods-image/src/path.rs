//! The Unix path form: `/dir/sub/name.ext;ver` for `[DIR.SUB]NAME.EXT;VER`.

/// Converts a Unix-style path to a VMS file specification; anything not
/// starting with `/` is taken to be one already. A trailing `/` names a
/// directory. Characters VMS syntax gives a meaning to are escaped with `^`
/// (ODS-5 escapes), except wildcards, `...` and the version's `;`.
pub fn to_vms(p: &str) -> String {
    if !p.starts_with('/') {
        return p.to_string();
    }
    let parts: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
    let (dirs, file) = match parts.split_last() {
        Some((last, dirs)) if !p.ends_with('/') => (dirs, Some(*last)),
        _ => (&parts[..], None),
    };
    let mut s = String::from("[");
    if dirs.is_empty() {
        s.push_str("000000");
    }
    for (i, d) in dirs.iter().enumerate() {
        if i > 0 {
            s.push('.');
        }
        let part = if *d == "..." { d.to_string() } else { escape(d, false) };
        s.push_str(&part);
    }
    s.push(']');
    if let Some(f) = file {
        let (name, version) = match f.rfind(';') {
            Some(i) if f[i + 1..].chars().all(|c| c.is_ascii_digit() || c == '-' || c == '*') => (&f[..i], &f[i..]),
            _ => (f, ""),
        };
        s.push_str(&escape(name, true));
        s.push_str(version);
    }
    s
}

/// Escapes VMS delimiters in one path component. With `type_dot`, the last
/// dot stays a name/type delimiter.
fn escape(s: &str, type_dot: bool) -> String {
    let last = if type_dot { s.rfind('.') } else { None };
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        match c {
            '.' if Some(i) == last => out.push('.'),
            '.' | ',' | ';' | '[' | ']' | '<' | '>' | ':' | '^' | '&' => {
                out.push('^');
                out.push(c);
            }
            ' ' => out.push_str("^_"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_vms;

    #[test]
    fn unix_to_vms() {
        for (unix, vms) in [
            ("[A.B]C.D;1", "[A.B]C.D;1"),
            ("C.D", "C.D"),
            ("/", "[000000]"),
            ("/c.txt", "[000000]c.txt"),
            ("/a/b/c.txt;2", "[a.b]c.txt;2"),
            ("/a/b/", "[a.b]"),
            ("/a//b/c;-1", "[a.b]c;-1"),
            ("/x.y.z", "[000000]x^.y.z"),
            ("/dir.with.dots/f", "[dir^.with^.dots]f"),
            ("/.../*.txt;*", "[...]*.txt;*"),
            ("/a b/c d.txt", "[a^_b]c^_d.txt"),
            ("/semi;colon.txt", "[000000]semi^;colon.txt"),
            ("/v;x", "[000000]v^;x"),
        ] {
            assert_eq!(to_vms(unix), vms, "{unix}");
        }
    }
}
