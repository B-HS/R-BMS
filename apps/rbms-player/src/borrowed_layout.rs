//! Playing a chart laid out as a replay of it was: the replay's lane arrangement and seed are
//! borrowed for a run the player plays by hand (LAYOUT PLAY). Nothing else of the replay is: the
//! gauge, SCRATCH AUTO, the LN MODE and the judge are the player's own, so the run is judged and
//! scored as any other and is kept and sent as one, stating the arrangement and the seed it was laid
//! out with. See `docs/reference/2026-10-24-replay-pattern-play-survey.md`.
//!
//! A borrowed layout lasts for the run it was borrowed for and for every run of that chart started
//! again laid out as it was. Any other way on -- the browser, the next song, a new arrangement --
//! lets go of it and the player's own arrangement is back in the settings.

use std::fmt;

use rbms_chart::shuffle::NoteOption;
use rbms_ir::{RandomOption, ReplayData};
use rbms_model::Language;
use rbms_model::translations::Catalog;
use rbms_render::text::language;
use rbms_store::{Replay, ScoreRecord};

use crate::AppShared;
use crate::app_play::OwnPlayOptions;
use crate::format::fmt_datetime;
use crate::ghost::GhostSource;
use crate::notify::{Level, notify_text, notify_with};
use crate::stage::{LoadingState, Stage, Transition};
use crate::text::layout_play::LayoutPlayText;
use crate::text::notices::NoticeText;

/// How much of a chart hash a refusal quotes.
const QUOTED_HASH_CHARS: usize = 8;

/// The name the score server gives the arrangement this player calls ROTATE.
const SERVER_ROTATE_NAME: &str = "Spiral";

/// What a run is asked to borrow the layout of: the chart the replay was recorded on, the lane
/// arrangement as the replay names it -- before any conversion, so an arrangement this player does
/// not have is seen for what it is -- the seed it was laid out with, and where it came from, for
/// the line that says whose layout the run is played on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutRequest {
    pub(crate) md5: String,
    pub(crate) random_label: String,
    pub(crate) seed: u64,
    pub(crate) source: String,
}

impl LayoutRequest {
    /// The request a replay downloaded from the score server makes, for a row of the ranking of the
    /// chart `row_md5`, whose player is `source`.
    ///
    /// The arrangement is named as the wire names it, so a CONVERGE is seen for what it is and not as
    /// the OFF it is played back as; a replay that names none is the OFF the score server takes a
    /// run for that names none. The chart is the one the replay states, and the row's own when it
    /// states none, which is what the row's ranking asked for.
    pub(crate) fn of_downloaded_replay(data: &ReplayData, row_md5: &str, source: String) -> LayoutRequest {
        let stated = data.chart.as_ref().map(|chart| chart.md5.as_str()).filter(|md5| !md5.is_empty());
        LayoutRequest {
            md5: stated.unwrap_or(row_md5).to_string(),
            random_label: format!("{:?}", data.random.unwrap_or(RandomOption::Off)),
            seed: data.seed.unwrap_or_default(),
            source,
        }
    }
}

/// A layout a run has borrowed: the arrangement and the seed its lanes are laid out with, and the
/// replay they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BorrowedLayout {
    pub(crate) random: NoteOption,
    pub(crate) seed: u64,
    pub(crate) source: String,
}

/// Why a layout cannot be borrowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LayoutRefusal {
    /// The replay is of another chart than the one to be played, or the chart is not in the
    /// library. The hashes are the replay's and the chart's, the latter empty when there is no chart.
    ChartMismatch { replay: String, chart: String },
    /// The replay names a shuffled arrangement without the seed it was laid out from.
    UnknownLayout,
    /// The replay was played on an arrangement this player cannot lay out.
    UnsupportedOption(String),
    /// The run to be started would not be one the player plays by hand on its own: a course stage,
    /// a practice slice or a run that plays itself.
    NotFreePlay,
    /// The replay is not one this machine recorded and shows OFF, which a replay from the score
    /// server shows for CONVERGE as well ([`crate::ir_replay::note_option_from_ir`]).
    ArrangementUnknown,
}

impl LayoutRefusal {
    /// The refusal in `language`.
    pub(crate) fn in_language(&self, language: Language) -> String {
        match self {
            LayoutRefusal::ChartMismatch { replay, chart } if chart.is_empty() => {
                NoticeText::LayoutRefusedChartNotInTheLibrary.fill(language, &[&quoted_hash(replay)])
            }
            LayoutRefusal::ChartMismatch { replay, chart } => {
                NoticeText::LayoutRefusedAnotherChart.fill(language, &[&quoted_hash(replay), &quoted_hash(chart)])
            }
            LayoutRefusal::UnknownLayout => NoticeText::LayoutRefusedNoSeed.get(language).to_string(),
            LayoutRefusal::UnsupportedOption(label) => NoticeText::LayoutRefusedUnsupportedOption.fill(language, &[label]),
            LayoutRefusal::NotFreePlay => NoticeText::LayoutRefusedNotFreePlay.get(language).to_string(),
            LayoutRefusal::ArrangementUnknown => LayoutPlayText::ArrangementUnknown.get(language).to_string(),
        }
    }

    /// Say the refusal over every screen: standard error gets it in English at once, and the screen
    /// gets it in the language it is drawn in when it takes the message.
    pub(crate) fn report(self) {
        notify_with(Level::Warn, move |language| self.in_language(language));
    }
}

impl fmt::Display for LayoutRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.in_language(language()))
    }
}

/// The start of a chart hash, as much of it as a message quotes.
fn quoted_hash(hash: &str) -> String {
    hash.chars().take(QUOTED_HASH_CHARS).collect()
}

