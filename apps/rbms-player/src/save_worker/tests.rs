//! The save thread: writes land in the order they were handed over, a settings save a later one
//! replaces is skipped, what fails comes back, and nothing handed over is lost when the application
//! ends.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// How many writes a test hands over at once.
const WRITES: usize = 20;

fn own_folder(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("rbms-save-worker-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is writable");
    folder
}

/// A write that counts itself and writes `text` to `path`.
fn counted(path: &Path, text: &str, runs: &Arc<AtomicUsize>) -> Job {
    let (path, text, runs) = (path.to_path_buf(), text.to_string(), Arc::clone(runs));
    Job {
        what: Saving::Settings,
        replaces: Some(path.clone()),
        work: Box::new(move || {
            runs.fetch_add(1, Ordering::Relaxed);
            rbms_store::write_atomic(&path, &text).map_err(|e| e.to_string())
        }),
    }
}

#[test]
fn a_settings_save_a_later_one_replaces_is_not_written() {
    let folder = own_folder("replaced");
    let path = folder.join("settings.ron");
    let (gate, held) = mpsc::channel();
    let worker = SaveWorker::spawn_gated(held);
    let runs = Arc::new(AtomicUsize::new(0));
    for at in 0..WRITES {
        worker.hand_over(counted(&path, &format!("save {at}"), &runs));
    }
    assert!(!path.exists(), "nothing is written before the thread gets to it");
    drop(gate);
    worker.flush();
    assert_eq!(std::fs::read_to_string(&path).expect("written"), format!("save {}", WRITES - 1), "the last save is what lands");
    assert_eq!(runs.load(Ordering::Relaxed), 1, "the saves it replaced were not written");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn writes_of_different_files_all_land_in_the_order_they_were_handed_over() {
    let folder = own_folder("ordered");
    let worker = SaveWorker::spawn();
    for at in 0..WRITES {
        worker.save_text(Saving::PendingRun { file: folder.join(format!("{at}.ron")) }, folder.join(format!("{at}.ron")), at.to_string());
    }
    worker.save_text(Saving::PendingRun { file: folder.join("0.ron") }, folder.join("0.ron"), "again".into());
    worker.flush();
    for at in 1..WRITES {
        assert_eq!(std::fs::read_to_string(folder.join(format!("{at}.ron"))).expect("written"), at.to_string());
    }
    assert_eq!(std::fs::read_to_string(folder.join("0.ron")).expect("written"), "again", "the later write of one file lands last");
    assert!(worker.failures().is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_write_that_fails_comes_back_to_the_frame_thread() {
    let folder = own_folder("failed");
    let in_the_way = folder.join("pending-runs");
    std::fs::write(&in_the_way, "a file where the folder should be").expect("written");
    let worker = SaveWorker::spawn();
    let target = in_the_way.join("run.ron");
    worker.save_text(Saving::PendingRun { file: target.clone() }, target.clone(), "run".into());
    worker.flush();
    let failures = worker.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].what, Saving::PendingRun { file: target });
    assert!(failure_line(&failures[0]).starts_with("run not kept: "), "{}", failure_line(&failures[0]));
    let _ = std::fs::remove_dir_all(&folder);
}

/// The application ending lets the thread finish every write it was handed: none is lost.
#[test]
fn dropping_the_worker_finishes_every_write_handed_over() {
    let folder = own_folder("drained");
    let (gate, held) = mpsc::channel();
    let worker = SaveWorker::spawn_gated(held);
    for at in 0..WRITES {
        worker.save_text(Saving::PendingRun { file: folder.join(format!("{at}.ron")) }, folder.join(format!("{at}.ron")), at.to_string());
    }
    drop(gate);
    drop(worker);
    assert_eq!(std::fs::read_dir(&folder).expect("lists").count(), WRITES, "every write landed before the thread ended");
    let _ = std::fs::remove_dir_all(&folder);
}

/// An answer that comes in while a run is played -- the rivals the server holds, a session it
/// confirmed -- saves the settings on the save thread, not on the frame the run is drawn on.
#[test]
fn an_answer_that_arrives_mid_run_writes_no_file_on_the_frame_thread() {
    let folder = own_folder("mid-run");
    let mut shared = crate::App::new(String::new(), Config::default(), crate::LaunchOptions::default(), folder.join("settings.ron")).shared;
    let (gate, held) = mpsc::channel();
    shared.saves = SaveWorker::spawn_gated(held);
    let _ = std::fs::remove_file(&shared.settings_path);
    let (tx, rx) = mpsc::channel();
    tx.send(Ok(vec![rbms_ir::PlayerProfile { id: "rival".into(), name: "rival".into(), total_plays: 0, rank_points: 0.0, extra: Default::default() }]))
        .expect("the receiver is open");
    shared.rivals_rx = Some(rx);
    shared.poll_network();
    assert_eq!(shared.config.network.rivals, ["rival"]);
    assert!(!shared.settings_path.exists(), "the frame thread wrote the settings");
    drop(gate);
    shared.saves.flush();
    let saved = rbms_config::load(&shared.settings_path).expect("the settings read back").config;
    assert_eq!(saved.network.rivals, ["rival"]);
    let _ = std::fs::remove_dir_all(&folder);
}

/// A flush answers once what was handed over before it is on disk, so a settings save handed over
/// after the flush does not replace the one before it, even when the thread finds the three waiting
/// together.
#[test]
fn a_save_after_a_flush_does_not_replace_the_one_before_it() {
    let folder = own_folder("flush-boundary");
    let path = folder.join("settings.ron");
    let (gate, held) = mpsc::channel();
    let worker = SaveWorker::spawn_gated(held);
    let (before, after) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    worker.hand_over(counted(&path, "before", &before));
    let (ack, flushed) = mpsc::channel();
    worker.tx.as_ref().expect("the thread runs").send(Message::Flush(ack)).expect("the thread listens");
    worker.hand_over(counted(&path, "after", &after));
    drop(gate);
    flushed.recv().expect("the flush is answered");
    assert_eq!(before.load(Ordering::Relaxed), 1, "the flush was answered before the save handed over ahead of it was written");
    worker.flush();
    assert_eq!(after.load(Ordering::Relaxed), 1);
    assert_eq!(std::fs::read_to_string(&path).expect("written"), "after", "the later save lands last");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Quit in the macOS menu or the Dock exits the process from inside the event loop, after the
/// loop's `exiting` and before anything is dropped. What the frame thread handed over -- a settings
/// change, a run kept on the side -- is on disk once the application finishes its saves there,
/// with no `Drop` of the worker to wait for it.
#[test]
fn the_saves_handed_over_are_on_disk_when_the_event_loop_ends_without_a_drop() {
    let folder = own_folder("exiting");
    let mut app = crate::App::new(String::new(), Config::default(), crate::LaunchOptions::default(), folder.join("settings.ron"));
    let (gate, held) = mpsc::channel();
    app.shared.saves = SaveWorker::spawn_gated(held);
    let _ = std::fs::remove_file(&app.shared.settings_path);
    app.shared.config.network.rivals = vec!["rival".into()];
    app.shared.save_settings();
    let kept = folder.join("kept-run.ron");
    app.shared.saves.save_text(Saving::PendingRun { file: kept.clone() }, kept.clone(), "run".into());
    drop(gate);
    app.shared.finish_saves();
    let saved = rbms_config::load(&app.shared.settings_path).expect("the settings are on disk").config;
    assert_eq!(saved.network.rivals, ["rival"]);
    assert_eq!(std::fs::read_to_string(&kept).expect("the kept run is on disk"), "run");
    std::mem::forget(app);
    let _ = std::fs::remove_dir_all(&folder);
}
