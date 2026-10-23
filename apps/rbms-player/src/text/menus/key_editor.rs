//! Fixed words of the key editor, by language.
//!
//! The screen's own headings, hints and notices, the names of the lines (a lane, a control of a run, a shortcut) and the headings of the shortcut
//! groups. A key is named as the key config file names it (`ESC`, `ENTER`, `LSHIFT`), a controller element as the controller reports it, and the
//! terms every language keeps (HI-SPEED, GREEN NUMBER, LIFT, HIDDEN+, START, SELECT, IR RANKING) are written as they are.

rbms_model::translations! {
    /// The headings, hints and labels of the key editor's screen.
    pub(crate) enum KeyEditorText {
        Lanes => ("LANES", "레인", "レーン"),
        Play => ("PLAY", "플레이", "プレー"),
        Controller => ("CONTROLLER", "컨트롤러", "コントローラー"),
        Defaults => ("DEFAULTS", "기본값", "初期設定"),
        Title => ("KEY CONFIG", "키 설정", "キー設定"),
        KeysHint => (
            "UP DOWN MOVE   LEFT RIGHT KEYBOARD / CONTROLLER   ENTER BIND   DEL CLEAR   BACKSPACE DEFAULT   TAB MODE   PGUP PGDN GROUP   ESC SAVE",
            "UP DOWN 이동   LEFT RIGHT 키보드 / 컨트롤러   ENTER 지정   DEL 해제   BACKSPACE 기본값   TAB 모드   PGUP PGDN 그룹   ESC 저장",
            "UP DOWN 移動   LEFT RIGHT キーボード／コントローラー   ENTER 割り当て   DEL 解除   BACKSPACE 初期設定   TAB モード   PGUP PGDN グループ   ESC 保存",
        ),
        CaptureKeyHint => (
            "PRESS THE KEY TO BIND   ESC CANCELS",
            "지정할 키를 누르세요   ESC 취소",
            "割り当てるキーを押してください   ESC キャンセル",
        ),
        CapturePadHint => (
            "PRESS A BUTTON OR MOVE AN AXIS OF THE CONTROLLER   ESC CANCELS",
            "컨트롤러의 버튼을 누르거나 축을 움직이세요   ESC 취소",
            "コントローラーのボタンを押すか軸を動かしてください   ESC キャンセル",
        ),
        ActionColumn => ("ACTION", "동작", "操作"),
        KeyboardColumn => ("KEYBOARD", "키보드", "キーボード"),
        ModeLabel => ("EDIT MODE (LANES OF)", "편집할 모드 (레인)", "編集するモード（レーン）"),
        PadDeviceLabel => ("CONTROLLER DEVICE", "컨트롤러 장치", "コントローラーデバイス"),
        PadAnalogLabel => ("TURNTABLE ALGORITHM", "턴테이블 알고리즘", "ターンテーブルアルゴリズム"),
        RestoreLabel => ("RESTORE EVERY KEY AND BUTTON", "모든 키와 버튼을 기본값으로", "すべてのキーとボタンを初期設定に戻す"),
        RestoreArmedValue => ("ENTER AGAIN", "ENTER 한 번 더", "もう一度ENTER"),
        Capturing => ("PRESS...", "누르세요...", "押してください..."),
        AnyDevice => ("ANY", "모두", "すべて"),
    }
}

