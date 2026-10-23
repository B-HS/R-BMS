//! Fixed words of the difficulty-table manager, by language.
//!
//! The reason a table or the folders failed, the name of a table and the location it is loaded from are data and stay as they come.

rbms_model::translations! {
    /// Words of the difficulty-table manager.
    pub(crate) enum TablesText {
        Title => ("DIFFICULTY TABLES", "난이도표", "難易度表"),
        ListKeys => (
            "UP DOWN MOVE   ENTER SELECT   R RELOAD   A RELOAD ALL   D REMOVE   ESC BACK",
            "UP DOWN 이동   ENTER 선택   R 다시 받기   A 모두 다시 받기   D 삭제   ESC 뒤로",
            "UP DOWN 移動   ENTER 選択   R 再取得   A すべて再取得   D 削除   ESC 戻る",
        ),
        UrlEntryKeys => (
            "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL  CTRL+V PASTE",
            "난이도표 URL 입력  -  ENTER 추가  ESC 취소  CTRL+V 붙여넣기",
            "難易度表のURLを入力  -  ENTER 追加  ESC キャンセル  CTRL+V 貼り付け",
        ),
        PickFile => (
            "PICK A TABLE FILE IN THE OPEN DIALOG",
            "열린 대화상자에서 난이도표 파일을 고르세요",
            "開いたダイアログで難易度表のファイルを選んでください",
        ),
        AddByUrl => ("+ ADD TABLE (URL)", "+ 난이도표 추가 (URL)", "+ 難易度表を追加 (URL)"),
        AddByFile => ("+ ADD TABLE (FILE)", "+ 난이도표 추가 (파일)", "+ 難易度表を追加 (ファイル)"),
        AlreadyAdded => ("table already added: {0}", "이미 추가된 난이도표입니다: {0}", "追加済みの難易度表です: {0}"),

        /// The row of the player's own folders while nobody is signed in.
        FoldersSignedOut => (
            "LOG IN TO LOAD YOUR FOLDERS",
            "내 폴더를 불러오려면 로그인하세요",
            "自分のフォルダを読み込むにはログインしてください",
        ),
        FoldersSignedOutDetail => (
            "LOG IN ON THE NETWORK TAB OF THE SETTINGS - YOUR FOLDERS ARE MADE AND EDITED ON THE WEB SITE",
            "설정의 네트워크 탭에서 로그인하세요 - 내 폴더는 웹사이트에서 만들고 편집합니다",
            "設定のネットワークタブでログインしてください - 自分のフォルダはウェブサイトで作成・編集します",
        ),
        FoldersNone => ("THE SERVER HAS NO FOLDERS", "서버에 폴더가 없습니다", "サーバーにフォルダがありません"),
        FoldersDetail => (
            "YOUR FOLDERS ARE MADE AND EDITED ON THE WEB SITE - R FETCHES THEM AGAIN",
            "내 폴더는 웹사이트에서 만들고 편집합니다 - R 키로 다시 받습니다",
            "自分のフォルダはウェブサイトで作成・編集します - Rキーで再取得します",
        ),
        FoldersNotRemoved => (
            "your folders are edited on the web site and cannot be removed here",
            "내 폴더는 웹사이트에서 편집하므로 여기서는 삭제할 수 없습니다",
            "自分のフォルダはウェブサイトで編集するため、ここでは削除できません",
        ),
        FoldersOwned => (
            "{0} OF {1} CHARTS OWNED  -  {2} FOLDERS",
            "차트 {1}개 중 {0}개 보유  -  폴더 {2}개",
            "{1}譜面中{0}譜面を所持  -  {2}フォルダ",
        ),
        FoldersReloadFailedDetail => (
            "RELOAD FAILED, THE FOLDERS ARE AS THEY WERE: {0}",
            "다시 받기 실패, 폴더는 그대로입니다: {0}",
            "再取得に失敗しました。フォルダは元のままです: {0}",
        ),

        /// What the row of a table says in place of its status while the table is on its way, and what is said under the list.
        Reloading => ("RELOADING...", "다시 받는 중...", "再取得中..."),
        ReloadingDetail => (
            "FETCHING THE TABLE AGAIN - IT STAYS AS IT IS UNTIL THE NEW ONE ARRIVES",
            "난이도표를 다시 받는 중 - 새 표가 도착할 때까지는 지금 그대로입니다",
            "難易度表を再取得中 - 新しい表が届くまでは現在のままです",
        ),
        Fetching => ("FETCHING...", "받는 중...", "取得中..."),
        FetchingDetail => (
            "FETCHING THE TABLE - ITS FOLDER APPEARS IN THE SONG LIST WHEN IT ARRIVES",
            "난이도표를 받는 중 - 도착하면 곡 목록에 폴더가 나타납니다",
            "難易度表を取得中 - 届くと曲リストにフォルダが表示されます",
        ),
        Reading => ("READING...", "읽는 중...", "読み込み中..."),
        ReadingDetail => (
            "READING THE TABLE FILE - ITS FOLDER APPEARS IN THE SONG LIST WHEN IT HAS BEEN READ",
            "난이도표 파일을 읽는 중 - 다 읽으면 곡 목록에 폴더가 나타납니다",
            "難易度表のファイルを読み込み中 - 読み込みが終わると曲リストにフォルダが表示されます",
        ),
        NotFetched => ("NOT FETCHED YET", "아직 받지 않음", "未取得"),
        NotFetchedDetail => (
            "NOT FETCHED YET - THE SONG LIST FETCHES IT WHEN IT IS NEXT UP, AND R FETCHES IT NOW",
            "아직 받지 않았습니다 - 곡 목록을 다음에 열 때 받고, R 키로 지금 받을 수 있습니다",
            "まだ取得していません - 曲リストを次に開くときに取得し、Rキーですぐに取得できます",
        ),

        /// The few words at the right edge of a table's row, and the lines under the list that spell them out.
        NoChartsListed => ("NO CHARTS LISTED", "등록된 차트 없음", "掲載譜面なし"),
        ChartsOwned => (
            "{0} OF {1} CHARTS OWNED  -  {2} LEVELS",
            "차트 {1}개 중 {0}개 보유  -  레벨 {2}개",
            "{1}譜面中{0}譜面を所持  -  {2}レベル",
        ),
        ReloadFailedOwned => (
            "RELOAD FAILED  -  {0} OF {1} CHARTS OWNED",
            "다시 받기 실패  -  차트 {1}개 중 {0}개 보유",
            "再取得に失敗  -  {1}譜面中{0}譜面を所持",
        ),
        LoadFailed => ("LOAD FAILED", "불러오기 실패", "読み込みに失敗"),
        LoadFailedDetail => ("LOAD FAILED: {0}", "불러오기 실패: {0}", "読み込みに失敗: {0}"),
        ReloadFailedDetail => (
            "RELOAD FAILED, THE TABLE IS AS IT WAS: {0}",
            "다시 받기 실패, 난이도표는 그대로입니다: {0}",
            "再取得に失敗しました。難易度表は元のままです: {0}",
        ),
        ListsNoCharts => (
            "THE TABLE WAS READ BUT LISTS NO CHARTS - IT HAS NO FOLDER IN THE SONG LIST",
            "난이도표를 읽었지만 차트가 하나도 없습니다 - 곡 목록에 폴더가 생기지 않습니다",
            "難易度表は読み込めましたが、譜面が1つもありません - 曲リストにフォルダはありません",
        ),
        NoneInLibrary => (
            "NONE OF THIS TABLE'S CHARTS ARE IN THE LIBRARY YET - ITS FOLDER IS EMPTY",
            "이 난이도표의 차트가 아직 라이브러리에 하나도 없습니다 - 폴더가 비어 있습니다",
            "この難易度表の譜面はまだライブラリに1つもありません - フォルダは空です",
        ),

        /// How old the saved copy of a table is.
        Saved => ("SAVED", "저장됨", "保存済み"),
        SavedAgo => ("SAVED {0}", "{0} 저장", "{0}保存"),
        JustNow => ("JUST NOW", "방금", "たった今"),
        MinuteAgo => ("{0} MINUTE AGO", "{0}분 전", "{0}分前"),
        MinutesAgo => ("{0} MINUTES AGO", "{0}분 전", "{0}分前"),
        HourAgo => ("{0} HOUR AGO", "{0}시간 전", "{0}時間前"),
        HoursAgo => ("{0} HOURS AGO", "{0}시간 전", "{0}時間前"),
        DayAgo => ("{0} DAY AGO", "{0}일 전", "{0}日前"),
        DaysAgo => ("{0} DAYS AGO", "{0}일 전", "{0}日前"),
        OldCopy => (
            "THE SAVED COPY IS {0} DAYS OLD - R FETCHES THE TABLE AGAIN",
            "저장된 사본이 {0}일 지났습니다 - R 키로 난이도표를 다시 받습니다",
            "保存コピーは{0}日経過しています - Rキーで難易度表を再取得します",
        ),

        /// What reloading is, along the bottom of the screen: one line each.
        ReloadNoteSaved => (
            "A TABLE AT AN ADDRESS IS FETCHED ONCE AND READ FROM ITS SAVED COPY AFTER THAT - THE ROW SHOWS HOW LONG AGO THE COPY WAS FETCHED.",
            "주소로 등록한 난이도표는 한 번 받은 뒤로는 저장된 사본에서 읽습니다 - 줄 끝에 사본을 받은 지 얼마나 지났는지 표시됩니다.",
            "アドレスで登録した難易度表は一度取得した後は保存コピーから読み込みます - 行の端にコピーを取得してからの経過時間が表示されます。",
        ),
        ReloadNoteBeside => (
            "A TABLE IS FETCHED WHILE YOU GO ON USING THE PLAYER, HERE OR IN THE SONG LIST - ITS FOLDER APPEARS WHEN IT ARRIVES.",
            "난이도표는 이 화면이나 곡 목록에서 플레이어를 계속 쓰는 동안 받습니다 - 도착하면 폴더가 나타납니다.",
            "難易度表はこの画面や曲リストでプレーヤーを使い続けている間に取得します - 届くとフォルダが表示されます。",
        ),
        ReloadNoteKeys => (
            "R FETCHES THE TABLE UNDER THE CURSOR AGAIN, A EVERY TABLE. A TABLE REGISTERED AS A FILE IS READ FROM THE FILE AGAIN.",
            "R 키는 커서가 있는 난이도표를, A 키는 모든 난이도표를 다시 받습니다. 파일로 등록한 난이도표는 파일에서 다시 읽습니다.",
            "Rキーはカーソル位置の難易度表を、Aキーはすべての難易度表を再取得します。ファイルで登録した難易度表はファイルから再読み込みします。",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(TablesText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_age_of_a_copy_is_told_in_each_language() {
        assert_eq!(TablesText::DaysAgo.fill(Language::English, &[&45]), "45 DAYS AGO");
        assert_eq!(TablesText::SavedAgo.fill(Language::Korean, &[&TablesText::DaysAgo.fill(Language::Korean, &[&3])]), "3일 전 저장");
        assert_eq!(TablesText::SavedAgo.fill(Language::Japanese, &[&TablesText::HoursAgo.fill(Language::Japanese, &[&2])]), "2時間前保存");
        assert_eq!(TablesText::SavedAgo.fill(Language::Japanese, &[&TablesText::JustNow.get(Language::Japanese)]), "たった今保存");
    }
}