/// The arrangement a replay's label names, or `None` when it names none this player has. The label
/// is read as the replay file writes it ([`NoteOption::label`] and its aliases) or as the score
/// server names it; anything else, the server's CONVERGE among them, is no arrangement here.
fn arrangement_of_label(label: &str) -> Option<NoteOption> {
    if label.eq_ignore_ascii_case(NoteOption::Off.label()) {
        return Some(NoteOption::Off);
    }
    if label.eq_ignore_ascii_case(SERVER_ROTATE_NAME) {
        return Some(NoteOption::Rotate);
    }
    Some(NoteOption::from_str(label)).filter(|option| *option != NoteOption::Off)
}

/// Whether `option` lays the notes out one by one rather than lane by lane, which is done on the
/// chart after its undefined long notes have been resolved by the LN MODE: the same seed lays such a
/// chart out alike only under the same LN MODE.
fn lays_out_note_by_note(option: NoteOption) -> bool {
    matches!(option, NoteOption::SRandom | NoteOption::HRandom | NoteOption::AllScratch)
}

/// Check `request` against the chart it is to be played on (its md5, `None` when there is no such
/// chart) and against whether the run would be one the player plays by hand on its own, and answer
/// the layout it borrows.
///
/// The hashes are compared without regard to case. A seed of zero lays nothing out that this player
/// recorded (a fresh seed is never zero) and is what the score server reads a seed it could not hold
/// as, so it is refused for any arrangement but OFF, which needs no seed.
pub(crate) fn check_layout_request(request: &LayoutRequest, chart_md5: Option<&str>, by_hand: bool) -> Result<BorrowedLayout, LayoutRefusal> {
    if !by_hand {
        return Err(LayoutRefusal::NotFreePlay);
    }
    let chart = chart_md5.unwrap_or_default();
    if request.md5.is_empty() || !request.md5.eq_ignore_ascii_case(chart) {
        return Err(LayoutRefusal::ChartMismatch { replay: request.md5.clone(), chart: chart.to_string() });
    }
    let random = arrangement_of_label(&request.random_label).ok_or_else(|| LayoutRefusal::UnsupportedOption(request.random_label.clone()))?;
    if request.seed == 0 && random != NoteOption::Off {
        return Err(LayoutRefusal::UnknownLayout);
    }
    Ok(BorrowedLayout { random, seed: request.seed, source: request.source.clone() })
}

/// The run of this machine a layout was borrowed from: the replay it kept and what its record says
/// it came to, which the runs on the layout are held against note by note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeptRun {
    pub(crate) ghost: GhostSource,
    pub(crate) ex: u32,
}

impl KeptRun {
    /// The run `record` is of, when it kept a replay.
    fn of(record: &ScoreRecord) -> Option<KeptRun> {
        GhostSource::of(record).map(|ghost| KeptRun { ghost, ex: record.ex_score })
    }
}

/// A borrowed layout as the app holds it: the layout, the chart it was borrowed for, whether the
/// next chart load is to be laid out with it, and the run of this machine it came from, if it did.
/// A load that is not to be laid out with it lets go of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Borrowing {
    layout: BorrowedLayout,
    md5: String,
    for_next_load: bool,
    kept: Option<KeptRun>,
}

impl AppShared {
    /// Borrow the layout `request` names for a run of the library's chart `chart_index`, and start
    /// that run. A replay being watched ends first and the player's own options are back; the run's
    /// arrangement is the borrowed one for as long as the layout is held, the rest of its options
    /// the player's.
    ///
    /// Started from the browser, the run is opened over it as any chart the browser starts is. Over a
    /// replay being watched, the run takes the place of the replay's, as a run started again from a
    /// result screen takes the place of that screen.
    pub(crate) fn borrow_layout(&mut self, request: LayoutRequest, chart_index: usize) -> Result<Transition, LayoutRefusal> {
        let layout = check_layout_request(&request, self.chart_md5_at(chart_index), self.plays_by_hand())?;
        let over_a_watched_replay = self.replay.is_some();
        self.end_replay();
        notify_text(Level::Info, NoticeText::LayoutOf, &[&layout.source, &layout.random.label()]);
        self.borrowed_layout = Some(Borrowing { layout, md5: request.md5, for_next_load: true, kept: None });
        if over_a_watched_replay {
            self.release_play_audio();
            return Ok(Transition::To(Stage::Loading(LoadingState::song(chart_index))));
        }
        Ok(Transition::Open(self.decided_song_stage(chart_index)))
    }

    /// Borrow the layout of the replay being watched for a run of its chart played by hand
    /// ([`AppShared::borrow_layout`]); `None` when no replay is being watched.
    ///
    /// The chart is the library's of the replay's hash. A replay a run of this machine kept is that
    /// run's, and the runs on its layout are held against it note by note under its name
    /// ([`AppShared::borrowed_kept_run`]). Any other replay may have come from the score server,
    /// which shows CONVERGE as OFF, so one that would be borrowed as OFF is refused instead.
    pub(crate) fn borrow_layout_of_watched_replay(&mut self) -> Option<Result<Transition, LayoutRefusal>> {
        let replay = self.replay.as_ref()?;
        let record = self.record_that_kept(replay);
        let source = record.map_or_else(|| LayoutPlayText::WatchedReplay.get(language()).to_string(), |record| fmt_datetime(record.played_at));
        let kept = record.and_then(KeptRun::of);
        let chart_index = self.library.songs().iter().position(|entry| entry.md5.eq_ignore_ascii_case(&replay.md5));
        let request = LayoutRequest { md5: replay.md5.clone(), random_label: replay.random.clone(), seed: replay.seed, source };
        let Some(chart_index) = chart_index else {
            return Some(Err(LayoutRefusal::ChartMismatch { replay: request.md5, chart: String::new() }));
        };
        if kept.is_none() {
            let checked = check_layout_request(&request, self.chart_md5_at(chart_index), self.plays_by_hand());
            if let Err(refusal) = checked.and_then(|layout| if layout.random == NoteOption::Off { Err(LayoutRefusal::ArrangementUnknown) } else { Ok(layout) })
            {
                return Some(Err(refusal));
            }
        }
        let started = self.borrow_layout(request, chart_index);
        if let Some(borrowing) = self.borrowed_layout.as_mut().filter(|_| started.is_ok()) {
            borrowing.kept = kept;
        }
        Some(started)
    }

