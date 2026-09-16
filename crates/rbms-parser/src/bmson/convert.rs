//! Mapping a decoded bmson document onto the shared chart model.
//!
//! Pulses become timelines in one forward pass: tempo, stops and scroll rate are placed first so
//! every pulse has a time, then notes are placed on the timeline their pulse landed on. Audio slices
//! need those times, which is why notes cannot be placed in the same pass.

use rbms_model::{LnKind, Micros, Mode, Model, ModelMeta, Note, NoteKind, TimeLine, US_PER_MEASURE_NUM, VOLWAV_DEFAULT_PERCENT};

use super::{BgaEvent, BmsonChart, BmsonNote, LN_TYPE_MAX, SILENT_WAV};

/// bmson `x` to lane for BEAT_5K: five keys, then the scratch at `x` 8 (reference implementation
/// `BMSONDecoder` `keyassign`). Every mode the reference does not tabulate maps `x` to lane `x - 1`.
const BEAT_5K_LANES: [i32; 8] = [0, 1, 2, 3, 4, -1, -1, 5];

/// bmson `x` to lane for BEAT_10K, the two-player form of [`BEAT_5K_LANES`].
const BEAT_10K_LANES: [i32; 16] = [0, 1, 2, 3, 4, -1, -1, 5, 6, 7, 8, 9, 10, -1, -1, 11];

/// Separator the reference implementation joins `info.subartists` with.
const SUBARTIST_SEPARATOR: &str = ",";

/// Separator between `info.subtitle` and `info.chart_name`, used only when both are non-empty.
const SUBTITLE_SEPARATOR: &str = " ";

/// A lane index no bmson `x` maps to, which sends the note to the background channel.
const NO_LANE: i32 = -1;

pub(super) fn to_model(chart: &BmsonChart, mode: Mode) -> Model {
    let mut grid = Grid::build(chart, mode);
    let wavmap = grid.place_channels(chart, mode);
    grid.place_bga(chart);
    let notes = grid.playable_notes();

    Model {
        mode,
        meta: ModelMeta {
            title: chart.info.title.clone(),
            subtitle: subtitle(chart),
            artist: chart.info.artist.clone(),
            subartist: chart.info.subartists.join(SUBARTIST_SEPARATOR),
            genre: chart.info.genre.clone(),
            play_level: chart.info.level.to_string(),
            difficulty: 0,
            rank: chart.rank(),
            defexrank: None,
            total: chart.total_for_notes(&mode, notes),
            volwav: VOLWAV_DEFAULT_PERCENT,
            stagefile: chart.info.eyecatch_image.clone(),
        },
        wavmap,
        bgamap: chart.bga.header.iter().map(|h| h.name.clone()).collect(),
        init_bpm: chart.info.init_bpm,
        timelines: grid.timelines,
        md5: chart.md5.clone(),
        sha256: chart.sha256.clone(),
    }
}

fn subtitle(chart: &BmsonChart) -> String {
    let (subtitle, chart_name) = (chart.info.subtitle.as_str(), chart.info.chart_name.as_str());
    let separator = if subtitle.is_empty() || chart_name.is_empty() { "" } else { SUBTITLE_SEPARATOR };
    format!("{subtitle}{separator}{chart_name}")
}

/// The timelines of one chart, indexed by the pulse each sits on.
struct Grid {
    pulses: Vec<i64>,
    timelines: Vec<TimeLine>,
    /// Open long notes per lane as `(head pulse, tail pulse)`, in the order they were placed.
    open_long: Vec<Vec<(i64, i64)>>,
    /// Keysounds of long-note tails whose head has not been read yet.
    pending_tails: Vec<PendingTail>,
}

/// An `up` note waiting for the long note it closes.
struct PendingTail {
    x: i64,
    y: i64,
    sound: Sound,
}

/// Where in a keysound a note plays from, and for how long. A zero duration plays to the end of the
/// file, which is every note of a chart that slices nothing.
#[derive(Clone, Copy)]
struct Sound {
    wav: i32,
    start_us: Micros,
    duration_us: Micros,
}

