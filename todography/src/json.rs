//! The map as JSON, for `todography json` and the page `todography build` writes. The
//! page reads exactly these fields, so change them together with it.

use crate::map::{Diag, FeatureStatus, Map, Status};
use pulldown_cmark::{Options, Parser, html};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct Doc<'a> {
    title: &'a str,
    generated: String,
    bands: Vec<Band<'a>>,
    milestones: Vec<Milestone<'a>>,
    areas: Vec<Area<'a>>,
    components: Vec<Component<'a>>,
    work: Vec<Work<'a>>,
    lint: Lint,
}

#[derive(Serialize)]
struct Band<'a> {
    id: &'a str,
    title: &'a str,
}

#[derive(Serialize)]
struct Milestone<'a> {
    id: &'a str,
    title: &'a str,
    summary: &'a str,
    refs: &'a [String],
}

#[derive(Serialize)]
struct Area<'a> {
    id: &'a str,
    title: &'a str,
    band: &'a str,
    summary: &'a str,
    body_html: String,
}

#[derive(Serialize)]
struct Component<'a> {
    label: String,
    area: &'a str,
    slug: &'a str,
    title: &'a str,
    summary: &'a str,
    aka: &'a [String],
    kind: &'static str,
    intent: &'static str,
    research: &'static str,
    milestone: Option<&'a str>,
    refs: &'a [String],
    status: &'static str,
    status_explicit: bool,
    body_html: String,
    features: Vec<Feature<'a>>,
    needs: Vec<Need<'a>>,
    needed_by: Vec<NeededBy<'a>>,
    wanted: bool,
    why_paths: Vec<Vec<String>>,
    debt: bool,
    progress: Progress,
}

#[derive(Serialize)]
struct Feature<'a> {
    id: &'a str,
    label: String,
    title: &'a str,
    status: &'static str,
    milestone: Option<&'a str>,
    note: Option<&'a str>,
    refs: &'a [String],
    needs: &'a [String],
}

#[derive(Serialize)]
struct Need<'a> {
    on: Option<&'a str>,
    fuzzy: Option<&'a str>,
    why: Option<&'a str>,
    soft: bool,
    from_feature: Option<&'a str>,
    target: Option<String>,
    target_feature: Option<&'a str>,
    met: bool,
}

#[derive(Serialize)]
struct NeededBy<'a> {
    from: String,
    from_feature: Option<&'a str>,
    on_feature: Option<&'a str>,
    why: Option<&'a str>,
    soft: bool,
}

#[derive(Serialize)]
struct Progress {
    done: usize,
    total: usize,
}

#[derive(Serialize)]
struct Work<'a> {
    label: &'a str,
    component: String,
    feature: Option<&'a str>,
    title: &'a str,
    status: &'static str,
    milestone: Option<&'a str>,
    wanted: bool,
    ready: bool,
    blocked_by: &'a [String],
}

#[derive(Serialize)]
struct Lint {
    errors: Vec<FileMsg>,
    warnings: Vec<FileMsg>,
}

#[derive(Serialize)]
struct FileMsg {
    file: String,
    msg: String,
}

/// Markdown to HTML, as the page shows a component's or an area's body.
pub fn markdown(src: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(src, opts));
    out
}

