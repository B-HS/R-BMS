//! Names on the settings screen: what each row, group and section is called, and the line beside
//! the title that says what the open group holds, by language.
//!
//! The rows and groups are named once in `rbms_config::SETTINGS` in English, which the screen's
//! own comparisons and `rbms-cli config` read, so these are what the screen writes instead, not
//! what it tells rows apart by. Every row has an entry, because the screen, the skin menu and the
//! play menu all name rows through [`row_label`]; a row added to the table does not compile until
//! it is named here. Option names, gauge names and the other terms the glossary keeps are written
//! once and marked as kept.

use rbms_config::{SettingId, SettingTab};
use rbms_model::translations::Catalog;
use rbms_render::text::language;

rbms_model::translations! {
    /// What each row of the settings table is called on screen.
    pub(crate) enum RowLabelText {
        Autoplay => ("AUTOPLAY"),
        HiSpeed => ("HI-SPEED"),
        HiSpeedStep => ("HI-SPEED STEP"),
        SpeedFix => ("SPEED FIX"),
        FixHiSpeed => ("HI-SPEED FIX"),
        Random => ("RANDOM"),
        LaneOption => ("LANE OPTION", "레인 옵션", "レーンオプション"),
        LegacyNote => ("LEGACY NOTE"),
        FiveKeyLayout => ("5KEYS LAYOUT", "5KEYS 레이아웃", "5KEYSレイアウト"),
        PlayEscapeMode => ("ESCAPE"),
        Gauge => ("GAUGE", "게이지", "ゲージ"),
        Lift => ("LIFT"),
        LaneCover => ("LANE COVER", "레인 커버", "レーンカバー"),
        ScratchSide => ("SCRATCH SIDE", "스크래치 위치", "スクラッチ位置"),
        ScratchAuto => ("SCRATCH AUTO"),
        JudgeOffset => ("JUDGE OFFSET", "판정 오프셋", "判定オフセット"),
        Bga => ("BGA"),
        KeyConfig => ("KEY CONFIG", "키 설정", "キー設定"),
        KeyDebounce => ("KEY DEBOUNCE", "키 디바운스", "キーデバウンス"),
        PadDebounce => ("PAD DEBOUNCE", "패드 디바운스", "パッドデバウンス"),
        JudgeAlgorithm => ("JUDGE ALGORITHM", "판정 알고리즘", "判定アルゴリズム"),
        JudgeWidthKeyPGreat => ("JUDGE WIDTH KEY PG", "판정 폭 건반 PG", "鍵盤PG判定幅"),
        JudgeWidthKeyGreat => ("JUDGE WIDTH KEY GR", "판정 폭 건반 GR", "鍵盤GR判定幅"),
        JudgeWidthKeyGood => ("JUDGE WIDTH KEY GD", "판정 폭 건반 GD", "鍵盤GD判定幅"),
        JudgeWidthScratchPGreat => ("JUDGE WIDTH SCR PG", "판정 폭 스크래치 PG", "スクラッチPG判定幅"),
        JudgeWidthScratchGreat => ("JUDGE WIDTH SCR GR", "판정 폭 스크래치 GR", "スクラッチGR判定幅"),
        JudgeWidthScratchGood => ("JUDGE WIDTH SCR GD", "판정 폭 스크래치 GD", "スクラッチGD判定幅"),
        LongNoteMargin => ("LN MARGIN", "LN 마진", "LNマージン"),
        LnMode => ("LN MODE", "LN 모드", "LNモード"),
        GaugeSet => ("GAUGE SET", "게이지 세트", "ゲージセット"),
        CourseGauge => ("COURSE GAUGE", "코스 게이지", "コースゲージ"),
        GaugeAutoShift => ("GAUGE AUTO SHIFT", "게이지 자동 전환", "ゲージオートシフト"),
        BottomShiftableGauge => ("BOTTOM SHIFTABLE", "전환 하한 게이지", "切替下限ゲージ"),
        Target => ("TARGET", "타깃", "ターゲット"),
        TargetFallback => ("TARGET FALLBACK", "대체 타깃", "代替ターゲット"),
        TargetRate => ("TARGET RATE", "타깃 비율", "ターゲット率"),
        Total => ("TOTAL"),
        Skin => ("SKIN", "스킨", "スキン"),
        SkinPack => ("PACK FOLDER", "팩 폴더", "パックフォルダ"),
        SkinScreen => ("SCREEN", "화면", "画面"),
        SkinDocument => ("SKIN", "스킨", "スキン"),
        SkinInfo => ("LOADED", "로드 상태", "読込状態"),
        SkinReload => ("RELOAD", "다시 불러오기", "再読み込み"),
        SkinReset => ("RESET", "초기화", "リセット"),
        AutoCal => ("AUTO ADJUST", "자동 보정", "自動補正"),
        AutoReplay => ("AUTO REPLAY", "자동 리플레이", "自動リプレイ"),
        DebugMode => ("DEBUG MODE", "디버그 모드", "デバッグモード"),
        Font => ("FONT", "글꼴", "フォント"),
        ScoreGraph => ("SCORE GRAPH", "스코어 그래프", "スコアグラフ"),
        ResultGraphs => ("RESULT GRAPHS", "결과 그래프", "リザルトグラフ"),
        ReplayAnalysis => ("REPLAY ANALYSIS", "리플레이 분석", "リプレイ分析"),
        Preview => ("PREVIEW", "미리듣기", "プレビュー"),
        EnableLift => ("LIFT ON"),
        EnableCover => ("LANE COVER ON", "레인 커버 ON", "レーンカバーON"),
        Hidden => ("HIDDEN+"),
        EnableHidden => ("HIDDEN+ ON"),
        LaneCoverFineStep => ("COVER FINE STEP", "커버 미세 조정 간격", "カバー微調整の刻み"),
        WhiteNumber => ("WHITE NUMBER"),
        JudgeTextY => ("JUDGE TEXT Y", "판정 글자 Y", "判定文字Y"),
        Letterbox => ("LETTERBOX", "레터박스", "レターボックス"),
        WindowResolution => ("RESOLUTION", "해상도", "解像度"),
        WindowMode => ("WINDOW MODE", "창 모드", "ウィンドウモード"),
        Language => ("LANGUAGE", "언어", "言語"),
        Sort => ("SORT", "정렬", "ソート"),
        FavoriteOnly => ("FAVORITE ONLY", "즐겨찾기만", "お気に入りのみ"),
        PreviewVolume => ("PREVIEW VOLUME", "미리듣기 볼륨", "プレビュー音量"),
        PreviewFade => ("PREVIEW FADE", "미리듣기 페이드", "プレビューフェード"),
        ServerUrl => ("SERVER URL", "서버 URL", "サーバーURL"),
        PlayerId => ("PLAYER ID", "플레이어 ID", "プレーヤーID"),
        Account => ("ACCOUNT", "계정", "アカウント"),
        Email => ("EMAIL", "이메일", "メールアドレス"),
        Password => ("PASSWORD", "비밀번호", "パスワード"),
        Login => ("LOGIN", "로그인", "ログイン"),
        Register => ("REGISTER", "회원가입", "新規登録"),
        Logout => ("LOGOUT", "로그아웃", "ログアウト"),
        SyncSettings => ("SYNC SETTINGS", "설정 동기화", "設定の同期"),
        UploadSettings => ("UPLOAD SETTINGS NOW", "지금 설정 업로드", "今すぐ設定をアップロード"),
        DownloadSettings => ("DOWNLOAD SETTINGS NOW", "지금 설정 다운로드", "今すぐ設定をダウンロード"),
        SyncRecords => ("SYNC RECORDS NOW", "지금 기록 동기화", "今すぐ記録を同期"),
        AutoUploadReplay => ("AUTO UPLOAD REPLAY", "리플레이 자동 업로드", "リプレイ自動アップロード"),
        AutoUploadTrace => ("AUTO UPLOAD TRACE", "트레이스 자동 업로드", "トレース自動アップロード"),
        Rivals => ("RIVALS", "라이벌", "ライバル"),
        IrProfiles => ("IR PROFILES", "IR 프로필", "IRプロファイル"),
        MasterVolume => ("MASTER VOL", "마스터 볼륨", "マスター音量"),
        KeyVolume => ("KEY VOL", "키음 볼륨", "キー音量"),
        BgmVolume => ("BGM VOL", "BGM 볼륨", "BGM音量"),
        SystemVolume => ("SYSTEM VOL", "시스템 볼륨", "システム音量"),
        AudioDevice => ("AUDIO DEVICE", "오디오 장치", "オーディオデバイス"),
        AudioBuffer => ("BUFFER SIZE", "버퍼 크기", "バッファサイズ"),
        AudioSampleRate => ("SAMPLE RATE", "샘플레이트", "サンプルレート"),
        AudioPolyphony => ("POLYPHONY", "동시 발음 수", "同時発音数"),
        SoundFolder => ("SOUND FOLDER", "사운드 폴더", "サウンドフォルダ"),
        GuideSe => ("GUIDE SE", "가이드 SE", "ガイドSE"),
        KeepGreenNumber => ("KEEP GREEN NUMBER"),
        GreenNumber => ("GREEN NUMBER"),
        ShowMissingCharts => ("MISSING CHARTS", "없는 차트", "未所持の譜面"),
        RankingKeys => ("RANKING KEYS", "랭킹 키", "ランキングキー"),
        DownloadFolder => ("DOWNLOAD FOLDER", "다운로드 폴더", "ダウンロードフォルダ"),
        DiscordStatus => ("DISCORD STATUS", "DISCORD 상태", "DISCORDステータス"),
    }
}

