//! What the map works out for itself: status, met needs, who needs what,
//! wanted, why paths, research debt, work items, and the lint checks that
//! need the whole graph.

use crate::map::{Comp, Edge, FeatureStatus, Intent, Map, Research, Status, Work};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// How many why paths each component keeps.
const WHY_PATHS: usize = 3;

pub fn compute(map: &mut Map) {
    for c in &mut map.comps {
        c.status = status(c);
    }
    met(map);
    needed_by(map);
    wanted(map);
    why_paths(map);
    for c in &mut map.comps {
        c.debt = c.wanted
            && (c.research == Research::Deferred || c.needs.iter().any(|n| n.fuzzy.is_some()));
    }
    work(map);
    graph_lint(map);
}

/// A component's status: from its features if it has any, else its field.
pub fn status(c: &Comp) -> Status {
    if c.features.is_empty() {
        return c.status_field.unwrap_or(Status::Todo);
    }
    let live: Vec<_> = c
        .features
        .iter()
        .filter(|f| f.status != FeatureStatus::Dropped)
        .collect();
    if live.is_empty() {
        Status::Dropped
    } else if live.iter().all(|f| f.status == FeatureStatus::Done) {
        Status::Done
    } else if live.iter().any(|f| f.status == FeatureStatus::Done) {
        Status::Partial
    } else if live.iter().any(|f| f.status == FeatureStatus::Wip) {
        Status::Wip
    } else {
        Status::Todo
    }
}

/// A need on a feature is met when the feature is done; on a component,
/// once the component is usable, partial or done. Fuzzy needs never are.
fn met(map: &mut Map) {
    for i in 0..map.comps.len() {
        for k in 0..map.comps[i].needs.len() {
            let n = &map.comps[i].needs[k];
            let met = match n.target {
                None => false,
                Some(t) => {
                    let t = &map.comps[t];
                    match &n.target_feature {
                        Some(f) => t
                            .feature(f)
                            .is_some_and(|f| f.status == FeatureStatus::Done),
                        None => matches!(t.status, Status::Done | Status::Partial),
                    }
                }
            };
            map.comps[i].needs[k].met = met;
        }
    }
}

fn needed_by(map: &mut Map) {
    let mut edges = vec![];
    for (i, c) in map.comps.iter().enumerate() {
        for n in c.needs.iter().filter(|n| n.active) {
            if let Some(t) = n.target {
                edges.push((
                    t,
                    Edge {
                        from: i,
                        from_feature: n.from_feature.clone(),
                        on_feature: n.target_feature.clone(),
                        why: n.why.clone(),
                        soft: n.soft,
                    },
                ));
            }
        }
    }
    for (t, e) in edges {
        map.comps[t].needed_by.push(e);
    }
}

