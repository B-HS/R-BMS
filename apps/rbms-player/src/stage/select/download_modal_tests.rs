//! The modal a chart a difficulty table lists and the library does not hold opens on: every way of
//! asking to play such a chart opens it, it offers getting the chart only for a link to a zip
//! archive and opening a link only for one the browser may be handed, and while it is up it takes
//! every key, controller key and press before the list or the skin under it does.
//!
//! The browser and the folder picker it reaches for are the tests' own ([`Reached`]): nothing here
//! opens a program of the machine's or a native panel, and nothing asks the network.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use rbms_config::{SortMode, TableSource};
use rbms_download::LinkKind;
use rbms_skin::property::generated::{BUTTON_AUTOPLAY, BUTTON_PLAY, BUTTON_PRACTICE, BUTTON_REPLAY};
use rbms_table::{DifficultyTable, Md5Index, TableEntry};
use winit::keyboard::KeyCode;

use super::download_modal::{DOWNLOADS_ARE_TO_COME, DownloadHands, ModalButton};
use super::keys::SelectKey;
use super::skinned_tests::{Browser, KEY_FIVE, KEY_ONE, KEY_THREE, SHORT_STEP, SLOT_BELOW};
use super::tests::entry;
use super::*;
use crate::keyconfig::key_index_of;
use crate::pointer::PointerInput;
use crate::tablesrc::MatchedTable;

/// Where the frames below are saved for a person to open, when this names a folder: the variable
/// the capture harness reads.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// What the table calls itself.
const TABLE: &str = "STELLA";

/// The chart whose link is a zip archive the player can fetch: who it is by, its song's page (with
/// the characters a shell would read) and its chart's link.
const ZIP_MD5: &str = "miss-zip";
const ZIP_TITLE: &str = "Zip Chart";
const ZIP_ARTIST: &str = "Zip Artist";
const ZIP_SONG_URL: &str = "https://songs.test/event.cgi?action=More_def&num=94&event=146";
const ZIP_CHART_URL: &str = "https://stellabms.xyz/upload/4536";

/// The chart whose link is a page, and the chart whose links the browser may not be handed.
const PAGE_MD5: &str = "miss-page";
const PAGE_CHART_URL: &str = "https://charts.test/difference.html";
const REFUSED_MD5: &str = "miss-refused";

/// The folder a test picks for the downloads.
const PICKED_FOLDER: &str = "/home/player/downloads";

/// One entry of the table.
fn chart(md5: &str, title: &str, artist: &str, url: &str, url_diff: &str) -> TableEntry {
    TableEntry {
        md5: md5.to_owned(),
        sha256: String::new(),
        level: "3".to_owned(),
        title: title.to_owned(),
        artist: artist.to_owned(),
        url: url.to_owned(),
        url_diff: url_diff.to_owned(),
    }
}

/// The level lists the one chart the library holds, then three it does not: one linking a zip
/// archive, one linking a page, and one whose links are no web addresses.
fn entries() -> Vec<TableEntry> {
    vec![
        chart("md5-alpha", "alpha", "a", "", ""),
        chart(ZIP_MD5, ZIP_TITLE, ZIP_ARTIST, ZIP_SONG_URL, ZIP_CHART_URL),
        chart(PAGE_MD5, "Page Chart", "Page Artist", "https://songs.test/page", PAGE_CHART_URL),
        chart(REFUSED_MD5, "Refused Chart", "Refused Artist", "javascript:alert(1)", ""),
    ]
}

/// Put the table in `browser`, open on its level in the table's own order.
fn into_the_level(browser: &mut Browser, tag: &str) {
    let shared = &mut browser.app.shared;
    shared.config.library.tables = vec![TableSource { name: TABLE.to_owned(), location: format!("https://tables.test/download/{tag}/"), group: String::new() }];
    let table = DifficultyTable::from_parts(None, entries());
    let index = Md5Index::new(shared.library.md5s());
    shared.set_tables(vec![MatchedTable::of(TABLE.to_owned(), &table, &index)]);
    shared.config.library.sort = SortMode::Default;
    let path = [shared.table_folder(0), shared.level_folder(0, 0)];
    shared.open_for_tests(path);
    shared.sel = 0;
    shared.rebuild_select_items();
}

