//! The EXPORT row of the skin menu, and the key that puts a row's value onto the other screens:
//! when the row is listed, what running it asks for and writes, what is said while an export runs
//! and once it has ended, how one is called off, and what a row that other screens declare says and
//! does.
//!
//! The menu is opened over the adjustable pack of the skin menu's fixtures, whose three documents
//! declare an option and offsets under the same names. With [`CAPTURE_DIR_ENV`] naming a folder the
//! frames are saved there, and the same rows are drawn over the pack this repository ships.

use std::path::Path;

use rbms_render::text_width;
use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};

use super::*;
use crate::skin_select::adjustments::ADJUSTMENTS_FILE;
use crate::skin_select::export::{EXPORT_SUFFIX, ExportStanding, job_for_tests};
use crate::skin_select::fixtures::write_adjustable_pack;

/// The EXPORT row.
const EXPORT_ROW: SettingRow = SettingRow::Skin(SkinRow::Export);

/// The rows of the adjustable pack's browser document the tests step: the option the other two
/// documents declare too, the one that is this document's alone, and its opacity.
const THEME_ROW: SettingRow = SettingRow::Skin(SkinRow::Property(0));
const COVER_ROW: SettingRow = SettingRow::Skin(SkinRow::Property(1));
const PANEL_ROW: SettingRow = SettingRow::Skin(SkinRow::Offset(1, OffsetAxis::A));

/// How many percent the panel's opacity is stepped down by before the pack is exported, and what the
/// row then reads.
const PANEL_STEPS_DOWN: usize = 30;
const PANEL_STEPPED_DOWN: &str = "70%";

/// How many megabytes the export that is drawn as running has to write, and has written.
const RUNNING_MEGABYTES: (u64, u64) = (212, 640);

/// How many of its files that export has written, and has to write.
const RUNNING_FILES: (usize, usize) = (812, 2431);

/// How many bytes a megabyte is.
const MEGABYTE: u64 = 1024 * 1024;

/// An export part of the way through a pack of some size.
const RUNNING: ExportStanding =
    ExportStanding { files: RUNNING_FILES.1, files_done: RUNNING_FILES.0, bytes: RUNNING_MEGABYTES.1 * MEGABYTE, bytes_done: RUNNING_MEGABYTES.0 * MEGABYTE };

/// The most an export can have to say for itself while it runs.
const MOST: ExportStanding = ExportStanding {
    files: crate::skin_select::export::EXPORT_MAX_FILES,
    files_done: crate::skin_select::export::EXPORT_MAX_FILES,
    bytes: crate::skin_select::export::EXPORT_MAX_BYTES,
    bytes_done: crate::skin_select::export::EXPORT_MAX_BYTES,
};

/// An app whose pack is the adjustable pack, written beside its settings file, and whose menu is
/// configuring the song browser. Answers the app and the folder the pack and the settings are in.
fn app_with_adjustable_pack(tag: &str) -> (App, PathBuf) {
    let mut app = bare_app(tag);
    let home = app.shared.settings_path.parent().expect("the settings file has a folder").to_path_buf();
    let pack = write_adjustable_pack(&home);
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    (app, home)
}

impl Menu {
    /// Answer the export picker with `folder`, and draw until the export it begins has ended.
    fn export_into(&mut self, folder: &Path) {
        let (tx, handle) = crate::dialog::handle_for_tests();
        self.state.export_picker = Some(handle);
        tx.send(Some(folder.to_path_buf())).expect("the handle is still held");
        for _ in 0..SETTLE_FRAMES {
            self.frame();
            if self.state.export_picker.is_none() && self.state.export.is_none() {
                return;
            }
            std::thread::sleep(SETTLE_PAUSE);
        }
        panic!("the export did not end in {SETTLE_FRAMES} frames");
    }

    /// What is said under the preview of what the last export or key came to.
    fn said(&self) -> (String, String) {
        self.state.said.clone().expect("something was said under the preview")
    }
}

