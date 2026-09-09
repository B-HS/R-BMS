use rbms_model::{LnKind, Micros, Mode, Model, ModelMeta, Note, NoteKind, TimeLine};

pub use rbms_model::{default_total, default_total_keyboard};
use rbms_parser::BmsSource;

pub mod scroll;
pub mod shuffle;

const SECTION_EPS: f64 = 1e-7;

/// Detect the play mode from the chart's used channels and filename. `.pms` is PMS
/// (POPN_9K); otherwise BMS-family: P2 channels (21-29) mean a double mode, and the
/// presence of keys 6/7 ('18'/'19') distinguishes 7-key from 5-key.
pub fn detect_mode(src: &BmsSource, filename: &str) -> Mode {
    let ext = std::path::Path::new(filename).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "pms" {
        return Mode::POPN_9K;
    }

    let mut used = std::collections::HashSet::new();
    for measure in src.measures.values() {
        for ch in &measure.channels {
            used.insert(ch.channel);
        }
    }
    let any = |range: std::ops::RangeInclusive<u32>| range.into_iter().any(|c| used.contains(&c));

    let p2 = any(73..=81);
    let keys67 = used.contains(&44) || used.contains(&45) || used.contains(&80) || used.contains(&81);

    match (p2, keys67) {
        (true, _) => Mode::BEAT_14K,
        (false, true) => Mode::BEAT_7K,
        (false, false) => Mode::BEAT_5K,
    }
}

enum Role {
    Bgm,
    BpmInline,
    BpmRef,
    Stop,
    Scroll,
    BgaBase,
    BgaPoor,
    BgaLayer,
    Note(usize),
    Ln(usize),
    Mine(usize),
    Hidden(usize),
    Ignore,
}

fn role(channel: u32, mode: Mode) -> Role {
    let lane = |raw: usize| mode.lane_of_raw(raw);
    match channel {
        1 => Role::Bgm,
        3 => Role::BpmInline,
        8 => Role::BpmRef,
        9 => Role::Stop,
        1020 => Role::Scroll,
        4 => Role::BgaBase,
        6 => Role::BgaPoor,
        7 => Role::BgaLayer,
        37..=45 => lane((channel - 37) as usize).map(Role::Note).unwrap_or(Role::Ignore),
        73..=81 => lane((channel - 73 + 9) as usize).map(Role::Note).unwrap_or(Role::Ignore),
        109..=117 => lane((channel - 109) as usize).map(Role::Hidden).unwrap_or(Role::Ignore),
        145..=153 => lane((channel - 145 + 9) as usize).map(Role::Hidden).unwrap_or(Role::Ignore),
        181..=189 => lane((channel - 181) as usize).map(Role::Ln).unwrap_or(Role::Ignore),
        217..=225 => lane((channel - 217 + 9) as usize).map(Role::Ln).unwrap_or(Role::Ignore),
        469..=477 => lane((channel - 469) as usize).map(Role::Mine).unwrap_or(Role::Ignore),
        505..=513 => lane((channel - 505 + 9) as usize).map(Role::Mine).unwrap_or(Role::Ignore),
        _ => Role::Ignore,
    }
}

enum EvKind {
    Note { lane: usize, wav: i32, lnobj: bool },
    Ln { lane: usize, wav: i32 },
    Mine { lane: usize, damage: f64 },
    Hidden { lane: usize, wav: i32 },
    Bgm { wav: i32 },
    Bpm(f64),
    Stop(f64),
    Scroll(f64),
    Bga(i32),
    Layer(i32),
    SectionLine,
}

struct Ev {
    section: f64,
    kind: EvKind,
}

