//! What the song browser hands the renderer: the row list, the focused chart's detail panel, the
//! record modal, and the cache that keeps all of it from being rebuilt on every frame of the
//! continuous redraw loop.
//!
//! The rows are not read off the library here. The browser keeps one model of its list
//! ([`BarList`], a [`SelectBar`] for every row), and both screens that can draw the list are
//! converted down from it: the built-in browser's rows ([`row_of`]) and the bars a skin's wheel
//! turns through ([`wheel_bar`]).
#![allow(clippy::wildcard_imports)]

use rbms_render::skin_render::frame::SongBar;

use crate::stage::select::SelectState;
use crate::stage::select::list::{ChartFacts, course_bars};
use crate::*;

/// How long a chart counts as newly added, in seconds: a day (`BarRenderer.prepare`,
/// `3600 * 24`).
const NEW_FOR_SECS: i64 = 86_400;

/// The colour of the lamp of a row nothing has been recorded on.
const LAMP_UNLIT: Color = Color::rgb(44, 44, 54);

/// The colour of the lamp of a course the library cannot supply every chart of.
const LAMP_UNPLAYABLE: Color = Color::rgb(70, 30, 30);

/// What the first-run hint says under its title. The built-in browser has a row of buttons to point
/// at; a browser a skin draws has none, so it names the keys instead.
const WELCOME_BODY: &str = "Add your music folder \u{2014} press O or click FOLDERS below";
const WELCOME_BODY_SKINNED: &str = "Add your music folder \u{2014} press O   (H lists every key)";

/// What a [`BarList`] was built from: the list's generation, how many records the score book
/// holds, the tab on show, how often the course list and the chart facts have been read, the play
/// mode the folders were counted under, and whether they were built for a skin to read.
type BarsKey = (u64, usize, SelectTab, u64, u64, Option<Mode>, bool);

/// The browser's list as bars, and the same list as the bars a skin's wheel reads.
///
/// The model is rebuilt when what it was built from changes ([`BarsKey`]), which leaves out the
/// cursor: moving through a list rebuilds nothing. The wheel's bars are made from the model the
/// first time a skin's frame asks for them and again when the model changes or a chart on show
/// stops being new.
#[derive(Default)]
pub(super) struct BarList {
    key: Option<BarsKey>,
    bars: Vec<SelectBar>,
    wheel: Option<Wheel>,
}

/// The wheel's bars, and the second up to which they hold.
struct Wheel {
    bars: Vec<SongBar>,
    /// The last second every bar marked as new still is, or `None` when none is.
    fresh_until: Option<i64>,
}

impl BarList {
    /// The model: one bar for every row of the list on show.
    pub(super) fn bars(&self) -> &[SelectBar] {
        &self.bars
    }

    /// The bars a skin's wheel turns through at `now_secs` on the wall clock.
    pub(super) fn wheel(&mut self, now_secs: i64) -> &[SongBar] {
        let stale = self.wheel.as_ref().is_none_or(|wheel| wheel.fresh_until.is_some_and(|until| now_secs > until));
        if stale {
            let fresh_until = self.bars.iter().filter_map(new_until).filter(|until| now_secs <= *until).min();
            self.wheel = Some(Wheel { bars: self.bars.iter().map(|bar| wheel_bar(bar, now_secs)).collect(), fresh_until });
        }
        self.wheel.as_ref().map_or(&[], |wheel| wheel.bars.as_slice())
    }
}

/// The last second a bar counts as newly added, for the bars that can: a chart the song database
/// knows the arrival of.
fn new_until(bar: &SelectBar) -> Option<i64> {
    Some(bar.chart.as_ref()?.added_at? + NEW_FOR_SECS)
}

/// One bar as a skin's wheel reads it at `now_secs` on the wall clock (`BarRenderer.prepare` and
/// `render`, with every question they ask a bar answered).
pub(super) fn wheel_bar(bar: &SelectBar, now_secs: i64) -> SongBar {
    let chart = bar.chart.as_ref();
    SongBar {
        kind: bar.kind,
        title: bar.full_title(),
        is_new: new_until(bar).is_some_and(|until| now_secs <= until),
        level: chart.map_or(0, |chart| chart.level),
        difficulty: chart.map_or(0, |chart| chart.difficulty),
        lamp: i32::from(bar.lamp.unwrap_or_default()),
        rival_lamp: i32::from(bar.rival_lamp.unwrap_or_default()),
        trophy: bar.trophy,
        features: chart.map(|chart| chart.features).or(bar.course.as_ref().map(|course| course.features)).unwrap_or_default(),
        distribution: bar.distribution.map(Box::new),
    }
}

