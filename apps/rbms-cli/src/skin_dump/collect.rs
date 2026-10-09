//! Loading one skin document against a scenario and describing what came out.

use std::path::Path;
use std::time::Instant;

use rbms_model::Mode;
use rbms_skin::dst::LuaFnId;
use rbms_skin::loader::{
    LoadedSkin, ParserKind, SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS,
    SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_BATTLE,
    SKIN_TYPE_PLAY_24KEYS_DOUBLE, SKIN_TYPE_RESULT, SKIN_TYPE_SKIN_SELECT, SkinHeader, SkinLoadOptions, SkinUserConfig, automatic_offsets, load_header,
    load_skin_with_host,
};
use rbms_skin::lua::{LuaDiagnostics, LuaFnKind, SkinLua};

use super::frames::{self, ALL_KINDS, kind_label};
use super::probe;
use super::report::{BodyReport, CountReport, DiagnosticsReport, DocumentReport, EntryReport, FileReport, FilesReport, HeaderReport, RowReport, TimingsReport};
use super::scenario::{Scenario, Screen};

const MILLIS_PER_SECOND: f64 = 1_000.0;

/// What the documents of one dump share.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The folder a document may read from.
    pub root: &'a Path,
    /// Where the skins' writes go.
    pub overlay: &'a Path,
    pub seed: u64,
    pub frames: usize,
    /// A scenario that replaces the built-in ones.
    pub scenario: Option<&'a Scenario>,
    pub probe_budget: bool,
}

const SKIN_TYPE_LABELS: &[(i32, &str)] = &[
    (SKIN_TYPE_PLAY_7KEYS, "play 7 keys"),
    (SKIN_TYPE_PLAY_5KEYS, "play 5 keys"),
    (SKIN_TYPE_PLAY_14KEYS, "play 14 keys"),
    (SKIN_TYPE_PLAY_10KEYS, "play 10 keys"),
    (SKIN_TYPE_PLAY_9KEYS, "play 9 keys"),
    (SKIN_TYPE_MUSIC_SELECT, "music select"),
    (SKIN_TYPE_DECIDE, "decide"),
    (SKIN_TYPE_RESULT, "result"),
    (SKIN_TYPE_KEY_CONFIG, "key config"),
    (SKIN_TYPE_SKIN_SELECT, "skin select"),
    (SKIN_TYPE_COURSE_RESULT, "course result"),
    (SKIN_TYPE_PLAY_24KEYS, "play 24 keys"),
    (SKIN_TYPE_PLAY_24KEYS_DOUBLE, "play 24 keys double"),
    (SKIN_TYPE_PLAY_24KEYS_BATTLE, "play 24 keys battle"),
];

fn type_label(skin_type: i32) -> String {
    SKIN_TYPE_LABELS.iter().find(|(id, _)| *id == skin_type).map_or_else(|| format!("type {skin_type}"), |(_, label)| (*label).to_owned())
}

const fn parser_label(parser: ParserKind) -> &'static str {
    match parser {
        ParserKind::Json => "json",
        ParserKind::Json5 => "json5",
        ParserKind::Lua => "lua",
    }
}

fn header_report(header: &SkinHeader) -> HeaderReport {
    HeaderReport {
        skin_type: header.skin_type,
        type_label: type_label(header.skin_type),
        name: header.name.clone(),
        author: header.author.clone(),
        width: header.width,
        height: header.height,
        parser: parser_label(header.parser).to_owned(),
        categories: header.categories.len(),
        options: header.properties.iter().map(|row| RowReport { name: row.name.clone(), default: row.def.clone() }).collect(),
        file_slots: header.custom_files.iter().map(|slot| RowReport { name: slot.name.clone(), default: slot.default.clone() }).collect(),
        offsets: header.offsets.iter().map(|offset| RowReport { name: offset.name.clone(), default: None }).collect(),
        automatic_offsets: automatic_offsets(header.skin_type).len(),
    }
}