pub fn to_model(src: &BmsSource, mode: Mode) -> Model {
    let base = src.base;
    let lnobj = src.headers.lnobj;
    // Reference `#LNMODE`: 0=undefined(→LN), 1=LN, 2=CN, 3=HCN. The kind applies to every long note
    // in the chart; judging is identical across kinds for now (the model records the kind so the
    // distinction is preserved for rendering/scoring).
    let ln_kind = match src.headers.lnmode {
        2 => LnKind::Cn,
        3 => LnKind::Hcn,
        _ => LnKind::Ln,
    };

    let max_measure = src.measures.keys().copied().max().unwrap_or(0);

    let mut section_start = vec![0.0f64; (max_measure + 2) as usize];
    for m in 0..=max_measure {
        let rate = src.measures.get(&m).map(|x| x.rate).unwrap_or(1.0);
        section_start[(m + 1) as usize] = section_start[m as usize] + rate;
    }

    let mut events: Vec<Ev> = Vec::new();
    for m in 0..=max_measure {
        let s0 = section_start[m as usize];
        let rate = src.measures.get(&m).map(|x| x.rate).unwrap_or(1.0);
        events.push(Ev { section: s0, kind: EvKind::SectionLine });

        let Some(measure) = src.measures.get(&m) else {
            continue;
        };
        for ch in &measure.channels {
            let r = role(ch.channel, mode);
            for obj in &ch.objects {
                let section = s0 + rate * obj.pos();
                let kind = match &r {
                    Role::Bgm => EvKind::Bgm { wav: obj.value(base) as i32 },
                    Role::BpmInline => EvKind::Bpm(obj.value16() as f64),
                    Role::BpmRef => match src.bpm_def.get(&obj.value(base)) {
                        Some(b) => EvKind::Bpm(*b),
                        None => continue,
                    },
                    Role::Stop => match src.stop_def.get(&obj.value(base)) {
                        Some(s) => EvKind::Stop(*s),
                        None => continue,
                    },
                    Role::Scroll => match src.scroll_def.get(&obj.value(base)) {
                        Some(s) => EvKind::Scroll(*s),
                        None => continue,
                    },
                    Role::BgaBase | Role::BgaPoor => EvKind::Bga(obj.value(base) as i32),
                    Role::BgaLayer => EvKind::Layer(obj.value(base) as i32),
                    Role::Note(lane) => {
                        let wav = obj.value(base);
                        EvKind::Note { lane: *lane, wav: wav as i32, lnobj: lnobj == Some(wav) }
                    }
                    Role::Ln(lane) => EvKind::Ln { lane: *lane, wav: obj.value(base) as i32 },
                    Role::Mine(lane) => EvKind::Mine { lane: *lane, damage: obj.value(base) as f64 },
                    Role::Hidden(lane) => EvKind::Hidden { lane: *lane, wav: obj.value(base) as i32 },
                    Role::Ignore => continue,
                };
                events.push(Ev { section, kind });
            }
        }
    }

    events.sort_by(|a, b| a.section.total_cmp(&b.section));

    let lanes = mode.key;
    let mut timelines: Vec<TimeLine> = Vec::new();
    let mut exp_bpm: Vec<Option<f64>> = Vec::new();
    let mut exp_scroll: Vec<Option<f64>> = Vec::new();
    let mut exp_stop: Vec<Option<f64>> = Vec::new();
    let mut ln_open: Vec<bool> = vec![false; lanes];
    let mut last_normal_tl: Vec<Option<usize>> = vec![None; lanes];

    for ev in &events {
        let idx = match timelines.last() {
            Some(last) if (ev.section - last.section).abs() < SECTION_EPS => timelines.len() - 1,
            _ => {
                timelines.push(TimeLine::empty(lanes, 0, ev.section, src.headers.init_bpm));
                exp_bpm.push(None);
                exp_scroll.push(None);
                exp_stop.push(None);
                timelines.len() - 1
            }
        };
        apply_event(&ev.kind, idx, ln_kind, &mut timelines, &mut exp_bpm, &mut exp_scroll, &mut exp_stop, &mut ln_open, &mut last_normal_tl);
    }

    assign_times(&mut timelines, &exp_bpm, &exp_scroll, &exp_stop, src.headers.init_bpm);

    let wavmap = build_resource_map(&src.wav);
    let bgamap = build_resource_map(&src.bmp);

    Model {
        mode,
        meta: ModelMeta {
            title: src.headers.title.clone(),
            subtitle: src.headers.subtitle.clone(),
            artist: src.headers.artist.clone(),
            subartist: src.headers.subartist.clone(),
            genre: src.headers.genre.clone(),
            play_level: src.headers.play_level.clone(),
            difficulty: src.headers.difficulty,
            rank: src.headers.rank,
            defexrank: src.headers.defexrank,
            total: src.headers.total.unwrap_or(0.0),
            stagefile: src.headers.stagefile.clone(),
        },
        wavmap,
        bgamap,
        init_bpm: src.headers.init_bpm,
        timelines,
        md5: src.md5.clone(),
        sha256: src.sha256.clone(),
    }
}

