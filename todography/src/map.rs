//! Reads map/: MAP.toml, each area's AREA.md and each component's file.
//! Whatever is wrong goes into the lint report; loading itself never fails.

use crate::label::{Label, is_slug};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// An enum of TOML strings, with its names for output.
macro_rules! names {
    ($name:ident { $($var:ident = $s:literal),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $s)] $var),*
        }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$var => $s),*
                }
            }
        }
    };
}

names!(Kind { Core = "core", Layered = "layered", Glue = "glue", Platform = "platform", Tool = "tool" });
names!(Intent { Goal = "goal", Aspiration = "aspiration", Induced = "induced" });
names!(Status { Done = "done", Partial = "partial", Wip = "wip", Todo = "todo", Dropped = "dropped" });
names!(FeatureStatus { Done = "done", Wip = "wip", Todo = "todo", Dropped = "dropped" });
names!(Research { Done = "done", Outline = "outline", Deferred = "deferred" });

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub refs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapFile {
    title: Option<String>,
    #[serde(default)]
    band: Vec<Band>,
    #[serde(default)]
    milestone: Vec<Milestone>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArea {
    title: Option<String>,
    band: Option<String>,
    summary: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawComp {
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    aka: Vec<String>,
    kind: Option<Kind>,
    intent: Option<Intent>,
    status: Option<Status>,
    research: Option<Research>,
    milestone: Option<String>,
    #[serde(default)]
    refs: Vec<String>,
    #[serde(default)]
    feature: Vec<RawFeature>,
    #[serde(default)]
    needs: Vec<RawNeed>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFeature {
    id: String,
    title: Option<String>,
    status: Option<FeatureStatus>,
    milestone: Option<String>,
    note: Option<String>,
    #[serde(default)]
    refs: Vec<String>,
    #[serde(default)]
    needs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNeed {
    on: Option<String>,
    fuzzy: Option<String>,
    why: Option<String>,
    #[serde(default)]
    soft: bool,
}

/// One line of the lint report.
#[derive(Clone, Debug)]
pub struct Diag {
    pub file: String,
    pub msg: String,
}

#[derive(Default, Debug)]
pub struct Lint {
    pub errors: Vec<Diag>,
    pub warnings: Vec<Diag>,
}

impl Lint {
    pub fn error(&mut self, file: &str, msg: impl Into<String>) {
        self.errors.push(Diag {
            file: file.to_string(),
            msg: msg.into(),
        });
    }
    pub fn warn(&mut self, file: &str, msg: impl Into<String>) {
        self.warnings.push(Diag {
            file: file.to_string(),
            msg: msg.into(),
        });
    }
}

#[derive(Clone, Debug)]
pub struct Area {
    /// The directory's path under map/, such as `exec`.
    pub id: String,
    pub file: String,
    pub title: String,
    pub band: String,
    pub summary: String,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub title: String,
    pub status: FeatureStatus,
    pub milestone: Option<String>,
    pub note: Option<String>,
    pub refs: Vec<String>,
    /// The labels it needs, absolute where they parse.
    pub needs: Vec<String>,
}

/// A dependency: a component's `[[needs]]` entry, or one label of a
/// feature's `needs` (then `from_feature` is set).
#[derive(Clone, Debug)]
pub struct Need {
    /// The label, absolute once resolved; as written if it doesn't parse.
    pub on: Option<String>,
    pub fuzzy: Option<String>,
    pub why: Option<String>,
    pub soft: bool,
    pub from_feature: Option<String>,
    /// The component it points at, once resolved.
    pub target: Option<usize>,
    pub target_feature: Option<String>,
    pub met: bool,
    /// False for the needs of a dropped feature: they pull nothing in and
    /// block nothing.
    pub active: bool,
}

impl Need {
    /// A need that pulls its target in and can block.
    pub fn hard(&self) -> bool {
        self.on.is_some() && !self.soft && self.active
    }
    pub fn blocks(&self) -> bool {
        self.hard() && !self.met
    }
}

/// A reverse edge: who needs this component.
#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub from_feature: Option<String>,
    pub on_feature: Option<String>,
    pub why: Option<String>,
    pub soft: bool,
}

#[derive(Clone, Debug)]
pub struct Comp {
    /// The file's path under map/ without `.md`, such as `exec/mailboxes`.
    pub path: String,
    pub file: String,
    pub area: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub aka: Vec<String>,
    pub kind: Kind,
    pub intent: Intent,
    pub research: Research,
    pub milestone: Option<String>,
    pub refs: Vec<String>,
    /// The `status` field; with features the status comes from them.
    pub status_field: Option<Status>,
    pub status: Status,
    pub body: String,
    /// Its front matter didn't load: labels into it aren't checked.
    pub broken: bool,
    pub features: Vec<Feature>,
    pub needs: Vec<Need>,
    pub needed_by: Vec<Edge>,
    pub wanted: bool,
    pub why_paths: Vec<Vec<usize>>,
    pub debt: bool,
}

impl Comp {
    pub fn label(&self) -> String {
        format!("t8y:/{}", self.path)
    }
    pub fn feature(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }
}

/// A unit of work: an unfinished feature, or an unfinished component
/// without features.
#[derive(Clone, Debug)]
pub struct Work {
    pub comp: usize,
    pub feature: Option<String>,
    pub label: String,
    pub title: String,
    pub status: &'static str,
    /// The feature's milestone, else its component's.
    pub milestone: Option<String>,
    pub wanted: bool,
    pub ready: bool,
    pub blocked_by: Vec<String>,
}

#[derive(Debug)]
pub struct Map {
    /// The map directory, holding MAP.toml.
    pub dir: PathBuf,
    /// The directory that holds the map directory: the workspace root.
    pub root: PathBuf,
    pub title: String,
    pub bands: Vec<Band>,
    pub milestones: Vec<Milestone>,
    pub areas: Vec<Area>,
    pub comps: Vec<Comp>,
    pub by_path: HashMap<String, usize>,
    pub work: Vec<Work>,
    pub lint: Lint,
}

/// Splits a file into its front matter, between a first line `+++` and the
/// next line that is exactly `+++`, and the Markdown body after it.
pub fn split_front(text: &str) -> Result<(&str, &str), String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or("");
    if first.trim_end_matches(['\r', '\n']) != "+++" {
        return Err("no front matter: the first line must be +++".into());
    }
    let start = first.len();
    let mut pos = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "+++" {
            return Ok((&text[start..pos], &text[pos + line.len()..]));
        }
        pos += line.len();
    }
    Err("the front matter has no closing +++ line".into())
}

/// Parses TOML that starts on line `first_line` of its file, and words the
/// error with the file's line number.
fn parse_toml<T: for<'de> Deserialize<'de>>(src: &str, first_line: usize) -> Result<T, String> {
    toml::from_str(src).map_err(|e| match e.span() {
        Some(span) => {
            let line = first_line + src[..span.start.min(src.len())].matches('\n').count();
            format!("line {line}: {}", e.message().trim_end())
        }
        None => e.message().trim_end().to_string(),
    })
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.trim().is_empty())
}