/// One bar as the row the built-in browser draws. `score_graph` is the SCORE GRAPH option, which
/// is what every rank on that screen is gated on.
fn row_of(bar: &SelectBar, score_graph: bool) -> SelectRow {
    let opens = SelectRow {
        folder: true,
        title: bar.title.clone(),
        mode_short: "",
        mode_color: Color::GRAY,
        level: String::new(),
        difficulty_color: Color::GRAY,
        lamp: LAMP_UNLIT,
        folder_count: None,
        dj_level: None,
        favorite: false,
    };
    if let Some(course) = &bar.course {
        let playable = bar.kind == BarKind::Course { complete: true };
        return SelectRow {
            level: course.badges.join(" "),
            difficulty_color: if playable { Color::WHITE } else { Color::RED },
            lamp: if playable { LAMP_UNLIT } else { LAMP_UNPLAYABLE },
            ..opens
        };
    }
    let Some(chart) = &bar.chart else {
        return SelectRow { folder: bar.kind.is_directory(), ..opens };
    };
    SelectRow {
        folder: false,
        mode_short: mode_short(chart.mode),
        mode_color: mode_color(chart.mode),
        level: chart.level_text.clone(),
        difficulty_color: difficulty_color(chart.difficulty),
        lamp: bar.lamp.map_or(LAMP_UNLIT, |clear| clear_label_color(clear_type_from_id(clear)).1),
        dj_level: chart.best.filter(|_| score_graph).map(|best| RANK_BANDS[dj_rank(best.ex, best.max_ex)].0),
        favorite: chart.favorite,
        ..opens
    }
}