impl Grid {
    fn build(chart: &BmsonChart, mode: Mode) -> Grid {
        let ppm = chart.pulses_per_measure();
        let pulses = pulse_set(chart);
        let mut timelines: Vec<TimeLine> = pulses.iter().map(|y| TimeLine::empty(mode.key, 0, *y as f64 / ppm, chart.info.init_bpm)).collect();
        let mut grid = Grid { pulses, timelines: Vec::new(), open_long: vec![Vec::new(); mode.key], pending_tails: Vec::new() };

        let mut bpm_at: Vec<Option<f64>> = vec![None; timelines.len()];
        let mut scroll_at: Vec<Option<f64>> = vec![None; timelines.len()];
        let mut stop_at: Vec<i64> = vec![0; timelines.len()];
        for event in &chart.bpm_events {
            if let Some(i) = grid.at(event.y).filter(|_| event.bpm > 0.0 && event.bpm.is_finite()) {
                bpm_at[i] = Some(event.bpm);
            }
        }
        for event in &chart.scroll_events {
            if let Some(i) = grid.at(event.y).filter(|_| event.rate.is_finite()) {
                scroll_at[i] = Some(event.rate);
            }
        }
        for event in &chart.stop_events {
            if let Some(i) = grid.at(event.y).filter(|_| event.duration >= 0) {
                stop_at[i] = event.duration;
            }
        }
        for line in &chart.lines {
            if let Some(i) = grid.at(line.y) {
                timelines[i].section_line = true;
            }
        }

        assign_times(&mut timelines, &grid.pulses, &bpm_at, &scroll_at, &stop_at, chart.info.init_bpm, ppm);
        grid.timelines = timelines;
        grid
    }

    fn at(&self, pulse: i64) -> Option<usize> {
        self.pulses.binary_search(&pulse).ok()
    }

    fn time_at(&self, pulse: i64) -> Micros {
        self.at(pulse).map(|i| self.timelines[i].time_us).unwrap_or(0)
    }

    /// Place every sound, invisible and mine channel, and return the keysound list they index into.
    fn place_channels(&mut self, chart: &BmsonChart, mode: Mode) -> Vec<String> {
        let lanes = lane_table(&mode);
        let mut wavmap = Vec::new();
        for channel in &chart.sound_channels {
            let wav = wavmap.len() as i32;
            wavmap.push(channel.name.clone());
            self.place_notes(&sorted(&channel.notes), wav, &lanes, chart);
        }
        for channel in &chart.key_channels {
            let wav = wavmap.len() as i32;
            wavmap.push(channel.name.clone());
            for note in sorted(&channel.notes) {
                let (Some(i), Some(lane)) = (self.at(note.y), lane_of(&lanes, note.x)) else {
                    continue;
                };
                let time_us = self.timelines[i].time_us;
                self.timelines[i].hidden[lane] = Some(Note::normal(wav, time_us, self.timelines[i].section));
            }
        }
        for channel in &chart.mine_channels {
            let wav = wavmap.len() as i32;
            wavmap.push(channel.name.clone());
            let mut notes = channel.notes.clone();
            notes.sort_by_key(|n| n.y);
            for mine in notes {
                let (Some(i), Some(lane)) = (self.at(mine.y), lane_of(&lanes, mine.x)) else {
                    continue;
                };
                if self.inside_long_note(lane, mine.y) || self.timelines[i].notes[lane].is_some() {
                    continue;
                }
                let mut note = Note::normal(wav, self.timelines[i].time_us, self.timelines[i].section);
                note.kind = NoteKind::Mine { damage: mine.damage };
                self.timelines[i].notes[lane] = Some(note);
            }
        }
        wavmap
    }

    fn place_notes(&mut self, notes: &[BmsonNote], wav: i32, lanes: &[i32], chart: &BmsonChart) {
        let mut start_us: Micros = 0;
        for (i, note) in notes.iter().enumerate() {
            if !note.c {
                start_us = 0;
            }
            let next = notes[i + 1..].iter().find(|n| n.y > note.y);
            let duration_us = match next {
                Some(n) if n.c => self.time_at(n.y) - self.time_at(note.y),
                _ => 0,
            };
            let sound = Sound { wav, start_us, duration_us };
            match lane_of(lanes, note.x) {
                None => self.place_background(note.y, sound),
                Some(lane) if note.up => self.close_long_note(lane, note, sound),
                Some(lane) => self.place_lane_note(lane, note, sound, chart),
            }
            start_us += duration_us;
        }
    }

    fn place_background(&mut self, pulse: i64, sound: Sound) {
        let Some(i) = self.at(pulse) else {
            return;
        };
        let note = sounded(&sound, self.timelines[i].time_us, self.timelines[i].section);
        self.timelines[i].bgnotes.push(note);
    }