rbms_model::translations! {
    /// What the key editor says under its hint about the last thing asked of it, and about the cell under the cursor. A line is named in it as the
    /// editor names it ([`LaneLabelText`], [`ControlLabelText`], [`ShortcutLabelText`]).
    pub(crate) enum KeyEditorNoticeText {
        Taken => (
            "THAT KEY IS {0} - PICK ANOTHER, OR CLEAR THAT ROW FIRST",
            "이 키는 {0}에 쓰이고 있습니다 - 다른 키를 고르거나 먼저 그 줄을 해제하세요",
            "そのキーは{0}に使われています - 別のキーを選ぶか、先にその行を解除してください",
        ),
        Fixed => (
            "THAT KEY IS FIXED ON THE SCREEN THIS ROW BELONGS TO - PICK ANOTHER",
            "이 키는 이 줄이 속한 화면에서 고정되어 있습니다 - 다른 키를 고르세요",
            "そのキーはこの行が属する画面で固定されています - 別のキーを選んでください",
        ),
        Unnamed => (
            "THAT KEY CANNOT BE BOUND - PICK ANOTHER",
            "이 키는 지정할 수 없습니다 - 다른 키를 고르세요",
            "そのキーは割り当てられません - 別のキーを選んでください",
        ),
        KeyboardOnly => (
            "THIS ROW IS READ FROM THE KEYBOARD ONLY",
            "이 줄은 키보드로만 입력받습니다",
            "この行はキーボードからのみ読み取られます",
        ),
        LaneKept => (
            "A LANE ALWAYS HAS A KEY - IT IS BACK ON THE ONE IT SHIPS WITH",
            "레인에는 항상 키가 있어야 합니다 - 기본 키로 되돌렸습니다",
            "レーンには常にキーが必要です - 初期のキーに戻しました",
        ),
        RestoreArmed => (
            "ENTER AGAIN PUTS EVERY KEY AND EVERY CONTROLLER BUTTON BACK AS SHIPPED - ANY OTHER KEY CANCELS",
            "ENTER 한 번 더: 모든 키와 컨트롤러 버튼을 기본값으로 되돌립니다 - 다른 키를 누르면 취소",
            "もう一度ENTERを押すと、すべてのキーとコントローラーのボタンが初期設定に戻ります - 他のキーで取り消し",
        ),
        Restored => (
            "EVERY KEY AND EVERY CONTROLLER BUTTON IS BACK AS SHIPPED",
            "모든 키와 컨트롤러 버튼이 기본값으로 돌아왔습니다",
            "すべてのキーとコントローラーのボタンを初期設定に戻しました",
        ),
        SharedWith => (
            "SHARED WITH {0} - ONE OF THE TWO WILL NOT WORK",
            "같은 키를 쓰는 줄: {0} - 둘 중 하나는 동작하지 않습니다",
            "{0}と共有しています - どちらか一方は動作しません",
        ),
        FixedOnItsScreen => (
            "THIS KEY IS FIXED ON THE SCREEN THE ROW BELONGS TO - THE ROW WILL NOT WORK",
            "이 키는 이 줄이 속한 화면에서 고정되어 있어 이 줄은 동작하지 않습니다",
            "このキーはその行が属する画面で固定されているため、この行は動作しません",
        ),
        AlsoLane => (
            "ALSO {0} - ON THE SONG SELECT BOTH ACT",
            "{0}도 같은 키를 씁니다 - 선곡 화면에서는 둘 다 동작합니다",
            "{0}にも割り当て済みです - 選曲画面では両方が動作します",
        ),
    }
}

rbms_model::translations! {
    /// What a lane is called, by its place.
    pub(crate) enum LaneLabelText {
        Lane => ("LANE {0}", "레인 {0}", "レーン{0}"),
        Scratch => ("SCRATCH {0}", "스크래치 {0}", "スクラッチ{0}"),
        ScratchReverse => ("SCRATCH {0} REVERSE", "스크래치 {0} 역방향", "スクラッチ{0}逆回転"),
    }
}

rbms_model::translations! {
    /// What each control of a run is called. The lane cover comes down from the top of the field, so the amount going up is the cover coming down; the
    /// lift raises the judgment line and the hidden cover grows upwards from it: the lines say which way the thing moves and what that does.
    pub(crate) enum ControlLabelText {
        HiSpeedUp => ("HI-SPEED UP", "HI-SPEED 올리기", "HI-SPEEDを上げる"),
        HiSpeedDown => ("HI-SPEED DOWN", "HI-SPEED 내리기", "HI-SPEEDを下げる"),
        CoverUp => ("LANE COVER DOWN (COVER MORE)", "레인 커버 내리기 (더 가림)", "レーンカバーを下げる（隠す量を増やす）"),
        CoverDown => ("LANE COVER UP (COVER LESS)", "레인 커버 올리기 (덜 가림)", "レーンカバーを上げる（隠す量を減らす）"),
        CoverToggle => ("LANE COVER ON/OFF", "레인 커버 ON/OFF", "レーンカバーON/OFF"),
        LiftUp => ("LIFT UP (RAISE THE LINE)", "LIFT 올리기 (판정선을 올림)", "LIFTを上げる（判定ラインを上げる）"),
        LiftDown => ("LIFT DOWN (LOWER THE LINE)", "LIFT 내리기 (판정선을 내림)", "LIFTを下げる（判定ラインを下げる）"),
        LiftToggle => ("LIFT ON/OFF"),
        HiddenUp => ("HIDDEN+ UP (COVER MORE)", "HIDDEN+ 올리기 (더 가림)", "HIDDEN+を上げる（隠す量を増やす）"),
        HiddenDown => ("HIDDEN+ DOWN (COVER LESS)", "HIDDEN+ 내리기 (덜 가림)", "HIDDEN+を下げる（隠す量を減らす）"),
        HiddenToggle => ("HIDDEN+ ON/OFF"),
        GreenUp => ("GREEN NUMBER UP (SLOWER)", "GREEN NUMBER 올리기 (느리게)", "GREEN NUMBERを上げる（遅くする）"),
        GreenDown => ("GREEN NUMBER DOWN (FASTER)", "GREEN NUMBER 내리기 (빠르게)", "GREEN NUMBERを下げる（速くする）"),
        KeepGreenToggle => ("KEEP GREEN NUMBER ON/OFF"),
        OffsetUp => ("JUDGE OFFSET +1 MS (HITTING LATE)", "판정 오프셋 +1 MS (늦게 침)", "判定オフセット+1 MS（遅く押している）"),
        OffsetDown => ("JUDGE OFFSET -1 MS (HITTING EARLY)", "판정 오프셋 -1 MS (일찍 침)", "判定オフセット-1 MS（早く押している）"),
        AutoAdjustToggle => ("AUTO ADJUST ON/OFF", "자동 보정 ON/OFF", "自動補正ON/OFF"),
        Fine => ("FINE STEP WHILE HELD (UNSET: SHIFT)", "누르는 동안 미세 조정 (미지정 시 SHIFT)", "押している間は微調整（未設定時はSHIFT）"),
        Start => ("START"),
        Select => ("SELECT"),
    }
}