fn apply_event(
    kind: &EvKind,
    idx: usize,
    ln_kind: LnKind,
    timelines: &mut [TimeLine],
    exp_bpm: &mut [Option<f64>],
    exp_scroll: &mut [Option<f64>],
    exp_stop: &mut [Option<f64>],
    ln_open: &mut [bool],
    last_normal: &mut [Option<usize>],
) {
    let section = timelines[idx].section;
    match kind {
        EvKind::SectionLine => timelines[idx].section_line = true,
        EvKind::Bpm(b) => {
            timelines[idx].bpm = *b;
            exp_bpm[idx] = Some(*b);
        }
        EvKind::Stop(s) => {
            exp_stop[idx] = Some(exp_stop[idx].unwrap_or(0.0) + *s);
        }
        EvKind::Scroll(s) => {
            timelines[idx].scroll = *s;
            exp_scroll[idx] = Some(*s);
        }
        EvKind::Bga(v) => timelines[idx].bga = *v,
        EvKind::Layer(v) => timelines[idx].layer = *v,
        EvKind::Bgm { wav } => timelines[idx].bgnotes.push(Note::normal(*wav, 0, section)),
        EvKind::Hidden { lane, wav } => {
            timelines[idx].hidden[*lane] = Some(Note::normal(*wav, 0, section));
        }
        EvKind::Mine { lane, damage } => {
            let mut n = Note::normal(0, 0, section);
            n.kind = NoteKind::Mine { damage: *damage };
            timelines[idx].notes[*lane] = Some(n);
        }
        EvKind::Note { lane, wav, lnobj } => {
            let converted = if *lnobj {
                match last_normal[*lane].take() {
                    Some(si) if matches!(timelines[si].notes[*lane].as_ref().map(|n| &n.kind), Some(NoteKind::Normal)) => {
                        timelines[si].notes[*lane].as_mut().unwrap().kind = NoteKind::LongStart { ln: ln_kind };
                        let mut end = Note::normal(*wav, 0, section);
                        end.kind = NoteKind::LongEnd { ln: ln_kind };
                        timelines[idx].notes[*lane] = Some(end);
                        true
                    }
                    _ => false,
                }
            } else {
                false
            };
            if !converted {
                timelines[idx].notes[*lane] = Some(Note::normal(*wav, 0, section));
                if !*lnobj {
                    last_normal[*lane] = Some(idx);
                }
            }
        }
        EvKind::Ln { lane, wav } => {
            let mut n = Note::normal(*wav, 0, section);
            if ln_open[*lane] {
                n.kind = NoteKind::LongEnd { ln: ln_kind };
                ln_open[*lane] = false;
            } else {
                n.kind = NoteKind::LongStart { ln: ln_kind };
                ln_open[*lane] = true;
            }
            timelines[idx].notes[*lane] = Some(n);
        }
    }
}

fn assign_times(timelines: &mut [TimeLine], exp_bpm: &[Option<f64>], exp_scroll: &[Option<f64>], exp_stop: &[Option<f64>], init_bpm: f64) {
    if timelines.is_empty() {
        return;
    }
    let mut bpm = if init_bpm > 0.0 { init_bpm } else { 130.0 };
    let mut scroll = 1.0;
    for i in 0..timelines.len() {
        let this_bpm = match exp_bpm[i] {
            Some(b) if b > 0.0 => b,
            _ => bpm,
        };
        timelines[i].bpm = this_bpm;
        timelines[i].scroll = exp_scroll[i].unwrap_or(scroll);
        timelines[i].stop_us = match exp_stop[i] {
            Some(s) if s > 0.0 => (240_000_000.0 * s / (192.0 * this_bpm)) as Micros,
            _ => 0,
        };
        if i == 0 {
            timelines[i].time_us = 0;
        } else {
            let prev = &timelines[i - 1];
            let dt = 240_000_000.0 * (timelines[i].section - prev.section) / prev.bpm;
            timelines[i].time_us = prev.time_us + prev.stop_us + dt as Micros;
        }
        for n in timelines[i].notes.iter_mut().flatten() {
            n.time_us = timelines[i].time_us;
        }
        for n in &mut timelines[i].bgnotes {
            n.time_us = timelines[i].time_us;
        }
        bpm = timelines[i].bpm;
        scroll = timelines[i].scroll;
    }
}

fn build_resource_map(defs: &std::collections::BTreeMap<u32, String>) -> Vec<String> {
    let max_id = defs.keys().copied().max().unwrap_or(0) as usize;
    let mut v = vec![String::new(); max_id + 1];
    for (id, name) in defs {
        v[*id as usize] = name.clone();
    }
    v
}

pub fn count_playable_notes(model: &Model) -> usize {
    model
        .timelines
        .iter()
        .flat_map(|tl| tl.notes.iter())
        .flatten()
        .filter(|n| match &n.kind {
            NoteKind::Mine { .. } => false,
            // CN/HCN ends are judged (counted) separately at release, so a charge note counts twice;
            // a plain LN end is not a separate judgment. Keeps the count in step with the judge engine.
            NoteKind::LongEnd { ln } => matches!(ln, LnKind::Cn | LnKind::Hcn),
            _ => true,
        })
        .count()
}

