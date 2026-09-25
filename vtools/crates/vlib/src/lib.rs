//! vlib: the librarian. Puts object modules into object libraries
//! (`docs/library-format.md`), as `LIBRARY/OBJECT/REPLACE` does.

use vms_obj::obj::{self, Gsd, Record, sym};
use vms_obj::olb::{Library, MAX_KEY, Module};

/// An empty library, created at `time` (VMS format).
pub fn new(time: u64) -> Library {
    Library {
        creator: concat!("vlib ", env!("CARGO_PKG_VERSION")).into(),
        created: time,
        updated: time,
        modules: Vec::new(),
    }
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
