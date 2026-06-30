//! It is still unclear what the index story will be.
//! Here we cache something that crudely fits our requirements.

use konduit_wire::auth::state::{self as wire, BackingUtxo, Point};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// The backing needs to carry enough state to:
/// - handle the `./auth/state` request, that is convert to the wire::Backing,
/// - handle thread switching if the user queries with alt utxo
/// - handle pay requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
pub struct Backing {
    /// If only one thread, then this is the focus.
    #[n(0)]
    focus: Thread,
    /// If more than one, then the query param may contain utxo that can be used to select a different thread as focus.
    #[n(1)]
    other: Vec<Thread>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
pub struct Thread {
    /// This is prunable
    #[n(0)]
    thread: Vec<wire::BackingUtxo>,
    /// If set, it indicates that the thread is closed.
    /// Usually this means it literally has been stepped with a `Close` step.
    /// However, it will include any spend not resulting in a continuing output.
    /// The thread should not be used for backing payments.
    #[n(1)]
    closed_at: Option<wire::Point>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("no thread matches the given input")]
    UnknownThread,
    #[error("thread is closed")]
    ThreadClosed,
}

/// Outcome of a `cont`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContOutcome {
    /// Cleanly extended the matched thread at its tip.
    Extended,
    /// Matched thread had descendants past the spent input; those descendants were
    /// split off into a new closed thread in `other`. This is the case where someone
    /// (legitimately or otherwise) submitted a competing branch from a shared ancestor
    /// — worth surfacing to the handler for logging/alerting.
    ExtendedWithFork,
    /// No existing thread contained the spent input; started a new thread in `other`.
    NewThread,
}

impl Backing {
    pub fn new() -> Self {
        Self::default()
    }

    /// FIXME :: this is actually a more involved question.
    /// The `subbed` and `useds` fields together with the receipt
    /// are required to assess what funds are legitimately available.
    pub fn available(&self) -> u64 {
        todo!()
    }

    /// Extend the thread whose history contains `input`, or start a new thread in `other`.
    /// If `input` matches an interior utxo, the descendants are split off as a new closed
    /// thread in `other` and the matched thread is extended with `output`.
    pub fn cont(&mut self, input: wire::Input, output: BackingUtxo) -> Result<ContOutcome, Error> {
        let Some(thread) = self.find_thread_mut(&input) else {
            let mut new = Thread::default();
            new.push(output);
            self.other.push(new);
            return Ok(ContOutcome::NewThread);
        };

        if thread.is_closed() {
            return Err(Error::ThreadClosed);
        }

        match thread.split_at_input(&input, output) {
            Some(SplitOutcome::Extended) => Ok(ContOutcome::Extended),
            Some(SplitOutcome::Forked(orphan)) => {
                self.other.push(orphan);
                Ok(ContOutcome::ExtendedWithFork)
            }
            None => Err(Error::UnknownThread),
        }
    }

    /// Mark the thread containing `input` as closed at `at`.
    pub fn close(&mut self, input: wire::Input, at: wire::Point) -> Result<(), Error> {
        let thread = self.find_thread_mut(&input).ok_or(Error::UnknownThread)?;
        if thread.is_closed() {
            return Err(Error::ThreadClosed);
        }
        thread.close(at);
        Ok(())
    }

    /// Drop utxos older than `prune_before` across all threads.
    /// Closed threads prune fully; open threads always keep their tip.
    /// Open threads that empty are dropped from `other`; closed threads are retained as record.
    pub fn prune_history(&mut self, prune_before: Point) -> Result<(), Error> {
        self.focus.prune_history_before(&prune_before);
        for t in &mut self.other {
            t.prune_history_before(&prune_before);
        }
        self.other.retain(|t| !t.is_empty() || t.is_closed());
        Ok(())
    }

    /// Prune but keep origin and tip.
    /// There is no indication that middle is lost.
    /// Nor commitment to user that the thread is complete.
    /// It may be helpful to atleast keep the origin (open) utxo.
    pub fn prune_history_middle(&mut self, prune_before: Point) -> Result<(), Error> {
        self.focus.prune_history_middle_before(&prune_before);
        for t in &mut self.other {
            t.prune_history_middle_before(&prune_before);
        }
        Ok(())
    }

    /// Promote a thread from `other` to focus, identified by any utxo input it contains.
    /// Previous focus is moved into `other`.
    /// Callers may treat `Err(UnknownThread)` as a no-op (the query param is optional).
    pub fn switch_focus(&mut self, input: &wire::Input) -> Result<(), Error> {
        let Some(idx) = self.other.iter().position(|t| t.contains(input)) else {
            return Err(Error::UnknownThread);
        };
        let promoted = self.other.swap_remove(idx);
        let demoted = std::mem::replace(&mut self.focus, promoted);
        if !demoted.is_empty() {
            self.other.push(demoted);
        }
        Ok(())
    }