fn counts(rows: &[(&'static str, usize)]) -> Vec<CountReport> {
    rows.iter().map(|(kind, count)| CountReport { kind, count: *count }).collect()
}

fn objects(skin: &LoadedSkin) -> Vec<CountReport> {
    let def = &skin.def;
    counts(&[
        ("image", def.image.len()),
        ("imageset", def.imageset.len()),
        ("value", def.value.len()),
        ("floatvalue", def.floatvalue.len()),
        ("text", def.text.len()),
        ("slider", def.slider.len()),
        ("graph", def.graph.len()),
        ("gaugegraph", def.gaugegraph.len()),
        ("judgegraph", def.judgegraph.len()),
        ("bpmgraph", def.bpmgraph.len()),
        ("hiterrorvisualizer", def.hiterrorvisualizer.len()),
        ("timingvisualizer", def.timingvisualizer.len()),
        ("timingdistributiongraph", def.timingdistributiongraph.len()),
        ("note", usize::from(def.note.is_some())),
        ("gauge", usize::from(def.gauge.is_some())),
        ("hiddenCover", def.hidden_cover.len()),
        ("liftCover", def.lift_cover.len()),
        ("bga", usize::from(def.bga.is_some())),
        ("skinpreview", usize::from(def.skinpreview.is_some())),
        ("practice", usize::from(def.practice.is_some())),
        ("judge", def.judge.len()),
        ("songlist", usize::from(def.songlist.is_some())),
        ("pmchara", def.pmchara.len()),
        ("skinSelect", usize::from(def.skin_select.is_some())),
        ("customEvents", def.custom_events.len()),
        ("customTimers", def.custom_timers.len()),
    ])
}

fn file_report(skin: &LoadedSkin, id: &str, path: &Path) -> FileReport {
    let relative = path.strip_prefix(&skin.root).unwrap_or(path);
    FileReport { id: id.to_owned(), path: relative.to_string_lossy().replace('\\', "/"), exists: path.is_file() }
}

fn files(skin: &LoadedSkin) -> FilesReport {
    FilesReport {
        sources_declared: skin.def.source.len(),
        sources: skin.sources.iter().map(|(id, path)| file_report(skin, id, path)).collect(),
        fonts_declared: skin.def.font.len(),
        fonts: skin.fonts.iter().map(|(id, path)| file_report(skin, id, path)).collect(),
    }
}

fn diagnostics(skin: &LoadedSkin, lua: &LuaDiagnostics) -> DiagnosticsReport {
    DiagnosticsReport {
        warnings: skin.warnings.clone(),
        swallowed: lua.swallowed.iter().map(|error| EntryReport { text: error.message.clone(), count: error.count }).collect(),
        swallowed_overflow: lua.swallowed_overflow,
        function_failures: lua
            .function_failures
            .iter()
            .map(|failure| EntryReport {
                text: format!("function #{} ({}): {}", failure.function.0, kind_label(failure.kind), failure.first_message),
                count: failure.count,
            })
            .collect(),
        prints: lua.prints.iter().map(|line| EntryReport { text: line.text.clone(), count: line.count }).collect(),
        prints_overflow: lua.prints_overflow,
        frames_over_budget: lua.frames_over_budget,
    }
}

fn function_kinds(runtime: &SkinLua) -> Vec<CountReport> {
    let registered: Vec<LuaFnKind> = (0..runtime.function_count()).filter_map(|index| runtime.kind_of(LuaFnId(u32::try_from(index).ok()?))).collect();
    ALL_KINDS.iter().map(|kind| CountReport { kind: kind_label(*kind), count: registered.iter().filter(|candidate| *candidate == kind).count() }).collect()
}

fn body_report(skin: &LoadedSkin) -> BodyReport {
    let runtime = skin.runtime();
    let def = &skin.def;
    BodyReport {
        timings: TimingsReport { scene: def.scene, input: def.input, fadeout: def.fadeout, loadend: def.loadend, playstart: def.playstart, close: def.close },
        destinations: def.destination.len(),
        destinations_assembled: skin.destinations.len(),
        objects: objects(skin),
        functions: runtime.map_or(0, SkinLua::function_count),
        function_kinds: runtime.map_or_else(Vec::new, function_kinds),
        files: files(skin),
        options_selected: skin.selected_options.len(),
        lua_memory_bytes: runtime.map(SkinLua::memory_used),
        diagnostics: diagnostics(skin, &runtime.map_or_else(LuaDiagnostics::default, SkinLua::diagnostics)),
    }
}

/// Dumps one document: its header, its body against a scenario, and when asked what its function
/// values cost per frame and the smallest Lua budgets it runs inside.
pub fn dump_document(context: &Context<'_>, entry: &Path) -> DocumentReport {
    let user = SkinUserConfig::default();
    let options =
        SkinLoadOptions { rng_seed: Some(context.seed), write_overlay: Some(context.overlay), ..SkinLoadOptions::new(context.root, &user, Mode::BEAT_7K) };
    let file = entry.file_name().map_or_else(|| entry.display().to_string(), |name| name.to_string_lossy().into_owned());

    let (header, header_error) = match load_header(entry, options) {
        Ok(header) => (Some(header), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let screen = header.as_ref().map_or(Screen::Other, |header| Screen::of(header.skin_type));
    let mut report = DocumentReport {
        file,
        header: header.as_ref().map(header_report),
        header_error,
        scenario: String::new(),
        load_millis: 0.0,
        load_error: None,
        body: None,
        frames: None,
        frames_error: None,
        probe: None,
    };

    let scenario = match context.scenario {
        Some(scenario) => Ok(scenario.clone()),
        None => Scenario::builtin(screen),
    };
    let (scenario, mut host) = match scenario.and_then(|scenario| scenario.host().map(|host| (scenario, host))) {
        Ok(pair) => pair,
        Err(error) => {
            report.load_error = Some(error);
            return report;
        }
    };
    report.scenario = scenario.label.clone();

    let started = Instant::now();
    let loaded = load_skin_with_host(entry, options, &host);
    report.load_millis = started.elapsed().as_secs_f64() * MILLIS_PER_SECOND;
    let skin = match loaded {
        Ok(skin) => skin,
        Err(error) => {
            report.load_error = Some(error.to_string());
            return report;
        }
    };
    report.body = Some(body_report(&skin));

    if context.frames > 0
        && let Some(runtime) = skin.runtime()
    {
        match frames::measure(runtime, &mut host, context.frames) {
            Ok(measured) => report.frames = Some(measured),
            Err(error) => report.frames_error = Some(error),
        }
    }
    if context.probe_budget {
        report.probe = Some(probe::run(entry, options, &scenario));
    }
    report
}
