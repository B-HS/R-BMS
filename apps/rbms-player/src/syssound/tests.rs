use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rbms_judge::Judge;

use super::{
    BgmStep, SELECT_BGM_MIN_DURATION_US, SYSTEM_SOUND_COUNT, SYSTEM_SOUND_NAMESPACE, SelectBgm, SystemSound, SystemSoundSet, course_result_sound,
    guide_for_judge, resolve_sound, result_sound,
};

const TEST_WAV_RATE: u32 = 44_100;
const TEST_WAV_CHANNELS: u16 = 1;
const TEST_WAV_FRAMES: usize = 32;
const TEST_WAV_AMPLITUDE: i16 = 1_000;
const TEST_GAIN: f32 = 0.5;
const MUSIC_SECONDS: usize = 4;
const MUSIC_FRAMES: usize = TEST_WAV_RATE as usize * MUSIC_SECONDS;
const ENGINE_A: u64 = 1;
const ENGINE_B: u64 = 2;

/// A scratch directory of this test's own, removed and recreated so a rerun starts clean.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-syssound-tests-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal 16-bit little-endian PCM WAV the decoder accepts.
fn wav_bytes() -> Vec<u8> {
    wav_bytes_of(TEST_WAV_FRAMES)
}

fn wav_bytes_of(frames: usize) -> Vec<u8> {
    let samples: Vec<i16> = (0..frames).map(|i| if i % 2 == 0 { TEST_WAV_AMPLITUDE } else { -TEST_WAV_AMPLITUDE }).collect();
    let data_size = (samples.len() * 2) as u32;
    let byte_rate = TEST_WAV_RATE * TEST_WAV_CHANNELS as u32 * 2;
    let block_align = TEST_WAV_CHANNELS * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_size).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&TEST_WAV_CHANNELS.to_le_bytes());
    out.extend_from_slice(&TEST_WAV_RATE.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn write_wav(dir: &Path, sound: SystemSound) {
    std::fs::write(dir.join(format!("{}.wav", sound.file_stem())), wav_bytes()).unwrap();
}

/// A shared app state of this test's own: its settings live in a scratch directory.
fn fresh_shared(name: &str) -> crate::AppShared {
    crate::App::new(String::new(), crate::Config::default(), crate::LaunchOptions::default(), temp_dir(name).join("settings.ron")).shared
}

fn write_music(dir: &Path, sound: SystemSound) {
    std::fs::write(dir.join(format!("{}.wav", sound.file_stem())), wav_bytes_of(MUSIC_FRAMES)).unwrap();
}

#[test]
fn every_stem_is_non_empty_and_unique() {
    assert_eq!(SystemSound::ALL.len(), SYSTEM_SOUND_COUNT);
    let stems: HashSet<&str> = SystemSound::ALL.iter().map(|s| s.file_stem()).collect();
    assert_eq!(stems.len(), SYSTEM_SOUND_COUNT);
    for sound in SystemSound::ALL {
        let stem = sound.file_stem();
        assert!(!stem.is_empty(), "{sound:?} has an empty stem");
        assert!(!stem.contains('.'), "{sound:?} stem carries an extension");
        assert!(!stem.contains('/') && !stem.contains('\\'), "{sound:?} stem carries a path separator");
    }
}

#[test]
fn stems_match_the_reference_sound_set() {
    let expected = [
        "scratch",
        "f-open",
        "f-close",
        "o-change",
        "o-open",
        "o-close",
        "playready",
        "playstop",
        "clear",
        "fail",
        "resultclose",
        "course_clear",
        "course_fail",
        "course_close",
        "guide-pg",
        "guide-gr",
        "guide-gd",
        "guide-bd",
        "guide-pr",
        "guide-ms",
        "select",
        "decide",
    ];
    let actual: Vec<&str> = SystemSound::ALL.iter().map(|s| s.file_stem()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn slots_are_positions_and_ids_stay_in_the_namespace() {
    let mut ids = HashSet::new();
    for (index, sound) in SystemSound::ALL.into_iter().enumerate() {
        assert_eq!(sound.slot(), index);
        assert!(SYSTEM_SOUND_NAMESPACE.contains(sound.sample_id()));
        assert!(ids.insert(sound.sample_id()));
    }
    assert_eq!(ids.len(), SYSTEM_SOUND_COUNT);
    assert!(SYSTEM_SOUND_NAMESPACE.len as usize >= SYSTEM_SOUND_COUNT);
}

#[test]
fn system_ids_do_not_collide_with_play_or_preview() {
    for sound in SystemSound::ALL {
        let id = sound.sample_id();
        assert!(!rbms_audio::IdNamespace::PLAY.contains(id));
        assert!(!rbms_audio::IdNamespace::PREVIEW.contains(id));
    }
}

#[test]
fn guide_flag_covers_exactly_the_six_judgment_cues() {
    let guides: Vec<SystemSound> = SystemSound::ALL.into_iter().filter(|s| s.is_guide()).collect();
    assert_eq!(
        guides,
        vec![SystemSound::GuidePg, SystemSound::GuideGr, SystemSound::GuideGd, SystemSound::GuideBd, SystemSound::GuidePr, SystemSound::GuideMs]
    );
}

#[test]
fn bgm_flag_covers_exactly_select_and_decide() {
    let bgms: Vec<SystemSound> = SystemSound::ALL.into_iter().filter(|s| s.is_bgm()).collect();
    assert_eq!(bgms, vec![SystemSound::Select, SystemSound::Decide]);
}

#[test]
fn each_judgment_maps_to_its_own_guide_cue() {
    let judges = [Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Poor, Judge::Miss];
    let mapped: Vec<SystemSound> = judges.into_iter().map(guide_for_judge).collect();
    assert_eq!(
        mapped,
        vec![SystemSound::GuidePg, SystemSound::GuideGr, SystemSound::GuideGd, SystemSound::GuideBd, SystemSound::GuidePr, SystemSound::GuideMs]
    );
    assert!(mapped.iter().all(|s| s.is_guide()));
}

#[test]
fn result_cues_follow_the_clear_flag() {
    assert_eq!(result_sound(true), SystemSound::ResultClear);
    assert_eq!(result_sound(false), SystemSound::ResultFail);
    assert_eq!(course_result_sound(true), SystemSound::CourseClear);
    assert_eq!(course_result_sound(false), SystemSound::CourseFail);
}

#[test]
fn a_missing_folder_is_silent_for_every_sound() {
    let missing = std::env::temp_dir().join(format!("rbms-syssound-absent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing);
    let set = SystemSoundSet::load(&missing);
    assert_eq!(set.resolved_count(), 0);
    for sound in SystemSound::ALL {
        assert!(!set.is_resolved(sound));
        assert!(set.cue(sound, TEST_GAIN).is_none(), "{sound:?} should be silent");
    }
}

#[test]
fn an_empty_folder_is_silent_for_every_sound() {
    let dir = temp_dir("empty");
    let set = SystemSoundSet::load(&dir);
    assert_eq!(set.resolved_count(), 0);
    for sound in SystemSound::ALL {
        assert!(set.cue(sound, TEST_GAIN).is_none());
    }
}

#[test]
fn an_unset_folder_is_silent() {
    let set = SystemSoundSet::load_optional(None);
    assert_eq!(set.resolved_count(), 0);
    assert!(SystemSoundSet::silent().cue(SystemSound::Scratch, TEST_GAIN).is_none());
    assert!(!SystemSoundSet::default().guide_enabled());
}

#[test]
fn a_full_folder_resolves_every_slot() {
    let dir = temp_dir("full");
    for sound in SystemSound::ALL {
        write_wav(&dir, sound);
    }
    let mut set = SystemSoundSet::load(&dir);
    set.set_guide_enabled(true);
    assert_eq!(set.resolved_count(), SYSTEM_SOUND_COUNT);
    for sound in SystemSound::ALL {
        let cue = set.cue(sound, TEST_GAIN).unwrap();
        assert_eq!(cue.id, sound.sample_id());
        assert_eq!(cue.gain, TEST_GAIN);
    }
}

#[test]
fn one_present_stem_leaves_the_others_silent() {
    let dir = temp_dir("partial");
    write_wav(&dir, SystemSound::Scratch);
    let set = SystemSoundSet::load(&dir);
    assert_eq!(set.resolved_count(), 1);
    assert!(set.cue(SystemSound::Scratch, TEST_GAIN).is_some());
    assert!(set.cue(SystemSound::Decide, TEST_GAIN).is_none());
}

#[test]
fn wav_wins_over_the_other_containers() {
    let dir = temp_dir("extorder");
    for ext in ["mp3", "flac", "ogg", "wav"] {
        std::fs::write(dir.join(format!("select.{ext}")), b"placeholder").unwrap();
    }
    let (path, ext) = resolve_sound(&dir, SystemSound::Select).unwrap();
    assert_eq!(ext, "wav");
    assert_eq!(path, dir.join("select.wav"));
}

#[test]
fn a_stem_resolves_through_any_supported_container() {
    let dir = temp_dir("extfallback");
    std::fs::write(dir.join("decide.flac"), b"placeholder").unwrap();
    let (path, ext) = resolve_sound(&dir, SystemSound::Decide).unwrap();
    assert_eq!(ext, "flac");
    assert_eq!(path, dir.join("decide.flac"));
    assert!(resolve_sound(&dir, SystemSound::Scratch).is_none());
}

#[test]
fn an_undecodable_file_leaves_its_slot_silent() {
    let dir = temp_dir("corrupt");
    std::fs::write(dir.join("clear.wav"), vec![0u8; 64]).unwrap();
    write_wav(&dir, SystemSound::ResultFail);
    let set = SystemSoundSet::load(&dir);
    assert!(!set.is_resolved(SystemSound::ResultClear));
    assert!(set.is_resolved(SystemSound::ResultFail));
    assert_eq!(set.resolved_count(), 1);
}

#[test]
fn guide_cues_stay_silent_until_the_switch_is_on() {
    let dir = temp_dir("guide");
    write_wav(&dir, SystemSound::GuidePg);
    write_wav(&dir, SystemSound::Scratch);
    let mut set = SystemSoundSet::load(&dir);
    assert!(!set.guide_enabled());
    assert!(set.is_resolved(SystemSound::GuidePg));
    assert!(set.cue(SystemSound::GuidePg, TEST_GAIN).is_none());
    assert!(set.cue(SystemSound::Scratch, TEST_GAIN).is_some());
    set.set_guide_enabled(true);
    assert!(set.cue(SystemSound::GuidePg, TEST_GAIN).is_some());
    set.set_guide_enabled(false);
    assert!(set.cue(SystemSound::GuidePg, TEST_GAIN).is_none());
}

#[test]
fn gain_is_clamped_and_a_non_finite_gain_is_silent() {
    let dir = temp_dir("gain");
    write_wav(&dir, SystemSound::Scratch);
    let set = SystemSoundSet::load(&dir);
    assert_eq!(set.cue(SystemSound::Scratch, 2.0).unwrap().gain, 1.0);
    assert_eq!(set.cue(SystemSound::Scratch, -1.0).unwrap().gain, 0.0);
    assert_eq!(set.cue(SystemSound::Scratch, 0.0).unwrap().gain, 0.0);
    assert!(set.cue(SystemSound::Scratch, f32::NAN).is_none());
    assert_eq!(set.cue(SystemSound::Scratch, f32::INFINITY), None);
}

/// The set shipped inside the binary is a whole one: once the first run has written it out, every
/// stem resolves to a file that decodes, so a player who configured no folder hears every cue.
#[test]
fn the_shipped_set_resolves_every_slot_once_it_is_installed() {
    let home = temp_dir("shipped");
    let settings = home.join("settings.ron");
    assert!(crate::assets::install_default_sounds(&settings), "the shipped set did not install");
    let folder = crate::assets::default_sound_folder(&settings).expect("the installed set is found");

    let mut set = SystemSoundSet::load_optional(Some(&folder));
    set.set_guide_enabled(true);
    assert_eq!(set.resolved_count(), SYSTEM_SOUND_COUNT, "a shipped sound is missing or does not decode");
    for sound in SystemSound::ALL {
        assert!(set.cue(sound, TEST_GAIN).is_some(), "{sound:?} is silent in the shipped set");
    }
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_sound_reports_how_long_it_lasts() {
    let dir = temp_dir("duration");
    write_music(&dir, SystemSound::Select);
    write_wav(&dir, SystemSound::Scratch);
    let set = SystemSoundSet::load(&dir);
    assert_eq!(set.duration_us(SystemSound::Select), Some(MUSIC_SECONDS as i64 * 1_000_000));
    let cue = set.duration_us(SystemSound::Scratch).unwrap();
    assert_eq!(cue, TEST_WAV_FRAMES as i64 * 1_000_000 / i64::from(TEST_WAV_RATE));
    assert_eq!(set.duration_us(SystemSound::Decide), None, "a slot with no file has no length");
}

/// A short `select` is the cue rbms has always played when a chart is started; only a sound as long
/// as music is looped by the browser, and then it is not also played as that cue.
#[test]
fn a_select_sound_is_music_only_when_it_is_as_long_as_music() {
    let short = temp_dir("select-short");
    write_wav(&short, SystemSound::Select);
    let set = SystemSoundSet::load(&short);
    assert!(!set.select_plays_as_bgm());
    assert!(set.cue(SystemSound::Select, TEST_GAIN).is_some(), "a short select is still the cue for starting a chart");

    let long = temp_dir("select-long");
    write_music(&long, SystemSound::Select);
    let set = SystemSoundSet::load(&long);
    assert!(set.duration_us(SystemSound::Select).unwrap() >= SELECT_BGM_MIN_DURATION_US);
    assert!(set.select_plays_as_bgm());
    assert!(set.cue(SystemSound::Select, TEST_GAIN).is_none(), "music must not also be played once as the cue");
    let looped = set.loop_cue(SystemSound::Select, TEST_GAIN).unwrap();
    assert_eq!(looped.id, SystemSound::Select.sample_id());

    assert!(!SystemSoundSet::silent().select_plays_as_bgm());
}

/// Only `select` has the music rule: a long `decide` or `clear` is still a cue and still loopable.
#[test]
fn no_other_sound_is_held_back_for_being_long() {
    let dir = temp_dir("long-others");
    for sound in [SystemSound::Decide, SystemSound::ResultClear, SystemSound::CourseFail] {
        write_music(&dir, sound);
    }
    let set = SystemSoundSet::load(&dir);
    for sound in [SystemSound::Decide, SystemSound::ResultClear, SystemSound::CourseFail] {
        assert!(set.cue(sound, TEST_GAIN).is_some(), "{sound:?} lost its cue");
        assert!(set.loop_cue(sound, TEST_GAIN).is_some(), "{sound:?} cannot be looped");
    }
}

/// A loop is asked for under the same gates as a cue except the music rule.
#[test]
fn a_loop_is_silent_where_a_cue_would_be() {
    let dir = temp_dir("loop-gates");
    write_wav(&dir, SystemSound::ResultClear);
    write_wav(&dir, SystemSound::GuidePg);
    let mut set = SystemSoundSet::load(&dir);
    assert!(set.loop_cue(SystemSound::ResultClear, TEST_GAIN).is_some());
    assert!(set.loop_cue(SystemSound::ResultFail, TEST_GAIN).is_none(), "no file");
    assert!(set.loop_cue(SystemSound::GuidePg, TEST_GAIN).is_none(), "guides are off");
    assert!(set.loop_cue(SystemSound::ResultClear, f32::NAN).is_none(), "gain is not a number");
    set.set_guide_enabled(true);
    assert!(set.loop_cue(SystemSound::GuidePg, TEST_GAIN).is_some());
}

#[test]
fn the_music_starts_when_the_browser_is_up_and_stops_when_it_is_left() {
    let mut bgm = SelectBgm::default();
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Start);
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Idle, "a running loop is not started twice");
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Idle);
    assert_eq!(bgm.step(false, Some(ENGINE_A), false), BgmStep::Stop);
    assert_eq!(bgm.step(false, Some(ENGINE_A), false), BgmStep::Idle, "leaving twice stops once");
}

#[test]
fn music_that_never_ran_is_not_stopped() {
    let mut bgm = SelectBgm::default();
    assert_eq!(bgm.step(false, Some(ENGINE_A), false), BgmStep::Idle, "stopping it would cut the cue a short select plays on the way out");
    assert_eq!(bgm.step(false, None, false), BgmStep::Idle);
}

#[test]
fn the_music_is_turned_down_under_a_preview_and_back_up_without_being_restarted() {
    let mut bgm = SelectBgm::default();
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Start);
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Level(0.0));
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Idle, "a held loop is told once");
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Level(1.0));
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Idle);
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Level(0.0), "and again for the next preview");
    assert_eq!(bgm.step(false, Some(ENGINE_A), true), BgmStep::Stop, "leaving while a preview sounds still stops it");
}

