//! Checks todography against the small map in tests/fixtures/map. Its mistakes
//! are on purpose: each one should come out as one lint line.

use std::path::{Path, PathBuf};
use std::process::Command;
use todography::Map;
use todography::map::{Comp, Need, Status, Work};
use todography::{json, query};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/map")
}

fn load() -> Map {
    Map::load(&dir())
}

fn comp<'a>(m: &'a Map, path: &str) -> &'a Comp {
    &m.comps[m.by_path[path]]
}

fn need<'a>(c: &'a Comp, on: &str) -> &'a Need {
    c.needs
        .iter()
        .find(|n| n.on.as_deref() == Some(on))
        .unwrap_or_else(|| panic!("{} has no need on {on}", c.label()))
}

fn work<'a>(m: &'a Map, label: &str) -> &'a Work {
    m.work
        .iter()
        .find(|w| w.label == label)
        .unwrap_or_else(|| panic!("no work item {label}"))
}

fn errors(m: &Map, file: &str) -> Vec<String> {
    m.lint
        .errors
        .iter()
        .filter(|d| d.file == file)
        .map(|d| d.msg.clone())
        .collect()
}

fn warnings(m: &Map, file: &str) -> Vec<String> {
    m.lint
        .warnings
        .iter()
        .filter(|d| d.file == file)
        .map(|d| d.msg.clone())
        .collect()
}

#[test]
fn reads_everything() {
    let m = load();
    assert_eq!(m.title, "fixture");
    assert_eq!(m.bands.len(), 4);
    assert_eq!(m.milestones[0].refs, ["docs/m0.md"]);
    let areas: Vec<_> = m.areas.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(areas, ["apps", "exec", "lib", "misc", "platform", "stray"]);
    assert!(!m.by_path.contains_key("README"));
    let e = comp(&m, "apps/editor");
    assert_eq!(e.area, "apps");
    assert_eq!(e.slug, "editor");
    assert_eq!(e.features.len(), 5);
    assert!(e.body.contains("</script>"));
    assert_eq!(e.features[2].needs, ["//lib/regex:engine"]);
    // A relative label comes out absolute.
    assert_eq!(
        comp(&m, "lib/regex").features[1].needs,
        ["//lib/regex:engine"]
    );
}

#[test]
fn status() {
    let m = load();
    // Features done and todo, and a dropped one that doesn't count.
    assert_eq!(comp(&m, "apps/editor").status, Status::Partial);
    // Features wip and todo.
    assert_eq!(comp(&m, "lib/regex").status, Status::Wip);
    // No features: the field, else todo.
    assert_eq!(comp(&m, "exec/qio").status, Status::Partial);
    assert_eq!(comp(&m, "apps/mail").status, Status::Todo);
    assert_eq!(comp(&m, "platform/sel4").status, Status::Done);
    assert!(comp(&m, "platform/sel4").status_field.is_some());
    assert!(comp(&m, "apps/editor").status_field.is_none());
}

#[test]
fn met_and_blocked() {
    let m = load();
    let e = comp(&m, "apps/editor");
    // A whole component that is partial is usable.
    assert!(need(e, "//exec/qio").met);
    // A feature that is only wip isn't done.
    assert!(!need(e, "//lib/regex:engine").met);
    let fuzzy = e.needs.iter().find(|n| n.fuzzy.is_some()).unwrap();
    assert!(!fuzzy.met);
    assert!(need(comp(&m, "apps/mail"), "//exec/timers:abs").blocks());

    // A feature's needs block it, not its siblings.
    assert_eq!(
        work(&m, "//apps/editor:search").blocked_by,
        ["//lib/regex:engine"]
    );
    assert!(!work(&m, "//apps/editor:search").ready);
    assert!(work(&m, "//apps/editor:undo").ready);
    // A component-level need that is unmet blocks a component's own work.
    assert_eq!(work(&m, "//apps/mail").blocked_by, ["//exec/timers:abs"]);
    // Fuzzy needs never block.
    assert!(work(&m, "//apps/editor:undo").blocked_by.is_empty());
    // A soft need never blocks: QIO's on gfx, which is todo.
    assert!(need(comp(&m, "exec/qio"), "//lib/gfx").soft);
    assert!(work(&m, "//exec/qio").ready);
    // CAD's need on gfx is hard.
    assert_eq!(work(&m, "//apps/cad").blocked_by, ["//lib/gfx"]);
    // Done and dropped features aren't work.
    assert!(
        m.work
            .iter()
            .all(|w| w.label != "//apps/editor:basic" && w.label != "//apps/editor:macros")
    );
    // A component that is done isn't work.
    assert!(m.work.iter().all(|w| w.label != "//platform/sel4"));
    // A feature needing a feature of its own component.
    assert_eq!(
        work(&m, "//lib/regex:unicode").blocked_by,
        ["//lib/regex:engine"]
    );
}