/// The components component `i` hard-needs, each once, in label order.
pub fn hard_targets(c: &Comp) -> Vec<usize> {
    let mut v: Vec<usize> = c
        .needs
        .iter()
        .filter(|n| n.hard())
        .filter_map(|n| n.target)
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Goals, and whatever a wanted component hard-needs, through any number
/// of steps.
fn wanted(map: &mut Map) {
    let mut queue: VecDeque<usize> = (0..map.comps.len())
        .filter(|&i| map.comps[i].intent == Intent::Goal)
        .collect();
    for &i in &queue {
        map.comps[i].wanted = true;
    }
    while let Some(i) = queue.pop_front() {
        for t in hard_targets(&map.comps[i]) {
            if !map.comps[t].wanted {
                map.comps[t].wanted = true;
                queue.push_back(t);
            }
        }
    }
}

/// For each wanted component that isn't a goal, the shortest chains of
/// hard needs from a goal down to it, goal first.
fn why_paths(map: &mut Map) {
    let n = map.comps.len();
    let targets: Vec<Vec<usize>> = map.comps.iter().map(hard_targets).collect();
    let mut up: Vec<Vec<usize>> = vec![vec![]; n];
    for (i, ts) in targets.iter().enumerate() {
        for &t in ts {
            if t != i {
                up[t].push(i);
            }
        }
    }
    // The distance from the nearest goal, going down.
    let mut dist = vec![usize::MAX; n];
    let mut queue = VecDeque::new();
    for (i, c) in map.comps.iter().enumerate() {
        if c.intent == Intent::Goal {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        for &t in &targets[i] {
            if dist[t] == usize::MAX {
                dist[t] = dist[i] + 1;
                queue.push_back(t);
            }
        }
    }
    for t in 0..n {
        if !map.comps[t].wanted || map.comps[t].intent == Intent::Goal || dist[t] == usize::MAX {
            continue;
        }
        map.comps[t].why_paths =
            k_shortest_up(t, &up, &dist, |i| map.comps[i].intent == Intent::Goal);
    }
}

/// Walks up from `t` toward the goals, best first. `dist` is each
/// component's distance from the nearest goal, a bound that never
/// overestimates, so complete paths come out shortest first.
fn k_shortest_up(
    t: usize,
    up: &[Vec<usize>],
    dist: &[usize],
    goal: impl Fn(usize) -> bool,
) -> Vec<Vec<usize>> {
    let mut found = vec![];
    // (length so far + distance left, partial path from t upward)
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((dist[t], vec![t])));
    let mut budget = 100_000;
    while let Some(Reverse((_, path))) = heap.pop() {
        let top = *path.last().unwrap();
        if goal(top) {
            let mut p = path.clone();
            p.reverse();
            found.push(p);
            if found.len() == WHY_PATHS {
                break;
            }
            continue;
        }
        for &u in &up[top] {
            if dist[u] == usize::MAX || path.contains(&u) {
                continue;
            }
            budget -= 1;
            if budget == 0 {
                return found;
            }
            let mut p = path.clone();
            p.push(u);
            heap.push(Reverse((p.len() - 1 + dist[u], p)));
        }
    }
    found
}

fn work(map: &mut Map) {
    let mut items = vec![];
    for (i, c) in map.comps.iter().enumerate() {
        let comp_blockers: Vec<String> = c
            .needs
            .iter()
            .filter(|n| n.from_feature.is_none() && n.blocks())
            .filter_map(|n| n.on.clone())
            .collect();
        let mut push = |feature: Option<String>,
                        title: String,
                        status: &'static str,
                        milestone: Option<String>| {
            let mut blocked_by: Vec<String> = c
                .needs
                .iter()
                .filter(|n| n.from_feature.is_some() && n.from_feature == feature && n.blocks())
                .filter_map(|n| n.on.clone())
                .collect();
            for b in &comp_blockers {
                if !blocked_by.contains(b) {
                    blocked_by.push(b.clone());
                }
            }
            let label = match &feature {
                Some(f) => format!("{}#{f}", c.label()),
                None => c.label(),
            };
            items.push(Work {
                comp: i,
                feature,
                label,
                title,
                status,
                milestone,
                wanted: c.wanted,
                ready: c.wanted && blocked_by.is_empty(),
                blocked_by,
            });
        };
        if c.features.is_empty() {
            if !matches!(c.status, Status::Done | Status::Dropped) {
                push(
                    None,
                    c.title.clone(),
                    c.status.as_str(),
                    c.milestone.clone(),
                );
            }
            continue;
        }
        for f in &c.features {
            if !matches!(f.status, FeatureStatus::Done | FeatureStatus::Dropped) {
                let milestone = f.milestone.clone().or_else(|| c.milestone.clone());
                push(
                    Some(f.id.clone()),
                    f.title.clone(),
                    f.status.as_str(),
                    milestone,
                );
            }
        }
    }
    map.work = items;
}

/// Orphans, wanted aspirations and cycles.
fn graph_lint(map: &mut Map) {
    for (i, c) in map.comps.iter().enumerate() {
        if c.broken {
            continue;
        }
        let orphan = c.needed_by.iter().all(|e| e.from == i);
        if c.intent == Intent::Induced && c.status != Status::Dropped && orphan {
            map.lint.warn(&c.file, "induced, but nothing needs it");
        }
        if c.intent == Intent::Aspiration && c.wanted {
            let mut by: Vec<String> = c
                .needed_by
                .iter()
                .filter(|e| !e.soft && map.comps[e.from].wanted && e.from != i)
                .map(|e| map.comps[e.from].label())
                .collect();
            by.sort();
            by.dedup();
            map.lint.warn(
                &c.file,
                format!(
                    "an aspiration, but wanted: {} needs it; make it induced or a goal",
                    by.join(", ")
                ),
            );
        }
    }
    for cycle in cycles(map) {
        let mut labels: Vec<String> = cycle.iter().map(|&i| map.comps[i].label()).collect();
        labels.push(labels[0].clone());
        let file = map.comps[cycle[0]].file.clone();
        map.lint
            .warn(&file, format!("dependency cycle: {}", labels.join(" -> ")));
    }
}

/// One shortest cycle through each knot of hard needs, starting at its
/// first component in label order.
pub fn cycles(map: &Map) -> Vec<Vec<usize>> {
    let n = map.comps.len();
    let targets: Vec<Vec<usize>> = map
        .comps
        .iter()
        .enumerate()
        .map(|(i, c)| hard_targets(c).into_iter().filter(|&t| t != i).collect())
        .collect();
    let mut out = vec![];
    for scc in tarjan(&targets) {
        if scc.len() < 2 {
            continue;
        }
        let start = *scc.iter().min().unwrap();
        let mut inside = vec![false; n];
        for &i in &scc {
            inside[i] = true;
        }
        // Breadth first from start, back to start.
        let mut prev = vec![usize::MAX; n];
        let mut queue = VecDeque::from([start]);
        let mut end = None;
        'search: while let Some(i) = queue.pop_front() {
            for &t in &targets[i] {
                if !inside[t] {
                    continue;
                }
                if t == start {
                    end = Some(i);
                    break 'search;
                }
                if prev[t] == usize::MAX {
                    prev[t] = i;
                    queue.push_back(t);
                }
            }
        }
        let Some(mut i) = end else { continue };
        let mut cycle = vec![];
        while i != start {
            cycle.push(i);
            i = prev[i];
        }
        cycle.push(start);
        cycle.reverse();
        out.push(cycle);
    }
    out.sort();
    out
}

/// Strongly connected components, iteratively.
fn tarjan(g: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = g.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack = vec![];
    let mut out = vec![];
    let mut next = 0;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut call = vec![(root, 0usize)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut k)) = call.last_mut() {
            if *k < g[v].len() {
                let w = g[v][*k];
                *k += 1;
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            call.pop();
            if let Some(&(parent, _)) = call.last() {
                low[parent] = low[parent].min(low[v]);
            }
            if low[v] == index[v] {
                let mut scc = vec![];
                loop {
                    let w = stack.pop().unwrap();
                    on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                scc.sort();
                out.push(scc);
            }
        }
    }
    out
}
