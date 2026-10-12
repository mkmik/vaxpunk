//! The text the queries print, for agents and people at a terminal.

use crate::label::Label;
use crate::map::{Comp, Map, Need, Work};
use std::collections::VecDeque;
use std::fmt::Write;

/// Finds a component or feature named on the command line: `t8y:/exec/qio`,
/// `/exec/qio` or plain `exec/qio`, each with an optional `#feature`.
pub fn lookup(map: &Map, text: &str) -> Result<(usize, Option<String>), String> {
    let rest = text.strip_prefix("t8y:").unwrap_or(text);
    let full = format!("t8y:/{}", rest.trim_start_matches('/'));
    let label = Label::parse(&full, "")?;
    let &i = map.by_path.get(&label.path).ok_or_else(|| {
        format!(
            "no component {}",
            Label {
                feature: None,
                ..label.clone()
            }
        )
    })?;
    if let Some(f) = &label.feature
        && map.comps[i].feature(f).is_none()
    {
        return Err(format!("{} has no feature #{f}", map.comps[i].label()));
    }
    Ok((i, label.feature))
}

fn need_state(n: &Need) -> &'static str {
    if n.fuzzy.is_some() {
        "fuzzy"
    } else if n.target.is_none() {
        "unresolved"
    } else if n.met {
        "met"
    } else if n.soft {
        "unmet, soft"
    } else if !n.active {
        "unmet, dropped feature"
    } else {
        "unmet"
    }
}

fn need_line(n: &Need) -> String {
    let what = match (&n.on, &n.fuzzy) {
        (Some(on), _) => on.clone(),
        (None, Some(f)) => format!("\"{f}\""),
        (None, None) => "?".into(),
    };
    let mut s = format!("{what}  [{}]", need_state(n));
    if let Some(why) = &n.why {
        let _ = write!(s, "  {why}");
    }
    s
}

