//! Fixed words the skin menus say of the skin in effect, by language.
//!
//! The LOADED row and the rows that name the pack, the colour channels and the export, and what the preview says in place of a frame. The reason a
//! document failed to load is the loader's own, and the name a skin gives its own rows and parts is the skin's: both stay as they come.

rbms_model::translations! {
    /// What the LOADED row, the pack rows and the offset rows of the skin menu say.
    pub(crate) enum SkinInfoText {
        BuiltInScreen => ("BUILT-IN SCREEN", "내장 화면", "内蔵画面"),
        NotReadYet => ("NOT READ YET", "아직 읽지 않음", "未読み込み"),
        ScreenNotSupported => (
            "SCREEN NOT SUPPORTED - DRAWING THE BUILT-IN ONE",
            "지원하지 않는 화면 - 내장 화면으로 그립니다",
            "未対応の画面 - 内蔵画面で描画します",
        ),
        Lua => ("LUA"),
        ReadWhenOpens => ("READ WHEN ITS SCREEN OPENS", "해당 화면을 열 때 읽습니다", "その画面を開くときに読み込みます"),
        /// The size, parser and warning count of the document being drawn; the author follows, if the document names one.
        Summary => ("{0}X{1} - {2}{3} - {4} WARNINGS{5}", "{0}X{1} - {2}{3} - 경고 {4}개{5}", "{0}X{1} - {2}{3} - 警告{4}件{5}"),
        /// Before the name of a document the pack chose rather than the player.
        PackDocumentPrefix => ("PACK: ", "팩: ", "パック: "),
        /// Before a pack folder that is not on the disk.
        MissingPackPrefix => ("(MISSING) ", "(없음) ", "(見つかりません) "),
        PackFolderMissing => ("THE PACK FOLDER IS NOT THERE", "팩 폴더가 없습니다", "パックフォルダがありません"),
        /// Between the name of a pack's skin file and why it could not be read.
        DidNotRead => (" DID NOT READ", " 읽지 못했습니다", " 読み込めませんでした"),
        /// After the first of several skin files of a pack that could not be read.
        AndMore => (" (AND {0} MORE)", " (외 {0}개)", " （他{0}件）"),
        ExportRow => ("EXPORT", "내보내기", "エクスポート"),
        ColorChannelsRow => ("COLOR CHANNELS", "색상 채널", "カラーチャンネル"),
        AngleAxis => ("ANGLE", "각도", "角度"),
        AlphaAxis => ("ALPHA", "알파", "アルファ"),
    }
}

rbms_model::translations! {
    /// What the preview says in place of a frame of the skin, and what it adds under it.
    pub(crate) enum PreviewStatusText {
        Loading => ("LOADING THE SKIN", "스킨을 불러오는 중", "スキンを読み込み中"),
        NoDocument => ("NO SKIN DOCUMENT FOR THIS SCREEN", "이 화면에 쓸 스킨 문서가 없습니다", "この画面のスキン文書がありません"),
        NoDocumentDetail => ("THE BUILT-IN SCREEN DRAWS IT", "내장 화면이 대신 그립니다", "内蔵画面が代わりに描画します"),
        Unsupported => ("THIS SCREEN IS NOT DRAWN BY A SKIN", "이 화면은 스킨으로 그리지 않습니다", "この画面はスキンでは描画されません"),
        UnsupportedDetail => (
            "ITS DOCUMENT IS LISTED BUT THE BUILT-IN SCREEN DRAWS IT",
            "문서는 목록에 있지만 내장 화면이 대신 그립니다",
            "文書は一覧にありますが、内蔵画面が代わりに描画します",
        ),
        NoPreview => ("NO PREVIEW FOR THIS SCREEN", "이 화면은 미리보기가 없습니다", "この画面のプレビューはありません"),
        NoPreviewDetail => (
            "THE SETTINGS ON THE LEFT STILL APPLY TO IT",
            "왼쪽의 설정은 그대로 적용됩니다",
            "左の設定はそのまま適用されます",
        ),
        Failed => ("THE SKIN DID NOT LOAD", "스킨을 불러오지 못했습니다", "スキンを読み込めませんでした"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(SkinInfoText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(PreviewStatusText::ALL), Vec::<String>::new());
    }
}