    /// Give a long-note tail its own keysound. A tail that has not been read yet keeps the sound
    /// until the long note arrives, which is how a tail sound in a later channel still lands.
    fn close_long_note(&mut self, lane: usize, note: &BmsonNote, sound: Sound) {
        let open = self.open_long[lane].iter().find(|(_, tail)| *tail == note.y).copied();
        match open.and_then(|(_, tail)| self.at(tail)) {
            Some(i) => {
                if let Some(tail) = self.timelines[i].notes[lane].as_mut() {
                    tail.wav = sound.wav;
                    tail.start_us = sound.start_us;
                    tail.duration_us = sound.duration_us;
                }
            }
            None => self.pending_tails.push(PendingTail { x: note.x, y: note.y, sound }),
        }
    }

    fn place_lane_note(&mut self, lane: usize, note: &BmsonNote, sound: Sound, chart: &BmsonChart) {
        if self.inside_long_note(lane, note.y) {
            self.place_background(note.y, sound);
            return;
        }
        if note.l > 0 { self.place_long_note(lane, note, sound, chart) } else { self.place_normal_note(lane, note.y, sound) }
    }

    fn place_normal_note(&mut self, lane: usize, pulse: i64, sound: Sound) {
        let Some(i) = self.at(pulse) else {
            return;
        };
        let (time_us, section) = (self.timelines[i].time_us, self.timelines[i].section);
        let note = sounded(&sound, time_us, section);
        match self.timelines[i].notes[lane].as_mut() {
            None => self.timelines[i].notes[lane] = Some(note),
            Some(existing) if existing.kind == NoteKind::Normal => existing.layered.push(note),
            Some(_) => {}
        }
    }

    fn place_long_note(&mut self, lane: usize, note: &BmsonNote, sound: Sound, chart: &BmsonChart) {
        let tail_pulse = note.y + note.l;
        let (Some(head_i), Some(tail_i)) = (self.at(note.y), self.at(tail_pulse)) else {
            return;
        };
        let flavour = ln_kind(if (1..=LN_TYPE_MAX).contains(&note.t) { note.t } else { chart.info.ln_type });
        let mut head = sounded(&sound, self.timelines[head_i].time_us, self.timelines[head_i].section);
        head.kind = NoteKind::LongStart { ln: flavour };

        if let Some(kind) = self.timelines[head_i].notes[lane].as_ref().map(|n| n.kind.clone()) {
            let same_span = matches!(kind, NoteKind::LongStart { .. }) && self.timelines[tail_i].notes[lane].is_some();
            if same_span && let Some(existing) = self.timelines[head_i].notes[lane].as_mut() {
                existing.layered.push(head);
            }
            return;
        }
        if self.occupied_between(lane, note.y, tail_pulse) {
            self.place_background(note.y, sound);
            return;
        }

        let tail_sound = self.take_pending_tail(note.x, tail_pulse);
        let mut tail = sounded(&tail_sound, self.timelines[tail_i].time_us, self.timelines[tail_i].section);
        tail.kind = NoteKind::LongEnd { ln: flavour };
        self.timelines[head_i].notes[lane] = Some(head);
        self.timelines[tail_i].notes[lane] = Some(tail);
        self.open_long[lane].push((note.y, tail_pulse));
    }

    fn take_pending_tail(&mut self, x: i64, tail_pulse: i64) -> Sound {
        match self.pending_tails.iter().position(|p| p.x == x && p.y == tail_pulse) {
            Some(i) => self.pending_tails.remove(i).sound,
            None => Sound { wav: SILENT_WAV, start_us: 0, duration_us: 0 },
        }
    }

    /// Whether `pulse` falls after the head and at or before the tail of a long note in `lane`.
    fn inside_long_note(&self, lane: usize, pulse: i64) -> bool {
        self.open_long[lane].iter().any(|(head, tail)| *head < pulse && pulse <= *tail)
    }

    /// Whether any timeline after `head_pulse`, up to and including `tail_pulse`, already plays
    /// something in `lane`. A long note may not be laid over one.
    fn occupied_between(&self, lane: usize, head_pulse: i64, tail_pulse: i64) -> bool {
        let first = self.pulses.partition_point(|pulse| *pulse <= head_pulse);
        self.pulses[first..]
            .iter()
            .take_while(|pulse| **pulse <= tail_pulse)
            .enumerate()
            .any(|(offset, _)| self.timelines[first + offset].notes[lane].is_some())
    }

    fn place_bga(&mut self, chart: &BmsonChart) {
        let index_of = |event: &BgaEvent| chart.bga.header.iter().position(|h| h.id == event.id).map(|i| i as i32);
        for event in &chart.bga.base {
            if let (Some(i), Some(picture)) = (self.at(event.y), index_of(event)) {
                self.timelines[i].bga = picture;
            }
        }
        for event in &chart.bga.layer {
            if let (Some(i), Some(picture)) = (self.at(event.y), index_of(event)) {
                self.timelines[i].layer = picture;
            }
        }
    }

