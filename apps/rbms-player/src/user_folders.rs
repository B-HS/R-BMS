//! The player's own folders of charts, kept on their account: fetched from the score server they
//! are signed in to, kept beside the settings, and shown in the song list as one row at the top of
//! the library (`MY FOLDERS`) holding a row per folder, each holding the charts of it the library
//! has.
//!
//! The folders are edited on the web site and only read here, the way a difficulty table is. A
//! folder names its charts by md5, which is how they are matched against the library; a chart the
//! library does not have is not a row, and the folder's row says how many of its charts are there
//! out of how many it lists.
//!
//! What was fetched last is kept per account and per server (`user-folders/<hash>.ron` beside the
//! settings, written in the settings' own format), so the folders are there from the first frame of a run, offline too, and a fetch
//! asks again only for the folders whose stamp has moved since. Each account's folders are its own:
//! another account signing in sees nothing of them, signing out deletes them, and a session the
//! server stopped accepting hides them until the account is signed in again.
//!
//! Nothing here waits on the network. A fetch runs on a worker beside the frame loop, and what it
//! comes back with is taken by whichever screen is up ([`apply_finished_fetch`]), the way a table
//! is ([`crate::tablesrc::apply_finished_fetches`]).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rbms_ir::{IrError, PlayerId, ScoreServer, UserFolderItem, UserFolderSummary, is_path_safe_folder_id};
use rbms_library::Library;
use serde::{Deserialize, Serialize};

use crate::AppShared;
use crate::ir_outcome::short_error;
use crate::notify::{Level, notify};
use crate::select_tree::{ListPlace, show_the_list_again};
use crate::tablesrc::{unix_now, url_hash};

/// The row the folders are under at the top of the song list, and what the table manager calls
/// them.
pub(crate) const USER_FOLDERS_LABEL: &str = "MY FOLDERS";

/// The folder beside the settings file the copies of each account's folders are kept in.
const COPY_FOLDER: &str = "user-folders";

/// What a copy's file is called after the hash of its account.
const COPY_EXTENSION: &str = "ron";

/// What goes between the server and the login id in the text a copy's file is named after.
const ACCOUNT_SEPARATOR: &str = "\n";

/// The most folders, and the most charts in one folder, a player can have on the server. What a
/// server sends past these is left out rather than trusted.
const MAX_FOLDERS: usize = 50;
const MAX_CHARTS_PER_FOLDER: usize = 2_000;

/// The longest name and description a folder can have on the server.
const NAME_MAX_CHARS: usize = 64;
const DESCRIPTION_MAX_CHARS: usize = 255;

/// The length of an md5 written in hex, which is how a chart of a folder is named.
const MD5_HEX_LEN: usize = 32;

/// How many folders are asked for at once.
const MAX_PARALLEL_FETCHES: usize = 4;

/// The largest copy that is read. Every folder a player can have at its fullest is a few megabytes.
const MAX_COPY_BYTES: u64 = 32 * 1024 * 1024;

/// What a folder whose name is nothing but what was taken out of it is called.
const UNNAMED_FOLDER: &str = "FOLDER";

/// Why there is no answer when the worker that was fetching stopped.
const FETCHER_STOPPED: &str = "the folder fetcher stopped unexpectedly";

/// The account a set of folders belongs to: the server, as the settings name it, and the login id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderAccount {
    server: String,
    player_id: String,
}

impl FolderAccount {
    /// The account signed in now on the score server of the settings, or `None` when nobody is
    /// signed in to one.
    pub(crate) fn of(shared: &AppShared) -> Option<FolderAccount> {
        let player_id = shared.session.signed_in_as()?;
        let server = shared.config.network.server_url.as_deref()?.trim().trim_end_matches('/');
        (!server.is_empty()).then(|| FolderAccount { server: server.to_string(), player_id: player_id.to_string() })
    }

    /// Where the copy of this account's folders is kept, for the settings file at `settings_path`.
    fn copy_path(&self, settings_path: &Path) -> PathBuf {
        let folder = settings_path.parent().map_or_else(|| PathBuf::from(COPY_FOLDER), |parent| parent.join(COPY_FOLDER));
        folder.join(format!("{}.{COPY_EXTENSION}", url_hash(&format!("{}{ACCOUNT_SEPARATOR}{}", self.server, self.player_id))))
    }
}

