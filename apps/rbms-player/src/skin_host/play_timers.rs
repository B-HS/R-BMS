//! The play screen's timer driver: what switches a play skin's timers on and off as a run goes.
//!
//! A timer is not read from a cluster. It lives in the scene's timer table, which the host reads
//! from directly, and something has to switch each one at the moment the reference does: the ready
//! and play timers, the bombs, the held long notes, the keys going down and coming up, the rhythm
//! timer, the full combo and the end of the notes. That driver belongs here, together with the
//! play screen's lift, lane cover and hidden offsets.
//!
//! Nothing is driven from here yet: the play screen still switches its timers through the
//! renderer's own driver.
