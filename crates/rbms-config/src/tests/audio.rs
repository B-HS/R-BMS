//! The AUDIO tab: what a hand-edited file is pulled back to, and how each row steps.

use super::*;

#[test]
fn restoring_audio_settings_clamps_a_hand_edited_file() {
    let mut c = Config::default();
    c.audio.device = Some("   ".into());
    c.audio.buffer_frames = Some(0);
    c.audio.sample_rate = Some(0);
    c.audio.polyphony = 100_000;
    c.audio.master = 4.0;
    c.audio.key = -1.0;
    c.audio.bg = f32::NAN;
    c.audio.system = 0.25;
    c.sanitise();
    assert_eq!(c.audio.device, None, "a blank device name means the system default");
    assert_eq!(c.audio.buffer_frames, None, "zero frames is not a buffer size");
    assert_eq!(c.audio.sample_rate, None);
    assert_eq!(c.audio.polyphony, AUDIO_POLYPHONY_MAX_VOICES);
    assert!((c.audio.master - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert!((c.audio.key - AUDIO_VOLUME_MIN_GAIN).abs() < 1e-6);
    assert!((c.audio.bg - DEFAULT_BUS_VOLUME).abs() < 1e-6, "a NaN gain falls back instead of silencing the bus");
    assert!((c.audio.system - 0.25).abs() < 1e-6);
    assert!(!c.audio.reopen_pending(), "restoring settings is not a parameter change");
}

#[test]
fn a_pending_reopen_is_not_part_of_the_document_two_configurations_are_compared_by() {
    let mut pending = Config::default();
    pending.audio.mark_reopen_pending();
    assert_eq!(pending, Config::default(), "a row waiting to be applied has not changed what would be written");
    assert!(pending.audio.reopen_pending(), "and the flag itself is still set");

    let mut moved = Config::default();
    moved.audio.polyphony += AUDIO_POLYPHONY_STEP_VOICES;
    assert_ne!(moved, Config::default(), "a parameter that actually moved is a difference");

    let mut sanitised = pending.clone();
    sanitised.sanitise();
    assert_eq!(sanitised.audio, pending.audio, "clearing the flag cannot change how the same document compares");
}

#[test]
fn the_shipped_audio_defaults_match_the_config_defaults() {
    assert_eq!(AudioOptions::default(), Config::default().audio);
}

#[test]
fn the_pending_reopen_flag_is_set_and_cleared_explicitly() {
    let mut audio = AudioOptions::default();
    assert!(!audio.reopen_pending());
    audio.mark_reopen_pending();
    assert!(audio.reopen_pending());
    audio.clear_reopen_pending();
    assert!(!audio.reopen_pending());
}

#[test]
fn the_pending_reopen_flag_is_never_written_to_disk() {
    let mut c = Config::default();
    c.audio.mark_reopen_pending();
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert!(!back.audio.reopen_pending(), "a pending reopen is live state, not configuration");
}

#[test]
fn a_volume_steps_by_five_percent_and_stops_at_the_ends() {
    assert_eq!(volume_percent(step_volume(0.5, 1)), 55);
    assert_eq!(volume_percent(step_volume(0.5, -1)), 45);
    assert_eq!(volume_percent(step_volume(1.0, 1)), AUDIO_VOLUME_MAX_PERCENT, "the loudest step stays at the top");
    assert_eq!(volume_percent(step_volume(0.0, -1)), 0, "a muted row cannot go negative");
    assert_eq!(volume_percent(step_volume(0.02, -1)), 0, "a partial step down lands on mute, not below it");
}

#[test]
fn stepping_a_volume_up_and_back_down_lands_on_the_same_percent() {
    let steps = (AUDIO_VOLUME_MAX_PERCENT - volume_percent(DEFAULT_BUS_VOLUME)) / AUDIO_VOLUME_STEP_PERCENT;
    let mut gain = DEFAULT_BUS_VOLUME;
    for _ in 0..steps {
        gain = step_volume(gain, 1);
    }
    assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT);
    for _ in 0..steps {
        gain = step_volume(gain, -1);
    }
    assert_eq!(volume_percent(gain), volume_percent(DEFAULT_BUS_VOLUME), "whole-percent steps do not drift");
}

#[test]
fn a_volume_row_only_ever_shows_a_multiple_of_its_step() {
    let mut gain = 0.0;
    for _ in 0..=(AUDIO_VOLUME_MAX_PERCENT / AUDIO_VOLUME_STEP_PERCENT) {
        assert_eq!(volume_percent(gain) % AUDIO_VOLUME_STEP_PERCENT, 0, "a step left the row off the percent grid");
        gain = step_volume(gain, 1);
    }
    assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT, "stepping past the loudest value stays there");
}

