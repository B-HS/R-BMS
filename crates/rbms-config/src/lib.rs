//! The player's persisted configuration: one versioned document for everything the settings screen
//! edits, plus the migration that folds the pre-version `settings.ron` / `folders.ron` /
//! `tables.ron` trio into it.
//!
//! There is a single runtime type — [`Config`] — rather than a persisted shape and a runtime shape
//! kept in step by hand. Values that used to be stored as free text (the gauge, the note option)
//! travel through serde adapters, so the file keeps the vocabulary it always had while the program
//! works with the engine's own enums.
//!
//! The crate deliberately stays free of the window, audio and network stacks: it depends on the
//! engine crates that own the two enums it stores, and on the store crate for the durable write.

#![forbid(unsafe_code)]

mod audio;
mod error;
mod gauge;
mod io;
mod judge;
mod legacy;
mod schema;
mod settings;

#[cfg(test)]
mod tests;

pub use audio::{
    AUDIO_BUFFER_FRAMES_CHOICES, AUDIO_POLYPHONY_MAX_VOICES, AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_STEP_VOICES, AUDIO_SAMPLE_RATE_HZ_CHOICES,
    AUDIO_VOLUME_MAX_GAIN, AUDIO_VOLUME_MAX_PERCENT, AUDIO_VOLUME_MIN_GAIN, AUDIO_VOLUME_STEP_PERCENT, AudioOptions, DEFAULT_BUS_VOLUME, DEFAULT_MASTER_VOLUME,
    DEFAULT_POLYPHONY_VOICES, clamp_volume, cycle_device, cycle_optional_u32, step_polyphony, step_volume, volume_percent,
};
pub use error::ConfigError;
pub use gauge::{gauge_from_name, gauge_token};
pub use io::{LoadOutcome, load, migrate, save};
pub use judge::{
    GAUGE_AUTO_SHIFT_LABELS, GAUGE_SET_CYCLE, GAUGE_SET_LABELS, JUDGE_ALGORITHM_LABELS, LN_MODE_LABELS, ScoreTarget, TARGET_LABELS, algorithm_from_token,
    algorithm_token, gauge_auto_shift_from_token, gauge_auto_shift_token, gauge_set_from_token, gauge_set_token, ln_mode_from_token, ln_mode_token,
    target_from_token,
};
pub use legacy::{LEGACY_FOLDERS_FILE, LEGACY_TABLES_FILE, LegacyV0, merge_legacy_lists};
pub use rbms_store::write_atomic;
pub use schema::{
    CURRENT_SCHEMA_VERSION, Config, DEFAULT_HISPEED, DEFAULT_PLAYER_ID, DEFAULT_SKIN, DisplayOptions, HISPEED_MAX, HISPEED_MIN, HISPEED_STEP,
    JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_STEP_MS, JUDGE_RATE_DEFAULT_PERCENT, JUDGE_RATE_MAX_PERCENT, JUDGE_RATE_MIN_PERCENT,
    JUDGE_RATE_STEP_PERCENT, JUDGE_WIDTH_TIER_COUNT, JudgeOptions, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP, LEGACY_SCHEMA_VERSION,
    LN_MARGIN_DEFAULT_PERCENT, LN_MARGIN_MAX_PERCENT, LN_MARGIN_MIN_PERCENT, LN_MARGIN_STEP_PERCENT, LibraryOptions, NetworkOptions, PlayOptions,
    SINGLE_JUDGE_WIDTH_SCHEMA_VERSION, TOTAL_FROM_CHART, TOTAL_STEP, TableSource, UNMODIFIED_JUDGE_RATES,
};
pub use settings::{
    ACTION_VALUE, AUDIO_VOLUME_STEP_GAIN, AUTO_VALUE, AdjustOutcome, CUSTOM_VALUE, DEFAULT_VALUE, EMAIL_MAX_LEN, GAUGE_CYCLE, GAUGE_LABELS, HISPEED_DECIMALS,
    MILLISECOND_UNIT, NO_UNIT, NONE_VALUE, OFF_VALUE, ON_VALUE, PASSWORD_MAX_LEN, PERCENT_SCALE, PERCENT_UNIT, PLAYER_ID_MAX_LEN, SERVER_URL_MAX_LEN,
    SETTING_COUNT, SETTINGS, SettingDescriptor, SettingId, SettingKind, SettingTab, TOTAL_DECIMALS, TOTAL_MAX, UNSET_VALUE, adjust, cycle_values, descriptor,
    display_value, step_skin, tab_rows,
};