/// The EXPORT row is the last of the list while there is a pack to export, and is not listed while
/// there is none. It reads as a row that is run.
#[test]
fn the_export_row_is_listed_under_the_actions_while_there_is_a_pack() {
    let bare = Menu::over(bare_app("export-bare"));
    assert!(!bare.state.rows.contains(&EXPORT_ROW), "a menu with no pack offers to export one");

    let (app, _) = app_with_adjustable_pack("export-listed");
    let menu = Menu::over(app);
    assert_eq!(menu.state.rows.last(), Some(&EXPORT_ROW));
    assert_eq!(menu.at(EXPORT_ROW), menu.at(SettingRow::Fixed(SettingId::SkinReset)) + 1, "the row is not under RESET");
    assert_eq!(menu.state.rows[..menu.state.rows.len() - 1], menu.app.shared.settings_rows(SettingTab::Skin));
    let line = &menu.state.lines[menu.at(EXPORT_ROW)];
    assert_eq!((line.label.as_str(), line.value.as_str(), line.number), ("EXPORT", ">", None));
    assert_eq!(row_help(&menu.app.shared, EXPORT_ROW), SkinMenuText::ExportHelp.get(language()));
}

/// Everything the menu says of an export and of a row other screens declare fits the panel it is
/// said on, at the most it can have to say.
#[test]
fn what_is_said_of_an_export_and_of_a_shared_row_fits_the_panel() {
    rbms_render::font::use_embedded_fonts_only();
    let room = INFO_RECT.w - INFO_PAD * 2.0;
    let most_screens = rbms_config::SKIN_SCREEN_LABELS.len();
    let said = [
        SkinMenuText::ExportPickerNote.get(language()).to_string(),
        SkinMenuText::ExportListingNote.get(language()).to_string(),
        SkinMenuText::ExportHelp.get(language()).to_string(),
        SkinMenuText::SaidHelp.get(language()).to_string(),
        export_note(MOST),
        shared_row_note(1).expect("one other screen is said"),
        shared_row_note(most_screens).expect("every other screen is said"),
    ];
    for line in said {
        assert!(text_width(&line, INFO_SCALE) <= room, "{line}");
    }
    assert_eq!(shared_row_note(0), None);
    assert_eq!(export_note(ExportStanding::default()), SkinMenuText::ExportListingNote.get(language()));
    assert_eq!(export_note(RUNNING), "EXPORTING 812 / 2431 FILES - 212 / 640 MB - ESC CANCELS");
}

/// Enter, Right and a press on the row put the picker up; a step to the left does not. While it is
/// up the menu takes no key and no press.
#[test]
fn the_export_row_puts_a_picker_up_and_the_menu_waits_behind_it() {
    let (app, _) = app_with_adjustable_pack("export-picker");
    let mut menu = Menu::over(app);
    menu.focus(EXPORT_ROW);
    menu.key(KeyCode::ArrowLeft);
    assert!(menu.state.export_picker.is_none(), "a step to the left put the picker up");
    for opens in [KeyCode::Enter, KeyCode::ArrowRight] {
        menu.state.export_picker = None;
        assert!(matches!(menu.key(opens), Transition::Stay));
        assert!(menu.state.export_picker.is_some(), "{opens:?}");
    }
    menu.state.export_picker = None;
    menu.frame();
    menu.click(menu.middle_of_hot(Hot::SkinMenuRow(menu.at(EXPORT_ROW))));
    assert!(menu.state.export_picker.is_some(), "a press on the row did not put the picker up");

    let (tx, handle) = crate::dialog::handle_for_tests();
    menu.state.export_picker = Some(handle);
    let (skin, cursor) = (menu.app.shared.config.skin.clone(), menu.state.sel);
    for code in [KeyCode::ArrowUp, KeyCode::ArrowRight, KeyCode::Enter, KeyCode::Tab, APPLY_TO_ALL_KEY, KeyCode::Escape] {
        assert!(matches!(menu.key(code), Transition::Stay), "{code:?} left the menu behind the picker");
    }
    menu.frame();
    menu.click(menu.middle_of_hot(Hot::SkinMenuKind(0)));
    assert_eq!((&menu.app.shared.config.skin, menu.state.sel), (&skin, cursor), "a key or a press behind the picker changed something");
    assert!(menu.state.export_picker.is_some(), "the menu gave up on a picker nobody has answered");
    menu.save("export-picker");

    tx.send(None).expect("the handle is still held");
    menu.frame();
    assert!(menu.state.export_picker.is_none() && menu.state.export.is_none(), "a dismissed picker began an export");
    assert!(menu.state.said.is_none());
}

