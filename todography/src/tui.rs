//! `todography tui`: the map page's Map, Focus and Roadmap views in a
//! terminal, worked out from the same Map the queries use. Where the page
//! draws lines and hover cards, the terminal marks related tiles with arrows
//! and shows the selected component in a status panel.

use crate::map::{Comp, FeatureStatus, Intent, Map, Need, Status};
use base64::Engine;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Opens the terminal UI on the map in `dir` and runs it until `q`.
pub fn run(dir: &Path) -> Result<(), String> {
    let mut app = App::new(dir);
    if app.map.comps.is_empty() {
        return Err("the map has no components".into());
    }
    let mut term = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    // ratatui's hook restores the screen on a panic; the mouse too, or the
    // shell gets an escape sequence for every move.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        hook(info);
    }));
    let r = app.run(&mut term);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    r.map_err(|e| e.to_string())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Map,
    Focus,
    Roadmap,
}

/// What the Map view shows besides the wanted components, as on the page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Show {
    Wanted,
    Aspirations,
    All,
}

/// Where a tile sat in the last frame of the Map view, for the arrow keys.
struct Tile {
    comp: usize,
    row: usize,
    x: usize,
    w: usize,
}

/// Something the mouse can click, and where the last frame drew it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hit {
    Tab(View),
    Tile(usize),
    /// Focus: a column (0 needs, 1 card, 2 needed by), a node in a side
    /// column, a feature in the card, a component on the trail.
    Column(usize),
    Node(usize, usize),
    Feature(usize),
    Trail(usize),
    /// Roadmap: a column, by its index in roadmap(), and an item in it.
    RoadCol(usize),
    Work(usize, usize),
    SearchBox,
    Search(usize),
}

/// Text being selected with the mouse: from where the button went down to
/// where it is, inside the column it went down in.
struct Drag {
    from: Position,
    to: Position,
    clip: Rect,
}

impl Drag {
    /// The selected cells, row by row, in reading order.
    fn rows(&self) -> Vec<(u16, Range<u16>)> {
        let (a, b) = if (self.from.y, self.from.x) <= (self.to.y, self.to.x) {
            (self.from, self.to)
        } else {
            (self.to, self.from)
        };
        (a.y..=b.y)
            .map(|y| {
                let x0 = if y == a.y { a.x } else { self.clip.x };
                let x1 = if y == b.y { b.x + 1 } else { self.clip.right() };
                (y, x0..x1)
            })
            .collect()
    }
}

pub struct App {
    dir: PathBuf,
    pub map: Map,
    /// The index in map.work of each work item, by label.
    work_at: HashMap<String, usize>,
    /// The roadmap column of each wanted work item, by index in map.work.
    level: HashMap<usize, usize>,
    pub view: View,
    /// The component every view is about: the map's selection, Focus's card.
    pub sel: usize,
    show: Show,
    /// Dims the tiles that neither need the selection nor are needed by it.
    trace: bool,
    tiles: Vec<Tile>,
    map_scroll: usize,
    /// Focus: the column with the keyboard (needs, card, needed by), the
    /// node picked in each side column, the feature picked in the card.
    col: usize,
    pick: [usize; 3],
    feature: Option<usize>,
    card_scroll: usize,
    trail: Vec<usize>,
    /// Roadmap: 0 is every milestone, 1..=n one of them, n+1 none.
    milestone: usize,
    rm_col: usize,
    rm_row: usize,
    /// The search box's text and picked hit, while it is open.
    search: Option<(String, usize)>,
    msg: String,
    quit: bool,
    /// What the last frame drew where, topmost last.
    hits: Vec<(Rect, Hit)>,
    /// The roadmap's first column on screen.
    rm_first: usize,
    /// Scroll the map and the card to the selection; the wheel turns it off
    /// until the next key or click.
    follow: bool,
    drag: Option<Drag>,
    /// The last frame, to copy selected text from.
    screen: Buffer,
    /// Text for the clipboard, which run() sends to the terminal.
    pub copied: Option<String>,
}

fn stub(s: &str) -> bool {
    s.trim().is_empty() || s.trim() == "STUB"
}

fn status_word(s: &str) -> &str {
    match s {
        "wip" => "in progress",
        "todo" => "to do",
        s => s,
    }
}

// Only foreground colours, and only the ones that read on light and dark
// backgrounds alike: a theme's yellow, grey and bright colours often don't.
const NEEDS: Color = Color::Blue;
const NEEDED_BY: Color = Color::Magenta;

/// A status as a circle that fills as the work gets done: ○ to do, ◌ in
/// progress, ◔ ◑ ◕ partial by the share of features done, ● done, × dropped.
fn glyph(status: &str, done: usize, total: usize) -> Span<'static> {
    match status {
        "done" => "●".fg(Color::Green),
        "partial" => match 8 * done / total.max(1) {
            0..=2 => "◔",
            3..=5 => "◑",
            _ => "◕",
        }
        .fg(Color::Green),
        "wip" => "◌".fg(Color::Blue),
        "dropped" => "×".dim(),
        _ => "○".into(),
    }
}

/// A status glyph and a space, for the start of a line.
fn status_mark(s: &str) -> Span<'static> {
    let g = glyph(s, 0, 1);
    Span::styled(format!("{} ", g.content), g.style)
}

fn comp_mark(c: &Comp) -> Span<'static> {
    let (done, total) = progress(c);
    let g = glyph(c.status.as_str(), done, total);
    Span::styled(format!("{} ", g.content), g.style)
}

/// Puts `text` on the clipboard with OSC 52, which the terminal handles, so
/// it works over ssh and through multiplexers that pass it on.
fn to_clipboard(text: &str) {
    let b64 = base64::engine::general_purpose::STANDARD.encode(text);
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]52;c;{b64}\x07").and_then(|_| out.flush());
}

/// Splits `text` at spaces into lines of at most `width` columns; a word
/// longer than that gets a line to itself.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() || out.is_empty() {
        out.push(line);
    }
    out
}

fn short(label: &str) -> &str {
    label.strip_prefix("t8y:/").unwrap_or(label)
}

fn map_file(c: &Comp) -> String {
    format!("docs/map/{}/{}.md", c.area, c.slug)
}

/// (done, total) features not dropped; a component without features counts as one.
fn progress(c: &Comp) -> (usize, usize) {
    if c.features.is_empty() {
        return (
            usize::from(c.status == Status::Done),
            usize::from(c.status != Status::Dropped),
        );
    }
    let live = c
        .features
        .iter()
        .filter(|f| f.status != FeatureStatus::Dropped);
    (
        live.clone()
            .filter(|f| f.status == FeatureStatus::Done)
            .count(),
        live.count(),
    )
}

/// Roadmap columns, as the page works them out: ready work is column 0,
/// other work one column after its deepest blocker. A blocker that is a
/// component with features counts at its earliest feature; a cycle is cut
/// where the walk meets itself.
fn levels(map: &Map, work_at: &HashMap<String, usize>) -> HashMap<usize, usize> {
    struct Walk<'a> {
        map: &'a Map,
        work_at: &'a HashMap<String, usize>,
        level: HashMap<usize, usize>,
        visiting: HashSet<usize>,
    }
    impl Walk<'_> {
        fn blocker(&mut self, label: &str) -> usize {
            if let Some(&k) = self.work_at.get(label)
                && self.map.work[k].wanted
            {
                return self.lv(k);
            }
            let Some(&i) = self.map.by_path.get(short(label)) else {
                return 0;
            };
            let ks: Vec<usize> = (0..self.map.work.len())
                .filter(|&k| self.map.work[k].comp == i && self.map.work[k].wanted)
                .collect();
            ks.into_iter().map(|k| self.lv(k)).min().unwrap_or(0)
        }
        fn lv(&mut self, k: usize) -> usize {
            if let Some(&l) = self.level.get(&k) {
                return l;
            }
            if !self.visiting.insert(k) {
                return 0;
            }
            let map = self.map;
            let l = map.work[k]
                .blocked_by
                .iter()
                .map(|b| 1 + self.blocker(b))
                .max()
                .unwrap_or(0);
            self.visiting.remove(&k);
            self.level.insert(k, l);
            l
        }
    }
    let mut walk = Walk {
        map,
        work_at,
        level: HashMap::new(),
        visiting: HashSet::new(),
    };
    for k in 0..map.work.len() {
        if map.work[k].wanted {
            walk.lv(k);
        }
    }
    walk.level
}