#[test]
fn music_is_not_started_while_a_preview_is_sounding() {
    let mut bgm = SelectBgm::default();
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Idle);
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Idle);
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Start, "it begins, at full level, once the preview is gone");
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Idle);
}

#[test]
fn music_waits_for_a_stream_and_starts_again_on_a_reopened_one() {
    let mut bgm = SelectBgm::default();
    assert_eq!(bgm.step(true, None, false), BgmStep::Idle, "no stream, nothing to start on");
    assert_eq!(bgm.step(true, Some(ENGINE_A), false), BgmStep::Start);
    assert_eq!(bgm.step(true, Some(ENGINE_A), true), BgmStep::Level(0.0));
    assert_eq!(bgm.step(true, Some(ENGINE_B), true), BgmStep::Idle, "the new stream has no loop, and a held one is not started");
    assert_eq!(bgm.step(true, Some(ENGINE_B), false), BgmStep::Start);
    assert_eq!(bgm.step(true, None, false), BgmStep::Idle, "a stream that went away took the loop with it");
    assert_eq!(bgm.step(true, Some(ENGINE_B), false), BgmStep::Start);
}

#[test]
fn a_reopened_stream_gets_the_loop_again_at_full_level_even_after_being_held() {
    let mut bgm = SelectBgm::default();
    bgm.step(true, Some(ENGINE_A), false);
    bgm.step(true, Some(ENGINE_A), true);
    assert_eq!(bgm.step(true, Some(ENGINE_B), false), BgmStep::Start);
    assert_eq!(bgm.step(true, Some(ENGINE_B), true), BgmStep::Level(0.0), "the held flag does not outlive the loop it described");
}

