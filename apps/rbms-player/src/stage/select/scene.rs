//! What the song browser hands the renderer: the row list, the focused chart's detail panel, the
//! record modal, and the cache that keeps all of it from being rebuilt on every frame of the
//! continuous redraw loop.
#![allow(clippy::wildcard_imports)]

use crate::stage::select::SelectState;
use crate::*;

impl SelectState {
    /// Assemble the backend-agnostic [`SelectScene`] from the current select state. Built before the
    /// canvas is borrowed so the render pass can stay a pure data → pixels call (the cover texture is
    /// uploaded separately into [`cover_rect`]).
    ///
    /// A list with no rows carries an onboarding hint instead: a first-run "add a folder" call to
    /// action, a note that the search matched nothing, or a generic "no charts" message.
    pub(super) fn build_scene(&self, shared: &AppShared) -> SelectScene {
        let rows: Vec<SelectRow> = shared
            .select_items
            .iter()
            .map(|item| match item {
                SelectItem::Folder { label, .. } => SelectRow {
                    folder: true,
                    title: label.clone(),
                    mode_short: "",
                    mode_color: Color::GRAY,
                    level: String::new(),
                    difficulty_color: Color::GRAY,
                    lamp: Color::rgb(44, 44, 54),
                    folder_count: None,
                },
                SelectItem::Song(si) => {
                    let e = &shared.library.songs()[*si];
                    let lamp = shared.scores.best_clear_for_md5(&e.md5).map(|c| clear_label_color(clear_type_from_id(c)).1).unwrap_or(Color::rgb(44, 44, 54));
                    SelectRow {
                        folder: false,
                        title: e.title.clone(),
                        mode_short: mode_short(e.mode),
                        mode_color: mode_color(e.mode),
                        level: e.level.clone(),
                        difficulty_color: difficulty_color(e.difficulty),
                        lamp,
                        folder_count: None,
                    }
                }
            })
            .collect();

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
                    cover: if self.cover_rgba.is_some() { CoverState::Present } else { CoverState::None },
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
        let empty_hint = if !rows.is_empty() {
            None
        } else if shared.searching {
            Some(("NO RESULTS", "Try a different search  \u{00B7}  Esc to clear"))
        } else if shared.config.library.folders.is_empty() {
            Some(("WELCOME TO rbms", "Add your music folder \u{2014} press O or click FOLDERS below"))
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
            search: shared.searching.then(|| shared.search.clone()),
            sort: shared.sort.label(),
            empty_hint,
        }
    }

    /// Rebuild the cached select scene only when [`SelectState::select_key`] changes. The frame loop
    /// redraws continuously, so rebuilding every frame would re-walk and re-allocate the whole song
    /// list.
    pub(super) fn refresh_scene_cache(&mut self, shared: &AppShared) {
        let key = self.select_key(shared);
        if self.cached_key != Some(key) || self.cached_scene.is_none() {
            self.cached_scene = Some(self.build_scene(shared));
            self.cached_key = Some(key);
        }
    }
}
