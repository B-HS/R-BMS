//! The skin's sounds as the player's frame loop reaches them: a scene beginning lets go of them, and
//! with no stream open a request has nothing to play on.

use rbms_config::Config;

use super::carry_out;
use super::tests::{FULL_VOLUME, MixerOutput, SHORT_FRAMES, play, temp_dir, write_wav};
use crate::{AppShared, LaunchOptions};

fn shared(name: &str) -> AppShared {
    let dir = temp_dir(name);
    crate::App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron")).shared
}

#[test]
fn beginning_a_scene_lets_go_of_what_the_skin_of_the_scene_before_asked_for() {
    let mut shared = shared("scene-begin");
    let dir = temp_dir("scene-begin-skin");
    let wav = write_wav(&dir, "hit.wav", SHORT_FRAMES);
    let mut out = MixerOutput::new(FULL_VOLUME);

    shared.skin_sounds.request(&mut out, &[dir], play(&wav, FULL_VOLUME, true));
    assert_eq!(shared.skin_sounds.known(), 1);
    shared.begin_skin_scene();
    assert_eq!(shared.skin_sounds.known(), 0, "the next scene must start with none of the last one's sounds");
}

#[test]
fn a_request_with_no_stream_to_play_on_is_dropped_without_decoding_anything() {
    let mut shared = shared("no-stream");
    let dir = temp_dir("no-stream-skin");
    let wav = write_wav(&dir, "hit.wav", SHORT_FRAMES);
    assert!(shared.audio.is_none(), "the tests run with no output device");

    carry_out(&mut shared, play(&wav, FULL_VOLUME, false));
    shared.settle_skin_sounds();
    assert_eq!(shared.skin_sounds.known(), 0);
}

#[test]
fn the_folders_a_skin_may_play_from_are_the_forced_pack_and_the_skin_folder() {
    let mut shared = shared("roots");
    let skin_folder = crate::skin_select::skin_root(&shared.settings_path, &shared.config);
    assert_eq!(super::skin_roots(&shared), vec![skin_folder.clone()]);

    let pack = temp_dir("roots-pack");
    shared.launch.skin_pack = Some(pack.clone());
    assert_eq!(super::skin_roots(&shared), vec![pack, skin_folder]);
}