    /// Build the wire representation from the focus thread.
    /// Splits by settlement cutoff: latest settled → `current`,
    /// older settled → `past`, anything newer than cutoff → `pending`.
    pub fn to_wire(&self, now_ms: u64, settle_ms: u64) -> wire::Backing {
        let cutoff = now_ms.saturating_sub(settle_ms);
        self.focus.to_wire(cutoff)
    }

    /// Find the thread (focus or other) containing `input` anywhere in its history.
    fn find_thread_mut(&mut self, input: &wire::Input) -> Option<&mut Thread> {
        if self.focus.contains(input) {
            return Some(&mut self.focus);
        }
        self.other.iter_mut().find(|t| t.contains(input))
    }
}

enum SplitOutcome {
    Extended,
    Forked(Thread),
}

impl Thread {
    pub fn available(&self) -> u64 {
        if self.is_closed() {
            return 0;
        }
        self.thread
            .iter()
            .map(|u| u.amount.saturating_sub(u.subbed))
            .sum()
    }

    pub fn push(&mut self, utxo: wire::BackingUtxo) {
        self.thread.push(utxo);
    }

    pub fn is_empty(&self) -> bool {
        self.thread.is_empty()
    }

    pub fn len(&self) -> usize {
        self.thread.len()
    }

    pub fn is_closed(&self) -> bool {
        self.closed_at.is_some()
    }

    pub fn close(&mut self, at: wire::Point) {
        if self.closed_at.is_none() {
            self.closed_at = Some(at);
        }
    }

    pub fn contains(&self, input: &wire::Input) -> bool {
        self.thread.iter().any(|u| &u.input == input)
    }

    /// Push `output`, treating `input` as the parent it spends.
    /// Returns `None` if `input` is not in this thread.
    /// If `input` is the tip: simple extend.
    /// If `input` is interior: utxos after `input` are orphaned into a closed thread.
    fn split_at_input(&mut self, input: &wire::Input, output: BackingUtxo) -> Option<SplitOutcome> {
        let pos = self.thread.iter().position(|u| &u.input == input)?;

        let tip_idx = self.thread.len() - 1;
        if pos == tip_idx {
            self.thread.push(output);
            return Some(SplitOutcome::Extended);
        }

        let orphan_utxos: Vec<_> = self.thread.drain(pos + 1..).collect();
        let orphan = Thread {
            closed_at: Some(output.created_at.clone()),
            thread: orphan_utxos,
        };
        self.thread.push(output);
        Some(SplitOutcome::Forked(orphan))
    }

    /// Drop utxos created before `cutoff`.
    /// Open threads always keep the tip regardless of its age.
    pub fn prune_history_before(&mut self, cutoff: &wire::Point) {
        let closed = self.is_closed();
        let cutoff_posix = cutoff.posix;
        let tip_idx = self.thread.len().saturating_sub(1);

        // `Vec::retain` visits elements in order, so enumerate's index is stable.
        self.thread = self
            .thread
            .drain(..)
            .enumerate()
            .filter(|(i, u)| {
                let keep_age = u.created_at.posix >= cutoff_posix;
                let keep_tip = !closed && *i == tip_idx;
                keep_age || keep_tip
            })
            .map(|(_, u)| u)
            .collect();
    }

    /// Drop interior utxos created before `cutoff`; keep origin (index 0) and tip.
    pub fn prune_history_middle_before(&mut self, cutoff: &wire::Point) {
        if self.thread.len() <= 2 {
            return;
        }
        let cutoff_posix = cutoff.posix;
        let last_idx = self.thread.len() - 1;

        self.thread = self
            .thread
            .drain(..)
            .enumerate()
            .filter(|(i, u)| *i == 0 || *i == last_idx || u.created_at.posix >= cutoff_posix)
            .map(|(_, u)| u)
            .collect();
    }

    /// Split into wire::Backing using a settlement cutoff (posix ms).
    fn to_wire(&self, cutoff: u64) -> wire::Backing {
        let (settled, pending): (Vec<_>, Vec<_>) = self
            .thread
            .iter()
            .cloned()
            .partition(|u| u.created_at.posix <= cutoff);

        let mut settled = settled;
        let current = settled.pop();
        wire::Backing {
            current,
            past: settled,
            pending,
        }
    }
}
