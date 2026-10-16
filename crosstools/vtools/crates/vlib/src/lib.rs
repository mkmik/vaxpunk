//! vlib: the librarian. Puts object modules into object libraries
//! (`docs/library-format.md`), as `LIBRARY/OBJECT/REPLACE` does, and help
//! text into help libraries, as `LIBRARY/HELP/REPLACE` does.

use vms_obj::obj::{self, Gsd, Record, sym};
use vms_obj::olb::{HelpLibrary, Library, MAX_HELP_KEY, MAX_KEY, Module, Topic};

const CREATOR: &str = concat!("vlib ", env!("CARGO_PKG_VERSION"));

/// An empty library, created at `time` (VMS format).
pub fn new(time: u64) -> Library {
    Library {
        creator: CREATOR.into(),
        created: time,
        updated: time,
        modules: Vec::new(),
    }
}

/// An empty help library, created at `time` (VMS format).
pub fn new_help(time: u64) -> HelpLibrary {
    HelpLibrary {
        creator: CREATOR.into(),
        created: time,
        updated: time,
        topics: Vec::new(),
    }
}

/// Puts the topics of help file `file`, its text `text`, into `lib`,
/// replacing topics of the same name. A line `1 NAME` starts a topic, named
/// NAME in capitals; its lines, that one first, go up to the next. The
/// error, if any, is a VMS message.
pub fn replace_help(
    lib: &mut HelpLibrary,
    file: &str,
    text: &str,
    time: u64,
) -> Result<(), String> {
    let mut topics: Vec<Topic> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let bad = |msg: &str| format!("%VLIB-F-BADHLP, {file}:{}: {msg}", n + 1);
        let line = line.trim_end();
        if line.len() > 255 {
            return Err(bad("line longer than 255 characters"));
        }
        if let Some(name) = line.strip_prefix("1 ").or_else(|| line.strip_prefix("1\t")) {
            let name = name.trim().to_ascii_uppercase();
            if name.is_empty() || name.len() > MAX_HELP_KEY || !name.is_ascii() {
                return Err(bad("a topic needs an ASCII name of 1 to 31 characters"));
            }
            topics.push(Topic {
                name,
                inserted: time,
                lines: Vec::new(),
            });
        }
        match topics.last_mut() {
            Some(t) => t.lines.push(line.into()),
            None if line.is_empty() || line.starts_with('!') => {}
            None => return Err(bad("text before the first topic, a line \"1 name\"")),
        }
    }
    for t in topics {
        lib.topics.retain(|o| o.name != t.name);
        lib.topics.push(t);
    }
    Ok(())
}

/// Puts the modules of object file `file` into `lib`, replacing modules of the
/// same name. The symbol index gets each module's strong global definitions,
/// unless another module has them already. Returns warnings; the error, if
/// any, is a VMS message.
pub fn replace(
    lib: &mut Library,
    file: &str,
    bytes: &[u8],
    time: u64,
) -> Result<Vec<String>, String> {
    let bad = |msg: &str| format!("%VLIB-F-BADOBJ, {file}: {msg}");
    let records = obj::parse(bytes).map_err(|e| bad(&format!("not an object file: {e}")))?;
    if records.is_empty() {
        return Err(bad("no modules"));
    }
    let mut warnings = Vec::new();
    for module in records.split_inclusive(|r| matches!(r, Record::Eom(..))) {
        let (Some(Record::Mhd(h)), Some(Record::Eom(..))) = (module.first(), module.last()) else {
            return Err(bad("a module lacks its header or end record"));
        };
        let mut symbols: Vec<String> = module
            .iter()
            .filter_map(|r| match r {
                Record::Gsd(g) => Some(g),
                _ => None,
            })
            .flatten()
            .filter_map(|g| match g {
                Gsd::Def(d) if d.flags & sym::WEAK == 0 => Some(d.name.clone()),
                _ => None,
            })
            .collect();
        symbols.sort();
        symbols.dedup();
        if let Some(long) = symbols.iter().chain([&h.name]).find(|s| s.len() > MAX_KEY) {
            return Err(format!(
                "%VLIB-F-KEYLNG, {file}: name {long} is longer than {MAX_KEY} characters"
            ));
        }
        lib.modules.retain(|m| m.name != h.name);
        symbols.retain(|s| match lib.modules.iter().find(|m| m.symbols.contains(s)) {
            Some(m) => {
                warnings.push(format!(
                    "%VLIB-W-DUPGLOBAL, global symbol {s} from module {} is already in the library, from module {}",
                    h.name, m.name
                ));
                false
            }
            None => true,
        });
        lib.modules.push(Module {
            name: h.name.clone(),
            ident: h.version.chars().take(31).collect(),
            inserted: time,
            symbols,
            object: obj::write(module),
        });
    }
    Ok(warnings)
}
