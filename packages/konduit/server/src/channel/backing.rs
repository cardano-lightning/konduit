//! It is still unclear what the index story will be.
//! Here we cache something that crudely fits our requirements.
//!
//! The backing needs to carry enough state to:
//!
//! - handle the `./auth/state` request, that is convert to the wire::Backing,
//! - handle thread switching if the user queries with alt utxo
//! - handle pay requests.
//!
//! Invariance. The constructors should make it impossible to represent impossible state.
//! Namely, time goes forward and inputs are unique.
//! Threads already enforce time and uniqueness.
//! Backing enforces uniqueness across threads subject to some assumptions.
//!
//! Assumptions:
//!
//! - Impossible inputs do not occur.
//! If an output is provided more than once, then we assume this is simply a replay.
//! It is too weird for anything else to be the case.
//!
//! Non assumptions:
//!
//! - Event uniqueness. Events can be replayed, and should be idempotent with respect to Backing
//!
//! The design is lenient. If you put bad data in, bad data will exist.
//! The design cannot know something is closed without being told so.

use konduit_wire::auth::state::{self as wire, BackingUtxo, Point};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::channel::{Receipt, thread};

use super::Thread;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Backing {
    /// In normal operations, backing has only a single thread and this is at focus.
    #[n(0)]
    focus: Option<Thread>,
    /// In exotic (non-normal) operations there more than one.
    /// User input can facilitate the switching of focus.
    #[n(1)]
    other: Vec<Thread>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Time must go forward")]
    Time,
    #[error("Inputs must be unique")]
    Unique,
}

impl From<thread::Error> for Error {
    fn from(value: thread::Error) -> Self {
        match value {
            thread::Error::Time => Self::Time,
            thread::Error::Unique => Self::Unique,
        }
    }
}

/// TBD if this reporting bares useful insights.
/// The flow will almost always be is_focus = True always,
/// and is_new = true initially, then fals, and all others = false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// The affected thread is `focus`.
    pub is_focus: bool,
    /// A stale copy of the target utxo was purged first.
    pub is_replace: bool,
    /// The operation truncated descendants past `input`.
    /// (`open` never forks; `cont` forks if `input` is interior; `close` forks if interior.)
    pub is_fork: bool,
    /// The matched thread was already closed and has been reopened.
    /// (`open` and `close` never reopen; only `cont` can.)
    pub is_reopen: bool,
    /// A new thread was created. (`open` always; `cont` if `input` unmatched; `close` never.)
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Loc {
    Focus,
    Other(usize),
}

impl Backing {
    pub fn new(origin: wire::BackingUtxo) -> Self {
        Self {
            focus: Some(Thread::new(origin)),
            other: Vec::new(),
        }
    }

    /// Start a new thread rooted at `output`.
    pub fn open(&mut self, output: BackingUtxo) -> Result<Outcome, Error> {
        let is_replace = self.drop_at(&output.input).is_some();
        let loc = self.place(Thread::new(output));
        Ok(Outcome {
            is_focus: matches!(loc, Loc::Focus),
            is_replace,
            is_new: true,
            is_fork: false,
            is_reopen: false,
        })
    }

    /// Extend the thread rooted at `parent` with `output`.
    /// If `parent` isn't found anywhere, starts a fresh thread at `output`
    /// (lenient: an unmatched parent is treated as an unobserved `open`).
    pub fn cont(&mut self, parent: &wire::Input, output: BackingUtxo) -> Result<Outcome, Error> {
        let is_replace = self.drop_at(&output.input).is_some();

        let Some(loc) = self.locate(parent) else {
            let loc = self.place(Thread::new(output));
            return Ok(Outcome {
                is_focus: matches!(loc, Loc::Focus),
                is_replace,
                is_new: true,
                is_fork: false,
                is_reopen: false,
            });
        };

        let thread = self.thread_at(&loc);
        let is_reopen = thread.is_closed();
        let inplace = thread
            .cont(parent, output)?
            .expect("locate() confirmed parent is present");

        Ok(Outcome {
            is_focus: matches!(loc, Loc::Focus),
            is_replace,
            is_new: false,
            is_reopen,
            is_fork: matches!(inplace, thread::Inplace::Fork),
        })
    }