/// Now, or `SOURCE_DATE_EPOCH` if set, as an ISO 8601 UTC timestamp.
pub fn now_iso() -> String {
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

fn diags(v: &[Diag]) -> Vec<FileMsg> {
    v.iter()
        .map(|d| FileMsg {
            file: d.file.clone(),
            msg: d.msg.clone(),
        })
        .collect()
}

pub fn to_json(map: &Map) -> String {
    let label = |i: usize| map.comps[i].label();
    let components = map
        .comps
        .iter()
        .map(|c| {
            let live = c
                .features
                .iter()
                .filter(|f| f.status != FeatureStatus::Dropped);
            let progress = if c.features.is_empty() {
                Progress {
                    done: usize::from(c.status == Status::Done),
                    total: usize::from(c.status != Status::Dropped),
                }
            } else {
                Progress {
                    done: live
                        .clone()
                        .filter(|f| f.status == FeatureStatus::Done)
                        .count(),
                    total: live.count(),
                }
            };
            Component {
                label: c.label(),
                area: &c.area,
                slug: &c.slug,
                title: &c.title,
                summary: &c.summary,
                aka: &c.aka,
                kind: c.kind.as_str(),
                intent: c.intent.as_str(),
                research: c.research.as_str(),
                milestone: c.milestone.as_deref(),
                refs: &c.refs,
                status: c.status.as_str(),
                status_explicit: c.status_field.is_some(),
                body_html: markdown(&c.body),
                features: c
                    .features
                    .iter()
                    .map(|f| Feature {
                        id: &f.id,
                        label: format!("{}#{}", c.label(), f.id),
                        title: &f.title,
                        status: f.status.as_str(),
                        milestone: f.milestone.as_deref(),
                        note: f.note.as_deref(),
                        refs: &f.refs,
                        needs: &f.needs,
                    })
                    .collect(),
                needs: c
                    .needs
                    .iter()
                    .map(|n| Need {
                        on: n.on.as_deref(),
                        fuzzy: n.fuzzy.as_deref(),
                        why: n.why.as_deref(),
                        soft: n.soft,
                        from_feature: n.from_feature.as_deref(),
                        target: n.target.map(label),
                        target_feature: n.target_feature.as_deref(),
                        met: n.met,
                    })
                    .collect(),
                needed_by: c
                    .needed_by
                    .iter()
                    .map(|e| NeededBy {
                        from: label(e.from),
                        from_feature: e.from_feature.as_deref(),
                        on_feature: e.on_feature.as_deref(),
                        why: e.why.as_deref(),
                        soft: e.soft,
                    })
                    .collect(),
                wanted: c.wanted,
                why_paths: c
                    .why_paths
                    .iter()
                    .map(|p| p.iter().map(|&i| label(i)).collect())
                    .collect(),
                debt: c.debt,
                progress,
            }
        })
        .collect();
    let doc = Doc {
        title: &map.title,
        generated: now_iso(),
        bands: map
            .bands
            .iter()
            .map(|b| Band {
                id: &b.id,
                title: &b.title,
            })
            .collect(),
        milestones: map
            .milestones
            .iter()
            .map(|m| Milestone {
                id: &m.id,
                title: &m.title,
                summary: &m.summary,
                refs: &m.refs,
            })
            .collect(),
        areas: map
            .areas
            .iter()
            .map(|a| Area {
                id: &a.id,
                title: &a.title,
                band: &a.band,
                summary: &a.summary,
                body_html: markdown(&a.body),
            })
            .collect(),
        components,
        work: map
            .work
            .iter()
            .map(|w| Work {
                label: &w.label,
                component: label(w.comp),
                feature: w.feature.as_deref(),
                title: &w.title,
                status: w.status,
                milestone: w.milestone.as_deref(),
                wanted: w.wanted,
                ready: w.ready,
                blocked_by: &w.blocked_by,
            })
            .collect(),
        lint: Lint {
            errors: diags(&map.lint.errors),
            warnings: diags(&map.lint.warnings),
        },
    };
    serde_json::to_string_pretty(&doc).expect("the map serializes")
}

/// The page: the template with the JSON in place of `__MAP_DATA__`. In
/// strings, `</` becomes `<\/` and `<!--` becomes `<!--`, so nothing in
/// the data can end the script or open an HTML comment in it.
pub fn page(map: &Map) -> String {
    const TEMPLATE: &str = include_str!("../assets/map.html");
    let data = to_json(map)
        .replace("</", "<\\/")
        .replace("<!--", "\\u003c!--");
    TEMPLATE.replacen("__MAP_DATA__", &data, 1)
}
