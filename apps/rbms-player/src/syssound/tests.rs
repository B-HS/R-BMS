use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rbms_judge::Judge;

use super::{SYSTEM_SOUND_COUNT, SYSTEM_SOUND_NAMESPACE, SystemSound, SystemSoundSet, course_result_sound, guide_for_judge, resolve_sound, result_sound};

const TEST_WAV_RATE: u32 = 44_100;
const TEST_WAV_CHANNELS: u16 = 1;
const TEST_WAV_FRAMES: usize = 32;
const TEST_WAV_AMPLITUDE: i16 = 1_000;
const TEST_GAIN: f32 = 0.5;

/// A scratch directory of this test's own, removed and recreated so a rerun starts clean.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-syssound-tests-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal 16-bit little-endian PCM WAV the decoder accepts.
fn wav_bytes() -> Vec<u8> {
    let samples: Vec<i16> = (0..TEST_WAV_FRAMES).map(|i| if i % 2 == 0 { TEST_WAV_AMPLITUDE } else { -TEST_WAV_AMPLITUDE }).collect();
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