impl RowLabelText {
    /// The entry that names `id`. A row added to the settings table is a compile error here until
    /// it is given a name.
    const fn of(id: SettingId) -> RowLabelText {
        match id {
            SettingId::Autoplay => RowLabelText::Autoplay,
            SettingId::HiSpeed => RowLabelText::HiSpeed,
            SettingId::HiSpeedStep => RowLabelText::HiSpeedStep,
            SettingId::SpeedFix => RowLabelText::SpeedFix,
            SettingId::FixHiSpeed => RowLabelText::FixHiSpeed,
            SettingId::Random => RowLabelText::Random,
            SettingId::LaneOption => RowLabelText::LaneOption,
            SettingId::LegacyNote => RowLabelText::LegacyNote,
            SettingId::FiveKeyLayout => RowLabelText::FiveKeyLayout,
            SettingId::PlayEscapeMode => RowLabelText::PlayEscapeMode,
            SettingId::Gauge => RowLabelText::Gauge,
            SettingId::Lift => RowLabelText::Lift,
            SettingId::LaneCover => RowLabelText::LaneCover,
            SettingId::ScratchSide => RowLabelText::ScratchSide,
            SettingId::ScratchAuto => RowLabelText::ScratchAuto,
            SettingId::JudgeOffset => RowLabelText::JudgeOffset,
            SettingId::Bga => RowLabelText::Bga,
            SettingId::KeyConfig => RowLabelText::KeyConfig,
            SettingId::KeyDebounce => RowLabelText::KeyDebounce,
            SettingId::PadDebounce => RowLabelText::PadDebounce,
            SettingId::JudgeAlgorithm => RowLabelText::JudgeAlgorithm,
            SettingId::JudgeWidthKeyPGreat => RowLabelText::JudgeWidthKeyPGreat,
            SettingId::JudgeWidthKeyGreat => RowLabelText::JudgeWidthKeyGreat,
            SettingId::JudgeWidthKeyGood => RowLabelText::JudgeWidthKeyGood,
            SettingId::JudgeWidthScratchPGreat => RowLabelText::JudgeWidthScratchPGreat,
            SettingId::JudgeWidthScratchGreat => RowLabelText::JudgeWidthScratchGreat,
            SettingId::JudgeWidthScratchGood => RowLabelText::JudgeWidthScratchGood,
            SettingId::LongNoteMargin => RowLabelText::LongNoteMargin,
            SettingId::LnMode => RowLabelText::LnMode,
            SettingId::GaugeSet => RowLabelText::GaugeSet,
            SettingId::CourseGauge => RowLabelText::CourseGauge,
            SettingId::GaugeAutoShift => RowLabelText::GaugeAutoShift,
            SettingId::BottomShiftableGauge => RowLabelText::BottomShiftableGauge,
            SettingId::Target => RowLabelText::Target,
            SettingId::TargetFallback => RowLabelText::TargetFallback,
            SettingId::TargetRate => RowLabelText::TargetRate,
            SettingId::Total => RowLabelText::Total,
            SettingId::Skin => RowLabelText::Skin,
            SettingId::SkinPack => RowLabelText::SkinPack,
            SettingId::SkinScreen => RowLabelText::SkinScreen,
            SettingId::SkinDocument => RowLabelText::SkinDocument,
            SettingId::SkinInfo => RowLabelText::SkinInfo,
            SettingId::SkinReload => RowLabelText::SkinReload,
            SettingId::SkinReset => RowLabelText::SkinReset,
            SettingId::AutoCal => RowLabelText::AutoCal,
            SettingId::AutoReplay => RowLabelText::AutoReplay,
            SettingId::DebugMode => RowLabelText::DebugMode,
            SettingId::Font => RowLabelText::Font,
            SettingId::ScoreGraph => RowLabelText::ScoreGraph,
            SettingId::ResultGraphs => RowLabelText::ResultGraphs,
            SettingId::ReplayAnalysis => RowLabelText::ReplayAnalysis,
            SettingId::Preview => RowLabelText::Preview,
            SettingId::EnableLift => RowLabelText::EnableLift,
            SettingId::EnableCover => RowLabelText::EnableCover,
            SettingId::Hidden => RowLabelText::Hidden,
            SettingId::EnableHidden => RowLabelText::EnableHidden,
            SettingId::LaneCoverFineStep => RowLabelText::LaneCoverFineStep,
            SettingId::WhiteNumber => RowLabelText::WhiteNumber,
            SettingId::JudgeTextY => RowLabelText::JudgeTextY,
            SettingId::Letterbox => RowLabelText::Letterbox,
            SettingId::WindowResolution => RowLabelText::WindowResolution,
            SettingId::WindowMode => RowLabelText::WindowMode,
            SettingId::Language => RowLabelText::Language,
            SettingId::Sort => RowLabelText::Sort,
            SettingId::FavoriteOnly => RowLabelText::FavoriteOnly,
            SettingId::PreviewVolume => RowLabelText::PreviewVolume,
            SettingId::PreviewFade => RowLabelText::PreviewFade,
            SettingId::ServerUrl => RowLabelText::ServerUrl,
            SettingId::PlayerId => RowLabelText::PlayerId,
            SettingId::Account => RowLabelText::Account,
            SettingId::Email => RowLabelText::Email,
            SettingId::Password => RowLabelText::Password,
            SettingId::Login => RowLabelText::Login,
            SettingId::Register => RowLabelText::Register,
            SettingId::Logout => RowLabelText::Logout,
            SettingId::SyncSettings => RowLabelText::SyncSettings,
            SettingId::UploadSettings => RowLabelText::UploadSettings,
            SettingId::DownloadSettings => RowLabelText::DownloadSettings,
            SettingId::SyncRecords => RowLabelText::SyncRecords,
            SettingId::AutoUploadReplay => RowLabelText::AutoUploadReplay,
            SettingId::AutoUploadTrace => RowLabelText::AutoUploadTrace,
            SettingId::Rivals => RowLabelText::Rivals,
            SettingId::IrProfiles => RowLabelText::IrProfiles,
            SettingId::MasterVolume => RowLabelText::MasterVolume,
            SettingId::KeyVolume => RowLabelText::KeyVolume,
            SettingId::BgmVolume => RowLabelText::BgmVolume,
            SettingId::SystemVolume => RowLabelText::SystemVolume,
            SettingId::AudioDevice => RowLabelText::AudioDevice,
            SettingId::AudioBuffer => RowLabelText::AudioBuffer,
            SettingId::AudioSampleRate => RowLabelText::AudioSampleRate,
            SettingId::AudioPolyphony => RowLabelText::AudioPolyphony,
            SettingId::SoundFolder => RowLabelText::SoundFolder,
            SettingId::GuideSe => RowLabelText::GuideSe,
            SettingId::KeepGreenNumber => RowLabelText::KeepGreenNumber,
            SettingId::GreenNumber => RowLabelText::GreenNumber,
            SettingId::ShowMissingCharts => RowLabelText::ShowMissingCharts,
            SettingId::RankingKeys => RowLabelText::RankingKeys,
            SettingId::DownloadFolder => RowLabelText::DownloadFolder,
            SettingId::DiscordStatus => RowLabelText::DiscordStatus,
        }
    }
}