#[test]
fn wanted() {
    let m = load();
    for p in [
        "apps/editor",
        "apps/mail",
        "lib/regex",
        "exec/qio",
        "exec/timers",
        "platform/sel4",
    ] {
        assert!(comp(&m, p).wanted, "{p} should be wanted");
    }
    // An aspiration nothing wanted needs, and what only it needs.
    assert!(!comp(&m, "apps/cad").wanted);
    // Needed by a goal's dropped feature, by an aspiration, and softly.
    assert!(!comp(&m, "lib/gfx").wanted);
    assert!(!comp(&m, "lib/unused").wanted);
    // Unwanted work is never ready.
    assert!(!work(&m, "//lib/unused").ready);
}

#[test]
fn why_paths() {
    let m = load();
    let labels = |c: &Comp| -> Vec<Vec<String>> {
        c.why_paths
            .iter()
            .map(|p| p.iter().map(|&i| m.comps[i].label()).collect())
            .collect()
    };
    assert_eq!(
        labels(comp(&m, "lib/regex")),
        [["//apps/editor", "//lib/regex"]]
    );
    // Shortest first, from either goal.
    assert_eq!(
        labels(comp(&m, "exec/timers")),
        [
            vec!["//apps/mail", "//exec/timers"],
            vec!["//apps/editor", "//exec/qio", "//exec/timers"]
        ]
    );
    assert_eq!(
        labels(comp(&m, "platform/sel4")),
        [
            vec!["//apps/editor", "//exec/qio", "//platform/sel4"],
            vec![
                "//apps/mail",
                "//exec/timers",
                "//exec/qio",
                "//platform/sel4"
            ],
        ]
    );
    assert!(comp(&m, "apps/editor").why_paths.is_empty());
    assert!(comp(&m, "lib/gfx").why_paths.is_empty());
}

#[test]
fn debt() {
    let m = load();
    let debt: Vec<_> = m
        .comps
        .iter()
        .filter(|c| c.debt)
        .map(|c| c.label())
        .collect();
    // The editor has a fuzzy need, regex is deferred. CAD has a fuzzy need
    // but isn't wanted; gfx is deferred but isn't wanted.
    assert_eq!(debt, ["//apps/editor", "//lib/regex"]);
}

#[test]
fn needed_by() {
    let m = load();
    let t = comp(&m, "exec/timers");
    let by: Vec<_> = t
        .needed_by
        .iter()
        .map(|e| (m.comps[e.from].label(), e.on_feature.clone()))
        .collect();
    assert_eq!(
        by,
        [
            ("//apps/mail".to_string(), Some("abs".to_string())),
            ("//exec/qio".to_string(), None)
        ]
    );
    let out = query::rdeps(&m, m.by_path["lib/regex"], Some("engine"), false);
    assert_eq!(
        out,
        "//apps/editor:search -> //lib/regex:engine\n//lib/regex:unicode -> //lib/regex:engine\n"
    );
}

