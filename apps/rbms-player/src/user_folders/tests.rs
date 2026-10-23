//! The player's own folders against a score server that lives in the test: what is asked for, what
//! is kept, what the song list shows, and what is let go of when the account changes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rbms_ir::{ChartId, CourseSubmission, PlayerProfile, ReplayData, ScoreRecord, ScoreSubmission, ServerInfo, SubmitResponse, UserFolder, UserFolderIndex};

use super::*;
use crate::ir_session::AccountSession;
use crate::select_tree::SelectFolder;
use crate::{App, Config, LaunchOptions, SelectItem};

pub(crate) const SERVER: &str = "https://folders.test/api";
pub(crate) const TOKEN: &str = "token-of-a-test";
pub(crate) const PLAYER: &str = "dj";
const OTHER_PLAYER: &str = "someone";

pub(crate) const PRACTICE: &str = "uf_00000000000000000000000000000a";
pub(crate) const LONG_NOTES: &str = "uf_00000000000000000000000000000b";

pub(crate) const OWNED_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(crate) const OWNED_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(crate) const MISSING: &str = "cccccccccccccccccccccccccccccccc";

/// How long a test waits for a fetch to come back before it calls the fetch lost.
const FETCH_WAIT: Duration = Duration::from_secs(10);

/// How long a test sleeps between two looks at a fetch that has not come back.
const FETCH_POLL: Duration = Duration::from_millis(2);

/// What the test server answers for the index.
#[derive(Clone, Default)]
pub(crate) enum IndexAnswer {
    /// The folders, in the player's order.
    Folders(Vec<UserFolderSummary>),
    /// A server from before folders: 404.
    #[default]
    NoRoute,
    /// A session the server does not accept: 401.
    Refused,
}

/// What the test server answers: the index, and each folder by id. A missing folder is a failure of
/// that folder alone.
#[derive(Default)]
pub(crate) struct FolderServer {
    index: Mutex<IndexAnswer>,
    folders: Mutex<HashMap<String, Vec<UserFolderItem>>>,
    folder_asks: AtomicUsize,
}

pub(crate) fn summary(id: &str, name: &str, updated_at: i64, items: usize) -> UserFolderSummary {
    UserFolderSummary {
        id: id.into(),
        name: name.into(),
        description: String::new(),
        is_public: false,
        position: 0,
        item_count: u32::try_from(items).expect("a small folder"),
        updated_at,
    }
}

fn item(md5: &str) -> UserFolderItem {
    UserFolderItem { md5: md5.into(), sha256: String::new(), title: String::new(), artist: String::new(), note: String::new() }
}

impl FolderServer {
    pub(crate) fn serving(index: Vec<UserFolderSummary>, folders: &[(&str, &[&str])]) -> Arc<FolderServer> {
        let server = FolderServer::default();
        *server.index.lock().expect("the index") = IndexAnswer::Folders(index);
        server.set_folders(folders);
        Arc::new(server)
    }

    pub(crate) fn set_folders(&self, folders: &[(&str, &[&str])]) {
        *self.folders.lock().expect("the folders") = folders.iter().map(|(id, md5s)| ((*id).to_string(), md5s.iter().map(|md5| item(md5)).collect())).collect();
    }

    pub(crate) fn set_index(&self, index: IndexAnswer) {
        *self.index.lock().expect("the index") = index;
    }

    pub(crate) fn folder_asks(&self) -> usize {
        self.folder_asks.load(Ordering::SeqCst)
    }
}

impl ScoreServer for FolderServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::Unsupported)
    }
    fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::Unsupported)
    }
    fn user_folders(&self, player: &PlayerId) -> Result<UserFolderIndex, IrError> {
        assert_eq!(player.id, PLAYER, "the folders asked for are the signed-in player's");
        match self.index.lock().expect("the index").clone() {
            IndexAnswer::Folders(folders) => Ok(UserFolderIndex { folders }),
            IndexAnswer::NoRoute => Err(IrError::NotFound("no such route".into())),
            IndexAnswer::Refused => Err(IrError::Unauthorized("sign in".into())),
        }
    }
    fn user_folder(&self, _player: &PlayerId, folder_id: &str) -> Result<UserFolder, IrError> {
        self.folder_asks.fetch_add(1, Ordering::SeqCst);
        let items = self.folders.lock().expect("the folders").get(folder_id).cloned().ok_or_else(|| IrError::Network("connection reset".into()))?;
        Ok(UserFolder { summary: summary(folder_id, "", 0, items.len()), items })
    }
}