fn path_line(map: &Map, p: &[usize]) -> String {
    p.iter()
        .map(|&i| map.comps[i].label())
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn facts(c: &Comp) -> String {
    let mut v = vec![
        c.kind.as_str().to_string(),
        c.intent.as_str().to_string(),
        c.status.as_str().to_string(),
        format!("research {}", c.research.as_str()),
    ];
    if let Some(m) = &c.milestone {
        v.push(format!("milestone {m}"));
    }
    v.push(if c.wanted { "wanted" } else { "not wanted" }.into());
    if c.debt {
        v.push("research debt".into());
    }
    v.join(", ")
}

pub fn show(map: &Map, i: usize, feature: Option<&str>) -> String {
    let c = &map.comps[i];
    let mut o = String::new();
    let _ = writeln!(o, "{}  {}", c.label(), c.title);
    if !c.summary.is_empty() {
        let _ = writeln!(o, "  {}", c.summary);
    }
    if !c.aka.is_empty() {
        let _ = writeln!(o, "  aka {}", c.aka.join(", "));
    }
    let _ = writeln!(o, "  {}", facts(c));
    if !c.refs.is_empty() {
        let _ = writeln!(o, "  refs {}", c.refs.join(", "));
    }
    let _ = writeln!(o, "  file {}", c.file);

    if !c.features.is_empty() {
        let _ = writeln!(o, "\nfeatures:");
        let w = c.features.iter().map(|f| f.id.len()).max().unwrap_or(0);
        for f in &c.features {
            let mark = if Some(f.id.as_str()) == feature {
                ">"
            } else {
                " "
            };
            let mut line = format!("{mark} {:<7} {:<w$}  {}", f.status.as_str(), f.id, f.title);
            if let Some(m) = &f.milestone {
                let _ = write!(line, "  ({m})");
            }
            let _ = writeln!(o, "{}", line.trim_end());
            if let Some(note) = &f.note {
                let _ = writeln!(o, "          {:w$}  note: {note}", "");
            }
        }
    }

    let needs: Vec<&Need> = c
        .needs
        .iter()
        .filter(|n| {
            feature.is_none() || n.from_feature.is_none() || n.from_feature.as_deref() == feature
        })
        .collect();
    if !needs.is_empty() {
        let _ = writeln!(o, "\nneeds:");
        for n in needs {
            let from = n
                .from_feature
                .as_ref()
                .map(|f| format!("#{f} -> "))
                .unwrap_or_default();
            let _ = writeln!(o, "  {from}{}", need_line(n));
        }
    }

    let by: Vec<_> = c
        .needed_by
        .iter()
        .filter(|e| {
            feature.is_none() || e.on_feature.is_none() || e.on_feature.as_deref() == feature
        })
        .collect();
    if !by.is_empty() {
        let _ = writeln!(o, "\nneeded by:");
        for e in by {
            let mut line = map.comps[e.from].label();
            if let Some(f) = &e.from_feature {
                let _ = write!(line, "#{f}");
            }
            if let Some(f) = &e.on_feature {
                let _ = write!(line, " -> #{f}");
            }
            if e.soft {
                line.push_str("  [soft]");
            }
            if let Some(why) = &e.why {
                let _ = write!(line, "  {why}");
            }
            let _ = writeln!(o, "  {line}");
        }
    }

    let _ = writeln!(o, "\nwhy:");
    o.push_str(&why_text(map, i));

    let blocked: Vec<&Work> = map
        .work
        .iter()
        .filter(|w| w.comp == i && !w.blocked_by.is_empty())
        .filter(|w| feature.is_none() || w.feature.as_deref() == feature)
        .collect();
    if !blocked.is_empty() {
        let _ = writeln!(o, "\nblocked:");
        for w in blocked {
            let _ = writeln!(o, "  {}  by {}", w.label, w.blocked_by.join(", "));
        }
    }
    o
}

fn why_text(map: &Map, i: usize) -> String {
    let c = &map.comps[i];
    if c.intent == crate::map::Intent::Goal {
        return "  a goal\n".into();
    }
    if !c.wanted {
        return "  not wanted: no goal needs it\n".into();
    }
    let mut o = String::new();
    for p in &c.why_paths {
        let _ = writeln!(o, "  {}", path_line(map, p));
    }
    o
}

pub fn why(map: &Map, i: usize) -> String {
    let c = &map.comps[i];
    format!("{}  {}\n{}", c.label(), c.title, why_text(map, i))
}

/// Who needs component `i`, or its feature `feature`. With `all`, also who
/// needs those, and so on; each component once, at its shortest distance.
pub fn rdeps(map: &Map, i: usize, feature: Option<&str>, all: bool) -> String {
    let mut o = String::new();
    let mut seen = vec![false; map.comps.len()];
    seen[i] = true;
    let mut queue = VecDeque::from([(i, 0usize)]);
    let mut count = 0;
    while let Some((t, depth)) = queue.pop_front() {
        let mut edges: Vec<_> = map.comps[t]
            .needed_by
            .iter()
            .filter(|e| t != i || feature.is_none() || e.on_feature.as_deref() == feature)
            // A feature needing another of its own component's counts
            // only when the question is about that feature.
            .filter(|e| e.from != t || (t == i && feature.is_some()))
            .collect();
        edges.sort_by_key(|e| (map.comps[e.from].path.as_str(), e.from_feature.as_deref()));
        for e in edges {
            let mut line = format!(
                "{:indent$}{}",
                "",
                map.comps[e.from].label(),
                indent = depth * 2
            );
            if let Some(f) = &e.from_feature {
                let _ = write!(line, "#{f}");
            }
            let _ = write!(line, " -> {}", map.comps[t].label());
            if let Some(f) = &e.on_feature {
                let _ = write!(line, "#{f}");
            }
            if e.soft {
                line.push_str("  [soft]");
            }
            let _ = writeln!(o, "{line}");
            count += 1;
            if all && !seen[e.from] {
                seen[e.from] = true;
                queue.push_back((e.from, depth + 1));
            }
        }
    }
    if count == 0 {
        o.push_str("nothing needs it\n");
    }
    o
}

fn milestone_rank(map: &Map, m: Option<&str>) -> usize {
    m.and_then(|m| map.milestones.iter().position(|x| x.id == m))
        .unwrap_or(map.milestones.len())
}

fn band_rank(map: &Map, c: &Comp) -> usize {
    map.areas
        .iter()
        .find(|a| a.id == c.area)
        .and_then(|a| map.bands.iter().position(|b| b.id == a.band))
        .unwrap_or(map.bands.len())
}

/// The work items `next` lists, in its order: by milestone, then the band
/// of their area, then label.
pub fn next_items<'a>(map: &'a Map, all: bool, milestone: Option<&str>) -> Vec<&'a Work> {
    let mut items: Vec<&Work> = map
        .work
        .iter()
        .filter(|w| w.wanted && (all || w.ready))
        .filter(|w| milestone.is_none() || w.milestone.as_deref() == milestone)
        .collect();
    items.sort_by_key(|w| {
        (
            milestone_rank(map, w.milestone.as_deref()),
            band_rank(map, &map.comps[w.comp]),
            w.label.as_str(),
        )
    });
    items
}