#[test]
fn lint_errors() {
    let m = load();
    let game = errors(&m, "map/apps/game.md");
    assert_eq!(game.len(), 2);
    assert!(
        game[0].contains("unresolved label //lib/regx")
            && game[0].contains("did you mean //lib/regex?")
    );
    assert!(game[1].contains("no feature :engin") && game[1].contains("did you mean :engine?"));

    assert!(errors(&m, "map/platform/broken.md")[0].contains("unknown field `colour`"));
    assert!(errors(&m, "map/platform/broken.md")[0].starts_with("line 8:"));
    assert!(errors(&m, "map/platform/badenum.md")[0].contains("unknown variant `finished`"));

    let dupes = errors(&m, "map/platform/dupes.md");
    for want in [
        "unknown milestone \"m7\"",
        "status is set",
        "duplicate feature id \"a\"",
        "feature a: unknown milestone \"m8\"",
    ] {
        assert!(
            dupes.iter().any(|e| e.contains(want)),
            "dupes.md: no error with {want}: {dupes:?}"
        );
    }
    let bad = errors(&m, "map/platform/badneeds.md");
    for want in [
        "both on and fuzzy",
        "neither on nor fuzzy",
        "bad feature id \"Bad_Id\"",
        "can't need itself",
        "bad label \"platform/sel4\"",
        "bad label \":Bad_Id\"",
    ] {
        assert!(
            bad.iter().any(|e| e.contains(want)),
            "badneeds.md: no error with {want}: {bad:?}"
        );
    }
    assert_eq!(errors(&m, "map/stray"), ["directory without AREA.md"]);
    assert_eq!(
        errors(&m, "map/misc/AREA.md"),
        ["missing summary", "unknown band \"nowhere\""]
    );
    assert_eq!(errors(&m, "map/misc/notitle.md"), ["missing title"]);
    assert!(errors(&m, "map/misc/nofront.md")[0].contains("no front matter"));
    assert!(errors(&m, "map/misc/unclosed.md")[0].contains("no closing +++"));
    // Nothing else.
    assert_eq!(m.lint.errors.len(), 20, "{:#?}", m.lint.errors);
}

#[test]
fn lint_warnings() {
    let m = load();
    assert_eq!(
        warnings(&m, "map/lib/gfx.md"),
        ["research is missing; taken as deferred"]
    );
    assert_eq!(
        warnings(&m, "map/lib/unused.md"),
        ["induced, but nothing needs it"]
    );
    assert!(
        warnings(&m, "map/platform/sel4.md")[0]
            .contains("an aspiration, but wanted: //exec/qio needs it")
    );
    assert_eq!(
        warnings(&m, "map/exec/qio.md"),
        ["dependency cycle: //exec/qio -> //exec/timers -> //exec/qio"]
    );
    assert_eq!(
        warnings(&m, "map/platform/dupes.md"),
        ["summary is 105 characters; keep it to 100"]
    );
    // A feature needing its own component's feature is no cycle.
    assert!(warnings(&m, "map/lib/regex.md").is_empty());
    assert_eq!(m.lint.warnings.len(), 7, "{:#?}", m.lint.warnings);
}

#[test]
fn next_order() {
    let m = load();
    let ready: Vec<_> = query::next_items(&m, false, None)
        .iter()
        .map(|w| w.label.clone())
        .collect();
    // m0 first; within it the apps band, then lib, then exec; then label.
    assert_eq!(
        ready,
        [
            "//apps/editor:undo",
            "//lib/regex:engine",
            "//exec/qio",
            "//exec/timers:abs"
        ]
    );
    let m1: Vec<_> = query::next_items(&m, true, Some("m1"))
        .iter()
        .map(|w| w.label.clone())
        .collect();
    assert_eq!(m1, ["//apps/editor:search", "//apps/mail"]);
    let all = query::next_items(&m, true, None);
    assert_eq!(all.len(), 8);
    assert_eq!(all.last().unwrap().label, "//lib/regex:unicode");
}

