//! ## Threads
//!
//! Threads encapsulate a channel's on-chain state in time.
//!
//! A thread's promise: if there is a rollback, then the parent may be at tip instead of the child.
//!
//! Threads are created with a `new`.
//! Threads are modified by either:
//!
//! - a step: continuation of `Opened` or a close.
//! - a rollback: dropping all children from some point.
//!
//! The design is lenient on input.
//! It does not assume all events are observed.
//! It assumes that the most recent data is most correct,
//! and does a best guess to accommodate this.
//!
//! For examples:
//!
//! - a `cont` where the input is element that is not `last` results in fork, its previous children discarded.
//! - a `drop_at` may result in no thread
//!
//! Modifiers return outcomes indicating whether or not anything happend.

use konduit_wire::auth::state::{self as wire};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Inplace modificiation
pub enum Inplace {
    Extend,
    Fork,
}

/// Consuming modification
pub enum Consumed {
    /// No change
    Unchanged(Thread),
    /// Change, and no thread remains, aka vanished. That is, dropped from origin.
    Vanished,
    /// Changed, with some thread remaining.
    Dropped(Thread),
}

impl Consumed {
    /// (replacement-or-none, changed?)
    pub fn to_tuple(self) -> (Option<Thread>, bool) {
        match self {
            Consumed::Unchanged(t) => (Some(t), false),
            Consumed::Vanished => (None, true),
            Consumed::Dropped(t) => (Some(t), true),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Time must go forward")]
    Time,
    #[error("Inputs must be unique")]
    Unique,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum Thread {
    #[n(0)]
    Open {
        #[n(0)]
        inner: Inner,
    },
    #[n(1)]
    Closed {
        #[n(0)]
        inner: Inner,
        #[n(1)]
        at: wire::Point,
    },
}

impl Thread {
    pub fn new(origin: wire::BackingUtxo) -> Self {
        Thread::Open {
            inner: Inner::new(origin),
        }
    }

    fn inner(&self) -> &Inner {
        match self {
            Thread::Open { inner } => inner,
            Thread::Closed { inner, .. } => inner,
        }
    }

    fn inner_mut(&mut self) -> &mut Inner {
        match self {
            Thread::Open { inner } => inner,
            Thread::Closed { inner, .. } => inner,
        }
    }

    pub fn len(&self) -> usize {
        self.inner().len()
    }

    pub fn contains(&self, input: &wire::Input) -> bool {
        self.inner().position(input).is_some()
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Thread::Closed { .. })
    }

    pub fn get(&self, pos: usize) -> Option<&wire::BackingUtxo> {
        self.inner().get(pos)
    }

    pub fn first(&self) -> &wire::BackingUtxo {
        self.inner().first()
    }

    pub fn last(&self) -> &wire::BackingUtxo {
        self.inner().last()
    }

    /// Extend or fork the thread. If the thread was closed, it is reopened
    /// (a `cont` on a closed thread is treated as rollback + extend).
    pub fn cont(
        &mut self,
        parent: &wire::Input,
        output: wire::BackingUtxo,
    ) -> Result<Option<Inplace>, Error> {
        let Some(outcome) = self.inner_mut().cont(parent, output)? else {
            return Ok(None);
        };
        if let Thread::Closed { inner, .. } = self {
            *self = Thread::Open {
                inner: Inner(std::mem::take(&mut inner.0)),
            };
        }
        Ok(Some(outcome))
    }

    /// Close the thread at `parent` with timestamp `at`.
    /// Descendants of `parent` are dropped (`Fork`) or none existed (`Extend`).
    /// Re-closing an already-closed thread overwrites `at`.
    pub fn close(
        &mut self,
        parent: &wire::Input,
        at: wire::Point,
    ) -> Result<Option<Inplace>, Error> {
        let Some(pos) = self.inner().position(parent) else {
            return Ok(None);
        };
        if at.posix < self.inner().get(pos).unwrap().created_at.posix {
            return Err(Error::Time);
        }

        let dropped = self.inner_mut().drop_after(parent).expect("checked above");
        let outcome = if dropped {
            Inplace::Fork
        } else {
            Inplace::Extend
        };

        let placeholder = Thread::Open {
            inner: Inner(vec![]),
        };
        let taken = std::mem::replace(self, placeholder);
        let inner = match taken {
            Thread::Open { inner } => inner,
            Thread::Closed { inner, .. } => inner,
        };
        *self = Thread::Closed { inner, at };
        Ok(Some(outcome))
    }

    /// Truncate at `input` (dropping it and descendants).
    pub fn drop_at(mut self, input: &wire::Input) -> Consumed {
        match self.inner().position(input) {
            None => Consumed::Unchanged(self),
            Some(0) => Consumed::Vanished,
            Some(pos) => {
                self.inner_mut().0.truncate(pos);
                Consumed::Dropped(self)
            }
        }
    }

    /// Drop utxos older than `at`. Vanishes if even the tip predates `at`.
    /// Does not touch Open/Closed — acts purely on `inner`.
    pub fn drop_before(mut self, at: &wire::Point) -> Consumed {
        if self.inner().last().created_at.posix < at.posix {
            return Consumed::Vanished;
        }
        if self.inner_mut().drop_before(at) {
            Consumed::Dropped(self)
        } else {
            Consumed::Unchanged(self)
        }
    }

    /// Drop utxos older than `at`, keeping the origin regardless. Never vanishes.
    /// Does not touch Open/Closed — acts purely on `inner`.
    pub fn drop_before_except_origin(mut self, at: &wire::Point) -> Consumed {
        if self.inner_mut().drop_before_except_origin(at) {
            Consumed::Dropped(self)
        } else {
            Consumed::Unchanged(self)
        }
    }

    /// See `Inner::to_wire`.
    /// FIXME :: We are not disclosing open/closeness!
    pub fn to_wire(&self, cutoff: u64) -> wire::Backing {
        self.inner().to_wire(cutoff)
    }
}

/// Maintains a non-empty vec of backing utxos
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
struct Inner(#[n(0)] Vec<wire::BackingUtxo>);

impl Inner {
    // -- Constructor --

    pub fn new(origin: wire::BackingUtxo) -> Self {
        Self(vec![origin])
    }

    // -- Properties --

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn position(&self, input: &wire::Input) -> Option<usize> {
        self.0.iter().position(|u| &u.input == input)
    }

    // -- Getters --

    pub fn get(&self, pos: usize) -> Option<&wire::BackingUtxo> {
        self.0.get(pos)
    }

    /// Non-empty vec must have first.
    /// If no dropping, then this is first.
    pub fn first(&self) -> &wire::BackingUtxo {
        self.0.first().unwrap()
    }

    /// Non-empty vec must have last.
    /// This ought to be utxo at tip.
    pub fn last(&self) -> &wire::BackingUtxo {
        self.0.last().unwrap()
    }

    // -- Modifiers --

    /// Append `output` after `parent`. Truncates descendants if parent is interior.
    /// `None` if parent absent. Errors on time-reversal or non-unique output.
    pub fn cont(
        &mut self,
        parent: &wire::Input,
        output: wire::BackingUtxo,
    ) -> Result<Option<Inplace>, Error> {
        let Some(pos) = self.position(parent) else {
            return Ok(None);
        };
        if output.created_at.posix < self.0[pos].created_at.posix {
            return Err(Error::Time);
        }
        if self.position(&output.input).is_some() {
            return Err(Error::Unique);
        }
        let is_fork = pos + 1 < self.0.len();
        self.0.truncate(pos + 1);
        self.0.push(output);
        Ok(if is_fork {
            Some(Inplace::Fork)
        } else {
            Some(Inplace::Extend)
        })
    }

    /// Drop everything strictly after `parent`. `None` if parent absent,
    /// `Some(true)` if descendants dropped, `Some(false)` if parent was already tip.
    pub fn drop_after(&mut self, parent: &wire::Input) -> Option<bool> {
        let Some(pos) = self.position(parent) else {
            return None;
        };
        if pos + 1 == self.0.len() {
            Some(false)
        } else {
            self.0.truncate(pos + 1);
            Some(true)
        }
    }
    /// Drop all utxos strictly older than `at`. Returns whether anything was dropped.
    /// Assumes entries are chronological (holds as long as they only ever arrive via `cont`).
    pub fn drop_before(&mut self, at: &wire::Point) -> bool {
        let cut = self.0.partition_point(|u| u.created_at.posix < at.posix);
        if cut == 0 {
            return false;
        }
        self.0.drain(0..cut);
        true
    }

    /// Same, but index 0 (the origin) is never touched.
    pub fn drop_before_except_origin(&mut self, at: &wire::Point) -> bool {
        if self.0.len() <= 1 {
            return false;
        }
        let cut = 1 + self.0[1..].partition_point(|u| u.created_at.posix < at.posix);
        if cut <= 1 {
            return false;
        }
        self.0.drain(1..cut);
        true
    }

    /// Split by settlement: entries with `created_at.posix < cutoff` are settled;
    /// the latest settled entry is `current`, older settled entries are `past`,
    /// everything at or after `cutoff` is `pending`.
    pub fn to_wire(&self, cutoff: u64) -> wire::Backing {
        let split = self.0.partition_point(|u| u.created_at.posix < cutoff);
        let (settled, pending) = self.0.split_at(split);
        let (past, current) = match settled.split_last() {
            Some((last, rest)) => (rest.to_vec(), Some(last.clone())),
            None => (Vec::new(), None),
        };
        wire::Backing {
            current,
            past,
            pending: pending.to_vec(),
        }
    }
}
