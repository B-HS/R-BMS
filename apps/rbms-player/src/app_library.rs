//! `App` methods for the song library's *sources*: the picked song folders and the difficulty
//! tables, plus the two management screens that edit them.
//!
//! Split out of `app_select.rs`, which owns the browsing side of the select screen. Everything
//! here writes the persisted lists and triggers the rescans that rebuild the browse view.
#![allow(clippy::wildcard_imports)]
use crate::*;

impl App {
    pub(crate) fn open_tables(&mut self) {
        self.tables_sel = 0;
        self.cancel_text_edit();
        self.stage = Stage::Tables;
    }

    pub(crate) fn open_folders(&mut self) {
        self.folders_sel = 0;
        self.stage = Stage::Folders;
    }

    pub(crate) fn folders_row_count(&self) -> usize {
        self.folders.len() + 1 // the folders + a trailing "+ ADD FOLDER" row
    }

    /// Pick a folder and add it to the library list (deduped, persisted). The merged rescan happens
    /// when the user leaves the Folders screen, so several folders can be added in one visit.
    pub(crate) fn add_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Add song folder").pick_folder() {
            let path = dir.to_string_lossy().to_string();
            if !self.folders.iter().any(|f| f == &path) {
                self.folders.push(path);
                FolderList { folders: self.folders.clone() }.save(&self.folders_path);
            }
        }
    }

    pub(crate) fn remove_folder(&mut self, idx: usize) {
        if idx < self.folders.len() {
            self.folders.remove(idx);
            FolderList { folders: self.folders.clone() }.save(&self.folders_path);
            self.folders_sel = self.folders_sel.min(self.folders_row_count().saturating_sub(1));
        }
    }

    /// Rescan every library folder off-thread and merge into one song list (then re-match tables),
    /// landing back on Select. Used after the folder list changes.
    pub(crate) fn rescan_all_folders(&mut self) {
        let dirs = self.folders.clone();
        let sources = self.table_sources.clone();
        self.scan_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = self.scan_count.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let songs = scan_folders(&dirs, &count);
            let (names, levels) = fetch_and_match(&sources, &songs);
            let _ = tx.send(ScanOutcome { songs, names, levels });
        });
        self.scan_rx = Some(rx);
        self.pending = Some(Loading::Scan);
        self.loading_drawn = false;
        self.stage = Stage::Loading;
    }

    /// Keys inside the folder manager. Leaving it rescans, so a folder added or removed here is
    /// merged back into the library on the way out.
    pub(crate) fn folders_input(&mut self, code: KeyCode) {
        let n = self.folders_row_count();
        let add_row = self.folders.len();
        match code {
            KeyCode::Escape => self.rescan_all_folders(),
            KeyCode::ArrowUp => self.folders_sel = self.folders_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.folders_sel = (self.folders_sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete => {
                if self.folders_sel < self.folders.len() {
                    self.remove_folder(self.folders_sel);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.folders_sel == add_row {
                    self.add_folder_dialog();
                }
            }
            _ => {}
        }
    }

    /// Rows in the table-manager: each source, then the two add actions.
    pub(crate) fn tables_row_count(&self) -> usize {
        self.table_sources.len() + 2
    }

    pub(crate) fn add_table_source(&mut self, src: TableSource) {
        let (name, levels) = load_and_match(&src, &self.songs);
        self.table_sources.push(src);
        self.table_names.push(name);
        self.table_levels.push(levels);
        TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
    }

    pub(crate) fn add_table_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Select table json").add_filter("json", &["json"]).pick_file() {
            let location = path.to_string_lossy().to_string();
            if self.table_sources.iter().any(|t| t.location == location) {
                eprintln!("table already added: {location}");
                return;
            }
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table").to_string();
            self.add_table_source(TableSource { name, location });
        }
    }

    pub(crate) fn remove_table_source(&mut self, idx: usize) {
        if idx < self.table_sources.len() {
            self.table_sources.remove(idx);
            if idx < self.table_names.len() {
                self.table_names.remove(idx);
            }
            if idx < self.table_levels.len() {
                self.table_levels.remove(idx);
            }
            TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
            self.tables_sel = self.tables_sel.min((self.table_sources.len() + 2).saturating_sub(1));
        }
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), remove (D), or leave (Esc — rebuilds the browse list).
    pub(crate) fn tables_input(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, typed: Option<&str>) {
        if self.text_input.is_some() {
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let url = self.text_input.take().unwrap_or_default().trim().to_string();
                    if !url.is_empty() {
                        if self.table_sources.iter().any(|t| t.location == url) {
                            eprintln!("table already added: {url}");
                        } else {
                            self.add_table_source(TableSource { name: String::new(), location: url });
                        }
                    }
                }
                KeyCode::Escape => self.text_input = None,
                KeyCode::Backspace => {
                    if let Some(b) = self.text_input.as_mut() {
                        b.pop();
                    }
                }
                _ => {
                    if let (Some(b), Some(t)) = (self.text_input.as_mut(), typed) {
                        b.extend(t.chars().filter(|c| !c.is_control()));
                    }
                }
            }
            return;
        }
        let n = self.tables_row_count();
        let add_url = self.table_sources.len();
        let add_file = self.table_sources.len() + 1;
        match code {
            KeyCode::Escape => {
                self.select_view = SelectView::Root;
                self.sel = 0;
                self.rebuild_select_items();
                self.stage = Stage::Select;
                let _ = event_loop;
            }
            KeyCode::ArrowUp => self.tables_sel = self.tables_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.tables_sel = (self.tables_sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete => {
                if self.tables_sel < self.table_sources.len() {
                    self.remove_table_source(self.tables_sel);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.tables_sel == add_url {
                    self.text_input = Some(String::new());
                } else if self.tables_sel == add_file {
                    self.add_table_file();
                }
            }
            _ => {}
        }
    }
}
