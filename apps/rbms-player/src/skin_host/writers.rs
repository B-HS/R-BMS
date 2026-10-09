//! What a skin writes back: the share a dragged slider is set to and the text typed into an
//! editable field.
//!
//! A write arrives while a frame is being drawn or the pointer is being routed, when nothing may
//! change, so the host records each one as it arrives and hands them over when the frame is done
//! ([`super::ScreenHost::take_calls`]). Applying them belongs here: which setting a rate id moves and
//! what a string id is typed into.
//!
//! Nothing is applied yet.