/// One folder as the copy keeps it: what the server said of it, without what nothing here reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SavedFolder {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    /// The server's stamp of the folder, which is how a fetch tells the folder has not changed.
    #[serde(default)]
    updated_at: i64,
    #[serde(default)]
    items: Vec<UserFolderItem>,
}

/// The copy of one account's folders, as it is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SavedFolders {
    server: String,
    player_id: String,
    /// When the folders were fetched, in seconds since the Unix epoch.
    #[serde(default)]
    fetched_at: Option<u64>,
    #[serde(default)]
    folders: Vec<SavedFolder>,
}

impl SavedFolders {
    /// The copy kept of `account`'s folders, when there is a readable one that says it is theirs. A
    /// file of another account that came to the same name, a damaged one and one too large to be a
    /// copy are no copy.
    fn read(account: &FolderAccount, settings_path: &Path) -> Option<SavedFolders> {
        let file = std::fs::File::open(account.copy_path(settings_path)).ok()?;
        let mut bytes = Vec::new();
        file.take(MAX_COPY_BYTES + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() as u64 > MAX_COPY_BYTES {
            return None;
        }
        let saved: SavedFolders = ron::from_str(std::str::from_utf8(&bytes).ok()?).ok()?;
        saved.is_of(account).then_some(saved)
    }

    fn is_of(&self, account: &FolderAccount) -> bool {
        self.server == account.server && self.player_id == account.player_id
    }

    /// Every chart the folders list.
    fn charts(&self) -> usize {
        self.folders.iter().map(|folder| folder.items.len()).sum()
    }
}

/// One folder as the song list shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShownFolder {
    /// The server's id of the folder, which a folder is found again by after its name changed.
    pub(crate) id: String,
    /// The name, with its Latin letters in the capitals the rest of the list uses.
    pub(crate) name: String,
    /// How many charts the folder lists, and how many of those the library has.
    pub(crate) listed: usize,
    pub(crate) owned: usize,
    /// The library positions of the charts the folder lists, in library order.
    pub(crate) charts: Vec<usize>,
}

impl ShownFolder {
    /// The folder's row in the song list: its name, then how many of its charts there are out of
    /// how many it lists, or only how many when the library has them all. `shown` is how many of
    /// them the filter and the search leave.
    pub(crate) fn row_label(&self, shown: usize) -> String {
        if shown >= self.listed { format!("{} ({shown})", self.name) } else { format!("{} ({shown}/{})", self.name, self.listed) }
    }
}

/// `folders` matched against `library`.
fn matched(folders: &[SavedFolder], library: &Library) -> Vec<ShownFolder> {
    folders
        .iter()
        .map(|folder| {
            let mut charts = Vec::new();
            let mut owned = 0;
            for item in &folder.items {
                let found = library.indices_for_md5(&item.md5);
                owned += usize::from(!found.is_empty());
                charts.extend_from_slice(found);
            }
            charts.sort_unstable();
            charts.dedup();
            let name = Some(folder.name.trim()).filter(|name| !name.is_empty()).unwrap_or(UNNAMED_FOLDER).to_ascii_uppercase();
            ShownFolder { id: folder.id.clone(), name, listed: folder.items.len(), owned, charts }
        })
        .collect()
}

