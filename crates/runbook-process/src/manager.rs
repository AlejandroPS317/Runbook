//! Every session of the app, by id. The app keeps one manager; closing the app stops all
//! of its sessions through `stop_all`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::session::{Session, lock};
use crate::tree::StopOutcome;

pub type SessionId = u32;

#[derive(Default)]
pub struct SessionManager {
    sessions: Mutex<HashMap<SessionId, Arc<Session>>>,
    next_id: AtomicU32,
}

impl SessionManager {
    /// Registers a session and returns its new id (ids start at 1 and are never reused).
    pub fn insert(&self, session: Arc<Session>) -> SessionId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        lock(&self.sessions).insert(id, session);
        id
    }

    /// The session, cloned out so no lock is held while it is used (a stop can take
    /// seconds).
    pub fn get(&self, id: SessionId) -> Option<Arc<Session>> {
        lock(&self.sessions).get(&id).cloned()
    }

    /// Forgets the session and returns it, e.g. to stop it.
    pub fn remove(&self, id: SessionId) -> Option<Arc<Session>> {
        lock(&self.sessions).remove(&id)
    }

    pub fn len(&self) -> usize {
        lock(&self.sessions).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Stops every session (script and process tree) in parallel and waits for all of
    /// them, so the slowest session bounds the total time (about `grace` plus the kill
    /// wait). The manager is empty afterwards.
    pub fn stop_all(&self, grace: Duration) -> Vec<StopOutcome> {
        let sessions: Vec<Arc<Session>> = lock(&self.sessions).drain().map(|(_, s)| s).collect();
        let stoppers: Vec<_> = sessions
            .into_iter()
            .map(|session| thread::spawn(move || session.stop(grace)))
            .collect();
        stoppers
            .into_iter()
            .map(|stopper| stopper.join().unwrap_or(StopOutcome::Survivors(Vec::new())))
            .collect()
    }
}
