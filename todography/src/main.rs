//! todography: checks and queries the map of components in map/.
//!
//!     cargo run -p todography -- lint
//!     cargo run -p todography -- show //exec/mailboxes

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use todography::{Map, json, query};

const USAGE: &str = "\
usage: todography [--map DIR] COMMAND

commands:
  lint [--strict]                 check the map; exit 1 on errors (--strict: on warnings too)
  show LABEL                      one component or feature: its needs, who needs it, why, blockers
  why LABEL                       the chains of needs from the goals down to it
  rdeps [--all] LABEL             what needs it (--all: and what needs those, and so on)
  next [--all] [--milestone M]    wanted work whose hard needs are met (--all: blocked work too)
  debt                            wanted components whose needs aren't researched yet
  json                            the map as JSON
  build [--out PATH]              write the interactive map (default out/map/index.html)

--map DIR is the map directory or the one above it; without it todography looks
for map/MAP.toml here and in every directory above.
";

struct Args {
    map: Option<PathBuf>,
    command: String,
    flags: Vec<String>,
    milestone: Option<String>,
    out: Option<PathBuf>,
    positional: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        map: None,
        command: String::new(),
        flags: vec![],
        milestone: None,
        out: None,
        positional: vec![],
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |what: &str| {
            inline
                .clone()
                .or_else(|| it.next())
                .ok_or(format!("{what} needs a value"))
        };
        match name.as_str() {
            "--map" => a.map = Some(PathBuf::from(value("--map")?)),
            "--milestone" => a.milestone = Some(value("--milestone")?),
            "--out" | "-o" => a.out = Some(PathBuf::from(value("--out")?)),
            "--all" | "--strict" => a.flags.push(name),
            "-h" | "--help" => a.command = "help".into(),
            s if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option {s}")),
            _ if a.command.is_empty() => a.command = arg,
            _ => a.positional.push(arg),
        }
    }
    Ok(a)
}

fn check_flags(a: &Args, allowed: &[&str], positional: usize) -> Result<(), String> {
    for f in &a.flags {
        if !allowed.contains(&f.as_str()) {
            return Err(format!("{} doesn't take {f}", a.command));
        }
    }
    if a.milestone.is_some() && !allowed.contains(&"--milestone") {
        return Err(format!("{} doesn't take --milestone", a.command));
    }
    if a.out.is_some() && !allowed.contains(&"--out") {
        return Err(format!("{} doesn't take --out", a.command));
    }
    if a.positional.len() != positional {
        return Err(match positional {
            0 => format!("{} takes no arguments", a.command),
            _ => format!("{} takes a label, such as //exec/mailboxes", a.command),
        });
    }
    Ok(())
}

/// `path` relative to the current directory where it is below it.
fn shown(path: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    path.strip_prefix(&cwd)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn run() -> Result<ExitCode, String> {
    let a = parse_args()?;
    if a.command.is_empty() || a.command == "help" {
        print!("{USAGE}");
        return Ok(if a.command.is_empty() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        });
    }
    let has = |f: &str| a.flags.iter().any(|x| x == f);
    match a.command.as_str() {
        "lint" => check_flags(&a, &["--strict"], 0)?,
        "show" | "why" => check_flags(&a, &[], 1)?,
        "rdeps" => check_flags(&a, &["--all"], 1)?,
        "next" => check_flags(&a, &["--all", "--milestone"], 0)?,
        "debt" | "json" => check_flags(&a, &[], 0)?,
        "build" => check_flags(&a, &["--out"], 0)?,
        c => return Err(format!("unknown command {c}; try todography help")),
    }
    let dir = Map::find(a.map.as_deref())?;
    let map = Map::load(&dir);
    if a.command != "lint"
        && a.command != "json"
        && a.command != "build"
        && !map.lint.errors.is_empty()
    {
        eprintln!(
            "todography: the map has {} lint errors; see todography lint",
            map.lint.errors.len()
        );
    }
    let label = || query::lookup(&map, &a.positional[0]);
    match a.command.as_str() {
        "lint" => {
            print!("{}", query::lint(&map));
            let fail =
                !map.lint.errors.is_empty() || (has("--strict") && !map.lint.warnings.is_empty());
            return Ok(if fail {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            });
        }
        "show" => {
            let (i, f) = label()?;
            print!("{}", query::show(&map, i, f.as_deref()));
        }
        "why" => print!("{}", query::why(&map, label()?.0)),
        "rdeps" => {
            let (i, f) = label()?;
            print!("{}", query::rdeps(&map, i, f.as_deref(), has("--all")));
        }
        "next" => {
            if let Some(m) = &a.milestone
                && !map.milestones.iter().any(|x| &x.id == m)
            {
                return Err(format!("unknown milestone {m}"));
            }
            print!(
                "{}",
                query::next(&map, has("--all"), a.milestone.as_deref())
            );
        }
        "debt" => print!("{}", query::debt(&map)),
        "json" => println!("{}", json::to_json(&map)),
        "build" => {
            let out = a
                .out
                .clone()
                .unwrap_or_else(|| map.root.join("out/map/index.html"));
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::write(&out, json::page(&map)).map_err(|e| format!("{}: {e}", out.display()))?;
            println!("{}", shown(&out));
        }
        _ => unreachable!(),
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("todography: {e}");
            ExitCode::from(2)
        }
    }
}