    /// Borrow the layout of `replay`, the replay `record` of this machine kept, for a run of the
    /// library's chart `chart_index` played by hand ([`AppShared::borrow_layout`]), as the record
    /// modal of the browser asks for it.
    ///
    /// The runs on the layout are held against the kept run under its name, by its replay note by
    /// note ([`AppShared::borrowed_kept_run`]), as they are when the replay is borrowed from while it
    /// is watched. Nothing is borrowed, and no run is held against, when the layout is refused.
    pub(crate) fn borrow_layout_of_kept_run(&mut self, record: &ScoreRecord, replay: &Replay, chart_index: usize) -> Result<Transition, LayoutRefusal> {
        let request = LayoutRequest { md5: replay.md5.clone(), random_label: replay.random.clone(), seed: replay.seed, source: fmt_datetime(record.played_at) };
        let started = self.borrow_layout(request, chart_index)?;
        if let Some(borrowing) = self.borrowed_layout.as_mut() {
            borrowing.kept = KeptRun::of(record);
        }
        Ok(started)
    }

    /// Whether a run started now would be one the player plays by hand on its own: no course stage,
    /// no practice slice and not a run that plays itself.
    fn plays_by_hand(&self) -> bool {
        self.course_run.is_none() && !self.practice_requested && !self.run_plays_itself()
    }

    /// The hash of the library's chart `chart_index`, `None` when there is no such chart.
    fn chart_md5_at(&self, chart_index: usize) -> Option<&str> {
        self.library.songs().get(chart_index).map(|entry| entry.md5.as_str())
    }

    /// The record of this machine that kept `replay`: of its chart, and whose replay file holds the
    /// very run, its seed, arrangement and inputs.
    fn record_that_kept(&self, replay: &Replay) -> Option<&ScoreRecord> {
        let holds_the_run = |file: &String| {
            Replay::load(&self.replay_dir.join(file)).is_ok_and(|kept| kept.seed == replay.seed && kept.random == replay.random && kept.events == replay.events)
        };
        self.scores.for_md5(&replay.md5).into_iter().find(|record| record.replay_file.as_ref().is_some_and(holds_the_run))
    }

    /// The layout the runs of the chart are borrowing, when they are.
    pub(crate) fn borrowed_layout(&self) -> Option<&BorrowedLayout> {
        self.borrowed_layout.as_ref().map(|borrowing| &borrowing.layout)
    }

    /// The layout the runs of the chart `md5` are borrowing and the run of this machine it came
    /// from, when they borrow one from such a run. Those runs are held against that run, by its
    /// replay note by note, in place of the TARGET row ([`crate::app_result::run_target`]).
    pub(crate) fn borrowed_kept_run(&self, md5: &str) -> Option<(&BorrowedLayout, &KeptRun)> {
        let borrowing = self.borrowed_layout.as_ref().filter(|borrowing| borrowing.md5.eq_ignore_ascii_case(md5))?;
        borrowing.kept.as_ref().map(|kept| (&borrowing.layout, kept))
    }

    /// Hand the borrowed layout on to the run the result screen or a failed run is starting again:
    /// kept when the run is started laid out as it was, which is what a carried seed of the borrowed
    /// one says, and let go of otherwise.
    pub(crate) fn hand_borrowed_layout_on(&mut self) {
        let carried = self.retry_seed;
        match self.borrowed_layout.as_mut().filter(|borrowing| Some(borrowing.layout.seed) == carried) {
            Some(borrowing) => borrowing.for_next_load = true,
            None => self.let_go_of_borrowed_layout(),
        }
    }

    /// Let go of a borrowed layout, if one is held, and put the player's own arrangement back.
    pub(crate) fn let_go_of_borrowed_layout(&mut self) {
        if self.borrowed_layout.take().is_some() {
            self.put_back_own_play_options();
        }
    }

    /// The arrangement and seed the chart of `md5` being loaded is laid out with when it is the
    /// chart a layout was borrowed for and the load is the one the layout was handed to. The
    /// settings hold the borrowed arrangement while it is played, the player's own kept to be
    /// saved in its place and put back after ([`AppShared::settings_to_save`]). Any other load
    /// lets go of the layout.
    pub(crate) fn lay_out_as_borrowed(&mut self, md5: &str) -> Option<(NoteOption, u64)> {
        let handed = self.borrowed_layout.as_mut().filter(|borrowing| borrowing.for_next_load && borrowing.md5.eq_ignore_ascii_case(md5));
        let Some(borrowing) = handed else {
            self.let_go_of_borrowed_layout();
            return None;
        };
        borrowing.for_next_load = false;
        let BorrowedLayout { random, seed, .. } = borrowing.layout;
        let own = OwnPlayOptions::of(&self.config.play);
        self.own_play_options.get_or_insert(own);
        self.config.play.random = random;
        Some((random, seed))
    }