impl App {
    pub fn new(dir: &Path) -> App {
        let map = Map::load(dir);
        let mut app = App {
            dir: dir.to_path_buf(),
            work_at: HashMap::new(),
            level: HashMap::new(),
            view: View::Map,
            sel: 0,
            show: Show::Aspirations,
            trace: false,
            tiles: vec![],
            map_scroll: 0,
            col: 1,
            pick: [0; 3],
            feature: None,
            card_scroll: 0,
            trail: vec![],
            milestone: 0,
            rm_col: 0,
            rm_row: 0,
            search: None,
            msg: String::new(),
            quit: false,
            hits: vec![],
            rm_first: 0,
            follow: true,
            drag: None,
            screen: Buffer::empty(Rect::ZERO),
            copied: None,
            map,
        };
        app.derive();
        app.sel = app.first_tile();
        // The roadmap opens on m1 when there is m1 work, as the page does.
        if let Some(m) = app.map.milestones.iter().position(|m| m.id == "m1")
            && app
                .map
                .work
                .iter()
                .any(|w| w.wanted && w.milestone.as_deref() == Some("m1"))
        {
            app.milestone = m + 1;
        }
        app
    }

    fn derive(&mut self) {
        self.work_at = (0..self.map.work.len())
            .map(|k| (self.map.work[k].label.clone(), k))
            .collect();
        self.level = levels(&self.map, &self.work_at);
    }

    /// Reads the map's files again, keeping the selection by label.
    fn reload(&mut self) {
        let path = self.map.comps.get(self.sel).map(|c| c.path.clone());
        self.map = Map::load(&self.dir);
        self.derive();
        self.sel = path
            .and_then(|p| self.map.by_path.get(&p).copied())
            .unwrap_or(0);
        self.trail.clear();
        self.feature = None;
        self.msg = format!(
            "reloaded: {} components, {} lint errors",
            self.map.comps.len(),
            self.map.lint.errors.len()
        );
    }

