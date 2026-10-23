//! Fixed words of the notices a player acts on, of the status lines of the account and the records
//! sync, and of the native pickers' titles, by language.
//!
//! A notice is reported with [`crate::notify::notify_text`] and worded by the frame thread in the
//! language it has installed. What stays as it was written is a notice that is, or ends in, the text
//! of an operating system, a library or the score server, and one that only says how the program
//! got on inside (an audio device, a state that was not kept): they are for whoever reads standard
//! error. The terms every language keeps (judgements, gauges, lamps, options) and the names of the
//! rows of the settings are written as they are in every language.

use std::fmt::Display;

use rbms_model::translations::Catalog;
use rbms_render::text::language;

rbms_model::translations! {
    /// Words of the notices shown over every screen.
    pub(crate) enum NoticeText {
        ImportedPendingRuns => (
            "{0} runs kept while the score database was closed are in it now",
            "스코어 데이터베이스가 닫혀 있는 동안 보관한 플레이 {0}건을 가져왔습니다",
            "スコアデータベースが閉じている間に保管していたプレー{0}件を取り込みました",
        ),
        KeptRunNotRead => (
            "kept run not read ({0}): left where it is",
            "보관한 플레이를 읽지 못했습니다 ({0}): 그대로 둡니다",
            "保管したプレーを読み込めませんでした（{0}）: そのまま残します",
        ),
        KeptRunNotRemoved => (
            "kept run read in but not removed ({0}): remove it by hand",
            "보관한 플레이를 가져왔지만 삭제하지 못했습니다 ({0}): 직접 삭제해 주세요",
            "保管したプレーは取り込みましたが削除できませんでした（{0}）: 手動で削除してください",
        ),
        ScoreRecordsImported => (
            "{0} score records imported into the database ({1} assisted, kept as history only); {2} moved aside",
            "스코어 기록 {0}건을 데이터베이스로 가져왔습니다 (어시스트 {1}건은 이력으로만 보관); 따로 옮긴 파일: {2}",
            "スコア記録{0}件をデータベースに取り込みました（アシスト{1}件は履歴としてのみ保管）。退避したファイル: {2}",
        ),
        CourseStagesMoved => (
            "{0} course stages stored on the gauge the settings chose are on the course gauge they ran on now",
            "설정에서 고른 게이지로 저장돼 있던 코스 스테이지 {0}개를 실제로 플레이한 코스 게이지로 옮겼습니다",
            "設定で選んだゲージで保存されていたコースのステージ{0}件を、実際にプレーしたコースゲージに移しました",
        ),
        NoTablesToReload => (
            "no difficulty tables to reload",
            "다시 불러올 난이도표가 없습니다",
            "再読み込みする難易度表がありません",
        ),
        TablesBeingFetched => (
            "the tables are being fetched already",
            "난이도표를 이미 받아 오는 중입니다",
            "難易度表は既に取得中です",
        ),
        ReloadingTables => (
            "reloading {0} difficulty table(s)",
            "난이도표 {0}개를 다시 불러오는 중입니다",
            "難易度表{0}件を再読み込みしています",
        ),
        FetchingTablesFirstTime => (
            "fetching {0} difficulty table(s) for the first time",
            "난이도표 {0}개를 처음으로 받아 오는 중입니다",
            "難易度表{0}件を初めて取得しています",
        ),
        TablesReloaded => (
            "difficulty tables reloaded: {0} updated, {1} failed",
            "난이도표를 다시 불러왔습니다: 갱신 {0}개, 실패 {1}개",
            "難易度表を再読み込みしました: 更新{0}件、失敗{1}件",
        ),
        TablesFetchedFirstTime => (
            "difficulty tables fetched for the first time: {0} added, {1} failed",
            "난이도표를 처음으로 받아 왔습니다: 추가 {0}개, 실패 {1}개",
            "難易度表を初めて取得しました: 追加{0}件、失敗{1}件",
        ),
        TableListsNothing => (
            "table read, but it lists no charts and has no folder: {0}",
            "난이도표를 읽었지만 차트가 없고 폴더도 없습니다: {0}",
            "難易度表を読み込みましたが、譜面がなくフォルダもありません: {0}",
        ),
        TableAdded => (
            "table added: {0} ({1} local charts)",
            "난이도표를 추가했습니다: {0} (보유 차트 {1}개)",
            "難易度表を追加しました: {0}（所持譜面{1}件）",
        ),
        UserFoldersLoaded => (
            "user folders: {0} folders, {1} charts ({2} owned)",
            "사용자 폴더: 폴더 {0}개, 차트 {1}개 (보유 {2}개)",
            "ユーザーフォルダ: フォルダ{0}件、譜面{1}件（所持{2}件）",
        ),
        NoFoldersOnTheServer => (
            "the score server has no folders to load",
            "스코어 서버에 불러올 폴더가 없습니다",
            "スコアサーバーに読み込むフォルダがありません",
        ),
        RunKeptOnTheSide => (
            "score database not open: this run is kept on the side and goes into it when it opens",
            "스코어 데이터베이스가 열려 있지 않아 이 플레이를 따로 보관했습니다. 데이터베이스가 열리면 그곳에 들어갑니다",
            "スコアデータベースが開いていないため、このプレーを別に保管しました。データベースが開くと取り込まれます",
        ),
        AbandonedCourseKept => (
            "course ended before its last stage kept as FAILED",
            "마지막 스테이지 전에 끝난 코스를 FAILED 처리해 보관했습니다",
            "最後のステージの前に終了したコースをFAILEDとして保管しました",
        ),
        QuitRunKept => (
            "run cut short by quitting kept; it is sent on the next start",
            "종료로 중단된 플레이를 보관했습니다. 다음 실행 때 전송합니다",
            "終了で中断されたプレーを保管しました。次回の起動時に送信します",
        ),
        RetriedRunKept => (
            "failed run kept before starting again",
            "다시 시작하기 전에 실패한 플레이를 보관했습니다",
            "やり直す前に、失敗したプレーを保管しました",
        ),
        CourseSent => (
            "course sent",
            "코스를 전송했습니다",
            "コースを送信しました",
        ),
        CourseNotKeptScoreDatabaseClosed => (
            "course not kept: score database not open",
            "코스를 보관하지 못했습니다: 스코어 데이터베이스가 열려 있지 않습니다",
            "コースを保管できませんでした: スコアデータベースが開いていません",
        ),
        CourseStageNotInTheLibrary => (
            "course stage not in the library: {0}",
            "라이브러리에 없는 코스 스테이지입니다: {0}",
            "ライブラリにないコースのステージです: {0}",
        ),
        DefaultSkinNotFullyInstalled => (
            "default skin not fully installed: {0} files could not be written beside {1}; a screen without its skin uses the built-in layout",
            "기본 스킨이 완전히 설치되지 않았습니다: {1} 옆에 파일 {0}개를 쓰지 못했습니다. 스킨이 없는 화면은 내장 레이아웃을 사용합니다",
            "標準スキンが完全にはインストールされていません: {1}の隣にファイル{0}件を書き込めませんでした。スキンのない画面は内蔵レイアウトを使います",
        ),
        FontNotLoadedNoUsableFace => (
            "font load failed (no usable face): {0}",
            "글꼴을 불러오지 못했습니다 (사용할 수 있는 서체 없음): {0}",
            "フォントを読み込めませんでした（使用できる書体がありません）: {0}",
        ),
        ChartCouldNotBeRead => (
            "chart could not be read: {0}",
            "차트를 읽을 수 없습니다: {0}",
            "譜面を読み込めません: {0}",
        ),
        ChartMd5Mismatch => (
            "warning: chart md5 mismatch (replay {0} vs file {1}); replay may desync",
            "경고: 차트 md5 불일치 (리플레이 {0}, 파일 {1}). 리플레이가 어긋날 수 있습니다",
            "警告: 譜面のmd5が一致しません（リプレイ{0}、ファイル{1}）。リプレイがずれる可能性があります",
        ),
        AudioReopenFailed => (
            "audio: reopen failed — trying the previous device settings",
            "오디오: 다시 열지 못했습니다 — 이전 장치 설정으로 시도합니다",
            "オーディオ: 再オープンに失敗しました — 前のデバイス設定で試します",
        ),
        AudioStreamStopped => (
            "audio stream stopped — falling back to the wall clock",
            "오디오 스트림이 멈췄습니다 — 시스템 시계 기준으로 전환합니다",
            "オーディオストリームが停止しました — システム時計に切り替えます",
        ),
        TokenNotRevoked => (
            "logged out, but the server did not revoke the token (delete it on the web site): {0}",
            "로그아웃했지만 서버가 토큰을 폐기하지 않았습니다 (웹 사이트에서 직접 삭제해 주세요): {0}",
            "ログアウトしましたが、サーバーがトークンを無効にしませんでした（Webサイトで削除してください）: {0}",
        ),
        LetGoOfLoginTokenNotRevoked => (
            "a login that was let go of left its token valid on the server (delete it on the web site): {0}",
            "취소된 로그인의 토큰이 서버에서 계속 유효합니다 (웹 사이트에서 삭제해 주세요): {0}",
            "破棄されたログインのトークンがサーバー上で有効なままです（Webサイトで削除してください）: {0}",
        ),
        LayoutOf => (
            "layout of {0} ({1})",
            "{0}의 배치 ({1})",
            "{0}の配置（{1}）",
        ),
        LayoutOfLnModeDecides => (
            "layout of {0}: {1} lays out this chart's long notes as LN MODE {2} resolves them, so it matches the replay only if it was recorded on {2}",
            "{0}의 배치: {1} 배치는 이 차트의 롱노트를 LN MODE {2} 기준으로 정하므로, LN MODE {2} 기준으로 기록한 리플레이일 때만 같은 배치가 됩니다",
            "{0}の配置: {1}はこの譜面のロングノートをLN MODE {2}で解決するため、LN MODE {2}で記録したリプレイの場合にのみ同じ配置になります",
        ),
        LayoutRefusedChartNotInTheLibrary => (
            "layout not borrowed: the chart of that replay ({0}) is not in the library",
            "배치를 빌리지 않음: 그 리플레이의 차트가 라이브러리에 없음 ({0})",
            "配置を借りません: そのリプレイの譜面（{0}）がライブラリにありません",
        ),
        LayoutRefusedAnotherChart => (
            "layout not borrowed: that replay is of another chart (replay {0}, chart {1})",
            "배치를 빌리지 않음: 그 리플레이는 다른 차트의 것 (리플레이 {0}, 차트 {1})",
            "配置を借りません: そのリプレイは別の譜面のものです（リプレイ{0}、譜面{1}）",
        ),
        LayoutRefusedNoSeed => (
            "layout not borrowed: that replay does not state the seed its lanes were laid out from",
            "배치를 빌리지 않음: 그 리플레이에 레인 배치에 쓴 시드가 기록되어 있지 않음",
            "配置を借りません: そのリプレイにはレーン配置に使われたシードが記録されていません",
        ),
        LayoutRefusedUnsupportedOption => (
            "layout not borrowed: that replay was played on {0}, which this player cannot lay out",
            "배치를 빌리지 않음: 그 리플레이의 배치 옵션 ({0})은 이 플레이어가 만들 수 없음",
            "配置を借りません: そのリプレイは{0}でプレーされており、このプレーヤーでは配置を作れません",
        ),
        LayoutRefusedNotFreePlay => (
            "layout not borrowed: only a run played by hand outside a course or practice can borrow one",
            "배치를 빌리지 않음: 코스나 연습이 아닌 직접 플레이에서만 배치를 빌릴 수 있음",
            "配置を借りません: コースや練習ではなく、手で行うプレーでのみ配置を借りられます",
        ),
    }
}

