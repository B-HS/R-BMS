//! Fixed words of the scene the browser hands the renderer, by language: the guide line under the list, the hints over a list with
//! nothing in it, the labels of the detail panel's figures and the sub line of the record modal.
//!
//! The figures themselves (`BPM`, the totals, `AUTO`) and the names the data supplies (a folder, a chart, an artist) are not here. `BPM`
//! and `TOTAL` are terms every language keeps and are written as they are.

rbms_model::translations! {
    /// Words of the browser's scene.
    pub(crate) enum SceneText {
        /// The header over the list at the top of a library whose songs folder has no name of its own.
        Root => ("ROOT", "루트", "ルート"),

        /// The guide line under the list, by what the browser is doing.
        QuitGuide => ("PRESS ESC AGAIN TO QUIT", "종료하려면 ESC를 한 번 더 누르세요", "終了するにはESCをもう一度押してください"),
        SearchTypingGuide => (
            "TYPE TO FILTER   \u{2191}\u{2193} MOVE   ENTER CONFIRM   ESC CANCEL",
            "입력해서 필터링   \u{2191}\u{2193} 이동   ENTER 확인   ESC 취소",
            "入力して絞り込み   \u{2191}\u{2193} 移動   ENTER 決定   ESC キャンセル",
        ),
        SearchOnShowGuide => (
            "\u{2191}\u{2193} MOVE   ENTER OPEN   / EDIT SEARCH   ESC CLEAR SEARCH",
            "\u{2191}\u{2193} 이동   ENTER 열기   / 검색 수정   ESC 검색 해제",
            "\u{2191}\u{2193} 移動   ENTER 開く   / 検索を編集   ESC 検索を解除",
        ),
        BrowseGuide => (
            "\u{2191}\u{2193} MOVE   ENTER OPEN   ESC BACK",
            "\u{2191}\u{2193} 이동   ENTER 열기   ESC 뒤로",
            "\u{2191}\u{2193} 移動   ENTER 開く   ESC 戻る",
        ),
        CourseGuide => (
            "\u{2191}\u{2193} MOVE   ENTER START   SHIFT+TAB SONGS   ESC BACK",
            "\u{2191}\u{2193} 이동   ENTER 시작   SHIFT+TAB 곡 목록   ESC 뒤로",
            "\u{2191}\u{2193} 移動   ENTER 開始   SHIFT+TAB 曲リスト   ESC 戻る",
        ),

        /// The first-run hint over an empty library: its title, and what it says under it in the built-in browser and in a skin's.
        WelcomeTitle => ("WELCOME TO rbms", "rbms에 오신 것을 환영합니다", "rbmsへようこそ"),
        WelcomeBody => (
            "Add your music folder \u{2014} press O or click FOLDERS below",
            "곡 폴더를 추가하세요 \u{2014} O 키를 누르거나 아래의 폴더 버튼을 클릭하세요",
            "曲フォルダを追加してください \u{2014} Oキーを押すか、下のフォルダボタンをクリックしてください",
        ),
        WelcomeBodySkinned => (
            "Add your music folder \u{2014} press O   (H lists every key)",
            "곡 폴더를 추가하세요 \u{2014} O 키를 누르세요   (H 키로 모든 키를 볼 수 있습니다)",
            "曲フォルダを追加してください \u{2014} Oキーを押してください   (Hキーで全キーを表示)",
        ),

        /// The hints over a list that has nothing in it, by why: a search that found nothing, a filter that takes everything out, an
        /// empty library, and no courses.
        NoResultsTitle => ("NO RESULTS", "결과 없음", "結果なし"),
        NoResultsBody => (
            "Try a different search  \u{00B7}  Esc to clear",
            "다른 검색어를 입력해 보세요  \u{00B7}  Esc 키로 해제",
            "別の検索語を試してください  \u{00B7}  Escキーで解除",
        ),
        NoChartsMatchTitle => ("NO CHARTS MATCH", "조건에 맞는 차트 없음", "条件に合う譜面なし"),
        NoChartsMatchBody => (
            "Press F2 to change the filter  \u{00B7}  Backspace there clears it",
            "F2 키로 필터를 바꾸세요  \u{00B7}  필터 화면에서 Backspace 키로 초기화",
            "F2キーでフィルターを変更  \u{00B7}  そこでBackspaceキーを押すとリセット",
        ),
        NoChartsTitle => ("NO CHARTS", "차트 없음", "譜面なし"),
        NoChartsBody => (
            "Press O to manage folders  \u{00B7}  T for difficulty tables",
            "O 키로 폴더 관리  \u{00B7}  T 키로 난이도표",
            "Oキーでフォルダ管理  \u{00B7}  Tキーで難易度表",
        ),
        NoCoursesTitle => ("NO COURSES", "코스 없음", "コースなし"),
        NoCoursesBody => (
            "Put course json files in the courses folder next to your settings",
            "설정 파일 옆의 courses 폴더에 코스 json 파일을 넣으세요",
            "設定ファイルの隣のcoursesフォルダにコースのjsonファイルを置いてください",
        ),

        /// The labels of the figures in the detail panel of a chart.
        StatDensity => ("DENSITY", "밀도", "密度"),
        StatNotes => ("NOTES", "노트", "ノーツ"),
        StatJudge => ("JUDGE", "판정", "判定"),
        StatLength => ("LENGTH", "길이", "長さ"),
        /// The average density in the DENSITY figure: notes a second.
        DensityValue => ("{0}/s", "{0}/초", "{0}/秒"),

        /// The sub line of the record modal: the mode, the random option, then the gauge the run was played on and the note on the
        /// rule version it was judged under, if any.
        ModalSub => ("{0}   {1}   GAUGE {2}{3}", "{0}   {1}   게이지 {2}{3}", "{0}   {1}   ゲージ {2}{3}"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(SceneText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_scene_carried_before_it_had_a_catalog() {
        let table: Vec<(SceneText, Vec<&dyn std::fmt::Display>, &str)> = vec![
            (SceneText::Root, vec![], "ROOT"),
            (SceneText::QuitGuide, vec![], "PRESS ESC AGAIN TO QUIT"),
            (SceneText::SearchTypingGuide, vec![], "TYPE TO FILTER   \u{2191}\u{2193} MOVE   ENTER CONFIRM   ESC CANCEL"),
            (SceneText::SearchOnShowGuide, vec![], "\u{2191}\u{2193} MOVE   ENTER OPEN   / EDIT SEARCH   ESC CLEAR SEARCH"),
            (SceneText::BrowseGuide, vec![], "\u{2191}\u{2193} MOVE   ENTER OPEN   ESC BACK"),
            (SceneText::CourseGuide, vec![], "\u{2191}\u{2193} MOVE   ENTER START   SHIFT+TAB SONGS   ESC BACK"),
            (SceneText::WelcomeTitle, vec![], "WELCOME TO rbms"),
            (SceneText::WelcomeBody, vec![], "Add your music folder \u{2014} press O or click FOLDERS below"),
            (SceneText::WelcomeBodySkinned, vec![], "Add your music folder \u{2014} press O   (H lists every key)"),
            (SceneText::NoResultsTitle, vec![], "NO RESULTS"),
            (SceneText::NoResultsBody, vec![], "Try a different search  \u{00B7}  Esc to clear"),
            (SceneText::NoChartsMatchTitle, vec![], "NO CHARTS MATCH"),
            (SceneText::NoChartsMatchBody, vec![], "Press F2 to change the filter  \u{00B7}  Backspace there clears it"),
            (SceneText::NoChartsTitle, vec![], "NO CHARTS"),
            (SceneText::NoChartsBody, vec![], "Press O to manage folders  \u{00B7}  T for difficulty tables"),
            (SceneText::NoCoursesTitle, vec![], "NO COURSES"),
            (SceneText::NoCoursesBody, vec![], "Put course json files in the courses folder next to your settings"),
            (SceneText::StatDensity, vec![], "DENSITY"),
            (SceneText::StatNotes, vec![], "NOTES"),
            (SceneText::StatJudge, vec![], "JUDGE"),
            (SceneText::StatLength, vec![], "LENGTH"),
            (SceneText::DensityValue, vec![&17], "17/s"),
            (SceneText::ModalSub, vec![&"7K", &"MIRROR", &"HARD", &" *"], "7K   MIRROR   GAUGE HARD *"),
        ];
        assert_eq!(table.len(), SceneText::ALL.len(), "an entry has no row in this table");
        for (text, args, expected) in table {
            assert_eq!(text.fill(Language::English, &args), expected, "{text:?}");
        }
    }

    #[test]
    fn the_modal_sub_line_puts_the_values_where_each_language_wants_them() {
        assert_eq!(SceneText::ModalSub.fill(Language::Korean, &[&"7K", &"MIRROR", &"HARD", &""]), "7K   MIRROR   게이지 HARD");
        assert_eq!(SceneText::ModalSub.fill(Language::Japanese, &[&"7K", &"MIRROR", &"HARD", &""]), "7K   MIRROR   ゲージ HARD");
    }
}