    fn run(&mut self, term: &mut DefaultTerminal) -> std::io::Result<()> {
        term.draw(|f| self.draw(f))?;
        while !self.quit {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => self.key(k.code, k.modifiers),
                Event::Mouse(m) if !matches!(m.kind, MouseEventKind::Moved) => {
                    self.mouse(m.kind, m.column, m.row)
                }
                Event::Resize(..) => {}
                _ => continue,
            }
            if let Some(text) = self.copied.take() {
                to_clipboard(&text);
            }
            term.draw(|f| self.draw(f))?;
        }
        Ok(())
    }

    fn comp_of(&self, label: &str) -> Option<usize> {
        let path = short(label).split('#').next()?;
        self.map.by_path.get(path).copied()
    }

    /// Moves Focus to component `i`, with `feature` picked if it has it.
    fn go(&mut self, i: usize, feature: Option<&str>) {
        if i != self.sel {
            self.trail.push(self.sel);
        }
        self.sel = i;
        self.feature =
            feature.and_then(|f| self.map.comps[i].features.iter().position(|x| x.id == f));
        self.pick = [0; 3];
        self.card_scroll = 0;
        self.view = View::Focus;
    }

    /// The work item `c` copies: the picked feature's, or the component's own.
    fn work_here(&self) -> Option<usize> {
        if self.view == View::Roadmap {
            return self.roadmap().get(self.rm_col)?.1.get(self.rm_row).copied();
        }
        let c = &self.map.comps[self.sel];
        let label = match self.feature {
            Some(f) if self.view == View::Focus => format!("{}#{}", c.label(), c.features[f].id),
            _ => c.label(),
        };
        self.work_at.get(&label).copied()
    }

    /// Every unfinished blocker of work item `k`, transitively, each after
    /// what it needs; the same list the page's copy button writes.
    fn blockers(&self, k: usize) -> Vec<usize> {
        fn visit(app: &App, label: &str, seen: &mut HashSet<String>, out: &mut Vec<usize>) {
            if !seen.insert(label.to_string()) {
                return;
            }
            let b = match app.work_at.get(label) {
                Some(&b) => b,
                None => {
                    let Some(i) = app.comp_of(label) else { return };
                    let Some(b) = (0..app.map.work.len())
                        .filter(|&x| app.map.work[x].comp == i)
                        .min_by_key(|x| app.level.get(x).copied().unwrap_or(0))
                    else {
                        return;
                    };
                    if !seen.insert(app.map.work[b].label.clone()) {
                        return;
                    }
                    b
                }
            };
            for l in &app.map.work[b].blocked_by {
                visit(app, l, seen, out);
            }
            out.push(b);
        }
        let mut seen = HashSet::from([self.map.work[k].label.clone()]);
        let mut out = vec![];
        for l in &self.map.work[k].blocked_by {
            visit(self, l, &mut seen, &mut out);
        }
        out
    }

    /// The prompt for an agent that the page's copy button writes.
    // ponytail: a second copy of workPrompt in assets/map.html; change both, or
    // move it into json.rs if a third place wants it.
    pub fn prompt(&self, k: usize) -> String {
        let w = &self.map.work[k];
        let c = &self.map.comps[w.comp];
        let f = w.feature.as_deref().and_then(|id| c.feature(id));
        let about = match f {
            Some(f) => f.note.clone().unwrap_or_default(),
            None if stub(&c.summary) => String::new(),
            None => c.summary.clone(),
        };
        let name = match f {
            Some(_) => format!("the feature \"{}\" of {}", w.title, c.title),
            None => w.title.clone(),
        };
        let blockers = self.blockers(k);
        let mut parts = vec![
            format!(
                "Implement {name}, {} on vaxpunk's map of components.",
                w.label
            ),
            about,
        ];
        if !blockers.is_empty() {
            parts.push(
                "It is blocked. This unfinished work comes first, each item after what it needs:"
                    .into(),
            );
            parts.push(
                blockers
                    .iter()
                    .map(|&b| {
                        let b = &self.map.work[b];
                        let bc = &self.map.comps[b.comp];
                        format!(
                            "- {}{}, {} ({}){}",
                            if b.feature.is_some() {
                                format!("{}: ", bc.title)
                            } else {
                                String::new()
                            },
                            b.title,
                            b.label,
                            map_file(bc),
                            if b.blocked_by.is_empty() {
                                ", ready now"
                            } else {
                                ""
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            parts.push(format!(
                "Implement all of it, then {name}. If that is too much for one go, say so before starting, and propose which of these dependencies to focus on first: the heavy ones that unblock the most, and why."
            ));
        }
        parts.push(format!(
            "Its entry is in {}; `cargo run -p todography -- show '{}'` prints it with its needs and why it is wanted{}",
            map_file(c),
            w.label,
            if blockers.is_empty() { "." } else { ", and does the same for each label above." }
        ));
        parts.push("When a piece lands, set its status = \"done\" in its map file.".into());
        parts.retain(|p| !p.is_empty());
        parts.join("\n\n")
    }

    /// Puts the prompt for the work picked on the clipboard.
    fn copy(&mut self) {
        let Some(k) = self.work_here() else {
            self.msg = if self.view == View::Roadmap {
                "nothing picked".into()
            } else {
                "no work here: done, or pick a feature in the card first".into()
            };
            return;
        };
        if !self.map.work[k].wanted {
            self.msg = "not wanted: no goal needs it, so there is no prompt".into();
            return;
        }
        self.copied = Some(self.prompt(k));
        self.msg = format!("copied a prompt for {}", self.map.work[k].label);
    }

    // ---------- keys ----------

    /// Opens search hit `n`: selects it on the map, else focuses it.
    fn open_hit(&mut self, n: usize) {
        let Some((q, _)) = &self.search else { return };
        let hits = self.search_hits(q);
        if let Some(&i) = hits.get(n.min(hits.len().saturating_sub(1))) {
            self.search = None;
            if self.view == View::Map {
                self.sel = i;
            } else {
                self.go(i, None);
            }
        }
    }

    pub fn key(&mut self, code: KeyCode, mods: KeyModifiers) {
        self.msg.clear();
        self.follow = true;
        if let Some((q, hit)) = &mut self.search {
            match code {
                KeyCode::Esc => self.search = None,
                KeyCode::Up => *hit = hit.saturating_sub(1),
                KeyCode::Down => *hit += 1,
                KeyCode::Backspace => {
                    q.pop();
                    *hit = 0;
                }
                KeyCode::Char(ch) => {
                    q.push(ch);
                    *hit = 0;
                }
                KeyCode::Enter => {
                    let hit = *hit;
                    self.open_hit(hit);
                }
                _ => {}
            }
            return;
        }
        match code {
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('1') => self.view = View::Map,
            KeyCode::Char('2') => self.view = View::Focus,
            KeyCode::Char('3') => self.view = View::Roadmap,
            KeyCode::Tab => {
                self.view = match self.view {
                    View::Map => View::Focus,
                    View::Focus => View::Roadmap,
                    View::Roadmap => View::Map,
                }
            }
            KeyCode::Char('/') => self.search = Some((String::new(), 0)),
            KeyCode::Char('r') => self.reload(),
            KeyCode::Char('c') => self.copy(),
            _ => match self.view {
                View::Map => self.map_key(code),
                View::Focus => self.focus_key(code),
                View::Roadmap => self.roadmap_key(code),
            },
        }
    }

    // ---------- mouse ----------

    /// A click picks what is under it, and a second click opens it, as a
    /// tap does on the page. A drag selects text inside the column it
    /// starts in and copies it on release, since the terminal can't select
    /// while the TUI has the mouse. The wheel scrolls or moves the pick in
    /// the column under it.
    pub fn mouse(&mut self, kind: MouseEventKind, x: u16, y: u16) {
        self.msg.clear();
        let at = Position { x, y };
        let here: Vec<(Rect, Hit)> = self
            .hits
            .iter()
            .rev()
            .filter(|h| h.0.contains(at))
            .copied()
            .collect();
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let clip = here
                    .iter()
                    .find(|h| matches!(h.1, Hit::Column(_) | Hit::RoadCol(_) | Hit::SearchBox))
                    .map_or(self.screen.area, |h| h.0.inner(Margin::new(1, 1)));
                self.drag = Some(Drag {
                    from: at,
                    to: at,
                    clip,
                });
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(d) = &mut self.drag {
                    d.to = Position {
                        x: x.clamp(d.clip.x, d.clip.right().saturating_sub(1)),
                        y: y.clamp(d.clip.y, d.clip.bottom().saturating_sub(1)),
                    };
                }
            }
            MouseEventKind::Up(MouseButton::Left) => match self.drag.take() {
                Some(d) if d.from != d.to => self.copy_selection(&d),
                Some(_) => self.click(here.first().map(|h| h.1)),
                None => {}
            },
            MouseEventKind::ScrollUp => self.wheel(&here, -1),
            MouseEventKind::ScrollDown => self.wheel(&here, 1),
            _ => {}
        }
    }

    /// The text of the last frame under `d`, each row without trailing spaces.
    fn selection(&self, d: &Drag) -> String {
        let area = self.screen.area;
        d.rows()
            .into_iter()
            .filter(|(y, _)| *y < area.bottom())
            .map(|(y, xs)| {
                let row: String = xs
                    .filter(|&x| x < area.right())
                    .map(|x| self.screen[(x, y)].symbol())
                    .collect();
                row.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn copy_selection(&mut self, d: &Drag) {
        let text = self.selection(d);
        self.msg = format!("copied {} characters", text.chars().count());
        self.copied = Some(text);
    }

    fn click(&mut self, hit: Option<Hit>) {
        if self.search.is_some() && !matches!(hit, Some(Hit::Search(_) | Hit::SearchBox)) {
            self.search = None;
            return;
        }
        self.follow = true;
        match hit {
            Some(Hit::Tab(v)) => self.view = v,
            Some(Hit::Tile(i)) if i == self.sel => self.map_key(KeyCode::Enter),
            Some(Hit::Tile(i)) => self.sel = i,
            Some(Hit::Column(c)) => self.col = c,
            Some(Hit::Node(c, n)) if self.col == c && self.pick[c] == n => {
                self.focus_key(KeyCode::Enter)
            }
            Some(Hit::Node(c, n)) => {
                self.col = c;
                self.pick[c] = n;
            }
            Some(Hit::Feature(k)) => {
                self.col = 1;
                self.feature = if self.feature == Some(k) {
                    None
                } else {
                    Some(k)
                };
            }
            Some(Hit::Trail(n)) => {
                self.sel = self.trail[n];
                self.trail.truncate(n);
                self.feature = None;
                self.pick = [0; 3];
                self.card_scroll = 0;
            }
            Some(Hit::RoadCol(c)) => self.rm_col = c,
            Some(Hit::Work(c, r)) if (self.rm_col, self.rm_row) == (c, r) => {
                self.roadmap_key(KeyCode::Enter)
            }
            Some(Hit::Work(c, r)) => (self.rm_col, self.rm_row) = (c, r),
            Some(Hit::Search(n)) => self.open_hit(n),
            Some(Hit::SearchBox) | None => {}
        }
    }

    fn wheel(&mut self, here: &[(Rect, Hit)], d: isize) {
        let key = if d < 0 { KeyCode::Up } else { KeyCode::Down };
        if let Some((_, hit)) = &mut self.search {
            *hit = hit.saturating_add_signed(d);
            return;
        }
        self.follow = false;
        let col = here.iter().find_map(|h| match h.1 {
            Hit::Column(c) | Hit::RoadCol(c) => Some(c),
            _ => None,
        });
        match (self.view, col) {
            (View::Map, _) => self.map_scroll = self.map_scroll.saturating_add_signed(3 * d),
            (View::Focus, Some(1)) => {
                self.card_scroll = self.card_scroll.saturating_add_signed(3 * d)
            }
            (View::Focus, Some(c)) => {
                self.col = c;
                self.focus_key(key);
            }
            (View::Roadmap, Some(c)) => {
                if c != self.rm_col {
                    (self.rm_col, self.rm_row) = (c, 0);
                }
                self.roadmap_key(key);
            }
            _ => {}
        }
    }

    /// Records where each item of a list drew: `inner` is the list's area
    /// inside its border, `heights` its items' heights from the first.
    fn list_hits(
        &mut self,
        inner: Rect,
        heights: &[usize],
        offset: usize,
        hit: impl Fn(usize) -> Hit,
    ) {
        let mut y = inner.y;
        for (n, &h) in heights.iter().enumerate().skip(offset) {
            if y >= inner.bottom() {
                break;
            }
            let h = (h as u16).min(inner.bottom() - y);
            self.hits.push((
                Rect {
                    y,
                    height: h,
                    ..inner
                },
                hit(n),
            ));
            y += h;
        }
    }

    /// Records where each span of a one-line widget drew, for those `hit` names.
    fn span_hits(&mut self, at: Rect, spans: &[Span], hit: impl Fn(usize) -> Option<Hit>) {
        let mut x = at.x;
        for (k, s) in spans.iter().enumerate() {
            let w = s.width() as u16;
            if let Some(h) = hit(k) {
                self.hits.push((
                    Rect {
                        x,
                        y: at.y,
                        width: w,
                        height: 1,
                    },
                    h,
                ));
            }
            x += w;
        }
    }

    fn map_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left | KeyCode::Char('h') => self.map_step(0, -1),
            KeyCode::Right | KeyCode::Char('l') => self.map_step(0, 1),
            KeyCode::Up | KeyCode::Char('k') => self.map_step(-1, 0),
            KeyCode::Down | KeyCode::Char('j') => self.map_step(1, 0),
            KeyCode::PageUp => (0..10).for_each(|_| self.map_step(-1, 0)),
            KeyCode::PageDown => (0..10).for_each(|_| self.map_step(1, 0)),
            KeyCode::Enter => {
                self.trail.clear();
                let sel = self.sel;
                self.go(sel, None);
            }
            KeyCode::Char('s') => {
                self.show = match self.show {
                    Show::Aspirations => Show::All,
                    Show::All => Show::Wanted,
                    Show::Wanted => Show::Aspirations,
                }
            }
            KeyCode::Char('t') => self.trace = !self.trace,
            _ => {}
        }
    }

    /// Moves the map's selection a tile across or a row of tiles up or down,
    /// to the tile there nearest in x.
    fn map_step(&mut self, drow: isize, dx: isize) {
        let Some(cur) = self.tiles.iter().position(|t| t.comp == self.sel) else {
            return;
        };
        let next = if dx != 0 {
            cur.checked_add_signed(dx).filter(|&n| n < self.tiles.len())
        } else {
            let t = &self.tiles[cur];
            let row = if drow < 0 {
                self.tiles
                    .iter()
                    .map(|x| x.row)
                    .filter(|&r| r < t.row)
                    .max()
            } else {
                self.tiles
                    .iter()
                    .map(|x| x.row)
                    .filter(|&r| r > t.row)
                    .min()
            };
            let mid = t.x + t.w / 2;
            row.and_then(|row| {
                (0..self.tiles.len())
                    .filter(|&n| self.tiles[n].row == row)
                    .min_by_key(|&n| (self.tiles[n].x + self.tiles[n].w / 2).abs_diff(mid))
            })
        };
        if let Some(n) = next {
            self.sel = self.tiles[n].comp;
        }
    }

    fn focus_key(&mut self, code: KeyCode) {
        let nodes = [
            self.needs_nodes(self.sel).len(),
            0,
            self.by_nodes(self.sel).len(),
        ];
        let nfeat = self.map.comps[self.sel].features.len();
        match code {
            KeyCode::Left | KeyCode::Char('h') => self.col = self.col.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => self.col = (self.col + 1).min(2),
            KeyCode::Up | KeyCode::Char('k') => match self.col {
                1 if nfeat == 0 => self.card_scroll = self.card_scroll.saturating_sub(1),
                1 => self.feature = self.feature.and_then(|f| f.checked_sub(1)),
                c => self.pick[c] = self.pick[c].saturating_sub(1),
            },
            KeyCode::Down | KeyCode::Char('j') => match self.col {
                1 if nfeat == 0 => self.card_scroll += 1,
                1 => self.feature = Some(self.feature.map_or(0, |f| (f + 1).min(nfeat - 1))),
                c => self.pick[c] = (self.pick[c] + 1).min(nodes[c].saturating_sub(1)),
            },
            KeyCode::PageUp => self.card_scroll = self.card_scroll.saturating_sub(10),
            KeyCode::PageDown => self.card_scroll += 10,
            KeyCode::Esc => self.feature = None,
            KeyCode::Backspace => {
                if let Some(p) = self.trail.pop() {
                    self.sel = p;
                    self.feature = None;
                    self.pick = [0; 3];
                    self.card_scroll = 0;
                }
            }
            KeyCode::Enter if self.col != 1 => {
                let to = if self.col == 0 {
                    self.needs_nodes(self.sel)
                        .get(self.pick[0])
                        .and_then(|n| Some((n.target?, n.lines.first()?.target_feature.clone())))
                } else {
                    self.by_nodes(self.sel)
                        .get(self.pick[2])
                        .map(|n| (n.from, n.lines.first().and_then(|e| e.from_feature.clone())))
                };
                if let Some((i, f)) = to {
                    self.go(i, f.as_deref());
                }
            }
            _ => {}
        }
    }

    fn roadmap_key(&mut self, code: KeyCode) {
        let cols = self.roadmap();
        match code {
            KeyCode::Left | KeyCode::Char('h') => self.rm_col = self.rm_col.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => {
                self.rm_col = (self.rm_col + 1).min(cols.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => self.rm_row = self.rm_row.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.rm_row += 1,
            KeyCode::PageUp => self.rm_row = self.rm_row.saturating_sub(10),
            KeyCode::PageDown => self.rm_row += 10,
            KeyCode::Char('m') => {
                self.milestone = (self.milestone + 1) % (self.map.milestones.len() + 2);
                self.rm_col = 0;
                self.rm_row = 0;
            }
            KeyCode::Enter => {
                if let Some(&k) = cols.get(self.rm_col).and_then(|c| c.1.get(self.rm_row)) {
                    let w = &self.map.work[k];
                    let (i, f) = (w.comp, w.feature.clone());
                    self.trail.clear();
                    self.go(i, f.as_deref());
                }
            }
            _ => {}
        }
        let len = self.roadmap().get(self.rm_col).map_or(0, |c| c.1.len());
        self.rm_row = self.rm_row.min(len.saturating_sub(1));
    }

    /// The page's search: an exact label, title or other name first, then
    /// prefixes, then words inside; wanted components win ties.
    fn search_hits(&self, q: &str) -> Vec<usize> {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let ql = format!(
            "t8y:/{}",
            q.strip_prefix("t8y:").unwrap_or(&q).trim_start_matches('/')
        );
        let mut hits: Vec<(usize, usize)> = vec![];
        for (i, c) in self.map.comps.iter().enumerate() {
            let title = c.title.to_lowercase();
            let label = c.label();
            let aka: Vec<String> = c.aka.iter().map(|a| a.to_lowercase()).collect();
            let s = if label == ql {
                100
            } else if title == q {
                95
            } else if aka.contains(&q) {
                85
            } else if title.starts_with(&q) {
                80
            } else if label.starts_with(&ql) || label.contains(&q) {
                60
            } else if title.contains(&q) {
                55
            } else if aka.iter().any(|a| a.contains(&q)) {
                45
            } else if !stub(&c.summary) && c.summary.to_lowercase().contains(&q) {
                20
            } else {
                0
            };
            if s > 0 {
                hits.push((s + usize::from(c.wanted), i));
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().take(12).map(|h| h.1).collect()
    }

    // ---------- drawing ----------

    pub fn draw(&mut self, f: &mut Frame) {
        let [head, body, foot] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(f.area());
        self.hits.clear();
        let header = self.header();
        let tabs = [View::Map, View::Focus, View::Roadmap];
        self.span_hits(head, &header.spans, |k| {
            tabs.get(k.wrapping_sub(2)).map(|&v| Hit::Tab(v))
        });
        f.render_widget(header, head);
        match self.view {
            View::Map => self.draw_map(f, body),
            View::Focus => self.draw_focus(f, body),
            View::Roadmap => self.draw_roadmap(f, body),
        }
        let hints = match self.view {
            View::Map => {
                "click or arrows pick  click again or ⏎ focus  wheel scroll  s show  t trace  / search  c copy prompt  r reload  q quit"
            }
            View::Focus => {
                "click or ←→↑↓ pick  click again or ⏎ go  ⌫ back  esc all needs  wheel or PgUp/PgDn scroll  c copy prompt  / search"
            }
            View::Roadmap => {
                "click or arrows pick  click again or ⏎ focus  m milestone  c copy prompt  / search  r reload  q quit"
            }
        };
        let foot_line = if self.msg.is_empty() {
            Line::from(hints.dim())
        } else {
            Line::from(self.msg.clone().fg(Color::Green))
        };
        f.render_widget(Paragraph::new(foot_line), foot);
        if self.search.is_some() {
            self.draw_search(f, body);
        }
        let buf = f.buffer_mut();
        self.screen = buf.clone();
        if let Some(d) = self.drag.as_ref().filter(|d| d.from != d.to) {
            for (y, xs) in d.rows() {
                for x in xs {
                    if buf.area.contains(Position { x, y }) {
                        buf[(x, y)].modifier.toggle(Modifier::REVERSED);
                    }
                }
            }
        }
    }

    fn header(&self) -> Line<'static> {
        let m = &self.map;
        let mut s: Vec<Span> = vec![format!(" {} ", m.title).bold().reversed(), " ".into()];
        for (n, (v, name)) in [
            (View::Map, "Map"),
            (View::Focus, "Focus"),
            (View::Roadmap, "Roadmap"),
        ]
        .into_iter()
        .enumerate()
        {
            let tab = format!(" {} {name} ", n + 1);
            s.push(if v == self.view {
                tab.bold().underlined()
            } else {
                tab.dim()
            });
        }
        let count = |p: fn(&Comp) -> bool| m.comps.iter().filter(|c| p(c)).count();
        s.push(
            format!(
                "   {} components · {} wanted · {} done · {} partial · {} ready",
                m.comps.len(),
                count(|c| c.wanted),
                count(|c| c.status == Status::Done),
                count(|c| c.status == Status::Partial),
                m.work.iter().filter(|w| w.ready).count()
            )
            .dim(),
        );
        if !m.lint.errors.is_empty() || !m.lint.warnings.is_empty() {
            s.push(
                format!(
                    "   lint: {} errors, {} warnings",
                    m.lint.errors.len(),
                    m.lint.warnings.len()
                )
                .fg(Color::Red),
            );
        }
        Line::from(s)
    }

    /// The first component the Map view shows, top left.
    fn first_tile(&self) -> usize {
        let m = &self.map;
        m.bands
            .iter()
            .flat_map(|b| m.areas.iter().filter(move |a| a.band == b.id))
            .flat_map(|a| (0..m.comps.len()).filter(move |&i| m.comps[i].area == a.id))
            .find(|&i| self.visible(&m.comps[i]))
            .unwrap_or(0)
    }

    fn visible(&self, c: &Comp) -> bool {
        c.wanted
            || match self.show {
                Show::Wanted => false,
                Show::All => true,
                Show::Aspirations => c.intent == Intent::Aspiration,
            }
    }

    /// What component `i` needs and what needs it, each component once.
    fn relations(&self, i: usize) -> (HashSet<usize>, HashSet<usize>) {
        let c = &self.map.comps[i];
        let needs = c
            .needs
            .iter()
            .filter_map(|n| n.target)
            .filter(|&t| t != i)
            .collect();
        let by = c
            .needed_by
            .iter()
            .map(|e| e.from)
            .filter(|&f| f != i)
            .collect();
        (needs, by)
    }

    /// One component as a tile: its status glyph and title, bold for a
    /// goal, faint when it isn't wanted. The selection is in reverse video;
    /// ↓ marks what it needs and ↑ what needs it, and with `t` everything
    /// else is faint. Each tile starts with the mark's column and ends with
    /// a space, so tiles keep two spaces between them and never move.
    fn tile(
        &self,
        i: usize,
        needs: &HashSet<usize>,
        by: &HashSet<usize>,
    ) -> (Vec<Span<'static>>, usize) {
        let c = &self.map.comps[i];
        let (mark, ink) = if needs.contains(&i) {
            ("↓", Style::new().fg(NEEDS).add_modifier(Modifier::BOLD))
        } else if by.contains(&i) {
            ("↑", Style::new().fg(NEEDED_BY).add_modifier(Modifier::BOLD))
        } else {
            (" ", Style::new())
        };
        let mut title: String = c.title.chars().take(26).collect();
        if c.title.chars().count() > 26 {
            title.push('…');
        }
        let (done, total) = progress(c);
        let mut g = glyph(c.status.as_str(), done, total);
        let mut text = ink;
        if c.intent == Intent::Goal {
            text = text.add_modifier(Modifier::BOLD);
        }
        if !c.wanted {
            text = text.add_modifier(Modifier::ITALIC);
        }
        if c.status == Status::Dropped {
            text = text.add_modifier(Modifier::CROSSED_OUT);
        }
        if !c.wanted || (self.trace && mark == " " && i != self.sel) {
            text = text.add_modifier(Modifier::DIM);
            g = g.dim();
        }
        let mut spans = vec![
            Span::styled(mark, ink),
            g,
            Span::styled(format!(" {title} "), text),
        ];
        if i == self.sel {
            for s in &mut spans {
                s.style = s.style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
            }
        }
        let width = spans.iter().map(Span::width).sum();
        (spans, width)
    }

    /// The bands top to bottom, each area's tiles flowed under its name.
    /// Fills self.tiles with where each tile went.
    fn map_lines(&mut self, width: usize) -> Vec<Line<'static>> {
        const INDENT: usize = 4;
        let (needs, by) = self.relations(self.sel);
        let mut lines = vec![];
        let mut tiles = vec![];
        for band in &self.map.bands {
            let mut band_lines = vec![];
            for a in self.map.areas.iter().filter(|a| a.band == band.id) {
                let comps: Vec<usize> = (0..self.map.comps.len())
                    .filter(|&i| self.map.comps[i].area == a.id && self.visible(&self.map.comps[i]))
                    .collect();
                if comps.is_empty() {
                    continue;
                }
                band_lines.push(Line::from(vec![
                    "  ".into(),
                    a.title.clone().fg(Color::Cyan).bold(),
                    format!("  {}", comps.len()).dim(),
                ]));
                let mut spans: Vec<Span> = vec![" ".repeat(INDENT).into()];
                let mut x = INDENT;
                for i in comps {
                    let (tile, w) = self.tile(i, &needs, &by);
                    if x > INDENT && x + w > width {
                        band_lines.push(Line::from(std::mem::take(&mut spans)));
                        spans.push(" ".repeat(INDENT).into());
                        x = INDENT;
                    }
                    tiles.push(Tile {
                        comp: i,
                        row: lines.len() + 1 + band_lines.len(),
                        x,
                        w,
                    });
                    spans.extend(tile);
                    x += w;
                }
                band_lines.push(Line::from(spans));
            }
            if band_lines.is_empty() {
                continue;
            }
            let rule = "─".repeat(width.saturating_sub(band.title.chars().count() + 4));
            lines.push(Line::from(vec![
                format!(" {} ", band.title).bold(),
                rule.dim(),
            ]));
            lines.extend(band_lines);
            lines.push(Line::default());
        }
        self.tiles = tiles;
        lines
    }

    fn draw_map(&mut self, f: &mut Frame, area: Rect) {
        let [top, info] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(5)]).areas(area);
        let lines = self.map_lines(top.width as usize);
        if !self.tiles.iter().any(|t| t.comp == self.sel) && !self.tiles.is_empty() {
            self.sel = self.first_tile();
            return self.draw_map(f, area);
        }
        let row = self
            .tiles
            .iter()
            .find(|t| t.comp == self.sel)
            .map_or(0, |t| t.row);
        let h = top.height as usize;
        // Keep the selected tile, and the area name above it, on screen.
        if self.follow && row < self.map_scroll + 2 {
            self.map_scroll = row.saturating_sub(2);
        } else if self.follow && row >= self.map_scroll + h {
            self.map_scroll = row + 1 - h;
        }
        self.map_scroll = self.map_scroll.min(lines.len().saturating_sub(h));
        let scroll = self.map_scroll;
        let tiles: Vec<(Rect, Hit)> = self
            .tiles
            .iter()
            .filter(|t| (scroll..scroll + h).contains(&t.row))
            .map(|t| {
                let at = Rect {
                    x: top.x + t.x as u16,
                    y: top.y + (t.row - scroll) as u16,
                    width: t.w as u16,
                    height: 1,
                };
                (at, Hit::Tile(t.comp))
            })
            .collect();
        self.hits.extend(tiles);
        f.render_widget(
            Paragraph::new(lines).scroll((self.map_scroll as u16, 0)),
            top,
        );
        f.render_widget(self.map_info(info.width as usize), info);
    }

    /// The panel under the map: what the page shows on hover.
    fn map_info(&self, width: usize) -> Paragraph<'static> {
        let c = &self.map.comps[self.sel];
        let (needs, by) = self.relations(self.sel);
        let (done, total) = progress(c);
        let mut facts = vec![status_word(c.status.as_str()).to_string()];
        if !c.features.is_empty() {
            facts.push(format!("{done} of {total} features"));
        }
        facts.push(c.intent.as_str().into());
        facts.push(if c.wanted { "wanted" } else { "not wanted" }.into());
        let names = |set: &HashSet<usize>| {
            let mut v: Vec<&str> = set
                .iter()
                .map(|&i| self.map.comps[i].title.as_str())
                .collect();
            v.sort();
            v.join(", ")
        };
        let show = match self.show {
            Show::Wanted => "wanted only",
            Show::Aspirations => "wanted + aspirations",
            Show::All => "everything",
        };
        let summary = if stub(&c.summary) {
            "A stub: not described yet."
        } else {
            c.summary.as_str()
        };
        let rel = format!("↓ needs {}: {}", needs.len(), names(&needs));
        let rel_by = format!("↑ needed by {}: {}", by.len(), names(&by));
        let clip = |s: String| -> String { s.chars().take(width.saturating_sub(1)).collect() };
        Paragraph::new(vec![
            Line::from(vec![
                comp_mark(c),
                c.title.clone().bold(),
                "  ".into(),
                c.label().dim(),
                format!("  {}", facts.join(" · ")).into(),
                format!("   showing {show}").dim(),
            ]),
            Line::from(clip(summary.to_string())),
            Line::from(clip(rel)).fg(NEEDS),
            Line::from(clip(rel_by)).fg(NEEDED_BY),
        ])
        .block(
            Block::new()
                .borders(ratatui::widgets::Borders::TOP)
                .border_style(Style::new().dim()),
        )
    }

    fn draw_focus(&mut self, f: &mut Frame, area: Rect) {
        let [trail, cols] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
        let mut t: Vec<Span> = vec!["  ".into()];
        let shown = self.trail.len().saturating_sub(6)..self.trail.len();
        if !self.trail.is_empty() {
            t.push("⌫ back: ".dim());
            for &i in &self.trail[shown.clone()] {
                t.push(self.map.comps[i].title.clone().dim());
                t.push(" › ".dim());
            }
        }
        t.push(self.map.comps[self.sel].title.clone().bold());
        // Spans 2, 4, 6... are the trail's titles.
        self.span_hits(trail, &t, |k| {
            (k >= 2 && k % 2 == 0 && k / 2 - 1 < shown.len())
                .then(|| Hit::Trail(shown.start + k / 2 - 1))
        });
        f.render_widget(Paragraph::new(Line::from(t)), trail);

        let [left, mid, right] = Layout::horizontal([
            Constraint::Percentage(28),
            Constraint::Percentage(44),
            Constraint::Percentage(28),
        ])
        .areas(cols);
        let block = |col: usize, title: String| {
            let b = Block::bordered().title(title);
            if col == self.col {
                b.border_style(Style::new().fg(Color::Cyan))
            } else {
                b.border_style(Style::new().dim())
            }
        };
        let highlight = |col: usize| {
            if col == self.col {
                Style::new().add_modifier(Modifier::REVERSED)
            } else {
                Style::new()
            }
        };

        let needs = self.needs_nodes(self.sel);
        let w = left.width.saturating_sub(2) as usize;
        let items: Vec<ListItem> = needs.iter().map(|n| self.need_item(n, w)).collect();
        let title = format!(" Needs ({}) ", needs.len());
        drop(needs);
        self.pick[0] = self.pick[0].min(items.len().saturating_sub(1));
        let heights: Vec<usize> = items.iter().map(ListItem::height).collect();
        let list = List::new(items)
            .block(block(0, title))
            .highlight_style(highlight(0));
        let mut state = ListState::default().with_selected(Some(self.pick[0]));
        f.render_stateful_widget(list, left, &mut state);

        let by = self.by_nodes(self.sel);
        let w = right.width.saturating_sub(2) as usize;
        let items: Vec<ListItem> = by.iter().map(|n| self.by_item(n, w)).collect();
        let title = format!(" Needed by ({}) ", by.len());
        drop(by);
        self.pick[2] = self.pick[2].min(items.len().saturating_sub(1));
        let by_heights: Vec<usize> = items.iter().map(ListItem::height).collect();
        let list = List::new(items)
            .block(block(2, title))
            .highlight_style(highlight(2));
        let mut by_state = ListState::default().with_selected(Some(self.pick[2]));
        f.render_stateful_widget(list, right, &mut by_state);

        let (lines, features) = self.card(mid.width.saturating_sub(2) as usize);
        let h = mid.height.saturating_sub(2) as usize;
        let mut scroll = self.card_scroll;
        if self.follow
            && let Some(r) = self.feature.and_then(|k| features.get(k)).map(|r| r.start)
        {
            if r < scroll {
                scroll = r;
            } else if r + 3 > scroll + h {
                scroll = (r + 3).saturating_sub(h);
            }
        }
        scroll = scroll.min(lines.len().saturating_sub(1));
        f.render_widget(
            Paragraph::new(lines)
                .block(block(1, " Card ".into()))
                .scroll((scroll as u16, 0)),
            mid,
        );
        self.card_scroll = scroll;

        // The columns first, so what is in them is on top.
        self.hits.extend([
            (left, Hit::Column(0)),
            (mid, Hit::Column(1)),
            (right, Hit::Column(2)),
        ]);
        let inner = left.inner(Margin::new(1, 1));
        self.list_hits(inner, &heights, state.offset(), |n| Hit::Node(0, n));
        let inner = right.inner(Margin::new(1, 1));
        self.list_hits(inner, &by_heights, by_state.offset(), |n| Hit::Node(2, n));
        let inner = mid.inner(Margin::new(1, 1));
        let on_screen = scroll..scroll + inner.height as usize;
        for (k, r) in features.iter().enumerate() {
            let top = r.start.max(on_screen.start);
            let bottom = r.end.min(on_screen.end);
            if top < bottom {
                let y = inner.y + (top - scroll) as u16;
                let height = (bottom - top) as u16;
                self.hits
                    .push((Rect { y, height, ..inner }, Hit::Feature(k)));
            }
        }
    }

    /// The needs of component `i`, one node per component they point at,
    /// unmet hard needs first, then met, soft and those of dropped features;
    /// fuzzy needs last.
    fn needs_nodes(&self, i: usize) -> Vec<NeedNode<'_>> {
        let c = &self.map.comps[i];
        let mut nodes: Vec<NeedNode> = vec![];
        for n in &c.needs {
            if n.fuzzy.is_some() {
                nodes.push(NeedNode {
                    target: None,
                    key: String::new(),
                    lines: vec![n],
                    rank: 5,
                });
                continue;
            }
            let Some(on) = &n.on else { continue };
            if n.target == Some(i) {
                continue;
            }
            let key = short(on).split('#').next().unwrap_or("").to_string();
            match nodes.iter_mut().find(|g| g.rank != 5 && g.key == key) {
                Some(g) => g.lines.push(n),
                None => nodes.push(NeedNode {
                    target: n.target,
                    key,
                    lines: vec![n],
                    rank: 0,
                }),
            }
        }
        for g in &mut nodes {
            if g.rank == 5 {
                continue;
            }
            let act: Vec<&&Need> = g.lines.iter().filter(|n| n.active).collect();
            let hard: Vec<&&&Need> = act.iter().filter(|n| !n.soft).collect();
            g.rank = if g.target.is_none() {
                if hard.is_empty() { 2 } else { 4 }
            } else if act.is_empty() {
                3
            } else if hard.is_empty() {
                2
            } else if hard.iter().any(|n| !n.met) {
                0
            } else {
                1
            };
        }
        nodes.sort_by_key(|g| g.rank);
        nodes
    }

    fn need_item(&self, g: &NeedNode, w: usize) -> ListItem<'static> {
        let c = &self.map.comps[self.sel];
        let feat = self.feature.map(|f| c.features[f].id.as_str());
        let mine = |n: &Need| {
            feat.is_none() || n.from_feature.is_none() || n.from_feature.as_deref() == feat
        };
        let mut lines: Vec<Line> = vec![];
        if g.rank == 5 {
            let n = g.lines[0];
            lines.push(Line::from(format!("≈ {}", n.fuzzy.as_deref().unwrap_or(""))).italic());
            lines.push(Line::from("  not researched yet".dim()));
        } else {
            let (mark, title, label) = match g.target {
                Some(t) => {
                    let t = &self.map.comps[t];
                    (comp_mark(t), t.title.clone(), t.label())
                }
                None => ("? ".fg(Color::Red), g.key.clone(), "not on the map".into()),
            };
            let state = match g.rank {
                0 => " not met".fg(Color::Red),
                1 => " met".fg(Color::Green),
                2 => " soft".dim(),
                3 => " dropped feature".dim(),
                _ => " missing".fg(Color::Red),
            };
            lines.push(Line::from(vec![mark, title.bold(), state]));
            lines.push(Line::from(format!("  {label}").dim()));
            for n in &g.lines {
                let mut meta = vec![];
                if let Some(f) = &n.from_feature {
                    meta.push(format!("for #{f}"));
                }
                if let Some(f) = &n.target_feature {
                    meta.push(format!("on #{f}"));
                }
                let text = match (&n.why, meta.is_empty()) {
                    (Some(why), true) => why.clone(),
                    (Some(why), false) => format!("{}: {why}", meta.join(", ")),
                    (None, false) => meta.join(", "),
                    (None, true) => continue,
                };
                for l in wrap(&text, w.saturating_sub(2)) {
                    lines.push(Line::from(format!("  {l}").italic()));
                }
            }
        }
        lines.push(Line::default());
        let mut item = ListItem::new(Text::from(lines));
        if !g.lines.iter().any(|n| mine(n)) {
            item = item.style(Style::new().add_modifier(Modifier::DIM));
        }
        item
    }

    /// Who needs component `i`, one node per component: wanted ones first,
    /// those with only soft needs after the hard ones.
    fn by_nodes(&self, i: usize) -> Vec<ByNode<'_>> {
        let c = &self.map.comps[i];
        let mut nodes: Vec<ByNode> = vec![];
        for e in c.needed_by.iter().filter(|e| e.from != i) {
            match nodes.iter_mut().find(|g| g.from == e.from) {
                Some(g) => g.lines.push(e),
                None => nodes.push(ByNode {
                    from: e.from,
                    lines: vec![e],
                }),
            }
        }
        let order = |g: &ByNode| {
            2 * usize::from(!self.map.comps[g.from].wanted)
                + usize::from(g.lines.iter().all(|e| e.soft))
        };
        nodes.sort_by_key(order);
        nodes
    }

    fn by_item(&self, g: &ByNode, w: usize) -> ListItem<'static> {
        let f = &self.map.comps[g.from];
        let mut lines = vec![
            Line::from(vec![comp_mark(f), f.title.clone().bold()]),
            Line::from(
                format!(
                    "  {}{}",
                    f.label(),
                    if f.wanted { "" } else { ", not wanted" }
                )
                .dim(),
            ),
        ];
        for e in &g.lines {
            let mut meta = vec![];
            if e.soft {
                meta.push("soft".to_string());
            }
            if let Some(x) = &e.from_feature {
                meta.push(format!("its #{x}"));
            }
            if let Some(x) = &e.on_feature {
                meta.push(format!("needs #{x}"));
            }
            let text = match (&e.why, meta.is_empty()) {
                (Some(why), true) => why.clone(),
                (Some(why), false) => format!("{}: {why}", meta.join(", ")),
                (None, false) => meta.join(", "),
                (None, true) => continue,
            };
            for l in wrap(&text, w.saturating_sub(2)) {
                lines.push(Line::from(format!("  {l}").italic()));
            }
        }
        lines.push(Line::default());
        let item = ListItem::new(Text::from(lines));
        if f.wanted {
            item
        } else {
            item.style(Style::new().add_modifier(Modifier::DIM))
        }
    }

    /// Whether work item `label` is ready, blocked, or neither, as a line.
    fn work_state(&self, label: &str) -> Option<Line<'static>> {
        let w = &self.map.work[*self.work_at.get(label)?];
        if !w.wanted {
            return None;
        }
        if w.ready {
            return Some(Line::from(
                "Ready to work on · c copies a prompt".fg(Color::Green),
            ));
        }
        let names: Vec<String> = w.blocked_by.iter().map(|b| self.name(b)).collect();
        Some(Line::from(
            format!("Blocked by {}", names.join(", ")).fg(Color::Red),
        ))
    }

    /// A label as the page links it: the component's title, then #feature.
    fn name(&self, label: &str) -> String {
        match (self.comp_of(label), label.split_once('#')) {
            (Some(i), Some((_, f))) => format!("{} #{f}", self.map.comps[i].title),
            (Some(i), None) => self.map.comps[i].title.clone(),
            (None, _) => format!("{label} (not on the map)"),
        }
    }

    /// The middle card's lines, wrapped to `w`, and the lines of each feature.
    fn card(&self, w: usize) -> (Vec<Line<'static>>, Vec<Range<usize>>) {
        let c = &self.map.comps[self.sel];
        let mut o: Vec<Line> = vec![];
        let para = |o: &mut Vec<Line<'static>>, text: &str, style: Style| {
            for l in wrap(text, w) {
                o.push(Line::styled(l, style));
            }
        };
        let head = |o: &mut Vec<Line<'static>>, text: &str| {
            o.push(Line::default());
            o.push(Line::from(text.to_uppercase()).bold().fg(Color::Cyan));
        };
        para(&mut o, &c.title, Style::new().bold());
        o.push(Line::from(c.label().dim()));
        if !c.aka.is_empty() {
            para(
                &mut o,
                &format!("Also: {}", c.aka.join(", ")),
                Style::new().dim(),
            );
        }
        if stub(&c.summary) {
            o.push(Line::from("A stub: not described yet.".italic().dim()));
        } else {
            para(&mut o, &c.summary, Style::new());
        }
        let chip = |k: &str, v: &str| format!("[{k}{}{v}] ", if k.is_empty() { "" } else { " " });
        let mut chips: Vec<Span> = vec![
            chip("kind", c.kind.as_str()).into(),
            chip("intent", c.intent.as_str()).into(),
            if c.wanted {
                chip("", "wanted").fg(Color::Green)
            } else {
                chip("", "not wanted").dim()
            },
            chip("status", status_word(c.status.as_str())).into(),
            chip("research", c.research.as_str()).into(),
        ];
        if let Some(m) = &c.milestone {
            chips.push(chip("milestone", m).into());
        }
        if c.debt {
            chips.push(chip("", "research debt").fg(Color::Red));
        }
        // The chips wrap whole.
        let mut line: Vec<Span> = vec![];
        for chip in chips {
            if !line.is_empty() && line.iter().map(Span::width).sum::<usize>() + chip.width() > w {
                o.push(Line::from(std::mem::take(&mut line)));
            }
            line.push(chip);
        }
        o.push(Line::from(line));
        let (done, total) = progress(c);
        if !c.features.is_empty() && total > 0 {
            let fill = 20 * done / total;
            o.push(Line::from(vec![
                format!("{done} of {total} features done  ").into(),
                "█".repeat(fill).fg(Color::Green),
                "░".repeat(20 - fill).dim(),
            ]));
        }
        if c.features.is_empty()
            && let Some(l) = self.work_state(&c.label())
        {
            o.push(l);
        }

        let mut features = vec![];
        if !c.features.is_empty() {
            head(&mut o, "Features");
            for (k, f) in c.features.iter().enumerate() {
                let picked = self.feature == Some(k);
                let start = o.len();
                let mut title = format!("{}  #{}", f.title, f.id);
                if let Some(m) = &f.milestone {
                    title = format!("{title}  {m}");
                }
                for (n, l) in wrap(&title, w.saturating_sub(2)).into_iter().enumerate() {
                    let mark = if n == 0 {
                        status_mark(f.status.as_str())
                    } else {
                        "  ".into()
                    };
                    let line = Line::from(vec![mark, l.into()]);
                    o.push(if picked { line.reversed() } else { line });
                }
                let sub = |o: &mut Vec<Line<'static>>, text: &str, style: Style| {
                    for l in wrap(text, w.saturating_sub(4)) {
                        o.push(Line::styled(format!("    {l}"), style));
                    }
                };
                if let Some(n) = &f.note {
                    sub(&mut o, n, Style::new().dim());
                }
                if !f.needs.is_empty() {
                    let needs: Vec<String> = f
                        .needs
                        .iter()
                        .map(|l| {
                            let n = c.needs.iter().find(|n| {
                                n.from_feature.as_deref() == Some(&f.id)
                                    && n.on.as_deref() == Some(l)
                            });
                            let mark = match n {
                                Some(n) if n.target.is_some() && n.met => " (met)",
                                Some(n) if n.target.is_some() => " (not met)",
                                _ => "",
                            };
                            format!("{}{mark}", self.name(l))
                        })
                        .collect();
                    sub(&mut o, &format!("Needs {}", needs.join(", ")), Style::new());
                }
                if !matches!(f.status, FeatureStatus::Done | FeatureStatus::Dropped)
                    && let Some(l) = self.work_state(&format!("{}#{}", c.label(), f.id))
                {
                    let mut l = l;
                    l.spans.insert(0, "    ".into());
                    o.push(l);
                }
                features.push(start..o.len());
            }
        }

        head(&mut o, "Why it exists");
        if c.intent == Intent::Goal {
            o.push(Line::from("A goal: wanted for itself."));
        } else if !c.wanted {
            para(
                &mut o,
                if c.intent == Intent::Aspiration {
                    "An aspiration: nothing wanted needs it, so it isn't wanted."
                } else {
                    "Induced, but nothing wanted needs it, so it isn't wanted."
                },
                Style::new(),
            );
        } else if c.why_paths.is_empty() {
            o.push(Line::from("Wanted."));
        } else {
            for p in &c.why_paths {
                let chain: Vec<&str> = p
                    .iter()
                    .map(|&i| self.map.comps[i].title.as_str())
                    .collect();
                para(&mut o, &format!("• {}", chain.join(" → ")), Style::new());
            }
        }

        head(&mut o, "Description");
        if c.body.trim().is_empty() {
            o.push(Line::from("Nothing written yet.".dim()));
        } else {
            for src in c.body.trim().lines() {
                if src.trim().is_empty() {
                    o.push(Line::default());
                } else {
                    para(&mut o, src, Style::new());
                }
            }
        }
        if !c.refs.is_empty() {
            head(&mut o, "References");
            for r in &c.refs {
                para(&mut o, r, Style::new().fg(Color::Blue));
            }
        }
        let fuzzy: Vec<&Need> = c.needs.iter().filter(|n| n.fuzzy.is_some()).collect();
        if !fuzzy.is_empty() {
            head(&mut o, "Not researched yet");
            for n in fuzzy {
                para(
                    &mut o,
                    &format!("≈ {}", n.fuzzy.as_deref().unwrap_or("")),
                    Style::new(),
                );
            }
        }
        o.push(Line::default());
        o.push(Line::from(map_file(c).dim()));
        (o, features)
    }

    /// The roadmap's columns: (steps away, work item indexes by label).
    fn roadmap(&self) -> Vec<(usize, Vec<usize>)> {
        let n = self.map.milestones.len();
        let want = match self.milestone {
            0 => None,
            m if m <= n => Some(Some(self.map.milestones[m - 1].id.as_str())),
            _ => Some(None),
        };
        let mut cols: Vec<(usize, Vec<usize>)> = vec![(0, vec![])];
        for (k, w) in self.map.work.iter().enumerate() {
            if !w.wanted || want.is_some_and(|m| w.milestone.as_deref() != m) {
                continue;
            }
            let l = self.level.get(&k).copied().unwrap_or(0);
            match cols.iter_mut().find(|c| c.0 == l) {
                Some(c) => c.1.push(k),
                None => cols.push((l, vec![k])),
            }
        }
        cols.sort_by_key(|c| c.0);
        for c in &mut cols {
            c.1.sort_by(|&a, &b| self.map.work[a].label.cmp(&self.map.work[b].label));
        }
        cols
    }

    fn draw_roadmap(&mut self, f: &mut Frame, area: Rect) {
        let [top, body] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
        let cols = self.roadmap();
        let n = self.map.milestones.len();
        let ms = match self.milestone {
            0 => "All milestones".to_string(),
            m if m <= n => self.map.milestones[m - 1].title.clone(),
            _ => "No milestone".to_string(),
        };
        let total: usize = cols.iter().map(|c| c.1.len()).sum();
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![" ".into(), ms.bold(), "  (m to change)".dim()]),
                Line::from(
                    format!(
                        " {total} wanted work items. Ready now has nothing blocking it; each column after it waits only on work in the columns before."
                    )
                    .dim(),
                ),
            ]),
            top,
        );
        self.rm_col = self.rm_col.min(cols.len() - 1);
        let fit = ((body.width / 34).max(1) as usize).min(cols.len());
        let colw = body.width / fit as u16;
        // Scroll the columns only as far as it takes to show the picked one.
        if self.rm_col < self.rm_first {
            self.rm_first = self.rm_col;
        } else if self.rm_col >= self.rm_first + fit {
            self.rm_first = self.rm_col + 1 - fit;
        }
        self.rm_first = self.rm_first.min(cols.len() - fit);
        let first = self.rm_first;
        let rects = Layout::horizontal(vec![Constraint::Length(colw); fit]).split(body);
        for (slot, (ci, (l, items))) in cols.iter().enumerate().skip(first).take(fit).enumerate() {
            let w = colw.saturating_sub(2) as usize;
            let list_items: Vec<ListItem> = items.iter().map(|&k| self.work_item(k, w)).collect();
            let heights: Vec<usize> = list_items.iter().map(ListItem::height).collect();
            let title = if *l == 0 {
                format!(" Ready now ({}) ", items.len())
            } else {
                format!(
                    " {l} step{} away ({}) ",
                    if *l == 1 { "" } else { "s" },
                    items.len()
                )
            };
            let focused = ci == self.rm_col;
            let block = Block::bordered().title(title).border_style(if focused {
                Style::new().fg(Color::Cyan)
            } else {
                Style::new().dim()
            });
            let list = List::new(list_items)
                .block(block)
                .highlight_style(if focused {
                    Style::new().add_modifier(Modifier::REVERSED)
                } else {
                    Style::new()
                });
            let sel = if focused { Some(self.rm_row) } else { None };
            let mut state = ListState::default().with_selected(sel);
            f.render_stateful_widget(list, rects[slot], &mut state);
            self.hits.push((rects[slot], Hit::RoadCol(ci)));
            let inner = rects[slot].inner(Margin::new(1, 1));
            self.list_hits(inner, &heights, state.offset(), |n| Hit::Work(ci, n));
        }
    }

    fn work_item(&self, k: usize, w: usize) -> ListItem<'static> {
        let x = &self.map.work[k];
        let c = &self.map.comps[x.comp];
        let mut lines: Vec<Line> = wrap(&x.title, w.saturating_sub(2))
            .into_iter()
            .enumerate()
            .map(|(n, l)| {
                if n == 0 {
                    Line::from(vec![status_mark(x.status), l.bold()])
                } else {
                    Line::from(format!("  {l}").bold())
                }
            })
            .collect();
        let mut meta = vec![];
        if x.feature.is_some() {
            meta.push(c.title.clone());
        }
        if self.milestone == 0
            && let Some(m) = &x.milestone
        {
            meta.push(m.clone());
        }
        if !meta.is_empty() {
            lines.push(Line::from(format!("  {}", meta.join(" · ")).dim()));
        }
        lines.push(Line::from(format!("  {}", short(&x.label)).dim()));
        if !x.blocked_by.is_empty() {
            let names: Vec<String> = x
                .blocked_by
                .iter()
                .map(|b| {
                    let ready = self.work_at.get(b).is_some_and(|&b| self.map.work[b].ready);
                    format!("{}{}", self.name(b), if ready { " (ready)" } else { "" })
                })
                .collect();
            for l in wrap(
                &format!("waits on {}", names.join(", ")),
                w.saturating_sub(2),
            ) {
                lines.push(Line::from(format!("  {l}").italic()));
            }
        }
        lines.push(Line::default());
        ListItem::new(Text::from(lines))
    }

    fn draw_search(&mut self, f: &mut Frame, area: Rect) {
        let Some((q, hit)) = &self.search else { return };
        let w = area.width.min(90);
        let rect = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + 1,
            width: w,
            height: area.height.min(16),
        };
        f.render_widget(Clear, rect);
        let hits = self.search_hits(q);
        self.hits.push((rect, Hit::SearchBox));
        for n in 0..hits.len() {
            let y = rect.y + 3 + n as u16;
            if y + 1 < rect.bottom() {
                self.hits.push((
                    Rect {
                        y,
                        height: 1,
                        ..rect
                    },
                    Hit::Search(n),
                ));
            }
        }
        let mut lines = vec![
            Line::from(vec!["/ ".fg(Color::Cyan), q.clone().bold(), "▏".into()]),
            Line::default(),
        ];
        if hits.is_empty() && !q.trim().is_empty() {
            lines.push(Line::from("no component matches".dim()));
        }
        let hit = (*hit).min(hits.len().saturating_sub(1));
        for (n, &i) in hits.iter().enumerate() {
            let c = &self.map.comps[i];
            let mut l = Line::from(vec![
                comp_mark(c),
                c.title.clone().bold(),
                format!("  {}", c.label()).dim(),
                format!("  {}", if stub(&c.summary) { "" } else { &c.summary }).dim(),
            ]);
            if n == hit {
                l = l.reversed();
            }
            lines.push(l);
        }
        f.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .title(" Search: title, label, other names, summary · ⏎ open · esc close ")
                    .border_style(Style::new().fg(Color::Cyan)),
            ),
            rect,
        );
    }
}

struct NeedNode<'a> {
    target: Option<usize>,
    /// The path it points at, to group needs on the same component.
    key: String,
    lines: Vec<&'a Need>,
    /// 0 unmet hard, 1 met, 2 soft, 3 of dropped features, 4 missing, 5 fuzzy.
    rank: usize,
}

struct ByNode<'a> {
    from: usize,
    lines: Vec<&'a crate::map::Edge>,
}
