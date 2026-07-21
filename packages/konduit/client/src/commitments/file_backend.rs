//! file_backend.rs — Human-readable JSONL (newline-delimited JSON)
//! file backend, native only.
//!
//! One JSON object per line, no wrapping array — deliberately, since
//! `commit` is append-only (a matching re-commit is a no-op, a fresh
//! lock is one appended line), so `commit` appends a single line
//! rather than rewriting the whole file. Only `sweep_before` rewrites
//! the file wholesale, matching the actual usage pattern: commits
//! happen every few minutes, sweeps roughly every two weeks.
//!
//! Not fast, not safe across multiple processes sharing the file (no
//! file locking) — for dev use, single user cli driven, not production scale.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use konduit_data::{Duration, Lock, Tag};
use serde::{Deserialize, Serialize};

use super::{Backend, Commitment, Error};

fn system_now() -> Duration {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    Duration::from_millis(millis)
}

#[derive(Serialize, Deserialize)]
struct Entry {
    lock: Lock,
    commitment: Commitment,
}

pub struct FileBackend {
    path: PathBuf,
    guard: Mutex<()>,
    now: Box<dyn Fn() -> Duration>,
}

impl FileBackend {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, Error> {
        Self::open_with_clock(path, system_now)
    }

    pub fn open_with_clock(
        path: impl Into<PathBuf>,
        now: impl Fn() -> Duration + 'static,
    ) -> Result<Self, Error> {
        let path = path.into();
        if !path.exists() {
            fs::write(&path, b"").map_err(|e| Error::Backend(e.to_string()))?;
        }
        Ok(Self {
            path,
            guard: Mutex::new(()),
            now: Box::new(now),
        })
    }

    fn read_all(&self) -> Result<Vec<Entry>, Error> {
        let text = fs::read_to_string(&self.path).map_err(|e| Error::Backend(e.to_string()))?;
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(|e| Error::Backend(e.to_string())))
            .collect()
    }

    fn append_line(&self, entry: &Entry) -> Result<(), Error> {
        let json = serde_json::to_string(entry).map_err(|e| Error::Backend(e.to_string()))?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|e| Error::Backend(e.to_string()))?;
        writeln!(file, "{json}").map_err(|e| Error::Backend(e.to_string()))
    }

    fn rewrite_all(&self, entries: &[Entry]) -> Result<(), Error> {
        let mut out = String::new();
        for entry in entries {
            let json = serde_json::to_string(entry).map_err(|e| Error::Backend(e.to_string()))?;
            out.push_str(&json);
            out.push('\n');
        }
        fs::write(&self.path, out).map_err(|e| Error::Backend(e.to_string()))
    }
}

#[async_trait::async_trait(?Send)]
impl Backend for FileBackend {
    async fn commit(&self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error> {
        let _guard = self
            .guard
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        let entries = self.read_all()?;

        match entries.iter().find(|e| e.lock == lock) {
            Some(entry) if entry.commitment.tag() == &tag && entry.commitment.index() == index => {
                Ok(())
            }
            Some(_) => Err(Error::Conflict),
            None => self.append_line(&Entry {
                lock,
                commitment: Commitment::new(tag, index, (self.now)()),
            }),
        }
    }

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        let _guard = self
            .guard
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        Ok(self
            .read_all()?
            .into_iter()
            .find(|e| &e.lock == lock)
            .map(|e| e.commitment))
    }

    async fn sweep_before(&self, threshold: Duration) -> Result<u64, Error> {
        let _guard = self
            .guard
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        let entries = self.read_all()?;
        let before = entries.len();
        let kept: Vec<Entry> = entries
            .into_iter()
            .filter(|e| *e.commitment.at() > threshold)
            .collect();
        let removed = (before - kept.len()) as u64;
        self.rewrite_all(&kept)?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use konduit_data::Duration;

    use super::FileBackend;
    use crate::commitments::backend_test_suite as suite;

    static TEST_FILE_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_backend(clock: impl Fn() -> Duration + 'static) -> FileBackend {
        let n = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("konduit-commitments-test-{pid}-{n}.jsonl"));
        FileBackend::open_with_clock(path, clock).unwrap()
    }

    #[tokio::test]
    async fn fresh_insert_succeeds() {
        suite::fresh_insert_succeeds(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn conflicting_target_is_rejected() {
        suite::conflicting_target_is_rejected(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn sweep_on_empty_store_is_noop() {
        suite::sweep_on_empty_store_is_noop(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn sweep_removes_only_old_entries() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::sweep_removes_only_old_entries(&backend, &clock).await;
    }

    #[tokio::test]
    async fn sweep_boundary_is_inclusive() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::sweep_boundary_is_inclusive(&backend, &clock).await;
    }

    #[tokio::test]
    async fn same_target_recommit_is_noop() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::same_target_recommit_is_noop(&backend, &clock).await;
    }

    #[tokio::test]
    async fn recommit_does_not_refresh_at() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::recommit_does_not_refresh_at(&backend, &clock).await;
    }
}
