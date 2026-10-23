//! The colour the notes SCRATCH AUTO plays are drawn in on the play menu: the row under SCRATCH AUTO
//! that reads it, the closed fold of its channels under that, and that the arrows, Enter and the
//! zero key do on them what they do on a colour of the play skin -- with the colour stored in the
//! play options, which is where every skin and the built-in field read it from.

use std::time::Instant;

use rbms_skin::dst::SkinColor;

use super::*;
use crate::app_play::SCRATCH_AUTO_SHIPPED_COLOR;
use crate::number_row::{REFUSAL_SHOWN_FOR, knob_rect};

/// How many rows the fold of the channels holds, and the colour the tests store, which no preset is.
const CHANNEL_COUNT: usize = 4;
const STORED: SkinColor = SkinColor::rgba(0x10, 0x20, 0x30, 0x40);

/// The shipped green in the preset one step after it, and in the two before it.
const PRESET_AFTER_GREEN: SkinColor = SkinColor::rgba(0x40, 0xe0, 0xff, u8::MAX);
const PRESET_TWO_BEFORE_GREEN: SkinColor = SkinColor::rgba(0xa0, 0xff, 0x40, u8::MAX);

/// The alpha a test puts the colour at, in steps, and the colour typed in hex.
const HALF_ALPHA_TYPED: &str = "128";
const TYPED_HEX: &str = "FF000080";
const TYPED_WITH_TOO_FEW_DIGITS: &str = "12";

impl Menu {
    /// The colour SCRATCH AUTO's notes are stored as, `None` while the shipped green is in force.
    fn stored_scratch_auto_color(&self) -> Option<SkinColor> {
        self.app.shared.config.play.scratch_auto_color
    }

    /// Show the rows of the channels, which the fold hides when the menu opens.
    fn open_channels(&mut self) {
        self.focus(Row::Fold(Fold::AutoChannels));
        self.key(KeyCode::Enter);
    }

    /// Type `text` into the row being typed into, a character at a time.
    fn type_digits(&mut self, text: &'static str) {
        for at in 0..text.len() {
            self.typed(&text[at..at + 1]);
        }
    }
}

/// The colour's row follows SCRATCH AUTO, with the fold of its channels under it, closed: opening
/// the fold lists a row for each channel, the red first and the alpha last.
#[test]
fn the_scratch_auto_colour_row_follows_scratch_auto_with_its_channels_folded() {
    let mut menu = Menu::over(bare_app("auto-color-rows"));
    let scratch_auto = menu.at(Row::Setting(SettingId::ScratchAuto));
    assert_eq!(&menu.state.rows[scratch_auto + 1..scratch_auto + 3], [Row::AutoColor, Row::Fold(Fold::AutoChannels)]);
    assert_eq!(menu.state.rows[scratch_auto + 3], Row::Setting(SettingId::AutoReplay), "the colour is not directly under SCRATCH AUTO");
    let fold = menu.line(Row::Fold(Fold::AutoChannels));
    assert_eq!((fold.label.as_str(), fold.value.as_str()), ("SCRATCH AUTO CHANNELS", format!("HIDDEN ({CHANNEL_COUNT})").as_str()));
    assert!(!menu.state.rows.iter().any(|row| matches!(row, Row::AutoChannel(_))), "a channel is on show");

    menu.open_channels();
    let fold_at = menu.at(Row::Fold(Fold::AutoChannels));
    assert_eq!(menu.line(Row::Fold(Fold::AutoChannels)).value, format!("SHOWN ({CHANNEL_COUNT})"));
    assert_eq!(menu.state.rows[fold_at + 1..fold_at + 1 + CHANNEL_COUNT], ColorChannel::ALL.map(Row::AutoChannel));
    let labels: Vec<String> = ColorChannel::ALL.map(|channel| menu.line(Row::AutoChannel(channel)).label).to_vec();
    assert_eq!(labels, ["SCRATCH AUTO COLOR R", "SCRATCH AUTO COLOR G", "SCRATCH AUTO COLOR B", "SCRATCH AUTO COLOR A"]);
    assert_eq!(menu.state.rows[fold_at + 1 + CHANNEL_COUNT], Row::Setting(SettingId::AutoReplay));
}

/// With nothing stored the row reads DEFAULT and the colour in force is the green the renderer
/// ships; storing one reads as that colour in hex.
#[test]
fn the_scratch_auto_colour_row_reads_default_until_a_colour_is_set() {
    let mut menu = Menu::over(bare_app("auto-color-default"));
    assert_eq!(menu.line(Row::AutoColor).value, "DEFAULT");
    assert_eq!(hex_color(scratch_auto_color(&menu.app.shared.config.play)), "#30E050FF");
    assert_eq!(scratch_auto_color(&menu.app.shared.config.play), SCRATCH_AUTO_SHIPPED_COLOR);

    menu.app.shared.config.play.scratch_auto_color = Some(STORED);
    assert_eq!(menu.line(Row::AutoColor).value, "#10203040");
}