/// What one row of the settings table is called, in the language the screens are drawn in.
pub(crate) fn row_label(id: SettingId) -> &'static str {
    RowLabelText::of(id).get(language())
}

rbms_model::translations! {
    /// What each group of rows is called on the rail, and over the play menu's rows.
    pub(crate) enum TabLabelText {
        PlaySpeed => ("SPEED", "속도", "速度"),
        PlayLane => ("LANE", "레인", "レーン"),
        Judge => ("JUDGE", "판정", "判定"),
        PlayChart => ("CHART", "차트", "譜面"),
        Display => ("DISPLAY", "표시", "表示"),
        Skin => ("SKIN", "스킨", "スキン"),
        Input => ("INPUT", "입력", "入力"),
        Network => ("NETWORK", "네트워크", "ネットワーク"),
        Audio => ("AUDIO", "오디오", "オーディオ"),
        Select => ("SELECT", "선곡", "選曲"),
    }
}

/// What one group of rows is called, in the language the screens are drawn in. The play menu's
/// JUDGE group and the settings screen's share their name.
pub(crate) fn tab_label(tab: SettingTab) -> &'static str {
    let entry = match tab {
        SettingTab::PlaySpeed => TabLabelText::PlaySpeed,
        SettingTab::PlayLane => TabLabelText::PlayLane,
        SettingTab::PlayJudge | SettingTab::Judge => TabLabelText::Judge,
        SettingTab::PlayChart => TabLabelText::PlayChart,
        SettingTab::Display => TabLabelText::Display,
        SettingTab::Skin => TabLabelText::Skin,
        SettingTab::Input => TabLabelText::Input,
        SettingTab::Network => TabLabelText::Network,
        SettingTab::Audio => TabLabelText::Audio,
        SettingTab::Select => TabLabelText::Select,
    };
    entry.get(language())
}