    /// Notes the gauge counts, the same set `rbms_chart::count_playable_notes` reports: every note
    /// but a mine, and a long-note tail only when the flavour makes the release a judgement.
    fn playable_notes(&self) -> usize {
        self.timelines
            .iter()
            .flat_map(|tl| tl.notes.iter())
            .flatten()
            .filter(|n| match &n.kind {
                NoteKind::Mine { .. } => false,
                NoteKind::LongEnd { ln } => matches!(ln, LnKind::Cn | LnKind::Hcn),
                _ => true,
            })
            .count()
    }
}

/// Every pulse the chart puts something on, sorted, always including pulse 0 so the chart starts at
/// time zero even when nothing sits there.
fn pulse_set(chart: &BmsonChart) -> Vec<i64> {
    let mut pulses = vec![0];
    pulses.extend(chart.bpm_events.iter().map(|e| e.y));
    pulses.extend(chart.stop_events.iter().map(|e| e.y));
    pulses.extend(chart.scroll_events.iter().map(|e| e.y));
    pulses.extend(chart.lines.iter().map(|e| e.y));
    pulses.extend(chart.bga.base.iter().chain(&chart.bga.layer).map(|e| e.y));
    for channel in &chart.sound_channels {
        for note in &channel.notes {
            pulses.push(note.y);
            if note.l > 0 {
                pulses.push(note.y + note.l);
            }
        }
    }
    for channel in &chart.key_channels {
        pulses.extend(channel.notes.iter().map(|n| n.y));
    }
    for channel in &chart.mine_channels {
        pulses.extend(channel.notes.iter().map(|n| n.y));
    }
    pulses.sort_unstable();
    pulses.dedup();
    pulses
}

/// Carry tempo and scroll rate forward, turn each stop into microseconds at the tempo it stops, and
/// integrate the pulse grid into times (`BMSONDecoder` `getTimeLine`). The running position stays a
/// float so truncation never accumulates.
fn assign_times(timelines: &mut [TimeLine], pulses: &[i64], bpm_at: &[Option<f64>], scroll_at: &[Option<f64>], stop_at: &[i64], init_bpm: f64, ppm: f64) {
    let mut bpm = init_bpm;
    let mut scroll = 1.0;
    let mut time = 0.0;
    for i in 0..timelines.len() {
        if let Some(value) = bpm_at[i] {
            bpm = value;
        }
        if let Some(value) = scroll_at[i] {
            scroll = value;
        }
        if i > 0 {
            let advanced = (pulses[i] - pulses[i - 1]) as f64;
            time += timelines[i - 1].stop_us as f64 + US_PER_MEASURE_NUM * (advanced / ppm) / timelines[i - 1].bpm;
        }
        timelines[i].bpm = bpm;
        timelines[i].scroll = scroll;
        timelines[i].time_us = time as Micros;
        timelines[i].stop_us = (US_PER_MEASURE_NUM * stop_at[i] as f64 / (bpm * ppm)) as Micros;
    }
}

/// `x` to lane for `mode`. Only the two modes whose lanes are not `x - 1` are tabulated.
fn lane_table(mode: &Mode) -> Vec<i32> {
    if *mode == Mode::BEAT_5K {
        BEAT_5K_LANES.to_vec()
    } else if *mode == Mode::BEAT_10K {
        BEAT_10K_LANES.to_vec()
    } else {
        (0..mode.key as i32).collect()
    }
}

/// The lane `x` plays in, or `None` for a background sound: `x` is one-based, and zero, a missing
/// value or one past the mode's lanes all mean the note is not played.
fn lane_of(lanes: &[i32], x: i64) -> Option<usize> {
    let index = usize::try_from(x - 1).ok()?;
    match lanes.get(index).copied().unwrap_or(NO_LANE) {
        NO_LANE => None,
        lane => usize::try_from(lane).ok(),
    }
}

fn sounded(sound: &Sound, time_us: Micros, section: f64) -> Note {
    let mut note = Note::normal(sound.wav, time_us, section);
    note.start_us = sound.start_us;
    note.duration_us = sound.duration_us;
    note
}

fn ln_kind(code: i64) -> LnKind {
    match code {
        1 => LnKind::Ln,
        2 => LnKind::Cn,
        3 => LnKind::Hcn,
        _ => LnKind::Undefined,
    }
}

fn sorted(notes: &[BmsonNote]) -> Vec<BmsonNote> {
    let mut ordered = notes.to_vec();
    ordered.sort_by_key(|n| n.y);
    ordered
}