rbms_model::translations! {
    /// Words of the status lines of the NETWORK tab and of the records sync, which the settings
    /// screen shows and the notices repeat.
    pub(crate) enum StatusText {
        SetServerUrlFirst => (
            "set SERVER URL first",
            "먼저 SERVER URL 행을 설정해 주세요",
            "先にSERVER URLを設定してください",
        ),
        LogInFirst => (
            "log in first",
            "먼저 로그인해 주세요",
            "先にログインしてください",
        ),
        AccountNotConfirmed => (
            "account not confirmed yet",
            "아직 계정이 확인되지 않았습니다",
            "アカウントはまだ確認されていません",
        ),
        AccountRequestsDropped => (
            "account or server changed: the request in flight was dropped",
            "계정 또는 서버가 바뀌어 진행 중이던 요청을 취소했습니다",
            "アカウントまたはサーバーが変わったため、実行中のリクエストを破棄しました",
        ),
        SetPlayerIdFirst => (
            "set PLAYER ID to your account id first",
            "먼저 PLAYER ID 행에 계정 ID를 입력해 주세요",
            "先にPLAYER IDにアカウントIDを入力してください",
        ),
        SetPasswordFirst => (
            "set PASSWORD first",
            "먼저 PASSWORD 행을 입력해 주세요",
            "先にPASSWORDを入力してください",
        ),
        SetEmailFirst => (
            "set EMAIL first",
            "먼저 EMAIL 행을 입력해 주세요",
            "先にEMAILを入力してください",
        ),
        LoggedOut => (
            "logged out",
            "로그아웃했습니다",
            "ログアウトしました",
        ),
        UploadingSettings => (
            "uploading settings...",
            "설정을 업로드하는 중...",
            "設定をアップロード中...",
        ),
        DownloadingSettings => (
            "downloading settings...",
            "설정을 다운로드하는 중...",
            "設定をダウンロード中...",
        ),
        SettingsDownloaded => (
            "settings downloaded",
            "설정을 다운로드했습니다",
            "設定をダウンロードしました",
        ),
        SettingsBlobUnreadable => (
            "settings blob unreadable: {0}",
            "설정 데이터를 읽을 수 없습니다: {0}",
            "設定データを読み込めません: {0}",
        ),
        SettingsUploaded => (
            "settings uploaded",
            "설정을 업로드했습니다",
            "設定をアップロードしました",
        ),
        SettingsUploadedLockNotRefreshed => (
            "settings uploaded (lock not refreshed; download before the next save)",
            "설정을 업로드했습니다 (잠금을 갱신하지 못했으니 다음 저장 전에 다운로드해 주세요)",
            "設定をアップロードしました（ロックを更新できませんでした。次の保存の前にダウンロードしてください）",
        ),
        SettingsUploadWorkerStopped => (
            "settings upload worker stopped",
            "설정 업로드 작업이 중단되었습니다",
            "設定アップロードの処理が停止しました",
        ),
        SettingsDownloadWorkerStopped => (
            "settings download worker stopped",
            "설정 다운로드 작업이 중단되었습니다",
            "設定ダウンロードの処理が停止しました",
        ),
        ServerCopy => (
            "{0} (server copy {1})",
            "{0} (서버 사본 {1})",
            "{0}（サーバーのコピー{1}）",
        ),
        SavingRivals => (
            "saving rivals...",
            "라이벌을 저장하는 중...",
            "ライバルを保存中...",
        ),
        Rivals => (
            "rivals: {0}",
            "라이벌: {0}",
            "ライバル: {0}",
        ),
        ReplayDownloadDropped => (
            "replay download dropped: left song select",
            "리플레이 다운로드를 취소했습니다: 선곡 화면을 벗어났습니다",
            "リプレイのダウンロードを中止しました: 選曲画面を離れました",
        ),
        RankingError => (
            "ranking: {0}",
            "랭킹: {0}",
            "ランキング: {0}",
        ),
        RankingWorkerStopped => (
            "ranking worker stopped",
            "랭킹 작업이 중단되었습니다",
            "ランキングの処理が停止しました",
        ),
        RecordsSyncAlreadyRunning => (
            "records sync already running",
            "기록 동기화가 이미 실행 중입니다",
            "記録の同期は既に実行中です",
        ),
        RecordsPickingWhatToSend => (
            "records: picking what to send",
            "기록: 전송할 항목을 고르는 중",
            "記録: 送信する項目を選んでいます",
        ),
        RecordsNothingToSend => (
            "records: nothing to send",
            "기록: 전송할 것이 없습니다",
            "記録: 送信するものがありません",
        ),
        SyncingReplays => (
            "syncing replays {0}/{1}",
            "리플레이 동기화 중 {0}/{1}",
            "リプレイを同期中 {0}/{1}",
        ),
        SyncingTraces => (
            "syncing traces {0}/{1}",
            "트레이스 동기화 중 {0}/{1}",
            "トレースを同期中 {0}/{1}",
        ),
        SyncingRecords => (
            "syncing records {0}/{1}",
            "기록 동기화 중 {0}/{1}",
            "記録を同期中 {0}/{1}",
        ),
        RowReplays => (
            "REPLAYS {0}/{1}",
            "리플레이 {0}/{1}",
            "リプレイ {0}/{1}",
        ),
        RowTraces => (
            "TRACES {0}/{1}",
            "트레이스 {0}/{1}",
            "トレース {0}/{1}",
        ),
        RowSending => (
            "SENDING {0}/{1}",
            "전송 중 {0}/{1}",
            "送信中 {0}/{1}",
        ),
        NothingSent => (
            "nothing sent",
            "전송한 것 없음",
            "送信なし",
        ),
        PartSent => (
            "{0} sent",
            "전송 {0}건",
            "送信{0}件",
        ),
        PartAlreadyOnTheServer => (
            "{0} already on server",
            "서버에 이미 있음 {0}건",
            "サーバーに登録済み{0}件",
        ),
        PartRejected => (
            "{0} rejected",
            "거부 {0}건",
            "拒否{0}件",
        ),
        PartFailed => (
            "{0} failed",
            "실패 {0}건",
            "失敗{0}件",
        ),
        PartSkipped => (
            "{0} skipped",
            "건너뜀 {0}건",
            "スキップ{0}件",
        ),
        PartUploaded => (
            "{0} uploaded",
            "업로드 {0}건",
            "アップロード{0}件",
        ),
        SectionRecords => (
            "records: {0}",
            "기록: {0}",
            "記録: {0}",
        ),
        SectionCourses => (
            "courses: {0}",
            "코스: {0}",
            "コース: {0}",
        ),
        SectionTraces => (
            "traces: {0}",
            "트레이스: {0}",
            "トレース: {0}",
        ),
        SectionReplays => (
            "replays: {0}",
            "리플레이: {0}",
            "リプレイ: {0}",
        ),
        SyncStoppedWith => (
            "records sync stopped: {0}",
            "기록 동기화를 중단했습니다: {0}",
            "記録の同期を中止しました: {0}",
        ),
        SyncStoppedServerUnreachable => (
            "records sync stopped: server unreachable",
            "기록 동기화를 중단했습니다: 서버에 연결할 수 없습니다",
            "記録の同期を中止しました: サーバーに接続できません",
        ),
        SyncStoppedRateLimited => (
            "records sync stopped: rate limited",
            "기록 동기화를 중단했습니다: 요청 횟수 제한",
            "記録の同期を中止しました: リクエスト数の制限",
        ),
        SyncCancelled => (
            "records sync cancelled",
            "기록 동기화를 취소했습니다",
            "記録の同期をキャンセルしました",
        ),
        SyncNotStarted => (
            "records sync not started: {0}",
            "기록 동기화를 시작하지 못했습니다: {0}",
            "記録の同期を開始できませんでした: {0}",
        ),
    }
}