rbms_model::translations! {
    /// The headings over the sections a group is cut into.
    pub(crate) enum SectionText {
        Judging => ("JUDGING", "판정", "判定"),
        LongNotes => ("LONG NOTES", "롱노트", "ロングノート"),
        Gauge => ("GAUGE", "게이지", "ゲージ"),
        Language => ("LANGUAGE", "언어", "言語"),
        Window => ("WINDOW", "창", "ウィンドウ"),
        Screens => ("SCREENS", "화면", "画面"),
        BuiltInPlayField => ("BUILT-IN PLAY FIELD", "내장 플레이 필드", "内蔵プレーフィールド"),
        Diagnostics => ("DIAGNOSTICS", "진단", "診断"),
        Controls => ("CONTROLS", "조작", "操作"),
        ServerAndAccount => ("SERVER AND ACCOUNT", "서버와 계정", "サーバーとアカウント"),
        Scores => ("SCORES", "스코어", "スコア"),
        Discord => ("DISCORD"),
        Volume => ("VOLUME", "볼륨", "音量"),
        Output => ("OUTPUT", "출력", "出力"),
        SystemSounds => ("SYSTEM SOUNDS", "시스템 사운드", "システム音"),
        SongList => ("SONG LIST", "곡 목록", "曲リスト"),
        Preview => ("PREVIEW", "미리듣기", "プレビュー"),
        Downloads => ("DOWNLOADS", "다운로드", "ダウンロード"),
        Other => ("OTHER", "기타", "その他"),
    }
}