/// The folder the picker answers with is where the pack is exported: a new folder named after the
/// pack, holding the pack and the choices made in the menu, and said under the preview once it is
/// whole. A run that names that folder as its pack lists the rows where this one left them.
#[test]
fn the_folder_that_is_picked_gets_the_pack_and_the_choices_made_here() {
    let (app, home) = app_with_adjustable_pack("export-run");
    let mut menu = Menu::over(app);
    menu.focus(THEME_ROW);
    menu.key(KeyCode::ArrowRight);
    menu.focus(PANEL_ROW);
    menu.keys(KeyCode::ArrowLeft, PANEL_STEPS_DOWN);
    let shown = (menu.value_of(THEME_ROW), menu.value_of(PANEL_ROW));
    assert_eq!(shown, ("LIGHT".to_owned(), PANEL_STEPPED_DOWN.to_owned()));
    menu.focus(EXPORT_ROW);
    menu.frame();
    menu.save("export-row");

    let outbox = home.join("outbox");
    std::fs::create_dir_all(&outbox).expect("the outbox is writable");
    menu.export_into(&outbox);
    let exported = outbox.join(format!("pack{EXPORT_SUFFIX}"));
    assert!(exported.join("select.json").is_file() && exported.join("bg/night.png").is_file(), "the pack was not written to {}", exported.display());
    assert!(exported.join(ADJUSTMENTS_FILE).is_file(), "the choices were not written");
    let (heading, detail) = menu.said();
    assert!(heading.starts_with("EXPORTED ") && heading.contains(" FILES "), "{heading}");
    assert_eq!(Path::new(&detail), exported.canonicalize().expect("the export is there").as_path(), "the folder that was written is not the one said");
    assert_eq!(menu.state.focused(), Some(EXPORT_ROW), "the cursor left the row that was run");
    menu.frame();
    menu.save("export-done");

    menu.key(KeyCode::ArrowUp);
    assert!(menu.state.said.is_none(), "what an export came to is still said after a key");

    let mut next = bare_app("export-run-next");
    next.shared.set_skin_pack(Some(exported.to_string_lossy().into_owned()));
    next.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    let reopened = Menu::over(next);
    assert_eq!((reopened.value_of(THEME_ROW), reopened.value_of(PANEL_ROW)), shown, "a run that names the export does not list what was exported");
    assert!(reopened.app.shared.config.skin.custom.is_empty());
}

/// An export into the pack itself is refused, and why is said under the preview.
#[test]
fn an_export_that_is_refused_says_why() {
    let (app, home) = app_with_adjustable_pack("export-refused");
    let mut menu = Menu::over(app);
    menu.focus(EXPORT_ROW);
    menu.export_into(&home.join("pack").join("bg"));
    let (heading, detail) = menu.said();
    assert_eq!(heading, SkinMenuText::ExportFailedHeading.get(language()));
    assert!(detail.contains("inside"), "{detail}");
    menu.frame();
    menu.save("export-refused");
}

/// While an export runs the menu says how far it has got and goes on taking keys. Escape calls the
/// export off and stays on the menu; once the export has ended, Escape leaves as it always did.
#[test]
fn a_running_export_says_how_far_it_has_got_and_escape_calls_it_off() {
    let (app, _) = app_with_adjustable_pack("export-running");
    let mut menu = Menu::over(app);
    let (tx, job) = job_for_tests(RUNNING);
    menu.state.export = Some(job);
    menu.focus(EXPORT_ROW);
    menu.frame();
    menu.save("export-running");
    menu.key(KeyCode::Enter);
    assert!(menu.state.export_picker.is_none(), "a second export was begun beside the one that is running");
    menu.key(KeyCode::ArrowUp);
    assert_eq!(menu.state.focused(), Some(SettingRow::Fixed(SettingId::SkinReset)), "the menu took no key while the export ran");

    assert!(matches!(menu.key(KeyCode::Escape), Transition::Stay), "Escape left the menu with an export running");
    assert!(menu.state.export.as_ref().is_some_and(|job| job.is_called_off()), "Escape did not call the export off");
    tx.send(Err(crate::skin_select::export::ExportError::CalledOff)).expect("the job is still held");
    menu.frame();
    assert!(menu.state.export.is_none());
    assert_eq!(menu.said().0, SkinMenuText::ExportCancelledHeading.get(language()));
    menu.save("export-cancelled");
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back), "Escape does not leave the menu once the export has ended");
}

