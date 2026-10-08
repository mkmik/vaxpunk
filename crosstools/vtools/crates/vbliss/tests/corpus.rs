//! The dot lint on the VAX/VMS V4.3 BLISS-32 sources (PRD-0004 *The lint,
//! measured*), which DEC shipped and are mostly right, so what the lint
//! says of them is mostly false positives. The sources are read where
//! $VBLISS_CORPUS says, by default ~/Library/Caches/vaxpunk/vms43, a clone
//! of github.com/ievukas/VAX-VMS-V4.3; they are never committed.
//!
//!     cargo test -p vbliss --test corpus -- --ignored --nocapture

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "needs the V4.3 sources, which aren't in the repository"]
fn corpus() {
    let root = std::env::var("VBLISS_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Caches/vaxpunk/vms43")
        });
    let mut files = Vec::new();
    walk(&root, &mut files);
    // Require files are found by name in any of the sources' directories.
    let mut include: Vec<PathBuf> = files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("req")))
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .collect();
    include.sort();
    include.dedup();
    include.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib"));
    files.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("b32")));
    files.sort();
    let (mut compiled, mut lines, mut warned) = (0, 0, 0);
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut errors: BTreeMap<String, usize> = BTreeMap::new();
    for p in &files {
        let Ok(text) = fs::read(p).map(|b| String::from_utf8_lossy(&b).into_owned()) else {
            continue;
        };
        let opts = vbliss::Options {
            include: include.clone(),
            ..vbliss::Options::default()
        };
        let out = vbliss::translate(p, &text, &opts);
        lines += text.lines().count();
        match out.error() {
            None => compiled += 1,
            Some(e) => {
                let words = if e.msg.starts_with("can't find") {
                    5
                } else {
                    3
                };
                let key: String = e
                    .msg
                    .split_whitespace()
                    .take(words)
                    .collect::<Vec<_>>()
                    .join(" ");
                *errors.entry(key).or_default() += 1;
            }
        }
        warned += out.lints.len();
        for l in &out.lints {
            let key: String = l
                .msg
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            *kinds.entry(key).or_default() += 1;
            println!("{}:{}: {}", p.display(), l.line, l.msg);
        }
    }
    println!(
        "corpus: {} modules, {lines} lines; {compiled} compile to the end; {warned} lint warnings",
        files.len()
    );
    for (k, n) in &kinds {
        println!("  {n:5} {k}");
    }
    println!("what stops the rest:");
    let mut errors: Vec<_> = errors.into_iter().collect();
    errors.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (k, n) in errors.iter().take(40) {
        println!("  {n:5} {k}");
    }
}