/// A browser no skin draws, open on the level with the cursor on the chart `md5`.
fn built_in_on(tag: &str, md5: &str) -> Browser {
    let mut browser = Browser::over(tag, vec![entry("alpha", "a", "5")], false);
    into_the_level(&mut browser, tag);
    put_the_cursor_on(&mut browser, md5);
    browser.frame();
    browser
}

/// A browser the fixture skin draws, open on the level with the cursor on the chart `md5`.
fn skinned_on(tag: &str, md5: &str) -> Browser {
    let mut browser = Browser::skinned(tag, vec![entry("alpha", "a", "5")]);
    into_the_level(&mut browser, tag);
    put_the_cursor_on(&mut browser, md5);
    browser.frame_after(SHORT_STEP);
    browser
}

fn put_the_cursor_on(browser: &mut Browser, md5: &str) {
    let shared = &mut browser.app.shared;
    let row = shared
        .select_items
        .iter()
        .position(|item| matches!(item, SelectItem::Missing(missing) if shared.missing_chart(missing).is_some_and(|chart| chart.md5 == md5)));
    shared.sel = row.expect("the level lists the chart");
}

/// What the modal reached outside the player for: the addresses it handed the browser and the
/// folder pickers it opened, each with the end a test answers it from.
#[derive(Default)]
struct Reached {
    opened: Vec<String>,
    pickers: Vec<Sender<Option<PathBuf>>>,
}

/// Hand the browser hands that note what they are asked for and open nothing.
fn watch(browser: &mut Browser) -> Rc<RefCell<Reached>> {
    let reached = Rc::new(RefCell::new(Reached::default()));
    let (opened, picked) = (Rc::clone(&reached), Rc::clone(&reached));
    browser.state.download_hands = DownloadHands {
        open_url: Box::new(move |url| opened.borrow_mut().opened.push(url.to_owned())),
        pick_folder: Box::new(move || {
            let (answer, handle) = crate::dialog::handle_for_tests();
            picked.borrow_mut().pickers.push(answer);
            handle
        }),
    };
    reached
}

/// The chart the modal is open over, if it is open.
fn modal_md5(browser: &Browser) -> Option<&str> {
    browser.state.download_modal.as_ref().map(|modal| modal.entry.md5.as_str())
}

fn focus(browser: &Browser) -> Option<ModalButton> {
    browser.state.download_modal.as_ref().map(|modal| modal.focus)
}

/// Where the frame drawn last put the region `wanted`, if it put one.
fn region(browser: &Browser, wanted: Hot) -> Option<(f32, f32)> {
    browser.app.shared.hot.iter().find(|(_, hot)| *hot == wanted).map(|(rect, _)| (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5))
}

/// A press of the left button at `at`, as the window hands it on.
fn left_press(browser: &mut Browser, at: (f32, f32)) {
    browser.pointer(at, PointerInput::Button { button: MouseButton::Left, pressed: true });
}

/// The lane of the controller whose key is the key index `index` in the mode that is up.
fn lane_of(browser: &Browser, index: usize) -> usize {
    let mode = browser.app.shared.mode;
    (0..mode.key).find(|lane| key_index_of(mode, *lane, ScratchDir::Forward) == Some(index)).expect("a lane carries the key")
}