/// `text` without control characters, cut to `max` characters.
fn tidy_text(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

/// The folder `summary` describes, holding `items` as a server sent them, with what is not to be
/// trusted taken out: control characters and excess length in its words, charts not named by an
/// md5, and charts past what a folder can hold.
fn tidied(summary: UserFolderSummary, mut items: Vec<UserFolderItem>) -> SavedFolder {
    items.retain(|item| item.md5.len() == MD5_HEX_LEN && item.md5.bytes().all(|byte| byte.is_ascii_hexdigit()));
    items.truncate(MAX_CHARTS_PER_FOLDER);
    for item in &mut items {
        item.md5.make_ascii_lowercase();
    }
    SavedFolder {
        name: tidy_text(&summary.name, NAME_MAX_CHARS),
        description: tidy_text(&summary.description, DESCRIPTION_MAX_CHARS),
        updated_at: summary.updated_at,
        id: summary.id,
        items,
    }
}

/// What a fetch of the folders came to.
#[derive(Debug)]
enum FetchOutcome {
    /// The folders as the server has them now, every one of them that could be fetched, and why
    /// each of the others could not. A folder that failed is what the copy held of it, or left out
    /// when the copy did not have it.
    Fetched { saved: SavedFolders, failures: Vec<String> },
    /// The server has no folders to give: it is from before them, or there is none.
    NoFolders,
    /// The server would not give this account's folders to the session.
    Refused(String),
    /// The folders could not be asked for.
    Failed(String),
}

/// Fetch every folder of `account` whose stamp is not the one `previous` holds, and put the
/// folders together in the order the server lists them.
fn fetch_folders(server: &dyn ScoreServer, account: &FolderAccount, previous: Option<&SavedFolders>, now: u64) -> FetchOutcome {
    let player = PlayerId { id: account.player_id.clone() };
    let index = match server.user_folders(&player) {
        Ok(index) => index,
        Err(IrError::NotFound(_) | IrError::Unsupported | IrError::NotConfigured) => return FetchOutcome::NoFolders,
        Err(error) if error.is_auth_failure() => return FetchOutcome::Refused(short_error(&error)),
        Err(error) => return FetchOutcome::Failed(short_error(&error)),
    };
    let summaries: Vec<UserFolderSummary> = index.folders.into_iter().filter(|summary| is_path_safe_folder_id(&summary.id)).take(MAX_FOLDERS).collect();
    let kept: HashMap<&str, &SavedFolder> = previous.map(|saved| saved.folders.iter().map(|folder| (folder.id.as_str(), folder)).collect()).unwrap_or_default();
    let is_current = |summary: &UserFolderSummary| kept.get(summary.id.as_str()).is_some_and(|folder| folder.updated_at == summary.updated_at);
    let stale: Vec<&UserFolderSummary> = summaries.iter().filter(|summary| !is_current(summary)).collect();
    let mut fetched: HashMap<String, Result<Vec<UserFolderItem>, IrError>> = HashMap::new();
    for batch in stale.chunks(MAX_PARALLEL_FETCHES) {
        let answers: Vec<(String, Result<Vec<UserFolderItem>, IrError>)> = std::thread::scope(|scope| {
            let asking: Vec<_> =
                batch.iter().map(|summary| (summary.id.clone(), scope.spawn(|| server.user_folder(&player, &summary.id).map(|folder| folder.items)))).collect();
            asking.into_iter().map(|(id, asked)| (id, asked.join().unwrap_or_else(|_| Err(IrError::Network(FETCHER_STOPPED.to_string()))))).collect()
        });
        fetched.extend(answers);
    }
    let mut folders = Vec::with_capacity(summaries.len());
    let mut failures = Vec::new();
    for summary in summaries {
        match fetched.remove(&summary.id) {
            None => {
                let items = kept.get(summary.id.as_str()).map(|folder| folder.items.clone()).unwrap_or_default();
                folders.push(tidied(summary, items));
            }
            Some(Ok(items)) => folders.push(tidied(summary, items)),
            Some(Err(error)) => {
                failures.push(format!("{}: {}", tidy_text(&summary.name, NAME_MAX_CHARS), short_error(&error)));
                if let Some(old) = kept.get(summary.id.as_str()) {
                    folders.push((*old).clone());
                }
            }
        }
    }
    let saved = SavedFolders { server: account.server.clone(), player_id: account.player_id.clone(), fetched_at: Some(now), folders };
    FetchOutcome::Fetched { saved, failures }
}

/// A fetch of one account's folders in flight.
struct FolderFetch {
    account: FolderAccount,
    rx: Receiver<FetchOutcome>,
    /// Raised when the account signed out while this was out. The worker holds it while it keeps
    /// what it fetched, so the copy is either written before the sign-out deletes it or not at all.
    cancelled: Arc<Mutex<bool>>,
    /// Whether the player asked for this fetch, and is told how it went whichever way it goes. A
    /// fetch made by itself at the start of a run says only that something changed.
    loud: bool,
    worker: Option<JoinHandle<()>>,
}

/// What became of the last fetch of the folders on show, for the table manager.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FolderProblem {
    /// The server has no folders to give.
    NoFolders,
    /// The fetch, or a folder of it, failed, and why.
    Failed(String),
}

/// What the table manager says of the folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FolderStatus {
    /// Nobody is signed in to a score server.
    SignedOut,
    /// The folders are being fetched.
    Fetching,
    /// The folders on show: how many there are, how many charts they list and how many of those the
    /// library has, when they were fetched, and what went wrong the last time, if anything.
    Shown { folders: usize, listed: usize, owned: usize, fetched_at: Option<u64>, problem: Option<FolderProblem> },
    /// Nothing has been fetched for this account, and why, when a fetch failed.
    NotFetched { problem: Option<FolderProblem> },
}