    /// Say so when the borrowed layout may not come out as the replay's did: an arrangement laid out
    /// note by note, on a chart whose long notes the LN MODE decides, matches only under the LN MODE
    /// the replay was recorded under.
    pub(crate) fn warn_of_a_borrowed_layout_the_ln_mode_decides(&self, ln_mode_decides_flavour: bool, ln_mode: &str) {
        let Some(layout) = self.borrowed_layout().filter(|layout| ln_mode_decides_flavour && lays_out_note_by_note(layout.random)) else {
            return;
        };
        notify_text(Level::Warn, NoticeText::LayoutOfLnModeDecides, &[&layout.source, &layout.random.label(), &ln_mode]);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};
    use rbms_store::SCORE_LN_MODE_FROM_CHART;

    use super::*;
    use crate::app_play::decode_chart;
    use crate::stage::PlayState;
    use crate::{App, Config, LaunchOptions, Library, SelectItem};

    /// A refusal is said over the screen in the language the screen is drawn in when it takes the
    /// message, and on standard error in English.
    #[test]
    fn a_refusal_is_reported_in_english_at_once_and_worded_for_the_screen_when_it_is_taken() {
        crate::notify::exclusive(|| {
            LayoutRefusal::NotFreePlay.report();
            rbms_render::text::set_language(Language::Korean);
            let mut taken = Vec::new();
            crate::notify::drain(&mut taken);
            rbms_render::text::set_language(Language::English);
            assert_eq!(taken, [(Level::Warn, LayoutRefusal::NotFreePlay.in_language(Language::Korean))]);
            assert_eq!(taken[0].1, "배치를 빌리지 않음: 코스나 연습이 아닌 직접 플레이에서만 배치를 빌릴 수 있음");
            assert_eq!(LayoutRefusal::NotFreePlay.to_string(), LayoutRefusal::NotFreePlay.in_language(Language::English));
        });
    }

    /// A seven-key chart with a note on every key lane in rows of its own and one on the turntable,
    /// so every arrangement but OFF moves some note.
    const CHART: &[u8] = b"#PLAYER 1\n#TITLE Borrowed\n#BPM 120\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n#00111:01000000\n#00112:00010000\n#00113:00000100\n#00114:00000001\n#00215:01000000\n#00218:00010000\n#00219:00000100\n#00216:00000001\n";

    /// The path the chart is listed under in the library.
    const CHART_NAME: &str = "borrowed.bms";

    /// The seed the replay a test borrows from was laid out with.
    pub(crate) const REPLAY_SEED: u64 = 0x5EED;

    /// How long a key stays down, the step a run is played at, and how far past the chart's own
    /// playing time it is played.
    const KEY_HELD_US: i64 = 50_000;
    const RUN_FRAME_US: i64 = 10_000;
    const RUN_MARGIN_US: i64 = 1_000_000;

    /// The gauge a refused course is opened on.
    const COURSE_OPENING_GAUGE: f32 = 100.0;

    /// The chart as the load path decodes it.
    fn decoded() -> crate::app_play::DecodedChart {
        decode_chart(CHART, CHART_NAME).expect("the chart decodes")
    }

    /// The chart's hash as the load path reads it.
    fn chart_md5() -> String {
        decoded().md5
    }