/// Save the frame the browser last drew as `<name>.png` in the capture folder, when one is named.
fn capture(browser: &Browser, name: &str) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    image::save_buffer(folder.join(format!("{name}.png")), browser.pixels.rgba(), CW, CH, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

#[test]
fn entering_a_missing_chart_opens_the_modal() {
    crate::notify::exclusive(|| {
        let mut browser = built_in_on("download-open-enter", ZIP_MD5);
        let sel = browser.sel();
        assert!(matches!(browser.key(KeyCode::Enter), Transition::Stay), "a missing chart started something");
        assert_eq!(modal_md5(&browser), Some(ZIP_MD5), "Enter did not open the modal over the chart");
        assert_eq!(browser.sel(), sel, "the cursor moved");
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        assert!(said.is_empty(), "opening the modal said {said:?}");
        let modal = browser.state.download_modal.as_ref().expect("the modal is open");
        assert_eq!((modal.table_title.as_str(), modal.level_title.as_str()), (TABLE, "LV 3"), "where the table lists the chart");
    });

    let mut browser = built_in_on("download-open-pad", ZIP_MD5);
    let open = BUILT_IN_KEY_LAYOUT.indices_of(SelectKey::FolderOpen)[0];
    let lane = lane_of(&browser, open);
    assert!(matches!(browser.pad(lane), Transition::Stay));
    assert_eq!(modal_md5(&browser), Some(ZIP_MD5), "the controller key that opens a row did not open the modal");

    for (code, name) in [(KEY_ONE, "play"), (KEY_THREE, "practice"), (KEY_FIVE, "auto")] {
        let mut browser = skinned_on(&format!("download-open-key-{name}"), ZIP_MD5);
        browser.hold(code);
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "the {name} key started something");
        assert_eq!(modal_md5(&browser), Some(ZIP_MD5), "the {name} key did not open the modal");
        assert!(!browser.app.shared.practice_requested && !browser.app.shared.autoplay_once, "the {name} key asked for a run");
    }

    let mut browser = skinned_on("download-open-events", ZIP_MD5);
    for (event, name) in [(BUTTON_PLAY, "play"), (BUTTON_AUTOPLAY, "autoplay"), (BUTTON_PRACTICE, "practice"), (BUTTON_REPLAY, "replay")] {
        browser.state.download_modal = None;
        assert!(matches!(browser.state.run_event(&mut browser.app.shared, event, 0), Transition::Stay), "the {name} event started something");
        assert_eq!(modal_md5(&browser), Some(ZIP_MD5), "the {name} event did not open the modal");
    }
}

#[test]
fn get_chart_is_offered_only_for_a_direct_zip() {
    let mut browser = built_in_on("download-get-chart", ZIP_MD5);
    browser.key(KeyCode::Enter);
    let modal = browser.state.download_modal.as_ref().expect("the modal is open");
    assert_eq!(modal.chart_link, LinkKind::DirectZip);
    assert_eq!(
        modal.buttons(),
        [(ModalButton::GetChart, true), (ModalButton::OpenSongUrl, true), (ModalButton::OpenChartUrl, true), (ModalButton::Close, true)]
    );
    assert_eq!(modal.focus, ModalButton::GetChart, "the cursor starts on the first button that is on");

    browser.key(KeyCode::Escape);
    put_the_cursor_on(&mut browser, PAGE_MD5);
    browser.key(KeyCode::Enter);
    let modal = browser.state.download_modal.as_ref().expect("the modal is open");
    assert_eq!(modal.chart_link, LinkKind::Browser);
    assert_eq!(modal.buttons()[0], (ModalButton::GetChart, false), "a page is no archive to get");
    assert_eq!(modal.focus, ModalButton::OpenSongUrl);
    assert_eq!(modal.chart_url.as_deref(), Some(PAGE_CHART_URL));
}