impl SelectState {
    /// Assemble the backend-agnostic [`SelectScene`] from the current select state. Built before the
    /// canvas is borrowed so the render pass can stay a pure data → pixels call (the cover texture is
    /// uploaded separately into [`cover_rect`]).
    ///
    /// A list with no rows carries an onboarding hint instead: a first-run "add a folder" call to
    /// action, a note that the search matched nothing, or a generic "no charts" message.
    pub(super) fn build_scene(&self, shared: &AppShared) -> SelectScene {
        if self.tab == SelectTab::Courses {
            return self.build_course_scene(shared);
        }
        let rows: Vec<SelectRow> = self.bars.bars().iter().map(|bar| row_of(bar, shared.config.display.score_graph)).collect();

        let header = match shared.select_view {
            SelectView::Root => {
                shared.config.library.songs_folder.as_deref().and_then(|p| Path::new(p).file_name()).and_then(|n| n.to_str()).unwrap_or("ROOT").to_string()
            }
            SelectView::AllSongs => "ALL SONGS".into(),
            SelectView::TableLevels(ti) => shared.table_names.get(ti).cloned().unwrap_or_default(),
            SelectView::TableLevel(ti, li) => {
                let table = shared.table_names.get(ti).cloned().unwrap_or_default();
                let level = shared.table_levels.get(ti).and_then(|ls| ls.get(li)).map(|(l, _)| format!("LV {l}")).unwrap_or_default();
                format!("{table} / {level}")
            }
        };

        let detail = match shared.select_items.get(shared.sel) {
            Some(SelectItem::Song(si)) => {
                let e = &shared.library.songs()[*si];
                let d = self.focused_detail.as_ref();
                let bpm = match d {
                    Some(d) if (d.bpm_max - d.bpm_min).abs() >= 1.0 => format!("{}\u{2013}{}", d.bpm_min.round() as i32, d.bpm_max.round() as i32),
                    Some(d) => format!("{}", d.bpm_min.round() as i32),
                    None => format!("{}", e.init_bpm.round() as i32),
                };
                let notes = d
                    .map(|d| if d.long_notes > 0 { format!("{} ({}LN)", d.notes, d.long_notes) } else { d.notes.to_string() })
                    .unwrap_or_else(|| "\u{2026}".into());
                let length = d.map(|d| fmt_duration(d.duration_us)).unwrap_or_else(|| "\u{2026}".into());
                let total = if e.total > 0.0 { format!("{}", e.total.round() as i32) } else { "AUTO".into() };
                let genre_maker = match (e.genre.trim(), e.maker.trim()) {
                    ("", "") => String::new(),
                    ("", m) => m.to_string(),
                    (g, "") => g.to_string(),
                    (g, m) => format!("{g}  \u{00B7}  {m}"),
                };
                let density = d.filter(|d| !d.density.is_empty()).map(|d| DensityView {
                    bins: d.density.clone(),
                    peak: d.peak_density,
                    avg: d.avg_density,
                    end: d.end_density,
                });

                let recs = shared.scores.for_md5(&e.md5);
                let plays = recs.len();
                let clears = recs.iter().filter(|r| r.clear >= 2).count();
                let best = recs.iter().max_by(|a, b| a.clear.cmp(&b.clear).then(a.ex_score.cmp(&b.ex_score)));
                let rank_bar = best.filter(|_| shared.config.display.score_graph).map(|b| (b.ex_score, b.max_ex));
                let best = best.map(|b| shared.record_row_view(b, None));
                let recent = recs.iter().enumerate().map(|(ri, r)| shared.record_row_view(r, recs.get(ri + 1).copied())).collect();

                SelectDetail::Song(Box::new(DetailView {
                    accent: mode_color(e.mode),
                    title: e.title.clone(),
                    subtitle: e.subtitle.clone(),
                    artist: e.artist.clone(),
                    genre_maker,
                    mode_short: mode_short(e.mode),
                    mode_color: mode_color(e.mode),
                    level: e.level.clone(),
                    difficulty_color: difficulty_color(e.difficulty),
                    difficulty_name: difficulty_name(e.difficulty),
                    cover: if self.cover_image.is_some() { CoverState::Present } else { CoverState::None },
                    stats: vec![
                        StatCell { label: "BPM", value: bpm },
                        StatCell { label: "DENSITY", value: d.map(|d| format!("{}/s", d.avg_density.round() as i32)).unwrap_or_else(|| "\u{2026}".into()) },
                        StatCell { label: "NOTES", value: notes },
                        StatCell { label: "JUDGE", value: rank_label(e.rank) },
                        StatCell { label: "LENGTH", value: length },
                        StatCell { label: "TOTAL", value: total },
                    ],
                    density,
                    records: RecordsView { plays, clears, best, rank_bar, recent },
                }))
            }
            Some(SelectItem::Folder { label, .. }) => SelectDetail::Folder { label: label.clone(), count: 0 },
            None => SelectDetail::Empty,
        };

        let modal = self.record_modal.and_then(|ri| {
            let si = match shared.select_items.get(shared.sel) {
                Some(SelectItem::Song(si)) => *si,
                _ => return None,
            };
            let e = &shared.library.songs()[si];
            let recs = shared.scores.for_md5(&e.md5);
            let r = recs.get(ri)?;
            let (clear_label, clear_color) = clear_label_color(clear_type_from_id(r.clear));
            let (rank, rank_color) = if shared.config.display.score_graph {
                let (name, col) = RANK_BANDS[dj_rank(r.ex_score, r.max_ex)];
                (Some(name), col)
            } else {
                (None, Color::WHITE)
            };
            Some(SelectModal {
                title: e.title.clone(),
                clear_label,
                clear_color,
                when: fmt_datetime(r.played_at),
                sub: format!("{}   {}   GAUGE {}{}", r.mode, r.random, r.gauge, rule_version_note(r.rule_version)),
                counts: r.counts,
                ex: r.ex_score,
                max_ex: r.max_ex,
                rank,
                rank_color,
                max_combo: r.max_combo,
                total_notes: r.total_notes,
                bp: r.counts[3] + r.counts[4] + r.counts[5],
                empty_poor: r.empty_poor,
                gauge_value: r.gauge_value.round() as i32,
                index: ri,
                total: recs.len(),
                has_replay: r.replay_file.is_some(),
            })
        });

        let guide = if self.esc_quit_armed() {
            "PRESS ESC AGAIN TO QUIT"
        } else if shared.searching {
            "TYPE TO FILTER   BACKSPACE EDIT   ENTER OPEN   ESC CLEAR"
        } else {
            "\u{2191}\u{2193} MOVE   ENTER OPEN   ESC BACK"
        };
        let filter = self.filter.summary(&shared.config);
        let empty_hint = if !rows.is_empty() {
            None
        } else if shared.searching {
            Some(("NO RESULTS", "Try a different search  \u{00B7}  Esc to clear"))
        } else if filter.is_some() {
            Some(("NO CHARTS MATCH", "Press F2 to change the filter  \u{00B7}  Backspace there clears it"))
        } else if shared.config.library.folders.is_empty() {
            Some(("WELCOME TO rbms", if self.is_skinned(shared) { WELCOME_BODY_SKINNED } else { WELCOME_BODY }))
        } else {
            Some(("NO CHARTS", "Press O to manage folders  \u{00B7}  T for difficulty tables"))
        };
        SelectScene {
            rows,
            sel: shared.sel,
            header,
            guide,
            detail,
            modal,
            score_graph: shared.config.display.score_graph,
            search: shared.searching.then(|| self.search_display()),
            sort: shared.config.library.sort.label(),
            filter,
            empty_hint,
        }
    }

