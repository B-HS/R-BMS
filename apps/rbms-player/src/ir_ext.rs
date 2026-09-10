//! Multiple score server profiles: the profile axis over the existing IR client, submitting to
//! every enabled profile and picking the primary one for panels.
//!
//! A player can hold accounts on several IRs at once. This module is the whole of that idea:
//! [`MultiIr`] is an ordered list of [`IrProfile`]s plus the index of the one panels read from,
//! [`MultiIr::submit_all`] fans one score out to every enabled profile in parallel and isolates a
//! failure to the profile it happened on, and [`MultiIr::primary_server`] answers which profile the
//! ranking panel queries.
//!
//! The legacy single server (`network.server_url` + `network.ir_token`) stays the first profile, so
//! an empty `network.ir_profiles` behaves exactly like the single-server client it replaces.
//!
//! Nothing here draws or blocks a frame: [`spawn_submit_all`] hands the fan-out to a worker thread
//! the frame loop polls, the same shape as [`rbms_ir::spawn_submit`].
//!
//! The module is complete but not yet called: the call sites named in
//! `docs/plan/phase-g-wiring/g7.md` belong to the integration branch, and the allow below goes away
//! with the first of them.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use rbms_config::{IrProfile, NetworkOptions};
use rbms_ir::{HttpScoreServer, IrError, NullScoreServer, ScoreServer, ScoreSubmission, SubmitResponse};

use crate::ir_outcome::short_error;

/// Name given to the profile built from the legacy single `server_url`, so it is distinguishable
/// from a user-added one in the settings row and on the panel's profile tab.
pub(crate) const LEGACY_PROFILE_NAME: &str = "MAIN";

/// Prefix of the fallback label for a profile whose `name` is blank: `IR 1`, `IR 2`, … in profile
/// order.
const UNNAMED_PROFILE_PREFIX: &str = "IR";

/// Offset that turns a zero-based profile index into the one-based number shown to the player.
const DISPLAY_INDEX_OFFSET: usize = 1;

/// Reported for a profile whose worker thread unwound. A `ScoreServer` implementation that panics
/// is a bug, but it must not take the other profiles' results down with it.
const PROFILE_PANIC_MESSAGE: &str = "profile worker panicked";

/// Summary text when there is nothing to submit to at all.
const NO_PROFILES_MESSAGE: &str = "no IR profiles";

/// Summary suffix for a profile that answered without taking the score.
const REJECTED_MESSAGE: &str = "rejected";

/// One profile's answer to a fan-out call: the label it came from, and what that server said.
pub(crate) type ProfileResult<T> = (String, Result<T, IrError>);

/// The label a profile is shown and reported under. Falls back to `IR <n>` when the profile has no
/// name, so an unnamed profile is still tellable from its neighbours in a summary line.
pub(crate) fn profile_label(profile: &IrProfile, index: usize) -> String {
    if profile.name.trim().is_empty() {
        return format!("{UNNAMED_PROFILE_PREFIX} {}", index + DISPLAY_INDEX_OFFSET);
    }
    profile.name.clone()
}

/// The score servers a play is submitted to, and which one the ranking panel reads.
///
/// `profiles` is the display and submission order. `primary` indexes into it. When `has_legacy` is
/// set, `profiles[0]` is the legacy single `server_url` rather than an `ir_profiles` entry, which is
/// what lets the already-built main server be reused instead of opening a second client to it (see
/// [`MultiIr::build_servers_reusing`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MultiIr {
    pub(crate) profiles: Vec<IrProfile>,
    pub(crate) primary: usize,
    pub(crate) has_legacy: bool,
}