    /// Close the thread containing `parent` at `at`.
    /// `Ok(None)` if `parent` isn't found anywhere (lenient no-op).
    pub fn close(
        &mut self,
        parent: &wire::Input,
        at: wire::Point,
    ) -> Result<Option<Outcome>, Error> {
        let Some(loc) = self.locate(parent) else {
            return Ok(None);
        };
        let thread = self.thread_at(&loc);
        let inplace = thread
            .close(parent, at)?
            .expect("locate() confirmed parent is present");

        Ok(Some(Outcome {
            is_focus: matches!(loc, Loc::Focus),
            is_replace: false,
            is_new: false,
            is_reopen: false,
            is_fork: matches!(inplace, thread::Inplace::Fork),
        }))
    }

    fn locate(&self, input: &wire::Input) -> Option<Loc> {
        if self.focus.as_ref().is_some_and(|t| t.contains(input)) {
            return Some(Loc::Focus);
        }
        self.other
            .iter()
            .position(|t| t.contains(input))
            .map(Loc::Other)
    }

    fn thread_at(&mut self, loc: &Loc) -> &mut Thread {
        match loc {
            Loc::Focus => self
                .focus
                .as_mut()
                .expect("Loc::Focus implies focus is Some"),
            Loc::Other(i) => &mut self.other[*i],
        }
    }

    /// Place a fresh thread: focus if empty, else `other`. Returns where it went.
    fn place(&mut self, new: Thread) -> Loc {
        if self.focus.is_none() {
            self.focus = Some(new);
            Loc::Focus
        } else {
            self.other.push(new);
            Loc::Other(self.other.len() - 1)
        }
    }

    /// Truncate whichever thread (if any) contains id, dropping it and its descendants.
    /// Does not close the survivor. Returns where the truncation happened.
    fn drop_at(&mut self, id: &wire::Input) -> Option<Loc> {
        if let Some(t) = self.focus.take() {
            let (t, hit) = t.drop_at(id).to_tuple();
            self.focus = t;
            if hit {
                return Some(Loc::Focus);
            }
        }
        let i = self.other.iter().position(|t| t.contains(id))?;
        let (t, hit) = self.other.remove(i).drop_at(id).to_tuple();
        t.into_iter().for_each(|t| self.other.insert(i, t));
        hit.then_some(Loc::Other(i))
    }

    /// Drop utxos older than `at` across every thread. A thread whose entire
    /// history predates `at` vanishes; vanished `other` threads are dropped,
    /// vanished `focus` leaves `focus: None` (no promotion — see open issue #4).
    pub fn drop_before(&mut self, at: &wire::Point) {
        self.focus = self
            .focus
            .take()
            .and_then(|t| t.drop_before(at).to_tuple().0);
        self.other = std::mem::take(&mut self.other)
            .into_iter()
            .filter_map(|t| t.drop_before(at).to_tuple().0)
            .collect();
    }

    /// Like `drop_before`, but every thread keeps its origin. Never vanishes any thread.
    pub fn drop_before_except_origin(&mut self, at: &wire::Point) {
        if let Some(t) = self.focus.take() {
            self.focus = Some(
                t.drop_before_except_origin(at)
                    .to_tuple()
                    .0
                    .expect("never vanishes"),
            );
        }
        for t in &mut self.other {
            // in-place is fine here: never vanishes, so no take/put-back needed
            let owned = std::mem::replace(t, Thread::new(t.last().clone()));
            *t = owned
                .drop_before_except_origin(at)
                .to_tuple()
                .0
                .expect("never vanishes");
        }
    }

    /// Promote a thread from `other` to focus, identified by any utxo input it contains.
    /// Previous focus (if any) is demoted into `other`. No-op if `input` is already
    /// in focus. `None` if `input` isn't found anywhere — `self` is left unchanged.
    pub fn focus_on(&mut self, input: &wire::Input) -> Option<()> {
        if self.focus.as_ref().is_some_and(|t| t.contains(input)) {
            return Some(());
        }
        let idx = self.other.iter().position(|t| t.contains(input))?;
        let promoted = self.other.swap_remove(idx);
        if let Some(demoted) = self.focus.replace(promoted) {
            self.other.push(demoted);
        }
        Some(())
    }

    pub fn available(&self, receipt: &Receipt) -> u64 {
        let Some(focus) = &self.focus else {
            return 0;
        };
        // needs: focus's subbed/used utxos reconciled against `receipt`
        todo!()
    }

    /// Build the wire representation from `focus`. An empty `Backing`
    /// (no focus) reports all-empty — no recognized channel utxos.
    pub fn to_wire(&self, now_ms: u64, settle_ms: u64) -> wire::Backing {
        let cutoff = now_ms.saturating_sub(settle_ms);
        match &self.focus {
            Some(t) => t.to_wire(cutoff),
            None => wire::Backing {
                current: None,
                past: Vec::new(),
                pending: Vec::new(),
            },
        }
    }
}