/// The folders on show and the fetch of them, kept with the state every screen shares.
#[derive(Default)]
pub(crate) struct UserFolders {
    /// The account whose folders are on show.
    account: Option<FolderAccount>,
    saved: Option<SavedFolders>,
    /// The folders as the song list shows them, in the order the player arranged them.
    pub(crate) shown: Vec<ShownFolder>,
    fetch: Option<FolderFetch>,
    problem: Option<FolderProblem>,
}

impl UserFolders {
    /// Whether a fetch of the folders is out.
    #[cfg(test)]
    pub(crate) fn is_fetching(&self) -> bool {
        self.fetch.is_some()
    }

    /// How many charts the folders list, and how many of those the library has.
    pub(crate) fn chart_counts(&self) -> (usize, usize) {
        self.shown.iter().fold((0, 0), |(listed, owned), folder| (listed + folder.listed, owned + folder.owned))
    }

    /// What the table manager says of the folders.
    pub(crate) fn status(&self) -> FolderStatus {
        if self.account.is_none() {
            return FolderStatus::SignedOut;
        }
        if self.fetch.is_some() {
            return FolderStatus::Fetching;
        }
        let problem = self.problem.clone();
        match &self.saved {
            Some(saved) => {
                let (listed, owned) = self.chart_counts();
                FolderStatus::Shown { folders: self.shown.len(), listed, owned, fetched_at: saved.fetched_at, problem }
            }
            None => FolderStatus::NotFetched { problem },
        }
    }
}

/// Put the folders of `saved` on show, matched against the library, with the song list kept where
/// it stood.
fn show(shared: &mut AppShared, saved: Option<SavedFolders>) {
    let place = ListPlace::of(shared);
    shared.user_folders.shown = saved.as_ref().map(|saved| matched(&saved.folders, &shared.library)).unwrap_or_default();
    shared.user_folders.saved = saved;
    show_the_list_again(shared, &place);
}

/// Match the folders on show against the library again, which has just been read again. Asks the
/// network nothing; the caller rebuilds the song list.
pub(crate) fn rematch(shared: &mut AppShared) {
    shared.user_folders.shown = shared.user_folders.saved.as_ref().map(|saved| matched(&saved.folders, &shared.library)).unwrap_or_default();
}

/// Show the folders of whoever is signed in now, when that is not whose folders are on show: the
/// copy kept of the new account's, or nothing. Called whenever the session or the server may have
/// changed; the same account keeps what is on show.
///
/// A fetch of another account that is out is let go of and told not to keep what it fetched: the
/// account it was for may be signing out, whose folders must not be written back after the
/// sign-out deleted them ([`forget_account`]).
pub(crate) fn follow_account(shared: &mut AppShared) {
    let account = FolderAccount::of(shared);
    if account == shared.user_folders.account {
        return;
    }
    if shared.user_folders.fetch.as_ref().is_some_and(|fetch| Some(&fetch.account) != account.as_ref()) {
        let_go_of_fetch(shared);
    }
    let saved = account.as_ref().and_then(|account| SavedFolders::read(account, &shared.settings_path));
    shared.user_folders.account = account;
    shared.user_folders.problem = None;
    show(shared, saved);
}

/// Let go of everything kept of `account`'s folders, which has signed out: a fetch of them that is
/// out is told not to keep what it fetched, and the copy is deleted. The folders leave the song list
/// with the account ([`follow_account`]).
pub(crate) fn forget_account(shared: &mut AppShared, account: &FolderAccount) {
    if shared.user_folders.fetch.as_ref().is_some_and(|fetch| &fetch.account == account) {
        let_go_of_fetch(shared);
    }
    let _ = std::fs::remove_file(account.copy_path(&shared.settings_path));
    follow_account(shared);
}

/// Drop the fetch that is out, telling its worker not to keep what it fetched. The worker holds the
/// flag while it writes, so once this returns the copy is either written already or never will be.
fn let_go_of_fetch(shared: &mut AppShared) {
    if let Some(fetch) = shared.user_folders.fetch.take() {
        *held(&fetch.cancelled) = true;
    }
}