/// A set with no select music never opens the output stream on the browser's account.
#[test]
fn a_set_without_select_music_leaves_the_stream_alone() {
    let mut shared = fresh_shared("no-music");
    shared.syssound = SystemSoundSet::silent();
    shared.drive_select_bgm(true, false, std::time::Duration::ZERO);
    shared.drive_select_bgm(true, true, std::time::Duration::ZERO);
    shared.drive_select_bgm(false, false, std::time::Duration::ZERO);
    assert!(shared.audio.is_none());

    let dir = temp_dir("cue-only");
    write_wav(&dir, SystemSound::Select);
    shared.syssound = SystemSoundSet::load(&dir);
    shared.drive_select_bgm(true, false, std::time::Duration::ZERO);
    assert!(shared.audio.is_none(), "a select cue is not music, so no stream is opened for it");
}

/// The whole chain on a real stream: the loop starts, is turned down without ending, and stops.
/// Silent: the System bus is set to nothing first.
#[test]
#[ignore = "requires a real audio output device"]
fn the_select_music_loops_is_held_and_stops_on_a_real_device() {
    const SETTLE: std::time::Duration = std::time::Duration::from_millis(300);
    let dir = temp_dir("device");
    write_music(&dir, SystemSound::Select);
    let mut shared = fresh_shared("device-settings");
    shared.syssound = SystemSoundSet::load(&dir);
    shared.ensure_audio();
    let engine = shared.audio.as_mut().expect("audio device");
    engine.set_bus_gain(rbms_audio::Bus::System, 0.0);
    shared.syssound.install(engine);

    shared.drive_select_bgm(true, false, std::time::Duration::ZERO);
    std::thread::sleep(SETTLE);
    assert_eq!(shared.audio.as_ref().unwrap().mix_stats().active_voices, 1, "the music did not start");

    shared.drive_select_bgm(true, true, std::time::Duration::ZERO);
    std::thread::sleep(SETTLE);
    assert_eq!(shared.audio.as_ref().unwrap().mix_stats().active_voices, 1, "turning the music down stopped it");

    shared.drive_select_bgm(false, false, std::time::Duration::ZERO);
    std::thread::sleep(SETTLE);
    assert_eq!(shared.audio.as_ref().unwrap().mix_stats().active_voices, 0, "the music did not stop");
}