impl MultiIr {
    /// Read the profile list out of the NETWORK settings: the legacy single server first (when one
    /// is configured), then every entry of `ir_profiles` in stored order.
    pub(crate) fn from_network(network: &NetworkOptions) -> MultiIr {
        let mut profiles = Vec::with_capacity(network.ir_profiles.len() + DISPLAY_INDEX_OFFSET);
        let has_legacy = network.server_url.is_some();
        if let Some(url) = &network.server_url {
            profiles.push(IrProfile { name: LEGACY_PROFILE_NAME.to_string(), base_url: url.clone(), token: network.ir_token.clone(), enabled: true });
        }
        profiles.extend(network.ir_profiles.iter().cloned());
        let primary = profiles.iter().position(|p| p.enabled).unwrap_or_default();
        MultiIr { profiles, primary, has_legacy }
    }

    /// Index of the legacy single server in `profiles`, when one is configured.
    pub(crate) fn legacy_index(&self) -> Option<usize> {
        self.has_legacy.then_some(0)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Every profile a score is submitted to, paired with its index in `profiles`.
    pub(crate) fn enabled_profiles(&self) -> impl Iterator<Item = (usize, &IrProfile)> {
        self.profiles.iter().enumerate().filter(|(_, profile)| profile.enabled)
    }

    /// Labels in profile order, for the panel's profile tab strip.
    pub(crate) fn labels(&self) -> Vec<String> {
        self.profiles.iter().enumerate().map(|(index, profile)| profile_label(profile, index)).collect()
    }

    /// Which profile the ranking panel queries: the primary one when it is in range and enabled,
    /// otherwise the first enabled profile, and `None` when no profile is enabled at all.
    ///
    /// The fallback matters because a profile can be disabled while it is still the selected tab;
    /// answering `None` there would blank a panel that has a perfectly good server behind it.
    pub(crate) fn primary_server(&self) -> Option<usize> {
        if self.profiles.get(self.primary).is_some_and(|profile| profile.enabled) {
            return Some(self.primary);
        }
        self.enabled_profiles().map(|(index, _)| index).next()
    }

    /// Move the panel to another profile. Rejects an index that is not a profile, so a stale tab
    /// index cannot push `primary` out of range.
    pub(crate) fn set_primary(&mut self, index: usize) -> bool {
        if index >= self.profiles.len() {
            return false;
        }
        self.primary = index;
        true
    }

    /// One client per profile, index-aligned with `profiles`. A disabled profile, a blank URL and a
    /// client that fails to build all get [`NullScoreServer`], so the alignment never breaks and a
    /// call against one of them fails fast instead of reaching the wrong server.
    pub(crate) fn build_servers(&self) -> Vec<Arc<dyn ScoreServer>> {
        self.profiles.iter().map(build_profile_server).collect()
    }

    /// Like [`MultiIr::build_servers`], but the legacy profile keeps the client the app already
    /// built for `network.server_url` — the one the health probe and the ranking panel share.
    pub(crate) fn build_servers_reusing(&self, main: Option<Arc<dyn ScoreServer>>) -> Vec<Arc<dyn ScoreServer>> {
        let mut servers = self.build_servers();
        if let (Some(index), Some(main)) = (self.legacy_index(), main)
            && let Some(slot) = servers.get_mut(index)
        {
            *slot = main;
        }
        servers
    }

    /// The same profile list with the legacy one switched off, for a caller that already submits to
    /// `network.server_url` through the single-server path.
    ///
    /// Disabling rather than removing keeps every index — and therefore every client in a
    /// [`MultiIr::build_servers`] vector and every `IR <n>` label — exactly where it was, so the two
    /// views of the list stay interchangeable.
    pub(crate) fn secondary(&self) -> MultiIr {
        let mut secondary = self.clone();
        if let Some(index) = self.legacy_index()
            && let Some(profile) = secondary.profiles.get_mut(index)
        {
            profile.enabled = false;
        }
        secondary
    }

    /// Submit one score to every enabled profile in parallel, reporting each profile's answer under
    /// its label and in profile order.
    ///
    /// Isolation is the whole point: one profile timing out, answering 500 or panicking leaves the
    /// others' results untouched. The submission is clamped once, before the fan-out, so every
    /// profile receives the identical bytes [`rbms_ir::spawn_submit`] would have sent.
    pub(crate) fn submit_all(&self, servers: &[Arc<dyn ScoreServer>], sub: &ScoreSubmission) -> Vec<ProfileResult<SubmitResponse>> {
        let mut clamped = sub.clone();
        clamped.clamp_to_server_bounds();
        self.fan_out(servers, |server| server.submit_score(&clamped))
    }

    /// Run one blocking call against every enabled profile at once and collect the answers in
    /// profile order. A profile with no server at its index reports [`IrError::NotConfigured`]; a
    /// profile whose thread unwound reports [`IrError::Network`] rather than poisoning the caller.
    pub(crate) fn fan_out<T, F>(&self, servers: &[Arc<dyn ScoreServer>], op: F) -> Vec<ProfileResult<T>>
    where
        T: Send,
        F: Fn(&dyn ScoreServer) -> Result<T, IrError> + Sync,
    {
        let targets: Vec<(String, Option<Arc<dyn ScoreServer>>)> =
            self.enabled_profiles().map(|(index, profile)| (profile_label(profile, index), servers.get(index).cloned())).collect();
        let mut out = Vec::with_capacity(targets.len());
        std::thread::scope(|scope| {
            let running: Vec<_> = targets
                .into_iter()
                .map(|(label, server)| {
                    let op = &op;
                    let handle = scope.spawn(move || match server {
                        Some(server) => op(server.as_ref()),
                        None => Err(IrError::NotConfigured),
                    });
                    (label, handle)
                })
                .collect();
            for (label, handle) in running {
                let result = handle.join().unwrap_or_else(|_| Err(IrError::Network(PROFILE_PANIC_MESSAGE.to_string())));
                out.push((label, result));
            }
        });
        out
    }
}

/// The client for one profile: an authenticated [`HttpScoreServer`] when the profile is usable,
/// [`NullScoreServer`] when it is disabled, has no URL, or its HTTP client could not be built.
fn build_profile_server(profile: &IrProfile) -> Arc<dyn ScoreServer> {
    if !profile.enabled || profile.base_url.trim().is_empty() {
        return Arc::new(NullScoreServer);
    }
    match HttpScoreServer::try_new(profile.base_url.clone(), profile.token.clone()) {
        Ok(server) => Arc::new(server),
        Err(_) => Arc::new(NullScoreServer),
    }
}

/// Run [`MultiIr::submit_all`] on a worker thread and deliver the whole fan-out over a channel the
/// frame loop polls with `try_recv`, so a slow profile never stalls a frame.
///
/// The thread ends as soon as the last profile answers; dropping the receiver does not cancel it.
pub(crate) fn spawn_submit_all(multi: MultiIr, servers: Vec<Arc<dyn ScoreServer>>, sub: ScoreSubmission) -> Receiver<Vec<ProfileResult<SubmitResponse>>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(multi.submit_all(&servers, &sub));
    });
    rx
}

/// One status line for a finished fan-out: how many profiles took the score, and the first profile
/// that did not, so a single failing IR is named instead of silently dropped.
pub(crate) fn submit_summary(results: &[ProfileResult<SubmitResponse>]) -> String {
    if results.is_empty() {
        return NO_PROFILES_MESSAGE.to_string();
    }
    let accepted = results.iter().filter(|(_, result)| matches!(result, Ok(response) if response.accepted)).count();
    let head = format!("{UNNAMED_PROFILE_PREFIX} {accepted}/{}", results.len());
    match results.iter().find_map(first_problem) {
        Some((label, detail)) => format!("{head} — {label}: {detail}"),
        None => head,
    }
}

/// The profile-level problem in one fan-out entry: a failed call, or a call the server answered
/// without accepting the score.
fn first_problem(entry: &ProfileResult<SubmitResponse>) -> Option<(&str, String)> {
    let (label, result) = entry;
    match result {
        Err(error) => Some((label.as_str(), short_error(error))),
        Ok(response) if !response.accepted => Some((label.as_str(), REJECTED_MESSAGE.to_string())),
        Ok(_) => None,
    }
}

#[cfg(test)]
mod tests;