rbms_model::translations! {
    /// What each shortcut is called in the key editor.
    pub(crate) enum ShortcutLabelText {
        Search => ("SEARCH", "검색", "検索"),
        Filter => ("FILTER PANEL", "필터 패널", "フィルターパネル"),
        Sort => ("SORT ORDER (SHIFT: BACKWARDS)", "정렬 순서 (SHIFT: 거꾸로)", "並び順（SHIFT: 逆順）"),
        Practice => ("PRACTICE", "연습", "練習"),
        Favourite => ("FAVOURITE", "즐겨찾기", "お気に入り"),
        Records => ("RECORDS OF THE CHART", "차트 기록", "譜面の記録"),
        Ranking => ("IR RANKING"),
        ReloadTables => ("RELOAD DIFFICULTY TABLES", "난이도표 다시 불러오기", "難易度表の再読み込み"),
        OptionsPin => ("OPTIONS OVERLAY (PIN)", "옵션 오버레이 (고정)", "オプションオーバーレイ（固定）"),
        OptionsHold => ("OPTIONS OVERLAY (HOLD)", "옵션 오버레이 (누르는 동안)", "オプションオーバーレイ（押している間）"),
        Settings => ("SETTINGS (SHIFT: COURSE LIST)", "설정 (SHIFT: 코스 목록)", "設定（SHIFT: コース一覧）"),
        SkinMenu => ("SKIN MENU", "스킨 메뉴", "スキンメニュー"),
        PlayMenu => ("PLAY MENU", "플레이 메뉴", "プレーメニュー"),
        KeyConfig => ("KEY CONFIG", "키 설정", "キー設定"),
        Folders => ("MUSIC FOLDERS", "곡 폴더", "曲フォルダ"),
        Tables => ("DIFFICULTY TABLES", "난이도표", "難易度表"),
        ModeFilter => ("MODE FILTER", "모드 필터", "モードフィルター"),
        SortOrder => ("SORT ORDER", "정렬 순서", "並び順"),
        LongNoteMode => ("LONG NOTE MODE", "롱노트 모드", "ロングノートモード"),
        ReplaySlot => ("REPLAY SLOT", "리플레이 슬롯", "リプレイスロット"),
        DetailOptions => ("DETAIL OPTIONS (HOLD)", "상세 옵션 (누르는 동안)", "詳細オプション（押している間）"),
        Retry => ("PLAY THE CHART AGAIN", "차트 다시 플레이", "譜面をもう一度プレー"),
        NextSong => ("NEXT SONG", "다음 곡", "次の曲"),
        ReplayPause => ("PAUSE / RESUME", "일시정지 / 재개", "一時停止／再開"),
        ReplayFaster => ("PLAYBACK RATE UP", "재생 속도 올리기", "再生速度を上げる"),
        ReplaySlower => ("PLAYBACK RATE DOWN", "재생 속도 내리기", "再生速度を下げる"),
        ReplayForward => ("SEEK FORWARD", "앞으로 탐색", "早送り"),
        ReplayBack => ("SEEK BACK", "뒤로 탐색", "巻き戻し"),
    }
}

rbms_model::translations! {
    /// The headings the key editor lists the shortcuts under.
    pub(crate) enum ShortcutGroupText {
        Browser => ("SELECT", "선곡", "選曲"),
        Screens => ("SCREENS (OPENED FROM SELECT)", "화면 (선곡에서 열기)", "画面（選曲から開く）"),
        SkinBrowser => (
            "SELECT - NUMBER KEYS OF A SKIN'S BROWSER",
            "선곡 - 스킨 선곡 화면의 숫자 키",
            "選曲 - スキンの選曲画面の数字キー",
        ),
        Result => ("RESULT", "결과", "リザルト"),
        Replay => ("REPLAY ANALYSIS", "리플레이 분석", "リプレイ分析"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(KeyEditorText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(KeyEditorNoticeText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(LaneLabelText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(ControlLabelText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(ShortcutLabelText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(ShortcutGroupText::ALL), Vec::<String>::new());
    }

    #[test]
    fn a_name_the_editor_gives_a_line_is_not_given_to_two_lines_of_one_kind() {
        for language in rbms_model::Language::ALL {
            let controls: Vec<&str> = ControlLabelText::ALL.iter().map(|entry| entry.get(language)).collect();
            let shortcuts: Vec<&str> = ShortcutLabelText::ALL.iter().map(|entry| entry.get(language)).collect();
            for names in [controls, shortcuts] {
                let mut unique = names.clone();
                unique.sort_unstable();
                unique.dedup();
                assert_eq!(unique.len(), names.len(), "{language:?} gives one name to two lines: {names:?}");
            }
        }
    }
}