#[test]
fn json_contract() {
    let m = load();
    let v: serde_json::Value = serde_json::from_str(&json::to_json(&m)).unwrap();
    for k in [
        "title",
        "generated",
        "bands",
        "milestones",
        "areas",
        "components",
        "work",
        "lint",
    ] {
        assert!(v.get(k).is_some(), "no {k}");
    }
    assert!(v["generated"].as_str().unwrap().ends_with('Z'));
    let area = &v["areas"][0];
    assert_eq!(area["id"], "apps");
    assert!(
        area["body_html"]
            .as_str()
            .unwrap()
            .contains("<strong>apps</strong>")
    );
    let comps = v["components"].as_array().unwrap();
    let e = comps
        .iter()
        .find(|c| c["label"] == "//apps/editor")
        .unwrap();
    for k in [
        "label",
        "area",
        "slug",
        "title",
        "summary",
        "aka",
        "kind",
        "intent",
        "research",
        "milestone",
        "refs",
        "status",
        "status_explicit",
        "body_html",
        "features",
        "needs",
        "needed_by",
        "wanted",
        "why_paths",
        "debt",
        "progress",
    ] {
        assert!(e.get(k).is_some(), "component has no {k}");
    }
    assert_eq!(e["status"], "partial");
    assert_eq!(e["status_explicit"], false);
    assert_eq!(e["milestone"], serde_json::Value::Null);
    assert_eq!(e["progress"], serde_json::json!({"done": 1, "total": 4}));
    assert_eq!(e["features"][2]["label"], "//apps/editor:search");
    assert_eq!(
        e["features"][2]["needs"],
        serde_json::json!(["//lib/regex:engine"])
    );
    let search = e["needs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["from_feature"] == "search")
        .unwrap();
    assert_eq!(
        *search,
        serde_json::json!({"on": "//lib/regex:engine", "fuzzy": null, "why": null, "soft": false,
            "from_feature": "search", "target": "//lib/regex", "target_feature": "engine", "met": false})
    );
    let t = comps
        .iter()
        .find(|c| c["label"] == "//exec/timers")
        .unwrap();
    assert_eq!(
        t["needed_by"][0],
        serde_json::json!({"from": "//apps/mail", "from_feature": null, "on_feature": "abs",
            "why": "Sends at a set time", "soft": false})
    );
    assert_eq!(
        t["why_paths"][0],
        serde_json::json!(["//apps/mail", "//exec/timers"])
    );
    let w = v["work"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["label"] == "//apps/mail")
        .unwrap();
    assert_eq!(
        *w,
        serde_json::json!({"label": "//apps/mail", "component": "//apps/mail", "feature": null, "title": "Mail",
            "status": "todo", "milestone": "m1", "wanted": true, "ready": false, "blocked_by": ["//exec/timers:abs"]})
    );
    assert_eq!(v["lint"]["errors"].as_array().unwrap().len(), 20);
    assert!(
        v["lint"]["errors"][0].get("file").is_some() && v["lint"]["errors"][0].get("msg").is_some()
    );
}

#[test]
fn page() {
    let m = load();
    let html = json::page(&m);
    assert!(!html.contains("__MAP_DATA__"));
    assert!(html.contains("const MAP = {"));
    // The editor's body says </script>; only the template's own is left.
    assert_eq!(html.matches("</script>").count(), 1);
    assert!(html.contains("<\\/script>"));
}

fn todography(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_todography"))
        .args(["--map", dir().to_str().unwrap()])
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
    )
}

#[test]
fn cli() {
    let (code, out) = todography(&["lint"]);
    assert_eq!(code, 1);
    assert!(out.starts_with("map/apps/game.md: error: unresolved label //lib/regx"));
    assert!(out.ends_with("20 errors, 7 warnings, 18 components, research debt: 2\n"));

    let (code, out) = todography(&["show", "//exec/timers"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("//exec/timers  Timers\n"));
    assert!(out.contains("needed by:\n  //apps/mail -> :abs  Sends at a set time\n"));
    assert!(out.contains("why:\n  //apps/mail -> //exec/timers\n"));

    let (_, out) = todography(&["why", "lib/regex"]);
    assert_eq!(out, "//lib/regex  Regex\n  //apps/editor -> //lib/regex\n");

    let (_, out) = todography(&["rdeps", "--all", "//platform/sel4"]);
    assert!(out.starts_with("//exec/qio -> //platform/sel4\n"));
    assert!(out.contains("  //exec/timers -> //exec/qio\n"));

    let (_, out) = todography(&["next"]);
    assert!(out.starts_with("m0: Milestone 0\n  //apps/editor:undo"));
    assert!(out.ends_with("4 ready\n"));

    let (_, out) = todography(&["debt"]);
    assert!(out.contains(
        "//apps/editor  research done\n    fuzzy: a terminal driver  (Draws the screen)\n"
    ));

    let (code, _) = todography(&["show", "//no/such"]);
    assert_eq!(code, 2);
    let (code, _) = todography(&["frobnicate"]);
    assert_eq!(code, 2);
}