/// The heading over a section, in the language the screens are drawn in, given the English heading
/// the catalog of sections names it by. A heading no entry carries is written as it was given.
pub(crate) fn section_heading(english: &'static str) -> &'static str {
    SectionText::ALL.iter().find(|entry| entry.get(rbms_model::Language::English) == english).map_or(english, |entry| entry.get(language()))
}

/// The heading over the rows of a group that no section names.
pub(crate) fn other_heading() -> &'static str {
    SectionText::Other.get(language())
}

rbms_model::translations! {
    /// What a group holds, written beside the screen's title.
    pub(crate) enum GroupSummaryText {
        Judge => (
            "THE RULES A RUN IS JUDGED AND ITS GAUGE IS KEPT BY",
            "플레이를 판정하고 게이지를 관리하는 규칙",
            "プレーの判定とゲージ管理のルール"
        ),
        Display => ("THE WINDOW AND WHAT THE SCREENS SHOW", "창과 화면에 표시되는 내용", "ウィンドウと画面の表示内容"),
        Input => ("KEYS, CONTROLLERS AND THE SCRATCH LANE", "키, 컨트롤러, 스크래치 레인", "キー、コントローラー、スクラッチレーン"),
        Network => (
            "THE SCORE SERVER, YOUR ACCOUNT ON IT AND DISCORD",
            "스코어 서버, 서버의 내 계정, DISCORD",
            "スコアサーバー、そのアカウント、DISCORD"
        ),
        Audio => ("VOLUMES, THE OUTPUT DEVICE AND SYSTEM SOUNDS", "볼륨, 출력 장치, 시스템 사운드", "音量、出力デバイス、システム音"),
        Select => ("THE SONG LIST AND ITS PREVIEW", "곡 목록과 미리듣기", "曲リストとそのプレビュー"),
    }
}