rbms_model::translations! {
    /// Titles of the native pickers, which the operating system shows in its own window.
    pub(crate) enum DialogText {
        SelectUiFont => (
            "Select UI font",
            "UI 글꼴 선택",
            "UIフォントを選択",
        ),
        AddSongFolder => (
            "Add song folder",
            "곡 폴더 추가",
            "曲フォルダを追加",
        ),
        SelectTableJson => (
            "Select table json",
            "난이도표 json 선택",
            "難易度表jsonを選択",
        ),
        SelectSystemSoundFolder => (
            "Select system sound folder",
            "시스템 사운드 폴더 선택",
            "システムサウンドフォルダを選択",
        ),
        SelectDownloadFolder => (
            "Select download folder",
            "다운로드 폴더 선택",
            "ダウンロードフォルダを選択",
        ),
        SelectSkinPackFolder => (
            "Select skin pack folder",
            "스킨 팩 폴더 선택",
            "スキンパックフォルダを選択",
        ),
        SelectFolderToExportTheSkinInto => (
            "Select folder to export the skin into",
            "스킨을 내보낼 폴더 선택",
            "スキンの書き出し先フォルダを選択",
        ),
    }
}

/// `text` in the language the frame thread has installed, for a line that is set once and shown
/// until another replaces it (a status line), not for a notice, which is worded when it is taken.
pub(crate) fn installed(text: impl Catalog) -> &'static str {
    text.get(language())
}

