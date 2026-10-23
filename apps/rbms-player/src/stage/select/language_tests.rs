//! The browser's words in the language the screens are written in: the scene it hands the renderer,
//! the names of the folders of its list, and what each is worded again as when the language is
//! changed behind its back (on the settings screen) and the browser is come back to.
//!
//! The harness draws with the bundled font alone, which has no Korean or Japanese letters, so these
//! read the words the scene carries and not the pixels they are drawn as.

use rbms_model::Language;

use super::skinned_tests::Browser;
use super::tests::entry;
use super::*;
use crate::select_tree::TableRef;
use crate::text::settings::InstalledLanguage;

/// A browser no skin draws over three charts, with its first frame drawn.
fn built_in(tag: &str) -> Browser {
    let mut browser = Browser::over(tag, ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec(), false);
    browser.frame();
    browser
}

/// The scene the browser drew last.
fn scene(browser: &Browser) -> &SelectScene {
    browser.state.cached_scene.as_ref().expect("the browser drew its scene")
}

/// The labels of the figures of the detail panel of the chart under the cursor.
fn stat_labels(scene: &SelectScene) -> Vec<&'static str> {
    match &scene.detail {
        SelectDetail::Song(detail) => detail.stats.iter().map(|stat| stat.label).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn the_scene_is_worded_again_when_the_language_is_changed() {
    let mut browser = built_in("language-scene");
    assert_eq!(scene(&browser).guide, "\u{2191}\u{2193} MOVE   ENTER OPEN   ESC BACK");
    assert_eq!(scene(&browser).sort, "TITLE");
    assert_eq!(stat_labels(scene(&browser)), ["BPM", "DENSITY", "NOTES", "JUDGE", "LENGTH", "TOTAL"]);

    let _korean = InstalledLanguage::new(Language::Korean);
    browser.frame();
    assert_eq!(scene(&browser).guide, "\u{2191}\u{2193} 이동   ENTER 열기   ESC 뒤로");
    assert_eq!(scene(&browser).sort, "제목", "the order is named as the settings screen names it");
    assert_eq!(stat_labels(scene(&browser)), ["BPM", "밀도", "노트", "판정", "길이", "TOTAL"]);

    let _japanese = InstalledLanguage::new(Language::Japanese);
    browser.frame();
    assert_eq!(scene(&browser).sort, "タイトル");
    assert_eq!(stat_labels(scene(&browser)), ["BPM", "密度", "ノーツ", "判定", "長さ", "TOTAL"]);
}

#[test]
fn the_folders_of_the_list_are_named_again_when_the_language_is_changed() {
    let mut browser = built_in("language-folders");
    browser.app.shared.open_for_tests([]);
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame();
    let labels = |browser: &Browser| {
        browser
            .app
            .shared
            .select_items
            .iter()
            .filter_map(|item| match item {
                SelectItem::Folder { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&browser), ["ALL SONGS (3)"]);

    let _korean = InstalledLanguage::new(Language::Korean);
    browser.frame();
    assert_eq!(labels(&browser), ["모든 곡 (3)"]);
    assert_eq!(scene(&browser).rows.first().map(|row| row.title.as_str()), Some("모든 곡 (3)"), "the scene was assembled from the old names");
}

#[test]
fn the_titles_of_the_folders_that_are_open_are_in_the_installed_language() {
    let mut browser = built_in("language-titles");
    browser.app.shared.open_for_tests([SelectFolder::AllSongs]);
    assert_eq!(browser.app.shared.open_titles(), ["ALL SONGS"]);
    let _korean = InstalledLanguage::new(Language::Korean);
    assert_eq!(browser.app.shared.open_titles(), ["모든 곡"]);
    browser.app.shared.open_for_tests([SelectFolder::UserFolders]);
    assert_eq!(browser.app.shared.open_titles(), ["내 폴더"]);
    let courses = SelectFolder::TableCourses(TableRef::of(&browser.app.shared, 0));
    browser.app.shared.open_for_tests([courses]);
    assert_eq!(browser.app.shared.open_titles(), ["코스"], "the folder of the courses a table lists is named as the course tab is");
}

#[test]
fn the_hint_over_a_list_with_nothing_in_it_is_worded_in_the_installed_language() {
    let mut browser = Browser::over("language-hint", Vec::new(), false);
    browser.frame();
    assert_eq!(scene(&browser).empty_hint, Some(("WELCOME TO rbms", "Add your music folder \u{2014} press O or click FOLDERS below")));

    let _korean = InstalledLanguage::new(Language::Korean);
    browser.frame();
    assert_eq!(
        scene(&browser).empty_hint,
        Some(("rbms에 오신 것을 환영합니다", "곡 폴더를 추가하세요 \u{2014} O 키를 누르거나 아래의 폴더 버튼을 클릭하세요"))
    );
}

#[test]
fn the_course_tab_scene_is_worded_in_the_installed_language() {
    let browser = built_in("language-courses");
    let state = SelectState::on_courses(Vec::new(), &browser.app.shared.library);
    let english = state.build_scene(&browser.app.shared);
    assert_eq!(english.guide, "\u{2191}\u{2193} MOVE   ENTER START   SHIFT+TAB SONGS   ESC BACK");
    assert_eq!(english.empty_hint.map(|(title, _)| title), Some("NO COURSES"));

    let _japanese = InstalledLanguage::new(Language::Japanese);
    let japanese = state.build_scene(&browser.app.shared);
    assert_eq!(japanese.guide, "\u{2191}\u{2193} 移動   ENTER 開始   SHIFT+TAB 曲リスト   ESC 戻る");
    assert_eq!(japanese.empty_hint, Some(("コースなし", "設定ファイルの隣のcoursesフォルダにコースのjsonファイルを置いてください")));
    assert_eq!(japanese.header, "コース");
}

#[test]
fn the_guide_line_while_a_word_is_typed_and_after_it_is_worded_in_the_installed_language() {
    let mut browser = built_in("language-search");
    let _korean = InstalledLanguage::new(Language::Korean);
    browser.app.shared.searching = true;
    let typing = browser.state.build_scene(&browser.app.shared).guide;
    assert_eq!(typing, "입력해서 필터링   \u{2191}\u{2193} 이동   ENTER 확인   ESC 취소");
}