/// What a group holds, in the language the screens are drawn in; empty for a group the settings
/// screen does not show.
pub(crate) fn group_summary(tab: SettingTab) -> &'static str {
    let entry = match tab {
        SettingTab::Judge => GroupSummaryText::Judge,
        SettingTab::Display => GroupSummaryText::Display,
        SettingTab::Input => GroupSummaryText::Input,
        SettingTab::Network => GroupSummaryText::Network,
        SettingTab::Audio => GroupSummaryText::Audio,
        SettingTab::Select => GroupSummaryText::Select,
        SettingTab::PlaySpeed | SettingTab::PlayLane | SettingTab::PlayJudge | SettingTab::PlayChart | SettingTab::Skin => return "",
    };
    entry.get(language())
}

#[cfg(test)]
mod tests {
    use rbms_config::{SETTINGS, descriptor};
    use rbms_model::Language;
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_catalog_is_written_in_every_language() {
        assert_eq!(catalog_problems(RowLabelText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(TabLabelText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(SectionText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(GroupSummaryText::ALL), Vec::<String>::new());
    }

    #[test]
    fn in_english_a_row_is_called_what_the_table_calls_it() {
        for id in SettingId::ALL {
            assert_eq!(RowLabelText::of(id).get(Language::English), descriptor(id).label, "{id:?}");
        }
        assert_eq!(RowLabelText::ALL.len(), SETTINGS.len());
    }

    #[test]
    fn in_english_a_group_is_called_what_the_table_calls_it() {
        for tab in SettingTab::ALL {
            let entry = match tab {
                SettingTab::PlaySpeed => TabLabelText::PlaySpeed,
                SettingTab::PlayLane => TabLabelText::PlayLane,
                SettingTab::PlayJudge | SettingTab::Judge => TabLabelText::Judge,
                SettingTab::PlayChart => TabLabelText::PlayChart,
                SettingTab::Display => TabLabelText::Display,
                SettingTab::Skin => TabLabelText::Skin,
                SettingTab::Input => TabLabelText::Input,
                SettingTab::Network => TabLabelText::Network,
                SettingTab::Audio => TabLabelText::Audio,
                SettingTab::Select => TabLabelText::Select,
            };
            assert_eq!(entry.get(Language::English), tab.label(), "{tab:?}");
        }
    }

    #[test]
    fn no_two_rows_of_a_group_are_called_alike_in_any_language() {
        for language in Language::ALL {
            for tab in SettingTab::SHOWN {
                let mut labels: Vec<&str> = SETTINGS.iter().filter(|row| row.tab == tab).map(|row| RowLabelText::of(row.id).get(language)).collect();
                let count = labels.len();
                labels.sort_unstable();
                labels.dedup();
                assert_eq!(labels.len(), count, "{tab:?} in {language:?} calls two rows alike");
            }
        }
    }

    #[test]
    fn a_heading_is_found_by_its_english_text_and_an_unknown_one_is_left_alone() {
        assert_eq!(section_heading("DISCORD"), "DISCORD");
        assert_eq!(section_heading("A HEADING NO ENTRY CARRIES"), "A HEADING NO ENTRY CARRIES");
        assert_eq!(other_heading(), "OTHER");
    }

    #[test]
    fn a_group_that_the_screen_does_not_show_says_nothing_of_itself() {
        assert_eq!(group_summary(SettingTab::Skin), "");
        assert_eq!(group_summary(SettingTab::PlaySpeed), "");
    }
}