/// Leaving the menu calls a running export off.
#[test]
fn leaving_the_menu_calls_a_running_export_off() {
    let (app, _) = app_with_adjustable_pack("export-left");
    let mut menu = Menu::over(app);
    let (_tx, job) = job_for_tests(RUNNING);
    let progress = job.progress();
    menu.state.export = Some(job);
    menu.state.on_exit(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 });
    assert!(menu.state.export.is_none());
    assert_eq!(progress.standing(), RUNNING, "calling an export off changed how far it had got");
    assert!(progress.is_called_off(), "the export was left running with nobody to collect it");
}

/// A row other screens declare says how many do under the preview, and the key puts its value onto
/// them: stored for each of their documents, written to the settings file, and said. A row that is
/// this screen's alone says nothing and the key does nothing on it.
#[test]
fn a_row_other_screens_declare_says_so_and_one_key_puts_its_value_on_them() {
    let (app, _) = app_with_adjustable_pack("apply");
    let mut menu = Menu::over(app);
    assert_eq!(peers_note(&menu.app.shared, THEME_ROW).as_deref(), Some("ALSO ON 2 OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL"));
    assert_eq!(peers_note(&menu.app.shared, PANEL_ROW).as_deref(), Some("ALSO ON 1 OTHER SCREEN - A APPLIES THIS VALUE TO IT"));
    for alone in [COVER_ROW, SettingRow::Skin(SkinRow::File(0)), SettingRow::Fixed(SettingId::SkinReload), EXPORT_ROW] {
        assert_eq!(peers_note(&menu.app.shared, alone), None, "{alone:?}");
    }

    menu.focus(THEME_ROW);
    menu.key(KeyCode::ArrowRight);
    let without = menu.frame().pixel_checksum();
    menu.save("apply-note");
    menu.focus(COVER_ROW);
    assert_ne!(menu.frame().pixel_checksum(), without);
    let skin = menu.app.shared.config.skin.clone();
    menu.key(APPLY_TO_ALL_KEY);
    assert_eq!(menu.app.shared.config.skin, skin, "the key changed something from a row that is this screen's alone");
    assert!(menu.state.said.is_none());

    let _ = std::fs::remove_file(&menu.app.shared.settings_path);
    menu.focus(THEME_ROW);
    assert!(matches!(menu.key(APPLY_TO_ALL_KEY), Transition::Stay));
    let (heading, detail) = menu.said();
    assert_eq!(heading, SkinMenuText::AppliedHeading.get(language()));
    assert_eq!(detail, "THEME: LIGHT APPLIED TO 1 OTHER SCREEN (1 SKIPPED: NO ITEM OF THAT NAME)");
    menu.frame();
    menu.save("apply-done");
    let theme_of = |menu: &Menu, screen: i32| menu.app.shared.skins.line_for(&menu.app.shared.config, screen, SkinRow::Property(0)).1;
    assert_eq!(theme_of(&menu, SKIN_TYPE_DECIDE), "LIGHT");
    assert_eq!(theme_of(&menu, SKIN_TYPE_RESULT), "DARK", "a screen with no item of that name was changed");
    assert_eq!(menu.value_of(THEME_ROW), "LIGHT", "the screen being configured was moved by its own value");
    let written = rbms_config::load(&menu.app.shared.settings_path).expect("the key wrote the settings file").config;
    assert_eq!(written.skin, menu.app.shared.config.skin, "what was put onto the other screens is not in the settings file");

    menu.choose(SKIN_TYPE_DECIDE);
    assert_eq!(menu.value_of(THEME_ROW), "LIGHT", "the other screen's own rows do not show what was put onto it");
}

/// The rows of this file over the pack this repository ships, saved for a person to look at: the
/// EXPORT row under the cursor, an export part of the way through, and a row every screen of the
/// pack declares.
#[test]
fn the_export_row_and_a_shared_row_over_the_shipped_pack_can_be_captured() {
    if !capturing() {
        return;
    }
    let mut menu = Menu::over(app_with_pack("export-capture", &shipped_pack()));
    menu.choose(SKIN_TYPE_PLAY_7KEYS);
    menu.focus(EXPORT_ROW);
    menu.capture_at("export-shipped-row", RUN_SECONDS);
    let (_tx, job) = job_for_tests(RUNNING);
    menu.state.export = Some(job);
    menu.capture_at("export-shipped-running", WHERE_IT_STANDS);
    menu.state.export = None;

    let shared = menu.state.rows.iter().copied().find(|row| peers_note(&menu.app.shared, *row).is_some());
    if let Some(shared) = shared {
        menu.focus(shared);
        menu.capture_at("apply-shipped-note", WHERE_IT_STANDS);
    }
}