    /// The course tab's own scene: one row per course, and a detail panel naming the focused one.
    ///
    /// Courses are drawn through the same row type the song list uses, as folders: a course is a
    /// list of charts to be played in order rather than one chart, and its row carries the stage
    /// count and the constraint badges where a chart's mode and level would go.
    fn build_course_scene(&self, shared: &AppShared) -> SelectScene {
        let rows: Vec<SelectRow> = self.bars.bars().iter().map(|bar| row_of(bar, shared.config.display.score_graph)).collect();
        let detail = match self.bars.bars().get(self.courses.cursor()) {
            Some(SelectBar { title, course: Some(course), .. }) => SelectDetail::Folder { label: title.clone(), count: course.stages },
            _ => SelectDetail::Empty,
        };
        let empty_hint = rows.is_empty().then_some(("NO COURSES", "Put course json files in the courses folder next to your settings"));
        SelectScene {
            rows,
            sel: self.courses.cursor(),
            header: SelectTab::Courses.label().to_string(),
            guide: "\u{2191}\u{2193} MOVE   ENTER START   SHIFT+TAB SONGS   ESC BACK",
            detail,
            modal: None,
            score_graph: shared.config.display.score_graph,
            search: None,
            sort: shared.config.library.sort.label(),
            filter: None,
            empty_hint,
        }
    }

    /// Rebuild the cached select scene only when [`SelectState::select_key`] changes. The frame loop
    /// redraws continuously, so rebuilding every frame would re-walk and re-allocate the whole song
    /// list.
    pub(super) fn refresh_scene_cache(&mut self, shared: &AppShared) {
        self.refresh_bars(shared);
        let key = self.select_key(shared);
        if self.cached_key != Some(key) || self.cached_scene.is_none() {
            self.cached_scene = Some(self.build_scene(shared));
            self.cached_key = Some(key);
        }
    }
}

impl SelectState {
    /// Bring the list's model up to date: read the chart facts a skin's wheel labels a chart by when
    /// a skin is drawing and they have not been read for this library, and rebuild the bars when
    /// anything they are built from has changed.
    pub(super) fn refresh_bars(&mut self, shared: &AppShared) {
        let skinned = shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT);
        if !self.facts.serve(&shared.library, skinned) {
            self.facts = if skinned { ChartFacts::read(shared.song_db.as_ref(), &shared.library) } else { ChartFacts::unread(&shared.library) };
            self.facts_generation = self.facts_generation.wrapping_add(1);
        }
        let mode = self.applied.and_then(|(_, filter, _)| filter.mode);
        let key = (shared.select_gen, shared.scores.records().len(), self.tab, self.courses_generation, self.facts_generation, mode, skinned);
        if self.bars.key == Some(key) {
            return;
        }
        let bars = match self.tab {
            SelectTab::Songs => shared.select_bars(&self.facts, mode, skinned),
            SelectTab::Courses => course_bars(&self.courses, &shared.library, &self.facts),
        };
        self.bars = BarList { key: Some(key), bars, wheel: None };
    }

    /// Where the cursor is in the list on show.
    pub(super) fn cursor(&self, shared: &AppShared) -> usize {
        match self.tab {
            SelectTab::Songs => shared.sel,
            SelectTab::Courses => self.courses.cursor(),
        }
    }
}