/// [`installed`] with the values its placeholders take.
pub(crate) fn installed_with(text: impl Catalog, args: &[&dyn Display]) -> String {
    text.fill(language(), args)
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn a_status_line_is_worded_in_the_installed_language() {
        assert_eq!(installed(StatusText::LoggedOut), "logged out");
        assert_eq!(installed_with(StatusText::Rivals, &[&2]), "rivals: 2");
        rbms_render::text::set_language(Language::Korean);
        let korean = (installed(StatusText::LoggedOut), installed_with(StatusText::Rivals, &[&2]));
        rbms_render::text::set_language(Language::Japanese);
        let japanese = installed(StatusText::LoggedOut);
        rbms_render::text::set_language(Language::English);
        assert_eq!(korean, ("로그아웃했습니다", "라이벌: 2".to_string()));
        assert_eq!(japanese, "ログアウトしました");
    }

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(NoticeText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(StatusText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(DialogText::ALL), Vec::<String>::new());
    }

    #[test]
    fn a_notice_is_the_sentence_it_always_was_in_english() {
        let table: Vec<(NoticeText, Vec<&dyn std::fmt::Display>, &str)> = vec![
            (NoticeText::ImportedPendingRuns, vec![&3], "3 runs kept while the score database was closed are in it now"),
            (NoticeText::KeptRunNotRead, vec![&"a.ron"], "kept run not read (a.ron): left where it is"),
            (NoticeText::KeptRunNotRemoved, vec![&"a.ron"], "kept run read in but not removed (a.ron): remove it by hand"),
            (
                NoticeText::ScoreRecordsImported,
                vec![&10, &2, &"scores.ron"],
                "10 score records imported into the database (2 assisted, kept as history only); scores.ron moved aside",
            ),
            (NoticeText::CourseStagesMoved, vec![&4], "4 course stages stored on the gauge the settings chose are on the course gauge they ran on now"),
            (NoticeText::NoTablesToReload, vec![], "no difficulty tables to reload"),
            (NoticeText::TablesBeingFetched, vec![], "the tables are being fetched already"),
            (NoticeText::ReloadingTables, vec![&2], "reloading 2 difficulty table(s)"),
            (NoticeText::FetchingTablesFirstTime, vec![&2], "fetching 2 difficulty table(s) for the first time"),
            (NoticeText::TablesReloaded, vec![&1, &0], "difficulty tables reloaded: 1 updated, 0 failed"),
            (NoticeText::TablesFetchedFirstTime, vec![&1, &2], "difficulty tables fetched for the first time: 1 added, 2 failed"),
            (NoticeText::TableListsNothing, vec![&"T"], "table read, but it lists no charts and has no folder: T"),
            (NoticeText::TableAdded, vec![&"T", &5], "table added: T (5 local charts)"),
            (NoticeText::UserFoldersLoaded, vec![&2, &30, &25], "user folders: 2 folders, 30 charts (25 owned)"),
            (NoticeText::NoFoldersOnTheServer, vec![], "the score server has no folders to load"),
            (NoticeText::RunKeptOnTheSide, vec![], "score database not open: this run is kept on the side and goes into it when it opens"),
            (NoticeText::AbandonedCourseKept, vec![], "course ended before its last stage kept as FAILED"),
            (NoticeText::QuitRunKept, vec![], "run cut short by quitting kept; it is sent on the next start"),
            (NoticeText::RetriedRunKept, vec![], "failed run kept before starting again"),
            (NoticeText::CourseSent, vec![], "course sent"),
            (NoticeText::CourseNotKeptScoreDatabaseClosed, vec![], "course not kept: score database not open"),
            (NoticeText::CourseStageNotInTheLibrary, vec![&"Stage"], "course stage not in the library: Stage"),
            (
                NoticeText::DefaultSkinNotFullyInstalled,
                vec![&3, &"/cfg/settings.ron"],
                "default skin not fully installed: 3 files could not be written beside /cfg/settings.ron; a screen without its skin uses the built-in layout",
            ),
            (NoticeText::FontNotLoadedNoUsableFace, vec![&"f.ttf"], "font load failed (no usable face): f.ttf"),
            (NoticeText::ChartCouldNotBeRead, vec![&"a.bms"], "chart could not be read: a.bms"),
            (NoticeText::ChartMd5Mismatch, vec![&"aa", &"bb"], "warning: chart md5 mismatch (replay aa vs file bb); replay may desync"),
            (NoticeText::AudioReopenFailed, vec![], "audio: reopen failed — trying the previous device settings"),
            (NoticeText::AudioStreamStopped, vec![], "audio stream stopped — falling back to the wall clock"),
            (NoticeText::TokenNotRevoked, vec![&"timeout"], "logged out, but the server did not revoke the token (delete it on the web site): timeout"),
            (
                NoticeText::LetGoOfLoginTokenNotRevoked,
                vec![&"timeout"],
                "a login that was let go of left its token valid on the server (delete it on the web site): timeout",
            ),
            (NoticeText::LayoutOf, vec![&"2026-10-24", &"MIRROR"], "layout of 2026-10-24 (MIRROR)"),
            (
                NoticeText::LayoutOfLnModeDecides,
                vec![&"day", &"S-RANDOM", &"LN"],
                "layout of day: S-RANDOM lays out this chart's long notes as LN MODE LN resolves them, so it matches the replay only if it was recorded on LN",
            ),
            (
                NoticeText::LayoutRefusedChartNotInTheLibrary,
                vec![&"abcd1234"],
                "layout not borrowed: the chart of that replay (abcd1234) is not in the library",
            ),
            (NoticeText::LayoutRefusedAnotherChart, vec![&"aa", &"bb"], "layout not borrowed: that replay is of another chart (replay aa, chart bb)"),
            (NoticeText::LayoutRefusedNoSeed, vec![], "layout not borrowed: that replay does not state the seed its lanes were laid out from"),
            (NoticeText::LayoutRefusedUnsupportedOption, vec![&"X"], "layout not borrowed: that replay was played on X, which this player cannot lay out"),
            (NoticeText::LayoutRefusedNotFreePlay, vec![], "layout not borrowed: only a run played by hand outside a course or practice can borrow one"),
        ];
        assert_eq!(table.len(), NoticeText::ALL.len(), "an entry has no row in this table");
        for (text, args, expected) in table {
            assert_eq!(text.fill(Language::English, &args), expected, "{text:?}");
        }
    }

    #[test]
    fn a_status_line_is_the_text_it_always_was_in_english() {
        let table: Vec<(StatusText, Vec<&dyn std::fmt::Display>, &str)> = vec![
            (StatusText::SetServerUrlFirst, vec![], "set SERVER URL first"),
            (StatusText::LogInFirst, vec![], "log in first"),
            (StatusText::AccountNotConfirmed, vec![], "account not confirmed yet"),
            (StatusText::AccountRequestsDropped, vec![], "account or server changed: the request in flight was dropped"),
            (StatusText::SetPlayerIdFirst, vec![], "set PLAYER ID to your account id first"),
            (StatusText::SetPasswordFirst, vec![], "set PASSWORD first"),
            (StatusText::SetEmailFirst, vec![], "set EMAIL first"),
            (StatusText::LoggedOut, vec![], "logged out"),
            (StatusText::UploadingSettings, vec![], "uploading settings..."),
            (StatusText::DownloadingSettings, vec![], "downloading settings..."),
            (StatusText::SettingsDownloaded, vec![], "settings downloaded"),
            (StatusText::SettingsBlobUnreadable, vec![&"bad"], "settings blob unreadable: bad"),
            (StatusText::SettingsUploaded, vec![], "settings uploaded"),
            (StatusText::SettingsUploadedLockNotRefreshed, vec![], "settings uploaded (lock not refreshed; download before the next save)"),
            (StatusText::SettingsUploadWorkerStopped, vec![], "settings upload worker stopped"),
            (StatusText::SettingsDownloadWorkerStopped, vec![], "settings download worker stopped"),
            (StatusText::ServerCopy, vec![&"conflict", &7], "conflict (server copy 7)"),
            (StatusText::SavingRivals, vec![], "saving rivals..."),
            (StatusText::Rivals, vec![&3], "rivals: 3"),
            (StatusText::ReplayDownloadDropped, vec![], "replay download dropped: left song select"),
            (StatusText::RankingError, vec![&"offline"], "ranking: offline"),
            (StatusText::RankingWorkerStopped, vec![], "ranking worker stopped"),
            (StatusText::RecordsSyncAlreadyRunning, vec![], "records sync already running"),
            (StatusText::RecordsPickingWhatToSend, vec![], "records: picking what to send"),
            (StatusText::RecordsNothingToSend, vec![], "records: nothing to send"),
            (StatusText::SyncingReplays, vec![&1, &2], "syncing replays 1/2"),
            (StatusText::SyncingTraces, vec![&1, &2], "syncing traces 1/2"),
            (StatusText::SyncingRecords, vec![&1, &2], "syncing records 1/2"),
            (StatusText::RowReplays, vec![&1, &2], "REPLAYS 1/2"),
            (StatusText::RowTraces, vec![&1, &2], "TRACES 1/2"),
            (StatusText::RowSending, vec![&1, &2], "SENDING 1/2"),
            (StatusText::NothingSent, vec![], "nothing sent"),
            (StatusText::PartSent, vec![&4], "4 sent"),
            (StatusText::PartAlreadyOnTheServer, vec![&4], "4 already on server"),
            (StatusText::PartRejected, vec![&4], "4 rejected"),
            (StatusText::PartFailed, vec![&4], "4 failed"),
            (StatusText::PartSkipped, vec![&4], "4 skipped"),
            (StatusText::PartUploaded, vec![&4], "4 uploaded"),
            (StatusText::SectionRecords, vec![&"x"], "records: x"),
            (StatusText::SectionCourses, vec![&"x"], "courses: x"),
            (StatusText::SectionTraces, vec![&"x"], "traces: x"),
            (StatusText::SectionReplays, vec![&"x"], "replays: x"),
            (StatusText::SyncStoppedWith, vec![&"nope"], "records sync stopped: nope"),
            (StatusText::SyncStoppedServerUnreachable, vec![], "records sync stopped: server unreachable"),
            (StatusText::SyncStoppedRateLimited, vec![], "records sync stopped: rate limited"),
            (StatusText::SyncCancelled, vec![], "records sync cancelled"),
            (StatusText::SyncNotStarted, vec![&"locked"], "records sync not started: locked"),
        ];
        assert_eq!(table.len(), StatusText::ALL.len(), "an entry has no row in this table");
        for (text, args, expected) in table {
            assert_eq!(text.fill(Language::English, &args), expected, "{text:?}");
        }
    }

    #[test]
    fn a_picker_title_is_the_title_it_always_was_in_english() {
        let titles: Vec<&str> = DialogText::ALL.iter().map(|title| title.get(Language::English)).collect();
        assert_eq!(
            titles,
            [
                "Select UI font",
                "Add song folder",
                "Select table json",
                "Select system sound folder",
                "Select download folder",
                "Select skin pack folder",
                "Select folder to export the skin into",
            ]
        );
    }

    #[test]
    fn a_notice_with_a_value_puts_it_where_each_language_wants_it() {
        assert_eq!(NoticeText::ImportedPendingRuns.fill(Language::Korean, &[&3]), "스코어 데이터베이스가 닫혀 있는 동안 보관한 플레이 3건을 가져왔습니다");
        assert_eq!(NoticeText::TableAdded.fill(Language::Japanese, &[&"BMS", &5]), "難易度表を追加しました: BMS（所持譜面5件）");
        assert_eq!(StatusText::SyncingRecords.fill(Language::Korean, &[&1, &9]), "기록 동기화 중 1/9");
    }

    #[test]
    fn a_term_every_language_keeps_is_written_as_it_is_inside_a_translated_sentence() {
        for language in Language::ALL {
            assert!(NoticeText::AbandonedCourseKept.get(language).contains("FAILED"), "{language:?}");
            assert!(NoticeText::LayoutOfLnModeDecides.get(language).contains("LN MODE {2}"), "{language:?}");
            assert!(StatusText::SetServerUrlFirst.get(language).contains("SERVER URL"), "{language:?}");
            assert!(StatusText::SetPlayerIdFirst.get(language).contains("PLAYER ID"), "{language:?}");
            assert!(StatusText::SetPasswordFirst.get(language).contains("PASSWORD"), "{language:?}");
            assert!(StatusText::SetEmailFirst.get(language).contains("EMAIL"), "{language:?}");
        }
    }
}