/// What a lock holds, whether or not a thread stopped while holding it.
fn held<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Start fetching the folders of whoever is signed in, beside whichever screen is up, and answer
/// whether a fetch was started. Nothing is started for nobody, or while a fetch of the same account
/// is out already; `loud` is whether the player asked, and is kept by a fetch that is out.
pub(crate) fn start_fetch(shared: &mut AppShared, loud: bool) -> bool {
    follow_account(shared);
    let Some(account) = shared.user_folders.account.clone() else {
        return false;
    };
    if let Some(fetch) = shared.user_folders.fetch.as_mut() {
        fetch.loud |= loud;
        return false;
    }
    let previous = shared.user_folders.saved.clone().filter(|saved| saved.is_of(&account));
    let server = Arc::clone(&shared.server);
    let path = account.copy_path(&shared.settings_path);
    let cancelled = Arc::new(Mutex::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let (worker_account, worker_cancelled) = (account.clone(), Arc::clone(&cancelled));
    let worker = std::thread::spawn(move || {
        let outcome = fetch_folders(server.as_ref(), &worker_account, previous.as_ref(), unix_now());
        if let FetchOutcome::Fetched { saved, .. } = &outcome {
            let cancelled = held(&worker_cancelled);
            if !*cancelled {
                keep(&path, saved);
            }
        }
        let _ = tx.send(outcome);
    });
    shared.user_folders.fetch = Some(FolderFetch { account, rx, cancelled, loud, worker: Some(worker) });
    true
}

/// Write `saved` whole to `path`, replacing the copy that was there.
fn keep(path: &Path, saved: &SavedFolders) {
    let written = ron::to_string(saved).map_err(|error| error.to_string()).and_then(|text| crate::write_atomic(path, &text).map_err(|error| error.to_string()));
    if let Err(reason) = written {
        notify(Level::Warn, format!("user folders not saved: {reason}"));
    }
}

/// Take what the fetch of the folders came back with, and answer whether anything was taken.
///
/// What arrives for an account that is no longer the one signed in is dropped. Folders that arrived
/// are matched against the library as it is now and the song list is rebuilt where it stands; a
/// failed fetch leaves the folders as they were and says why when the player asked for it, and a
/// server that refused the session takes the folders off the list (the copy stays, for when the
/// account signs in again).
pub(crate) fn apply_finished_fetch(shared: &mut AppShared) -> bool {
    let Some(fetch) = shared.user_folders.fetch.as_ref() else {
        return false;
    };
    let outcome = match fetch.rx.try_recv() {
        Ok(outcome) => outcome,
        Err(TryRecvError::Empty) => return false,
        Err(TryRecvError::Disconnected) => FetchOutcome::Failed(FETCHER_STOPPED.to_string()),
    };
    let Some(mut fetch) = shared.user_folders.fetch.take() else {
        return false;
    };
    if let Some(worker) = fetch.worker.take() {
        let _ = worker.join();
    }
    if shared.user_folders.account.as_ref() != Some(&fetch.account) {
        return true;
    }
    match outcome {
        FetchOutcome::Fetched { saved, failures } => {
            let changed = shared.user_folders.saved.as_ref().is_none_or(|before| before.folders != saved.folders);
            shared.user_folders.problem = failures.first().map(|failure| FolderProblem::Failed(failure.clone()));
            if let Some(failure) = failures.first() {
                notify(Level::Warn, format!("user folders not fully refreshed ({} failed): {failure}", failures.len()));
            }
            let (folders, listed) = (saved.folders.len(), saved.charts());
            show(shared, Some(saved));
            let owned = shared.user_folders.chart_counts().1;
            if changed || fetch.loud {
                notify(Level::Info, format!("user folders: {folders} folders, {listed} charts ({owned} owned)"));
            }
        }
        FetchOutcome::NoFolders => {
            shared.user_folders.problem = Some(FolderProblem::NoFolders);
            if fetch.loud {
                notify(Level::Info, "the score server has no folders to load");
            }
        }
        FetchOutcome::Refused(reason) => {
            shared.user_folders.problem = Some(FolderProblem::Failed(reason.clone()));
            show(shared, None);
            if fetch.loud {
                notify(Level::Warn, format!("user folders not refreshed: {reason}"));
            }
        }
        FetchOutcome::Failed(reason) => {
            shared.user_folders.problem = Some(FolderProblem::Failed(reason.clone()));
            if fetch.loud {
                notify(Level::Warn, format!("user folders not refreshed: {reason}"));
            }
        }
    }
    true
}

#[cfg(test)]
pub(crate) mod tests;