#[test]
fn getting_a_chart_asks_for_the_download_folder_first() {
    crate::notify::exclusive(|| {
        let mut browser = built_in_on("download-folder-first", ZIP_MD5);
        let reached = watch(&mut browser);
        browser.app.shared.config.library.download_folder = None;
        browser.key(KeyCode::Enter);
        browser.key(KeyCode::Enter);
        browser.key(KeyCode::Enter);
        assert_eq!(reached.borrow().pickers.len(), 1, "one picker is opened, and not another while it is up");
        browser.frame();
        assert_eq!(browser.app.shared.config.library.download_folder, None, "nothing is kept before a folder is picked");

        let picker = reached.borrow_mut().pickers.remove(0);
        picker.send(None).expect("the modal still holds the picker");
        browser.frame();
        assert_eq!(browser.app.shared.config.library.download_folder, None, "a picker closed without a folder keeps none");
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        assert!(said.is_empty(), "a closed picker started the download: {said:?}");

        browser.key(KeyCode::Enter);
        let picker = reached.borrow_mut().pickers.remove(0);
        picker.send(Some(PathBuf::from(PICKED_FOLDER))).expect("the modal still holds the picker");
        browser.frame();
        assert_eq!(browser.app.shared.config.library.download_folder.as_deref(), Some(PICKED_FOLDER), "the folder picked is the download folder");
        crate::notify::drain(&mut said);
        assert_eq!(said, [(crate::notify::Level::Warn, DOWNLOADS_ARE_TO_COME.to_owned())], "the download was not started into the folder picked");

        browser.key(KeyCode::Enter);
        assert!(reached.borrow().pickers.is_empty(), "a player with a download folder is asked for one again");
        said.clear();
        crate::notify::drain(&mut said);
        assert_eq!(said, [(crate::notify::Level::Warn, DOWNLOADS_ARE_TO_COME.to_owned())]);
        assert!(reached.borrow().opened.is_empty(), "getting the chart opened the browser");
    });
}

#[test]
fn the_modal_takes_every_key_and_esc_closes_it() {
    let mut browser = built_in_on("download-keys", ZIP_MD5);
    let reached = watch(&mut browser);
    let (sel, path, sort) = (browser.sel(), browser.app.shared.opened(), browser.app.shared.config.library.sort);
    browser.key(KeyCode::Enter);

    for code in [KeyCode::ArrowDown, KeyCode::ArrowUp, KeyCode::F3, KeyCode::KeyR, KeyCode::Tab, KeyCode::Slash] {
        assert!(matches!(browser.key(code), Transition::Stay), "{code:?} left the browser");
    }
    assert_eq!((browser.sel(), browser.app.shared.opened(), browser.app.shared.config.library.sort), (sel, path.clone(), sort), "a key reached the list");
    assert!(!browser.app.shared.searching && browser.state.record_modal.is_none(), "a key reached the browser's shortcuts");

    let mut walked = Vec::new();
    for _ in 0..4 {
        browser.key(KeyCode::ArrowRight);
        walked.extend(focus(&browser));
    }
    assert_eq!(walked, [ModalButton::OpenSongUrl, ModalButton::OpenChartUrl, ModalButton::Close, ModalButton::GetChart], "the cursor goes round the row");
    browser.key(KeyCode::ArrowLeft);
    assert_eq!(focus(&browser), Some(ModalButton::Close));

    let turntable = lane_of(&browser, crate::keyconfig::SCRATCH_FORWARD_INDEX);
    browser.pad(turntable);
    browser.pad(turntable);
    assert_eq!(focus(&browser), Some(ModalButton::OpenSongUrl), "the turntable moves the cursor");
    assert!(matches!(browser.key(KeyCode::Enter), Transition::Stay));
    assert_eq!(reached.borrow().opened, [ZIP_SONG_URL], "the song's page was not handed to the browser");
    assert!(browser.state.download_modal.is_some(), "opening a page closed the modal");

    let close = BUILT_IN_KEY_LAYOUT.indices_of(SelectKey::FolderClose)[0];
    let lane = lane_of(&browser, close);
    browser.pad(lane);
    assert!(browser.state.download_modal.is_none(), "the controller key that closes a folder did not close the modal");
    assert_eq!((browser.sel(), browser.app.shared.opened()), (sel, path.clone()), "closing the modal left the folder");

    browser.key(KeyCode::Enter);
    assert!(matches!(browser.key(KeyCode::Escape), Transition::Stay));
    assert!(browser.state.download_modal.is_none(), "Esc did not close the modal");
    assert_eq!((browser.sel(), browser.app.shared.opened()), (sel, path), "Esc left the folder as well as the modal");
}