/// The closest of `options` to `want`, if it is close enough to be a typo.
fn closest<'a>(want: &str, options: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (want.len() / 4).clamp(1, 3);
    options
        .map(|o| (distance(want, o), o))
        .filter(|&(d, _)| d <= limit)
        .min()
        .map(|(_, o)| o)
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

impl Map {
    /// Finds the map: `dir` itself or `dir/docs/map` if given, else, walking
    /// up from the current directory, the first directory that holds
    /// `MAP.toml` or `docs/map/MAP.toml`.
    pub fn find(dir: Option<&Path>) -> Result<PathBuf, String> {
        if let Some(dir) = dir {
            if dir.join("MAP.toml").is_file() || !dir.join("docs/map/MAP.toml").is_file() {
                return Ok(dir.to_path_buf());
            }
            return Ok(dir.join("docs/map"));
        }
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        for d in cwd.ancestors() {
            if d.join("MAP.toml").is_file() {
                return Ok(d.to_path_buf());
            }
            if d.join("docs/map/MAP.toml").is_file() {
                return Ok(d.join("docs/map"));
            }
        }
        Err("no docs/map/MAP.toml here or above; pass --map DIR".into())
    }

    /// Reads the map at `dir` and works out everything the map says.
    pub fn load(dir: &Path) -> Map {
        let dir = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        // The workspace root: above docs/ when the map is docs/map, else the
        // map's own parent.
        let root = match dir.parent() {
            Some(p) if p.file_name().is_some_and(|n| n == "docs") => {
                p.parent().unwrap_or(p).to_path_buf()
            }
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let mut map = Map {
            dir,
            root,
            title: String::new(),
            bands: vec![],
            milestones: vec![],
            areas: vec![],
            comps: vec![],
            by_path: HashMap::new(),
            work: vec![],
            lint: Lint::default(),
        };
        let mut raws = vec![];
        map.read_meta();
        map.read_dir("", &mut raws);
        map.comps.sort_by(|a, b| a.path.cmp(&b.path));
        raws.sort_by(|a: &(String, Option<RawComp>), b| a.0.cmp(&b.0));
        map.areas.sort_by(|a, b| a.id.cmp(&b.id));
        for (i, c) in map.comps.iter().enumerate() {
            map.by_path.insert(c.path.clone(), i);
        }
        for (i, (_, raw)) in raws.into_iter().enumerate() {
            if let Some(raw) = raw {
                map.fill(i, raw);
            }
        }
        map.resolve();
        crate::compute::compute(&mut map);
        // By file, keeping each file's own order.
        map.lint.errors.sort_by(|a, b| a.file.cmp(&b.file));
        map.lint.warnings.sort_by(|a, b| a.file.cmp(&b.file));
        map
    }

    /// The name of a file in messages: relative to the workspace root.
    fn display(&self, rel: &str) -> String {
        let name = self
            .dir
            .strip_prefix(&self.root)
            .ok()
            .and_then(|p| p.to_str())
            .filter(|p| !p.is_empty())
            .unwrap_or("map");
        if rel.is_empty() {
            format!("{name}/")
        } else {
            format!("{name}/{rel}")
        }
    }

    fn read_meta(&mut self) {
        let file = self.display("MAP.toml");
        let text = match fs::read_to_string(self.dir.join("MAP.toml")) {
            Ok(t) => t,
            Err(e) => {
                self.lint.error(&file, format!("can't read it: {e}"));
                return;
            }
        };
        match parse_toml::<MapFile>(&text, 1) {
            Ok(m) => {
                match nonempty(m.title) {
                    Some(t) => self.title = t,
                    None => self.lint.error(&file, "missing title"),
                }
                let mut seen = vec![];
                for id in m
                    .band
                    .iter()
                    .map(|b| &b.id)
                    .chain(m.milestone.iter().map(|m| &m.id))
                {
                    if !is_slug(id) {
                        self.lint.error(
                            &file,
                            format!("bad id \"{id}\": use lowercase letters, digits and -"),
                        );
                    }
                    if seen.contains(&id) {
                        self.lint.error(&file, format!("duplicate id \"{id}\""));
                    }
                    seen.push(id);
                }
                self.bands = m.band;
                self.milestones = m.milestone;
            }
            Err(e) => self.lint.error(&file, e),
        }
    }

    /// Reads the directory at `rel` under the map, and those below it.
    fn read_dir(&mut self, rel: &str, raws: &mut Vec<(String, Option<RawComp>)>) {
        let path = self.dir.join(rel);
        let mut entries: Vec<_> = match fs::read_dir(&path) {
            Ok(rd) => rd.filter_map(Result::ok).collect(),
            Err(e) => {
                self.lint
                    .error(&self.display(rel), format!("can't read the directory: {e}"));
                return;
            }
        };
        entries.sort_by_key(|e| e.file_name());
        if !rel.is_empty() {
            self.read_area(rel);
        }
        for e in entries {
            let Some(name) = e.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let sub = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if e.path().is_dir() {
                if !is_slug(&name) {
                    self.lint.error(
                        &self.display(&sub),
                        "bad directory name: use lowercase letters, digits and -",
                    );
                }
                self.read_dir(&sub, raws);
                continue;
            }
            let Some(stem) = name.strip_suffix(".md") else {
                continue;
            };
            if name == "README.md" || name == "AREA.md" {
                continue;
            }
            if rel.is_empty() {
                self.lint.error(
                    &self.display(&sub),
                    "a component must sit in an area's directory",
                );
                continue;
            }
            let raw = self.read_comp(rel, stem, &sub);
            raws.push((format!("{rel}/{stem}"), raw));
        }
    }

    fn read_area(&mut self, rel: &str) {
        let file = self.display(&format!("{rel}/AREA.md"));
        let mut area = Area {
            id: rel.to_string(),
            file: file.clone(),
            title: rel.to_string(),
            band: String::new(),
            summary: String::new(),
            body: String::new(),
        };
        match fs::read_to_string(self.dir.join(rel).join("AREA.md")) {
            Err(_) => self
                .lint
                .error(&self.display(rel), "directory without AREA.md"),
            Ok(text) => match split_front(&text) {
                Err(e) => self.lint.error(&file, e),
                Ok((front, body)) => {
                    area.body = body.to_string();
                    match parse_toml::<RawArea>(front, 2) {
                        Err(e) => self.lint.error(&file, e),
                        Ok(raw) => {
                            match nonempty(raw.title) {
                                Some(t) => area.title = t,
                                None => self.lint.error(&file, "missing title"),
                            }
                            match nonempty(raw.summary) {
                                Some(s) => area.summary = s,
                                None => self.lint.error(&file, "missing summary"),
                            }
                            self.check_summary(&file, &area.summary);
                            match raw.band {
                                None => self.lint.error(&file, "missing band"),
                                Some(b) => {
                                    if !self.bands.iter().any(|x| x.id == b) {
                                        self.lint.error(&file, format!("unknown band \"{b}\""));
                                    }
                                    area.band = b;
                                }
                            }
                        }
                    }
                }
            },
        }
        self.areas.push(area);
    }

    fn check_summary(&mut self, file: &str, summary: &str) {
        let n = summary.chars().count();
        if n > 100 {
            self.lint
                .warn(file, format!("summary is {n} characters; keep it to 100"));
        }
    }

    /// Reads a component's file. Its fields are filled in later, once every
    /// component is known; until then it has placeholders.
    fn read_comp(&mut self, area: &str, stem: &str, rel: &str) -> Option<RawComp> {
        let file = self.display(rel);
        let path = format!("{area}/{stem}");
        if !is_slug(stem) {
            self.lint.error(
                &file,
                "bad file name: use lowercase letters, digits and -, then .md",
            );
        }
        let mut comp = Comp {
            path: path.clone(),
            file: file.clone(),
            area: area.to_string(),
            slug: stem.to_string(),
            title: stem.to_string(),
            summary: String::new(),
            aka: vec![],
            kind: Kind::Core,
            intent: Intent::Induced,
            research: Research::Deferred,
            milestone: None,
            refs: vec![],
            status_field: None,
            status: Status::Todo,
            body: String::new(),
            broken: true,
            features: vec![],
            needs: vec![],
            needed_by: vec![],
            wanted: false,
            why_paths: vec![],
            debt: false,
        };
        let mut raw = None;
        match fs::read_to_string(self.dir.join(rel)) {
            Err(e) => self.lint.error(&file, format!("can't read it: {e}")),
            Ok(text) => match split_front(&text) {
                Err(e) => self.lint.error(&file, e),
                Ok((front, body)) => {
                    comp.body = body.to_string();
                    match parse_toml::<RawComp>(front, 2) {
                        Err(e) => self.lint.error(&file, e),
                        Ok(r) => {
                            comp.broken = false;
                            raw = Some(r);
                        }
                    }
                }
            },
        }
        self.comps.push(comp);
        raw
    }

    /// Fills component `i` from its front matter, and checks what can be
    /// checked within the file.
    fn fill(&mut self, i: usize, raw: RawComp) {
        let file = self.comps[i].file.clone();
        let mut lint = std::mem::take(&mut self.lint);
        let milestones: Vec<String> = self.milestones.iter().map(|m| m.id.clone()).collect();
        let check_milestone = |lint: &mut Lint, m: &Option<String>, what: &str| {
            if let Some(m) = m
                && !milestones.contains(m)
            {
                lint.error(&file, format!("{what}unknown milestone \"{m}\""));
            }
        };
        let c = &mut self.comps[i];
        match nonempty(raw.title) {
            Some(t) => c.title = t,
            None => lint.error(&file, "missing title"),
        }
        match nonempty(raw.summary) {
            Some(s) => c.summary = s,
            None => lint.error(&file, "missing summary"),
        }
        match raw.kind {
            Some(k) => c.kind = k,
            None => lint.error(&file, "missing kind"),
        }
        match raw.intent {
            Some(x) => c.intent = x,
            None => lint.error(&file, "missing intent"),
        }
        match raw.research {
            Some(r) => c.research = r,
            None => lint.warn(&file, "research is missing; taken as deferred"),
        }
        c.aka = raw.aka;
        c.refs = raw.refs;
        check_milestone(&mut lint, &raw.milestone, "");
        c.milestone = raw.milestone;
        c.status_field = raw.status;
        if raw.status.is_some() && !raw.feature.is_empty() {
            lint.error(
                &file,
                "status is set, but it has features: its status comes from them",
            );
        }
        for rn in raw.needs {
            match (&rn.on, &rn.fuzzy) {
                (Some(_), Some(_)) => lint.error(&file, "a need has both on and fuzzy: pick one"),
                (None, None) => lint.error(&file, "a need has neither on nor fuzzy"),
                _ => {}
            }
            c.needs.push(Need {
                on: rn.on,
                fuzzy: rn.fuzzy,
                why: rn.why,
                soft: rn.soft,
                from_feature: None,
                target: None,
                target_feature: None,
                met: false,
                active: true,
            });
        }
        for rf in raw.feature {
            let what = format!("feature {}: ", rf.id);
            if !is_slug(&rf.id) {
                lint.error(
                    &file,
                    format!(
                        "bad feature id \"{}\": use lowercase letters, digits and -",
                        rf.id
                    ),
                );
            }
            if c.features.iter().any(|f| f.id == rf.id) {
                lint.error(&file, format!("duplicate feature id \"{}\"", rf.id));
            }
            let title = nonempty(rf.title).unwrap_or_else(|| {
                lint.error(&file, format!("{what}missing title"));
                rf.id.clone()
            });
            let status = rf.status.unwrap_or_else(|| {
                lint.error(&file, format!("{what}missing status"));
                FeatureStatus::Todo
            });
            check_milestone(&mut lint, &rf.milestone, &what);
            for on in &rf.needs {
                c.needs.push(Need {
                    on: Some(on.clone()),
                    fuzzy: None,
                    why: None,
                    soft: false,
                    from_feature: Some(rf.id.clone()),
                    target: None,
                    target_feature: None,
                    met: false,
                    active: status != FeatureStatus::Dropped,
                });
            }
            c.features.push(Feature {
                id: rf.id,
                title,
                status,
                milestone: rf.milestone,
                note: rf.note,
                refs: rf.refs,
                needs: rf.needs,
            });
        }
        let summary = c.summary.clone();
        self.lint = lint;
        self.check_summary(&file, &summary);
    }

    /// Resolves every `on` label to the component and feature it names.
    fn resolve(&mut self) {
        for i in 0..self.comps.len() {
            let here = self.comps[i].path.clone();
            let file = self.comps[i].file.clone();
            let mut needs = std::mem::take(&mut self.comps[i].needs);
            for n in &mut needs {
                let Some(text) = n.on.clone() else { continue };
                let label = match Label::parse(&text, &here) {
                    Ok(l) => l,
                    Err(e) => {
                        self.lint.error(&file, e);
                        continue;
                    }
                };
                n.on = Some(label.to_string());
                let Some(&t) = self.by_path.get(&label.path) else {
                    let hint = closest(&label.path, self.comps.iter().map(|c| c.path.as_str()))
                        .map(|p| format!("; did you mean t8y:/{p}?"))
                        .unwrap_or_default();
                    self.lint.error(
                        &file,
                        format!(
                            "unresolved label {text}: no component t8y:/{}{hint}",
                            label.path
                        ),
                    );
                    continue;
                };
                if let Some(f) = &label.feature {
                    let target = &self.comps[t];
                    if !target.broken && target.feature(f).is_none() {
                        let hint = closest(f, target.features.iter().map(|x| x.id.as_str()))
                            .map(|x| format!("; did you mean #{x}?"))
                            .unwrap_or_default();
                        self.lint.error(
                            &file,
                            format!(
                                "unresolved label {text}: t8y:/{} has no feature #{f}{hint}",
                                label.path
                            ),
                        );
                        continue;
                    }
                }
                if t == i {
                    let own_feature = n.from_feature.is_some() && label.feature.is_some();
                    if !own_feature || label.feature == n.from_feature {
                        self.lint
                            .error(&file, format!("{text}: a component can't need itself"));
                        continue;
                    }
                }
                n.target = Some(t);
                n.target_feature = label.feature;
            }
            // A feature's needs list shows the absolute labels.
            let c = &mut self.comps[i];
            for f in &mut c.features {
                f.needs = needs
                    .iter()
                    .filter(|n| n.from_feature.as_deref() == Some(&f.id))
                    .filter_map(|n| n.on.clone())
                    .collect();
            }
            c.needs = needs;
        }
    }
}