#[test]
fn volume_percent_reports_whole_percent_within_the_range() {
    assert_eq!(volume_percent(0.0), 0);
    assert_eq!(volume_percent(0.5), 50);
    assert_eq!(volume_percent(1.0), AUDIO_VOLUME_MAX_PERCENT);
    assert_eq!(volume_percent(9.0), AUDIO_VOLUME_MAX_PERCENT, "an out-of-range gain still shows a legal percent");
    assert_eq!(volume_percent(-2.0), 0);
}

#[test]
fn clamping_a_gain_falls_back_only_for_a_non_number() {
    assert!((clamp_volume(f32::NAN, DEFAULT_BUS_VOLUME) - DEFAULT_BUS_VOLUME).abs() < 1e-6);
    assert!((clamp_volume(2.0, DEFAULT_BUS_VOLUME) - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert!((clamp_volume(-2.0, DEFAULT_BUS_VOLUME) - AUDIO_VOLUME_MIN_GAIN).abs() < 1e-6);
    assert!((clamp_volume(0.25, DEFAULT_BUS_VOLUME) - 0.25).abs() < 1e-6);
}

#[test]
fn an_optional_choice_row_cycles_auto_first_and_wraps_both_ways() {
    let choices = AUDIO_BUFFER_FRAMES_CHOICES;
    assert_eq!(cycle_optional_u32(None, &choices, 1), Some(choices[0]));
    assert_eq!(cycle_optional_u32(Some(choices[0]), &choices, -1), None);
    assert_eq!(cycle_optional_u32(Some(choices[choices.len() - 1]), &choices, 1), None, "past the last size wraps to AUTO");
    assert_eq!(cycle_optional_u32(None, &choices, -1), Some(choices[choices.len() - 1]));
    assert_eq!(cycle_optional_u32(Some(333), &choices, 1), Some(choices[0]), "a size the row never offers steps from AUTO");
}

#[test]
fn the_sample_rate_row_offers_auto_and_the_four_rates() {
    assert_eq!(cycle_optional_u32(None, &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(44_100));
    assert_eq!(cycle_optional_u32(Some(44_100), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(48_000));
    assert_eq!(cycle_optional_u32(Some(96_000), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), None);
}

#[test]
fn the_device_row_cycles_the_default_and_the_reported_names() {
    let names = vec!["Built-in Output".to_string(), "Studio Monitors".to_string()];
    assert_eq!(cycle_device(None, &names, 1).as_deref(), Some("Built-in Output"));
    assert_eq!(cycle_device(Some("Built-in Output"), &names, 1).as_deref(), Some("Studio Monitors"));
    assert_eq!(cycle_device(Some("Studio Monitors"), &names, 1), None, "past the last device is the system default");
    assert_eq!(cycle_device(None, &names, -1).as_deref(), Some("Studio Monitors"));
    assert_eq!(
        cycle_device(Some("Unplugged Interface"), &names, 1).as_deref(),
        Some("Built-in Output"),
        "a device the host no longer reports steps from the default"
    );
    assert_eq!(cycle_device(None, &[], 1), None, "with no devices the row stays on the system default");
}

#[test]
fn polyphony_steps_by_sixty_four_within_the_voice_range() {
    assert_eq!(step_polyphony(DEFAULT_POLYPHONY_VOICES, 1), DEFAULT_POLYPHONY_VOICES + AUDIO_POLYPHONY_STEP_VOICES);
    assert_eq!(step_polyphony(DEFAULT_POLYPHONY_VOICES, -1), DEFAULT_POLYPHONY_VOICES - AUDIO_POLYPHONY_STEP_VOICES);
    assert_eq!(step_polyphony(AUDIO_POLYPHONY_MIN_VOICES, -1), AUDIO_POLYPHONY_MIN_VOICES);
    assert_eq!(step_polyphony(AUDIO_POLYPHONY_MAX_VOICES, 1), AUDIO_POLYPHONY_MAX_VOICES);
    assert_eq!(step_polyphony(0, -1), AUDIO_POLYPHONY_MIN_VOICES, "a hand-edited zero is pulled back into range");
}