#[test]
fn a_press_outside_the_buttons_does_not_reach_the_list() {
    let mut browser = built_in_on("download-press-outside", ZIP_MD5);
    let reached = watch(&mut browser);
    let other_row = region(&browser, Hot::SelectRow(0)).expect("the list drew its first row");
    browser.key(KeyCode::Enter);
    browser.frame();
    capture(&browser, "download-modal");

    let sel = browser.sel();
    left_press(&mut browser, other_row);
    assert_eq!(browser.sel(), sel, "a press over the list moved the cursor under the modal");
    assert!(browser.state.download_modal.is_some(), "a press beside the buttons closed the modal");

    let chart_page = region(&browser, Hot::DownloadButton(2)).expect("the modal drew its chart page button");
    left_press(&mut browser, chart_page);
    assert_eq!(reached.borrow().opened, [ZIP_CHART_URL], "a press on the button did not open the chart's page");
    assert_eq!(focus(&browser), Some(ModalButton::OpenChartUrl), "the button pressed is the one under the cursor");

    let close = region(&browser, Hot::DownloadButton(3)).expect("the modal drew its close button");
    left_press(&mut browser, close);
    assert!(browser.state.download_modal.is_none(), "a press on CLOSE did not close the modal");
    assert_eq!(browser.sel(), sel);
}

#[test]
fn a_url_the_check_refuses_is_no_button() {
    let mut browser = built_in_on("download-refused", REFUSED_MD5);
    let reached = watch(&mut browser);
    browser.key(KeyCode::Enter);
    let modal = browser.state.download_modal.as_ref().expect("the modal is open");
    assert_eq!((modal.chart_link, modal.song_url.as_deref(), modal.chart_url.as_deref()), (LinkKind::Unusable, None, None));
    assert_eq!(
        modal.buttons(),
        [(ModalButton::GetChart, false), (ModalButton::OpenSongUrl, false), (ModalButton::OpenChartUrl, false), (ModalButton::Close, true)]
    );
    assert_eq!(modal.focus, ModalButton::Close);

    browser.frame();
    let buttons: Vec<Hot> = browser.app.shared.hot.iter().map(|(_, hot)| *hot).filter(|hot| matches!(hot, Hot::DownloadButton(_))).collect();
    assert_eq!(buttons, [Hot::DownloadButton(3)], "a button that is off takes a press");
    for code in [KeyCode::ArrowRight, KeyCode::ArrowLeft] {
        browser.key(code);
        assert_eq!(focus(&browser), Some(ModalButton::Close), "the cursor stopped on a button that is off");
    }
    browser.key(KeyCode::Enter);
    assert!(browser.state.download_modal.is_none(), "Enter on CLOSE did not close the modal");
    assert!(reached.borrow().opened.is_empty(), "an address the check refuses was handed to the browser");
}

#[test]
fn a_press_on_a_missing_bar_opens_the_modal_over_the_skin() {
    let mut browser = skinned_on("download-skinned", ZIP_MD5);
    let reached = watch(&mut browser);
    let sel = browser.sel();
    assert!(matches!(browser.click(SLOT_BELOW, MouseButton::Left), Transition::Stay));
    assert_eq!(modal_md5(&browser), Some(ZIP_MD5), "a press on the wheel did not open the modal over the chart under the cursor");
    assert_eq!(browser.sel(), sel);
    browser.frame_after(SHORT_STEP);
    capture(&browser, "download-modal-skinned");
    assert!(region(&browser, Hot::DownloadButton(0)).is_some(), "the modal is not drawn over the skin");

    browser.click(SLOT_BELOW, MouseButton::Left);
    browser.scroll(1.0);
    assert_eq!(browser.sel(), sel, "a press or the wheel reached the skin's list under the modal");
    assert!(browser.state.download_modal.is_some(), "a press beside the buttons closed the modal");

    let song_page = region(&browser, Hot::DownloadButton(1)).expect("the modal drew its song page button");
    left_press(&mut browser, song_page);
    assert_eq!(reached.borrow().opened, [ZIP_SONG_URL]);
}