/// The arrows go round the presets from the shipped green, keeping the alpha the colour has.
#[test]
fn the_arrows_go_round_the_presets_from_the_shipped_green_keeping_the_alpha() {
    let mut menu = Menu::over(bare_app("auto-color-arrows"));
    menu.focus(Row::AutoColor);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.stored_scratch_auto_color(), Some(PRESET_AFTER_GREEN));
    menu.keys(KeyCode::ArrowLeft, 2);
    assert_eq!(menu.stored_scratch_auto_color(), Some(PRESET_TWO_BEFORE_GREEN));

    menu.open_channels();
    menu.focus(Row::AutoChannel(ColorChannel::A));
    menu.key(KeyCode::Enter);
    menu.type_digits(HALF_ALPHA_TYPED);
    menu.key(KeyCode::Enter);
    menu.focus(Row::AutoColor);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x30, 0xe0, 0x50, 0x80)), "the step lost the alpha");
}

/// Enter opens the colour row to have a colour typed in hex; a figure that is no colour is not
/// stored and the row says so for a while.
#[test]
fn a_typed_hex_colour_is_stored_and_one_that_is_no_colour_is_refused() {
    let mut menu = Menu::over(bare_app("auto-color-hex"));
    menu.focus(Row::AutoColor);
    menu.key(KeyCode::Enter);
    assert_eq!(menu.state.numbers.editing(), Some(Row::AutoColor));
    menu.type_digits(TYPED_HEX);
    menu.key(KeyCode::Enter);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0xff, 0x00, 0x00, 0x80)));
    assert_eq!(menu.line(Row::AutoColor).value, "#FF000080");

    let now = Instant::now();
    menu.key(KeyCode::Enter);
    menu.type_digits(TYPED_WITH_TOO_FEW_DIGITS);
    menu.key_at(KeyCode::Enter, now);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0xff, 0x00, 0x00, 0x80)), "a figure that is no colour was stored");
    assert!(menu.state.numbers.is_refused(Row::AutoColor, now), "the row did not say it was refused");
    assert!(!menu.state.numbers.is_refused(Row::AutoColor, now + REFUSAL_SHOWN_FOR), "the row went on saying so");
}

/// The zero key gives the colour back to the shipped green, and puts a channel back to where the
/// green has it without touching the others; a channel already there stores nothing.
#[test]
fn the_zero_key_gives_the_colour_back_and_a_channel_its_shipped_value() {
    let mut menu = Menu::over(bare_app("auto-color-zero"));
    menu.open_channels();
    menu.focus(Row::AutoChannel(ColorChannel::G));
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.stored_scratch_auto_color(), None, "a channel that was where the green has it stored a colour");

    menu.app.shared.config.play.scratch_auto_color = Some(STORED);
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x10, 0xe0, 0x30, 0x40)), "the zero key moved more than its channel");

    menu.focus(Row::AutoColor);
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.stored_scratch_auto_color(), None);
    assert_eq!(menu.line(Row::AutoColor).value, "DEFAULT");
}

/// A channel row moves one channel of the colour in force, one step with an arrow and ten with
/// Shift, and the first move stores the colour whole.
#[test]
fn a_channel_row_moves_one_channel_of_the_colour() {
    let mut menu = Menu::over(bare_app("auto-color-channel"));
    menu.open_channels();
    let red = Row::AutoChannel(ColorChannel::R);
    menu.focus(red);
    assert_eq!(menu.steps_of(red), i64::from(SCRATCH_AUTO_SHIPPED_COLOR.r));
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x31, 0xe0, 0x50, u8::MAX)));
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x3b, 0xe0, 0x50, u8::MAX)));
    assert_eq!(menu.line(red).value, menu.steps_of(red).to_string());
}

/// The slider of a channel row carries the channel to wherever the pointer is, and the settings file
/// is written once, when the drag ends.
#[test]
fn a_channel_slider_carries_its_channel_and_the_file_is_written_when_the_drag_ends() {
    let mut menu = Menu::over(bare_app("auto-color-drag"));
    menu.open_channels();
    menu.frame();
    let settings = menu.app.shared.settings_path.clone();
    let slider = menu.slider_of(Row::AutoChannel(ColorChannel::B));
    let along = |fraction: f32| middle(knob_rect(slider, fraction));

    menu.click(along(1.0), MouseButton::Left);
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x30, 0xe0, u8::MAX, u8::MAX)), "a press on the slider did not put the channel there");
    menu.drag(along(0.0));
    assert_eq!(menu.stored_scratch_auto_color(), Some(SkinColor::rgba(0x30, 0xe0, 0x00, u8::MAX)), "the drag did not carry the channel");
    assert!(!settings.exists(), "the settings file was written while the slider was being dragged");

    menu.let_go(along(0.0));
    let written = rbms_config::load(&settings).expect("the file the drag wrote reads back").config;
    assert_eq!(written.play.scratch_auto_color, Some(SkinColor::rgba(0x30, 0xe0, 0x00, u8::MAX)));
}