pub fn next(map: &Map, all: bool, milestone: Option<&str>) -> String {
    let items = next_items(map, all, milestone);
    let mut o = String::new();
    let mut group: Option<Option<&str>> = None;
    let w = items.iter().map(|x| x.label.len()).max().unwrap_or(0);
    for x in &items {
        let m = x.milestone.as_deref();
        if group != Some(m) {
            if group.is_some() {
                o.push('\n');
            }
            let head = match m.and_then(|m| map.milestones.iter().find(|x| x.id == m)) {
                Some(ms) => format!("{}: {}", ms.id, ms.title),
                None => match m {
                    Some(m) => format!("{m}: unknown milestone"),
                    None => "no milestone".into(),
                },
            };
            let _ = writeln!(o, "{head}");
            group = Some(m);
        }
        let _ = writeln!(o, "  {:<w$}  {:<7}  {}", x.label, x.status, x.title);
        if !x.blocked_by.is_empty() {
            let _ = writeln!(o, "  {:<w$}  blocked by {}", "", x.blocked_by.join(", "));
        }
    }
    let ready = items.iter().filter(|x| x.ready).count();
    if all {
        let _ = writeln!(
            o,
            "{}{ready} ready, {} blocked",
            if items.is_empty() { "" } else { "\n" },
            items.len() - ready
        );
    } else {
        let _ = writeln!(
            o,
            "{}{ready} ready",
            if items.is_empty() { "" } else { "\n" }
        );
    }
    o
}

pub fn debt(map: &Map) -> String {
    let mut o = String::new();
    let mut n = 0;
    for c in map.comps.iter().filter(|c| c.debt) {
        n += 1;
        let _ = writeln!(o, "{}  research {}", c.label(), c.research.as_str());
        for need in c.needs.iter().filter(|x| x.fuzzy.is_some()) {
            let mut line = format!("    fuzzy: {}", need.fuzzy.as_deref().unwrap_or(""));
            if let Some(why) = &need.why {
                let _ = write!(line, "  ({why})");
            }
            let _ = writeln!(o, "{line}");
        }
    }
    let _ = writeln!(o, "{}research debt: {n}", if n > 0 { "\n" } else { "" });
    o
}

pub fn lint(map: &Map) -> String {
    let mut o = String::new();
    for d in &map.lint.errors {
        let _ = writeln!(o, "{}: error: {}", d.file, d.msg);
    }
    for d in &map.lint.warnings {
        let _ = writeln!(o, "{}: warning: {}", d.file, d.msg);
    }
    let _ = writeln!(
        o,
        "{} errors, {} warnings, {} components, research debt: {}",
        map.lint.errors.len(),
        map.lint.warnings.len(),
        map.comps.len(),
        map.comps.iter().filter(|c| c.debt).count()
    );
    o
}
