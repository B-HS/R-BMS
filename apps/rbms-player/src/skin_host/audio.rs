//! What a skin asks of the sound system: `main_state.audio_play`, `audio_loop`, `audio_stop` and the
//! rest.
//!
//! A skin makes these requests while a frame is being drawn, when nothing may change, so the host
//! records each one as it arrives and hands them over when the frame is done
//! ([`super::ScreenHost::take_calls`]). Carrying them out belongs here: playing a file on the skin's
//! own bus at the volume the skin asked for times the system volume, looping it, and stopping it.
//!
//! Nothing is carried out yet.