/// Per-second note-distribution density, ported from the reference implementation's `SongInformation`
/// (`song/SongInformation.java`). `bins` holds per-second note totals (categories excluding mines)
/// for the histogram; `peak`/`avg`/`end` are notes-per-second scalars matching the reference implementation's
/// MusicSelect density readout.
pub struct NoteDensity {
    pub bins: Vec<u32>,
    pub peak: f64,
    pub avg: f64,
    pub end: f64,
}

/// Bin the chart into 1-second slots (long-note bodies count in every spanned second), then derive
/// the three densities: `peak` = busiest second, `avg` = mean over seconds at or above the
/// `total_notes/bins/4` activity threshold, `end` = max 5-second-window density after the gauge
/// "border" point (`total_notes*(1-100/total_value)` cumulative notes). `total_value` is `#TOTAL`
/// (≤0 ⇒ the standard BMS default total derived from the note count, mirroring the reference implementation which never
/// sees a zero total).
pub fn note_density(model: &Model, total_value: f64) -> NoteDensity {
    // Size the bins from the last NOTE-bearing timeline, mirroring the reference implementation's `BMSModel.getLastMilliTime`:
    // `to_model` emits a bar-line timeline per measure, so the unconditional last timeline can sit many
    // empty seconds past the final note and would pad the histogram with phantom bins.
    let last_us = model.timelines.iter().rev().find(|t| t.notes.iter().any(Option::is_some)).map(|t| t.time_us).unwrap_or(0);
    let bins = (last_us / 1_000_000) as usize + 2;
    // [0]=scr LN-head, [1]=scr LN-body, [2]=scr normal, [3]=key LN-head, [4]=key LN-body, [5]=key normal, [6]=mine.
    let mut data = vec![[0i32; 7]; bins];
    let scr = |lane: usize| model.mode.scratch.contains(&lane);
    let mut open_head = vec![None::<usize>; model.mode.key];
    let mut total_notes = 0i32;
    let mut counted = vec![0i32; bins];
    for tl in &model.timelines {
        let sec = ((tl.time_us / 1_000_000) as usize).min(bins - 1);
        for lane in 0..model.mode.key.min(tl.notes.len()) {
            let Some(note) = &tl.notes[lane] else {
                continue;
            };
            match note.kind {
                NoteKind::Normal => {
                    data[sec][if scr(lane) { 2 } else { 5 }] += 1;
                    total_notes += 1;
                    counted[sec] += 1;
                }
                NoteKind::Mine { .. } => {
                    data[sec][6] += 1;
                    counted[sec] += 1;
                }
                // Count the head at head-time (independent of seeing the tail, so dangling heads still
                // register); the body then fills head+1..=end. Net per-second sums equal the reference implementation's.
                NoteKind::LongStart { .. } => {
                    data[sec][if scr(lane) { 0 } else { 3 }] += 1;
                    total_notes += 1;
                    counted[sec] += 1;
                    open_head[lane] = Some(sec);
                }
                NoteKind::LongEnd { .. } => {
                    if let Some(head) = open_head[lane].take() {
                        for b in (head + 1)..=sec {
                            data[b][if scr(lane) { 1 } else { 4 }] += 1;
                        }
                    }
                }
            }
        }
    }
    let bins_v: Vec<u32> = data.iter().map(|d| (d[0] + d[1] + d[2] + d[3] + d[4] + d[5]).max(0) as u32).collect();
    let peak = bins_v.iter().copied().max().unwrap_or(0) as f64;
    let bd = total_notes / bins.max(1) as i32 / 4;
    let (sum, count) = bins_v.iter().fold((0u64, 0u64), |(s, c), &n| if n as i32 >= bd { (s + n as u64, c + 1) } else { (s, c) });
    let avg = if count > 0 { sum as f64 / count as f64 } else { 0.0 };
    let total_value = if total_value > 0.0 { total_value } else { default_total(total_notes.max(0) as usize) };
    let border = (total_notes as f64 * (1.0 - 100.0 / total_value)) as i32;
    let mut cum = 0i32;
    let mut borderpos = 0usize;
    if border > 0 {
        for (i, &c) in counted.iter().enumerate() {
            cum += c;
            if cum >= border {
                borderpos = i;
                break;
            }
        }
    }
    let d = 5.min(bins.saturating_sub(borderpos + 1));
    let mut end = 0.0;
    if d > 0 {
        for i in borderpos..bins.saturating_sub(d) {
            let window: u32 = bins_v[i..i + d].iter().sum();
            end = f64::max(end, window as f64 / d as f64);
        }
    }
    NoteDensity { bins: bins_v, peak, avg, end }
}

#[cfg(test)]
mod tests;