/// One library entry, with only the fields the matcher reads filled in.
fn entry(md5: &str) -> rbms_library::SongEntry {
    rbms_library::SongEntry {
        path: PathBuf::from(format!("/songs/{md5}.bms")),
        title: md5.into(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: "1".into(),
        difficulty: 3,
        init_bpm: 150.0,
        rank: 2,
        total: 300.0,
        mode: rbms_model::Mode::BEAT_7K,
        md5: md5.into(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

/// A player signed in to [`SERVER`] as [`PLAYER`], whose library has two charts, with the settings
/// in a folder of the test's own. The score server is the test's: nothing here reaches a network.
struct Signed {
    app: App,
    server: Arc<FolderServer>,
}

impl Signed {
    fn new(tag: &str, server: Arc<FolderServer>) -> Signed {
        let folder = std::env::temp_dir().join(format!("rbms-user-folders-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let mut app = App::new(String::new(), Config::default(), LaunchOptions::default(), folder.join("settings.ron"));
        app.shared.library = Library::from_songs(vec![entry(OWNED_A), entry(OWNED_B)]);
        app.shared.config.network.server_url = Some(SERVER.into());
        app.shared.server = server.clone();
        sign_in(&mut app, PLAYER);
        Signed { app, server }
    }

    fn account(&self) -> FolderAccount {
        FolderAccount::of(&self.app.shared).expect("the player is signed in")
    }

    /// Start a fetch and take what it comes back with.
    fn fetch(&mut self, loud: bool) {
        assert!(start_fetch(&mut self.app.shared, loud), "a fetch was started");
        self.land();
    }

    /// Take the fetch that is out once it has come back.
    fn land(&mut self) {
        let deadline = Instant::now() + FETCH_WAIT;
        while !apply_finished_fetch(&mut self.app.shared) {
            assert!(Instant::now() < deadline, "the fetch never came back");
            std::thread::sleep(FETCH_POLL);
        }
    }

    /// The rows of the song list looking into `view`.
    fn rows(&mut self, open: fn(&crate::AppShared) -> Vec<SelectFolder>) -> Vec<String> {
        let folders = open(&self.app.shared);
        self.app.shared.open_for_tests(folders);
        self.app.shared.rebuild_select_items();
        self.app
            .shared
            .select_items
            .iter()
            .map(|row| match row {
                SelectItem::Folder { label, .. } => label.clone(),
                SelectItem::Song(index) => self.app.shared.library.songs()[*index].md5.clone(),
                SelectItem::Missing(missing) => self.app.shared.missing_chart(missing).map(|chart| chart.md5.clone()).unwrap_or_default(),
            })
            .collect()
    }

    fn copy_path(&self) -> PathBuf {
        self.account().copy_path(&self.app.shared.settings_path)
    }
}

impl Drop for Signed {
    fn drop(&mut self) {
        if let Some(folder) = self.app.shared.settings_path.parent() {
            let _ = std::fs::remove_dir_all(folder);
        }
    }
}

fn sign_in(app: &mut App, player: &str) {
    app.shared.session = AccountSession::restored(Some(TOKEN.into()), Some(player.into()));
    follow_account(&mut app.shared);
}

pub(crate) fn two_folders() -> Arc<FolderServer> {
    FolderServer::serving(
        vec![summary(PRACTICE, "practice 12", 10, 3), summary(LONG_NOTES, "ln", 20, 1)],
        &[(PRACTICE, &[OWNED_A, MISSING, OWNED_B]), (LONG_NOTES, &[OWNED_B])],
    )
}

#[test]
fn the_folders_are_one_row_at_the_top_holding_a_row_per_folder() {
    let mut signed = Signed::new("rows", two_folders());
    assert!(signed.rows(|_| Vec::new()).iter().all(|row| !row.starts_with(USER_FOLDERS_LABEL)), "no folders before they are fetched");

    signed.fetch(true);
    assert_eq!(signed.rows(|_| Vec::new()), vec!["ALL SONGS (2)".to_string(), "MY FOLDERS (3)".to_string()]);
    assert_eq!(
        signed.rows(|_| vec![SelectFolder::UserFolders]),
        vec!["PRACTICE 12 (2/3)".to_string(), "LN (1)".to_string()],
        "owned out of listed, or only owned when all are"
    );
    assert_eq!(
        signed.rows(|shared| vec![SelectFolder::UserFolders, shared.user_folder(0)]),
        vec![OWNED_A.to_string(), OWNED_B.to_string()],
        "a chart the library does not have is no row"
    );
    assert_eq!(signed.app.shared.user_folders.chart_counts(), (4, 3));
}

/// What was fetched is kept for the account, and the next fetch asks only for the folders whose
/// stamp moved since; a folder the index no longer lists is gone from the copy and the list.
#[test]
fn a_second_fetch_asks_only_for_the_folders_that_changed() {
    let mut signed = Signed::new("changed", two_folders());
    signed.fetch(false);
    assert_eq!(signed.server.folder_asks(), 2);
    assert!(signed.copy_path().exists(), "the folders are kept");

    signed.server.set_index(IndexAnswer::Folders(vec![summary(LONG_NOTES, "ln", 21, 2)]));
    signed.server.set_folders(&[(LONG_NOTES, &[OWNED_B, OWNED_A])]);
    signed.fetch(false);
    assert_eq!(signed.server.folder_asks(), 3, "only the folder whose stamp moved was asked for again");
    assert_eq!(signed.rows(|_| vec![SelectFolder::UserFolders]), vec!["LN (2)".to_string()], "the folder the index no longer lists is gone");

    signed.server.set_folders(&[]);
    signed.fetch(false);
    assert_eq!(signed.server.folder_asks(), 3, "nothing changed, nothing asked for");
    assert_eq!(signed.rows(|_| vec![SelectFolder::UserFolders]), vec!["LN (2)".to_string()]);
}

/// A folder that fails keeps what the copy held of it, and one the copy did not have is left out
/// this time; the others arrive.
#[test]
fn a_folder_that_fails_keeps_what_was_kept_of_it() {
    let mut signed = Signed::new("partial", two_folders());
    signed.fetch(false);
    signed.server.set_index(IndexAnswer::Folders(vec![
        summary(PRACTICE, "practice 12", 11, 1),
        summary(LONG_NOTES, "ln", 20, 1),
        summary(&format!("{PRACTICE}0"), "new", 1, 1),
    ]));
    signed.server.set_folders(&[]);
    signed.fetch(true);
    assert_eq!(signed.rows(|_| vec![SelectFolder::UserFolders]), vec!["PRACTICE 12 (2/3)".to_string(), "LN (1)".to_string()]);
    assert!(matches!(signed.app.shared.user_folders.status(), FolderStatus::Shown { problem: Some(FolderProblem::Failed(_)), .. }));
}

/// A server from before folders answers 404, which is no failure to report; the player's folders
/// are simply not there.
#[test]
fn a_server_without_folders_is_passed_over_quietly() {
    let server = Arc::new(FolderServer::default());
    let mut signed = Signed::new("no-folders", server);
    signed.fetch(false);
    assert!(signed.app.shared.user_folders.shown.is_empty());
    assert_eq!(signed.app.shared.user_folders.status(), FolderStatus::NotFetched { problem: Some(FolderProblem::NoFolders) });
    assert!(!signed.copy_path().exists(), "nothing was kept of no folders");
}

/// A session the server refuses hides the folders; what was kept of them stays for the account to
/// sign in to again.
#[test]
fn a_refused_session_hides_the_folders_and_keeps_the_copy() {
    let mut signed = Signed::new("refused", two_folders());
    signed.fetch(false);
    signed.server.set_index(IndexAnswer::Refused);
    signed.fetch(true);
    assert!(signed.app.shared.user_folders.shown.is_empty(), "the folders of a refused session are on show");
    assert!(signed.copy_path().exists());
    assert!(signed.rows(|_| Vec::new()).iter().all(|row| !row.starts_with(USER_FOLDERS_LABEL)));
}

/// Signing out deletes the account's folders from the machine, and a fetch of them that was out
/// keeps nothing when it comes back.
#[test]
fn signing_out_lets_go_of_the_folders_and_of_a_fetch_that_is_out() {
    let mut signed = Signed::new("sign-out", two_folders());
    signed.fetch(false);
    let account = signed.account();
    let copy = signed.copy_path();
    assert!(copy.exists());

    assert!(start_fetch(&mut signed.app.shared, true));
    signed.app.shared.session.logout();
    follow_account(&mut signed.app.shared);
    forget_account(&mut signed.app.shared, &account);
    assert!(!copy.exists(), "the copy outlived the sign-out");
    assert!(signed.app.shared.user_folders.shown.is_empty());
    assert_eq!(signed.app.shared.user_folders.status(), FolderStatus::SignedOut);
    assert!(!apply_finished_fetch(&mut signed.app.shared), "the fetch of the signed-out account was not let go of");
    std::thread::sleep(FETCH_POLL * 50);
    assert!(!copy.exists(), "the fetch that was out kept the folders of an account that signed out");
}

/// Each account sees its own folders only: another account signing in finds nothing of the first
/// one's, even when its copy came to the same file, and the first account's copy is on show again
/// the moment it is back, before anything is fetched.
#[test]
fn one_accounts_folders_are_never_another_accounts() {
    let mut signed = Signed::new("accounts", two_folders());
    signed.fetch(false);
    let first = signed.copy_path();

    sign_in(&mut signed.app, OTHER_PLAYER);
    assert!(signed.app.shared.user_folders.shown.is_empty(), "another account sees the first one's folders");
    let other = signed.copy_path();
    std::fs::copy(&first, &other).expect("the first account's copy lands where the other's would be");
    signed.app.shared.user_folders = UserFolders::default();
    follow_account(&mut signed.app.shared);
    assert!(signed.app.shared.user_folders.shown.is_empty(), "a copy that names another account was read");

    sign_in(&mut signed.app, PLAYER);
    assert_eq!(signed.app.shared.user_folders.shown.len(), 2, "the account's own copy is on show without a fetch");
}

/// A fetch that comes back after the account it was for has signed out of the session is dropped.
#[test]
fn a_fetch_for_an_account_that_is_no_longer_signed_in_is_dropped() {
    let mut signed = Signed::new("switched", two_folders());
    assert!(start_fetch(&mut signed.app.shared, false));
    sign_in(&mut signed.app, OTHER_PLAYER);
    assert!(!signed.app.shared.user_folders.is_fetching(), "the other account's fetch was kept");
    assert!(signed.app.shared.user_folders.shown.is_empty());
}

/// What a server sends is not trusted: an id that would not be safe in a path is never asked for,
/// a chart not named by an md5 is no chart, and a name is cut and cleaned.
#[test]
fn what_the_server_sends_is_tidied() {
    let long_name = format!("a\u{7}{}", "x".repeat(NAME_MAX_CHARS * 2));
    let server = FolderServer::serving(
        vec![summary(PRACTICE, &long_name, 1, 3), summary("../settings/player", "evil", 1, 1)],
        &[(PRACTICE, &[OWNED_A, "not-an-md5", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"])],
    );
    let mut signed = Signed::new("tidied", server);
    signed.fetch(false);
    assert_eq!(signed.server.folder_asks(), 1, "the unsafe id was asked for");
    let shown = &signed.app.shared.user_folders.shown;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].name.chars().count(), NAME_MAX_CHARS);
    assert!(!shown[0].name.contains('\u{7}'));
    assert_eq!(shown[0].listed, 2, "the chart without an md5 is no chart");
    assert_eq!(shown[0].charts, vec![0], "an md5 in capitals is the same chart");
}

/// The list keeps its place while folders arrive: a folder being looked into is found again by its
/// id after its name changed and another folder came before it, and a folder that is gone leaves the
/// list on the folders.
#[test]
fn the_list_keeps_its_place_by_the_folders_id() {
    let mut signed = Signed::new("place", two_folders());
    signed.fetch(false);
    signed.app.shared.open_for_tests([SelectFolder::UserFolders, signed.app.shared.user_folder(1)]);
    signed.app.shared.rebuild_select_items();

    signed.server.set_index(IndexAnswer::Folders(vec![summary(LONG_NOTES, "long notes", 30, 1), summary(PRACTICE, "practice 12", 10, 3)]));
    signed.server.set_folders(&[(LONG_NOTES, &[OWNED_B])]);
    signed.fetch(false);
    assert!(
        signed.app.shared.opened() == [SelectFolder::UserFolders, signed.app.shared.user_folder(0)],
        "the list is no longer looking into the folder it was in"
    );

    signed.server.set_index(IndexAnswer::Folders(vec![summary(PRACTICE, "practice 12", 10, 3)]));
    signed.fetch(false);
    assert!(signed.app.shared.opened() == [SelectFolder::UserFolders], "a folder that is gone is still being looked into");

    signed.server.set_index(IndexAnswer::Folders(Vec::new()));
    signed.fetch(false);
    assert!(signed.app.shared.is_at_root(), "no folders left, and the list is still on them");
}

/// A run that starts signed in has the folders kept from the last one on show from its first frame,
/// before anything has been fetched, and matched against the library it starts with.
#[test]
fn the_folders_kept_are_on_show_from_the_start_of_a_run() {
    let mut signed = Signed::new("restart", two_folders());
    signed.fetch(false);
    let settings = signed.app.shared.settings_path.clone();
    let mut config = Config::default();
    config.network.ir_token = Some(TOKEN.into());
    config.network.ir_login_id = Some(PLAYER.into());
    let restarted = App::new(String::new(), config, LaunchOptions::default(), settings);
    assert!(restarted.shared.user_folders.shown.is_empty(), "a run without the server the folders came from shows them");

    signed.app.shared.user_folders = UserFolders::default();
    follow_account(&mut signed.app.shared);
    assert_eq!(signed.app.shared.user_folders.shown.len(), 2);
    signed.app.shared.library = Library::from_songs(vec![entry(MISSING)]);
    rematch(&mut signed.app.shared);
    assert_eq!(signed.app.shared.user_folders.chart_counts(), (4, 1), "the folders were not matched against the new library");
}
