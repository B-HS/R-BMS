use serde::{Deserialize, Serialize};

/// The longest folder id a client puts into a request path.
pub const FOLDER_ID_MAX_LEN: usize = 64;

/// One folder of a player's, as the folder index lists it: what it is called and when it last
/// changed, without the charts in it (`GET /players/{id}/folders`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub position: u32,
    #[serde(default)]
    pub item_count: u32,
    /// When the folder or anything in it last changed, in milliseconds since the Unix epoch. A
    /// folder whose stamp has not moved since the client last read it holds what it held then.
    #[serde(default)]
    pub updated_at: i64,
}

/// One chart in a folder. Charts are matched to a library by `md5`; `sha256` is empty when the
/// folder does not know it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderItem {
    pub md5: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub note: String,
}

/// Every folder of a player's, in the order the player arranged them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderIndex {
    #[serde(default)]
    pub folders: Vec<UserFolderSummary>,
}

/// One folder with its charts, in the order the player arranged them
/// (`GET /players/{id}/folders/{folderId}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolder {
    #[serde(flatten)]
    pub summary: UserFolderSummary,
    #[serde(default)]
    pub items: Vec<UserFolderItem>,
}

/// Whether `id` may be put into a request path as it is: the client interpolates identifiers
/// without percent-encoding them, so an id the server answered with is used only when it is made
/// of letters, digits, `_` and `-` and is no longer than [`FOLDER_ID_MAX_LEN`].
pub fn is_path_safe_folder_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= FOLDER_ID_MAX_LEN && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_folder_reads_the_fields_the_server_answers_with() {
        let folder: UserFolder = serde_json::from_value(json!({
            "id": "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b", "name": "practice", "description": "daily", "is_public": false,
            "position": 0, "item_count": 2, "updated_at": 1_760_400_005_000_i64,
            "items": [
                { "md5": "0123456789abcdef0123456789abcdef", "sha256": "", "title": "Example", "artist": "Someone", "note": "" },
                { "md5": "fedcba9876543210fedcba9876543210", "sha256": "9f86", "title": "Another", "artist": "Else", "note": "right hand" }
            ]
        }))
        .expect("the server's folder reads");
        assert_eq!(folder.summary.name, "practice");
        assert_eq!(folder.summary.updated_at, 1_760_400_005_000);
        assert_eq!(folder.items.len(), 2);
        assert_eq!(folder.items[1].note, "right hand");
    }

    #[test]
    fn what_a_server_leaves_out_reads_as_nothing() {
        let index: UserFolderIndex = serde_json::from_value(json!({ "folders": [{ "id": "uf_1", "name": "A" }] })).expect("a sparse index reads");
        let summary = &index.folders[0];
        assert_eq!((summary.item_count, summary.updated_at, summary.position, summary.is_public), (0, 0, 0, false));
        assert_eq!(serde_json::from_value::<UserFolderIndex>(json!({})).expect("an empty object is an empty index"), UserFolderIndex::default());
        let bare: UserFolder = serde_json::from_value(json!({ "id": "uf_1", "name": "A" })).expect("a folder without items reads");
        assert!(bare.items.is_empty());
        let item: UserFolderItem = serde_json::from_value(json!({ "md5": "aa" })).expect("an item with only its md5 reads");
        assert_eq!((item.sha256.as_str(), item.title.as_str(), item.note.as_str()), ("", "", ""));
    }

    #[test]
    fn only_an_id_made_of_safe_characters_goes_into_a_path() {
        assert!(is_path_safe_folder_id("uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b"));
        assert!(is_path_safe_folder_id("a-B_9"));
        assert!(!is_path_safe_folder_id(""));
        assert!(!is_path_safe_folder_id("../settings/player"));
        assert!(!is_path_safe_folder_id("uf_1?x=1"));
        assert!(!is_path_safe_folder_id("uf 1"));
        assert!(is_path_safe_folder_id(&"a".repeat(FOLDER_ID_MAX_LEN)));
        assert!(!is_path_safe_folder_id(&"a".repeat(FOLDER_ID_MAX_LEN + 1)));
    }
}