    /// An app with no score server set up, nobody signed in and the chart in its library as the one
    /// song of the browser's list, keeping its settings and scores in a folder of the test's own.
    pub(crate) fn a_library_of_one_chart(tag: &str) -> AppShared {
        let folder = std::env::temp_dir().join(format!("rbms-borrowed-layout-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the test's folder is writable");
        let mut config = Config::default();
        config.play.autoplay = false;
        let mut shared = App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron")).shared;
        shared.audio_failed = true;
        let decoded = decoded();
        let entry = rbms_library::SongEntry {
            path: PathBuf::from(CHART_NAME),
            title: decoded.model.meta.title.clone(),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: String::new(),
            difficulty: 0,
            init_bpm: decoded.model.init_bpm,
            rank: 2,
            total: decoded.model.meta.total,
            mode: decoded.mode,
            md5: decoded.md5,
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        };
        shared.library = Library::from_songs(vec![entry]);
        shared.select_items = vec![SelectItem::Song(0)];
        shared.sel = 0;
        shared
    }

    /// A request for the layout of a replay of the chart played on `label` from `seed`.
    pub(crate) fn request(label: &str, seed: u64) -> LayoutRequest {
        LayoutRequest { md5: chart_md5(), random_label: label.to_string(), seed, source: "a replay".to_string() }
    }

    /// The arrangement and the seed the next load of the chart lays it out with, as the load reads
    /// them, the seed a run started again hands on included.
    fn next_layout(shared: &mut AppShared) -> (NoteOption, u64) {
        shared.chart_path = CHART_NAME.to_string();
        let carried_seed = shared.retry_seed.take();
        shared.layout_to_load(&chart_md5(), carried_seed)
    }

    /// The next run of the chart, laid out and set up as the load sets it up.
    pub(crate) fn next_run(shared: &mut AppShared) -> PlayState {
        let (random, seed) = next_layout(shared);
        let mut decoded = decoded();
        crate::app_play::lay_out_notes(&mut decoded.model, random, seed, None, &decoded.mode);
        shared.mode = decoded.mode;
        let session = PlaySession::new(decoded.model, SessionOptions { seed, gauge: shared.config.play.gauge, ..SessionOptions::default() });
        PlayState::new(session, HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
    }

    /// Play `state` through by hand, every note pressed on its time.
    pub(crate) fn play_through(state: &mut PlayState) {
        let mut inputs: Vec<(i64, usize, bool)> = Vec::new();
        for line in &state.session.model().timelines {
            for (lane, _) in line.notes.iter().enumerate().filter(|(_, note)| note.is_some()) {
                inputs.extend([(line.time_us, lane, true), (line.time_us + KEY_HELD_US, lane, false)]);
            }
        }
        inputs.sort_unstable();
        let run_us = state.session.play_time_ms() * rbms_skin::timer::MICROS_PER_MILLI + RUN_MARGIN_US;
        let mut next = 0;
        for frame in 0..=run_us / RUN_FRAME_US {
            let now_us = frame * RUN_FRAME_US;
            while let Some((at_us, lane, press)) = inputs.get(next).copied().filter(|(at_us, _, _)| *at_us <= now_us) {
                if press {
                    state.session.press(lane, at_us, &mut NullSink);
                } else {
                    state.session.release(lane, at_us);
                }
                next += 1;
            }
            state.session.tick(SessionClock::at(now_us), &mut NullSink);
        }
    }

    /// A replay of the chart recorded on `random` from the replay seed, with options of its own.
    fn watched_replay(random: NoteOption) -> rbms_store::Replay {
        rbms_store::Replay {
            chart_path: CHART_NAME.to_string(),
            md5: chart_md5(),
            mode: String::new(),
            random: random.label().to_string(),
            seed: REPLAY_SEED,
            offset_ms: 0,
            scratch_auto: true,
            gauge: String::new(),
            judge: Default::default(),
            events: Vec::new(),
        }
    }

    /// The EX and the day of the run of this machine a test borrows the layout of.
    const KEPT_EX: u32 = 321;
    const KEPT_PLAYED_AT: i64 = 1_700_000_000_000;

    /// The record of a run of the chart that kept `replay_file` as its replay.
    fn kept_record(replay_file: Option<&str>) -> ScoreRecord {
        ScoreRecord {
            md5: chart_md5(),
            title: String::new(),
            mode: rbms_model::Mode::BEAT_7K.name.to_string(),
            clear: 5,
            ex_score: KEPT_EX,
            max_ex: 1000,
            counts: [0; 6],
            empty_poor: 0,
            max_combo: 0,
            total_notes: 500,
            gauge: String::new(),
            gauge_value: 100.0,
            random: String::new(),
            played_at: KEPT_PLAYED_AT,
            replay_file: replay_file.map(str::to_string),
            rule_version: rbms_store::SCORE_RULE_VERSION,
            ln_mode: SCORE_LN_MODE_FROM_CHART.to_string(),
            assisted: false,
            assist: 0,
            log_id: None,
        }
    }

    #[test]
    fn a_kept_run_lends_its_layout_and_the_runs_on_it_are_held_against_that_run() {
        let mut shared = a_library_of_one_chart("kept-run");
        let record = kept_record(Some("kept.ron"));
        let replay = watched_replay(NoteOption::Random);
        let opened = shared.borrow_layout_of_kept_run(&record, &replay, 0);
        assert!(matches!(opened, Ok(Transition::Open(_))), "the browser does not open the run over itself");
        let source = fmt_datetime(KEPT_PLAYED_AT);
        assert_eq!(shared.borrowed_layout(), Some(&BorrowedLayout { random: NoteOption::Random, seed: REPLAY_SEED, source: source.clone() }));
        let (layout, kept) = shared.borrowed_kept_run(&chart_md5()).expect("the run on the layout is not held against the kept run");
        assert_eq!((layout.source.as_str(), kept.ex), (source.as_str(), KEPT_EX));
        assert_eq!(Some(&kept.ghost), GhostSource::of(&record).as_ref(), "the kept run is not the one its record names");
        assert_eq!(next_layout(&mut shared), (NoteOption::Random, REPLAY_SEED));
    }

    #[test]
    fn a_record_that_kept_no_replay_lends_a_layout_and_no_run() {
        let mut shared = a_library_of_one_chart("kept-nothing");
        shared.borrow_layout_of_kept_run(&kept_record(None), &watched_replay(NoteOption::Mirror), 0).expect("the layout is borrowed");
        assert!(shared.borrowed_layout().is_some());
        assert!(shared.borrowed_kept_run(&chart_md5()).is_none(), "a run that kept no replay was held against");
    }

    #[test]
    fn a_kept_run_whose_layout_is_refused_borrows_nothing_and_holds_nothing() {
        let record = kept_record(Some("kept.ron"));
        let refused = |shared: &mut AppShared, replay: &Replay, chart_index: usize| shared.borrow_layout_of_kept_run(&record, replay, chart_index).err();
        let mut shared = a_library_of_one_chart("kept-refused");
        let other_chart = Replay { md5: "0123456789abcdef".to_string(), ..watched_replay(NoteOption::Random) };
        assert!(matches!(refused(&mut shared, &other_chart, 0), Some(LayoutRefusal::ChartMismatch { .. })));
        let no_seed = Replay { seed: 0, ..watched_replay(NoteOption::Random) };
        assert_eq!(refused(&mut shared, &no_seed, 0), Some(LayoutRefusal::UnknownLayout));
        assert!(matches!(refused(&mut shared, &watched_replay(NoteOption::Random), 1), Some(LayoutRefusal::ChartMismatch { .. })), "no chart at that index");
        shared.course_run = Some(rbms_course::CourseRun::new(rbms_course::Course::default(), COURSE_OPENING_GAUGE));
        assert_eq!(refused(&mut shared, &watched_replay(NoteOption::Random), 0), Some(LayoutRefusal::NotFreePlay));
        assert!(shared.borrowed_layout().is_none() && shared.borrowed_kept_run(&chart_md5()).is_none());
    }

    #[test]
    fn a_downloaded_replay_is_asked_for_as_the_wire_names_it() {
        let md5 = chart_md5();
        let data = |random, seed, stated: &str| ReplayData {
            random,
            seed,
            chart: Some(rbms_ir::ChartId { md5: stated.to_string(), sha256: String::new() }),
            ..ReplayData::default()
        };
        let asked = |data: &ReplayData| LayoutRequest::of_downloaded_replay(data, &md5, "Alice".to_string());

        let spiral = asked(&data(Some(RandomOption::Spiral), Some(REPLAY_SEED), &md5));
        assert_eq!((spiral.random_label.as_str(), spiral.seed, spiral.source.as_str()), ("Spiral", REPLAY_SEED, "Alice"));
        assert_eq!(check_layout_request(&spiral, Some(&md5), true).map(|layout| layout.random), Ok(NoteOption::Rotate));

        let converge = asked(&data(Some(RandomOption::Converge), Some(REPLAY_SEED), &md5));
        assert_eq!(
            check_layout_request(&converge, Some(&md5), true),
            Err(LayoutRefusal::UnsupportedOption("Converge".to_string())),
            "a CONVERGE was taken for OFF"
        );

        let unstated = asked(&data(None, None, &md5));
        assert_eq!((unstated.random_label.as_str(), unstated.seed), ("Off", 0), "a replay naming no arrangement is the server's own default");
        assert!(check_layout_request(&unstated, Some(&md5), true).is_ok());

        let no_seed = asked(&data(Some(RandomOption::Random), None, &md5));
        assert_eq!(check_layout_request(&no_seed, Some(&md5), true), Err(LayoutRefusal::UnknownLayout));

        let elsewhere = asked(&data(Some(RandomOption::Random), Some(REPLAY_SEED), "0123456789abcdef"));
        assert!(
            matches!(check_layout_request(&elsewhere, Some(&md5), true), Err(LayoutRefusal::ChartMismatch { .. })),
            "a replay of another chart was borrowed from"
        );

        let silent = asked(&data(Some(RandomOption::Random), Some(REPLAY_SEED), ""));
        assert_eq!(silent.md5, md5, "a replay stating no chart is the row's own");
        let no_chart = asked(&ReplayData { chart: None, ..data(Some(RandomOption::Random), Some(REPLAY_SEED), "") });
        assert_eq!(no_chart.md5, md5);
    }

    #[test]
    fn a_request_names_its_arrangement_as_a_replay_file_or_the_score_server_does() {
        for option in NoteOption::ALL {
            assert_eq!(arrangement_of_label(option.label()), Some(option), "{option:?}");
            assert_eq!(arrangement_of_label(&option.label().to_ascii_lowercase()), Some(option), "{option:?} in lower case");
        }
        for wire in [
            rbms_ir::RandomOption::Off,
            rbms_ir::RandomOption::Mirror,
            rbms_ir::RandomOption::Random,
            rbms_ir::RandomOption::RRandom,
            rbms_ir::RandomOption::SRandom,
            rbms_ir::RandomOption::Spiral,
            rbms_ir::RandomOption::HRandom,
            rbms_ir::RandomOption::AllScratch,
        ] {
            assert_eq!(arrangement_of_label(&format!("{wire:?}")), Some(crate::ir_replay::note_option_from_ir(wire)), "{wire:?}");
        }
        for unknown in [format!("{:?}", rbms_ir::RandomOption::Converge), String::new(), "FLIP".to_string()] {
            assert_eq!(arrangement_of_label(&unknown), None, "{unknown:?} named an arrangement");
        }
    }

    #[test]
    fn a_request_is_checked_against_the_chart_and_the_run() {
        let md5 = chart_md5();
        let upper = md5.to_ascii_uppercase();
        let borrowed = |random| Ok(BorrowedLayout { random, seed: REPLAY_SEED, source: "a replay".to_string() });
        assert_eq!(check_layout_request(&request("RANDOM", REPLAY_SEED), Some(&upper), true), borrowed(NoteOption::Random), "the case of a hash");
        assert_eq!(check_layout_request(&request("Spiral", REPLAY_SEED), Some(&md5), true), borrowed(NoteOption::Rotate));
        assert_eq!(
            check_layout_request(&request("OFF", 0), Some(&md5), true),
            Ok(BorrowedLayout { random: NoteOption::Off, seed: 0, source: "a replay".to_string() }),
            "OFF lays nothing out and needs no seed"
        );
        assert_eq!(check_layout_request(&request("S-RANDOM", 0), Some(&md5), true), Err(LayoutRefusal::UnknownLayout));
        assert_eq!(check_layout_request(&request("Converge", REPLAY_SEED), Some(&md5), true), Err(LayoutRefusal::UnsupportedOption("Converge".to_string())));
        assert_eq!(
            check_layout_request(&request("RANDOM", REPLAY_SEED), Some("0123456789abcdef"), true),
            Err(LayoutRefusal::ChartMismatch { replay: md5.clone(), chart: "0123456789abcdef".to_string() })
        );
        assert_eq!(
            check_layout_request(&request("RANDOM", REPLAY_SEED), None, true),
            Err(LayoutRefusal::ChartMismatch { replay: md5.clone(), chart: String::new() })
        );
        let unnamed = LayoutRequest { md5: String::new(), ..request("RANDOM", REPLAY_SEED) };
        assert_eq!(check_layout_request(&unnamed, Some(""), true), Err(LayoutRefusal::ChartMismatch { replay: String::new(), chart: String::new() }));
        assert_eq!(check_layout_request(&request("RANDOM", REPLAY_SEED), Some(&md5), false), Err(LayoutRefusal::NotFreePlay));
    }

    #[test]
    fn a_refusal_says_why_and_quotes_the_start_of_each_hash() {
        let mismatch = LayoutRefusal::ChartMismatch { replay: "0123456789abcdef".to_string(), chart: "fedcba9876543210".to_string() };
        let said = mismatch.to_string();
        assert!(said.contains("replay 01234567,") && said.contains("chart fedcba98)"), "{said}");
        let missing = LayoutRefusal::ChartMismatch { replay: "0123456789abcdef".to_string(), chart: String::new() };
        assert!(missing.to_string().contains("(01234567)"), "{missing}");
        assert!(LayoutRefusal::UnsupportedOption("Converge".to_string()).to_string().contains("Converge"));
    }

    #[test]
    fn a_refusal_is_worded_in_the_installed_language_and_quotes_what_it_names() {
        let refusals = [
            LayoutRefusal::ChartMismatch { replay: "0123456789abcdef".to_string(), chart: "fedcba9876543210".to_string() },
            LayoutRefusal::ChartMismatch { replay: "0123456789abcdef".to_string(), chart: String::new() },
            LayoutRefusal::UnknownLayout,
            LayoutRefusal::UnsupportedOption("Converge".to_string()),
            LayoutRefusal::NotFreePlay,
            LayoutRefusal::ArrangementUnknown,
        ];
        let english: Vec<String> = refusals.iter().map(ToString::to_string).collect();
        assert!(english.iter().all(|said| said.starts_with("layout not borrowed: ")), "{english:?}");
        rbms_render::text::set_language(rbms_model::Language::Korean);
        let korean: Vec<String> = refusals.iter().map(ToString::to_string).collect();
        rbms_render::text::set_language(rbms_model::Language::Japanese);
        let japanese: Vec<String> = refusals.iter().map(ToString::to_string).collect();
        rbms_render::text::set_language(rbms_model::Language::English);
        assert!(korean.iter().all(|said| said.starts_with("배치를 빌리지 않음: ")), "{korean:?}");
        assert!(japanese.iter().all(|said| said.starts_with("配置を借りません: ")), "{japanese:?}");
        assert!(korean[0].contains("01234567") && korean[0].contains("fedcba98"), "{}", korean[0]);
        assert!(japanese[1].contains("01234567") && !japanese[1].contains("fedcba98"), "{}", japanese[1]);
        assert!(korean[3].contains("Converge") && japanese[3].contains("Converge"));
    }

    #[test]
    fn the_notice_of_a_borrowed_layout_is_worded_when_it_is_taken() {
        crate::notify::exclusive(|| {
            let mut shared = a_library_of_one_chart("layout-notice");
            shared.borrow_layout(request("RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
            let mut said = Vec::new();
            rbms_render::text::set_language(rbms_model::Language::Korean);
            crate::notify::drain(&mut said);
            rbms_render::text::set_language(rbms_model::Language::English);
            assert!(said.iter().any(|(_, line)| line.ends_with("(RANDOM)") && line.contains("의 배치")), "{said:?}");
        });
    }

    #[test]
    fn a_run_that_is_not_the_players_own_borrows_nothing() {
        let refuse = |shared: &mut AppShared, why: &str| {
            let random = shared.config.play.random;
            assert_eq!(shared.borrow_layout(request("RANDOM", REPLAY_SEED), 0).err(), Some(LayoutRefusal::NotFreePlay), "{why}");
            assert_eq!(shared.borrowed_layout(), None, "{why}");
            assert_eq!(shared.config.play.random, random, "{why}");
        };
        let mut shared = a_library_of_one_chart("autoplay");
        shared.config.play.autoplay = true;
        refuse(&mut shared, "a run that plays itself");
        let mut shared = a_library_of_one_chart("autoplay-once");
        shared.autoplay_once = true;
        refuse(&mut shared, "a run the browser starts to play itself");
        let mut shared = a_library_of_one_chart("practice");
        shared.practice_requested = true;
        refuse(&mut shared, "a practice slice");
        let mut shared = a_library_of_one_chart("course");
        shared.course_run = Some(rbms_course::CourseRun::new(rbms_course::Course::default(), COURSE_OPENING_GAUGE));
        refuse(&mut shared, "a course stage");

        let mut shared = a_library_of_one_chart("other-chart");
        let other = LayoutRequest { md5: "0123456789abcdef".to_string(), ..request("RANDOM", REPLAY_SEED) };
        assert!(matches!(shared.borrow_layout(other, 0), Err(LayoutRefusal::ChartMismatch { .. })));
        assert!(matches!(shared.borrow_layout(request("RANDOM", REPLAY_SEED), 1), Err(LayoutRefusal::ChartMismatch { .. })), "no chart at that index");
        assert_eq!(shared.borrowed_layout(), None);
    }

    #[test]
    fn the_browser_opens_the_run_and_a_watched_replay_hands_its_place_to_it() {
        let mut shared = a_library_of_one_chart("from-browser");
        let opened = shared.borrow_layout(request("MIRROR", REPLAY_SEED), 0);
        assert!(matches!(opened, Ok(Transition::Open(_))), "the browser does not open the run over itself");
        assert_eq!(shared.borrowed_layout(), Some(&BorrowedLayout { random: NoteOption::Mirror, seed: REPLAY_SEED, source: "a replay".to_string() }));

        let mut shared = a_library_of_one_chart("over-replay");
        let own = OwnPlayOptions { scratch_auto: false, gauge: rbms_judge::GaugeKind::Easy, random: NoteOption::Off };
        own.put_into(&mut shared.config.play);
        shared.replay = Some(watched_replay(NoteOption::Random));
        assert!(shared.play_as_recorded().is_some());
        let replaced = shared.borrow_layout(request("MIRROR", REPLAY_SEED), 0);
        assert!(matches!(replaced, Ok(Transition::To(Stage::Loading(_)))), "the run does not take the replay's place");
        assert!(shared.replay.is_none(), "the replay is still being watched");
        assert_eq!(OwnPlayOptions::of(&shared.config.play), own, "the replay's options outlived it");
    }

    #[test]
    fn a_run_on_a_borrowed_layout_is_laid_out_as_the_replay_is_watched() {
        for option in NoteOption::ALL {
            let mut shared = a_library_of_one_chart(&format!("as-watched-{}", option.label()));
            shared.replay = Some(watched_replay(option));
            let watched = next_layout(&mut shared);
            shared.end_replay();
            shared.borrow_layout(request(option.label(), REPLAY_SEED), 0).expect("the layout is borrowed");
            assert_eq!(next_layout(&mut shared), watched, "{option:?}");
        }
    }

    #[test]
    fn the_borrowed_arrangement_is_played_and_the_players_own_is_what_is_saved() {
        let mut shared = a_library_of_one_chart("saved");
        shared.config.play.random = NoteOption::Mirror;
        shared.retry_seed = Some(1);
        shared.borrow_layout(request("S-RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
        assert_eq!(next_layout(&mut shared), (NoteOption::SRandom, REPLAY_SEED), "a carried seed outranked the borrowed one");
        assert_eq!(shared.config.play.random, NoteOption::SRandom, "the run does not play on the borrowed arrangement");
        assert_eq!(shared.settings_to_save().play.random, NoteOption::Mirror, "the settings file would keep the borrowed arrangement");

        let (random, seed) = next_layout(&mut shared);
        assert_eq!(random, NoteOption::Mirror, "a load the layout was not handed to borrowed it");
        assert_ne!(seed, REPLAY_SEED);
        assert_eq!(shared.borrowed_layout(), None);
        assert_eq!(shared.config.play.random, NoteOption::Mirror, "the player's arrangement is not back");
    }

    #[test]
    fn a_layout_borrowed_for_one_chart_never_lays_out_another() {
        let mut shared = a_library_of_one_chart("another-chart");
        shared.config.play.random = NoteOption::Off;
        shared.borrow_layout(request("RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
        let (random, _) = shared.layout_to_load("0123456789abcdef", None);
        assert_eq!(random, NoteOption::Off);
        assert_eq!(shared.borrowed_layout(), None);
        assert_eq!(shared.config.play.random, NoteOption::Off);
    }

    #[test]
    fn a_replay_in_hand_is_laid_out_as_it_was_recorded_whatever_layout_is_held() {
        let mut shared = a_library_of_one_chart("replay-first");
        shared.borrow_layout(request("MIRROR", REPLAY_SEED + 1), 0).expect("the layout is borrowed");
        shared.replay = Some(watched_replay(NoteOption::Random));
        assert_eq!(next_layout(&mut shared), (NoteOption::Random, REPLAY_SEED));
        assert_eq!(shared.borrowed_layout(), None, "the layout outlived a replay watched in its place");
    }

    #[test]
    fn the_same_layout_again_keeps_the_borrowed_one_and_any_other_way_on_lets_it_go() {
        let started_again = |carry: bool| {
            let mut shared = a_library_of_one_chart(&format!("again-{carry}"));
            shared.config.play.random = NoteOption::Mirror;
            shared.borrow_layout(request("RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
            let state = next_run(&mut shared);
            shared.retry_seed = carry.then(|| state.session.seed());
            assert!(matches!(crate::app_result::retry(&mut shared), Transition::To(Stage::Loading(_))));
            let layout = next_layout(&mut shared);
            (shared, layout)
        };
        let (shared, layout) = started_again(true);
        assert_eq!(layout, (NoteOption::Random, REPLAY_SEED));
        assert!(shared.borrowed_layout().is_some());

        let (shared, (random, seed)) = started_again(false);
        assert_eq!(random, NoteOption::Mirror, "a new arrangement kept the borrowed one");
        assert_ne!(seed, REPLAY_SEED);
        assert_eq!(shared.borrowed_layout(), None);

        let mut shared = a_library_of_one_chart("browser");
        shared.config.play.random = NoteOption::Mirror;
        shared.borrow_layout(request("RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
        next_run(&mut shared);
        assert!(matches!(shared.leave_play(), Transition::Back));
        assert_eq!(shared.borrowed_layout(), None, "the browser kept the layout");
        assert_eq!(shared.config.play.random, NoteOption::Mirror);
    }

    #[test]
    fn a_run_started_again_on_another_runs_seed_lets_the_layout_go() {
        let mut shared = a_library_of_one_chart("failed-again");
        shared.borrow_layout(request("H-RANDOM", REPLAY_SEED), 0).expect("the layout is borrowed");
        let state = next_run(&mut shared);
        shared.retry_seed = Some(state.session.seed());
        shared.hand_borrowed_layout_on();
        assert_eq!(next_layout(&mut shared), (NoteOption::HRandom, REPLAY_SEED));
        shared.retry_seed = Some(REPLAY_SEED + 1);
        shared.hand_borrowed_layout_on();
        assert_eq!(shared.borrowed_layout(), None, "a seed of another run kept the layout");
    }
}
