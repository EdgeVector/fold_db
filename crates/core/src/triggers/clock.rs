//! Clock abstraction for the TriggerRunner.
//!
//! The runner treats wall-clock time as an injected service so tests can
//! drive the scheduler deterministically with `MockClock`, without real
//! `tokio::time::sleep` calls. `SystemClock` is the production impl.

use std::collections::BinaryHeap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use std::cmp::Reverse;
use std::future::Future;
use tokio::sync::Notify;

/// Wall-clock time in milliseconds since the Unix epoch, plus a `sleep`
/// primitive. `sleep` is modeled as `Future` instead of `async fn` so the
/// trait is object-safe via `Arc<dyn Clock>`.
pub trait Clock: Send + Sync + 'static {
    fn now_ms(&self) -> i64;

    /// Sleep for at least `ms` milliseconds of simulated time. Returns a
    /// boxed future; object-safe for `Arc<dyn Clock>`.
    fn sleep<'a>(&'a self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn sleep<'a>(&'a self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(tokio::time::sleep(std::time::Duration::from_millis(ms)))
    }
}

/// Deterministic clock for tests. `now_ms` and pending sleeps advance only
/// when `advance(ms)` is called. Sleeps whose deadline has passed after an
/// `advance` are woken via a `tokio::sync::Notify`.
pub struct MockClock {
    inner: Arc<MockClockInner>,
}

struct MockClockInner {
    state: Mutex<MockState>,
}

struct MockState {
    now_ms: i64,
    pending: BinaryHeap<Reverse<Sleeper>>,
    next_id: u64,
}

struct Sleeper {
    deadline: i64,
    id: u64,
    notify: Arc<Notify>,
}

impl PartialEq for Sleeper {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.id == other.id
    }
}
impl Eq for Sleeper {}
impl PartialOrd for Sleeper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Sleeper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deadline
            .cmp(&other.deadline)
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl MockClock {
    pub fn new(start_ms: i64) -> Self {
        Self {
            inner: Arc::new(MockClockInner {
                state: Mutex::new(MockState {
                    now_ms: start_ms,
                    pending: BinaryHeap::new(),
                    next_id: 0,
                }),
            }),
        }
    }

    /// Advance the clock by `ms` milliseconds and wake any sleepers whose
    /// deadline is at or before the new now. Multiple woken sleepers get
    /// notified in deadline order.
    pub fn advance(&self, ms: u64) {
        let to_wake = {
            let mut state = self.inner.state.lock().unwrap();
            state.now_ms += ms as i64;
            let mut wake = Vec::new();
            while let Some(Reverse(sleeper)) = state.pending.peek() {
                if sleeper.deadline <= state.now_ms {
                    let Reverse(s) = state.pending.pop().unwrap();
                    wake.push(s.notify.clone());
                } else {
                    break;
                }
            }
            wake
        };
        for n in to_wake {
            n.notify_one();
        }
    }

    /// Number of sleepers currently queued (useful in tests to assert the
    /// runner actually parked a fire waiting on time).
    pub fn pending_sleeps(&self) -> usize {
        self.inner.state.lock().unwrap().pending.len()
    }
}

impl Clock for MockClock {
    fn now_ms(&self) -> i64 {
        self.inner.state.lock().unwrap().now_ms
    }

    fn sleep<'a>(&'a self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        let notify = Arc::new(Notify::new());
        let (deadline, immediate) = {
            let mut state = self.inner.state.lock().unwrap();
            let deadline = state.now_ms + ms as i64;
            if deadline <= state.now_ms {
                (deadline, true)
            } else {
                let id = state.next_id;
                state.next_id += 1;
                state.pending.push(Reverse(Sleeper {
                    deadline,
                    id,
                    notify: Arc::clone(&notify),
                }));
                (deadline, false)
            }
        };

        if immediate {
            Box::pin(async move {})
        } else {
            Box::pin(async move {
                notify.notified().await;
                let _ = deadline;
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test(flavor = "current_thread")]
    async fn mock_clock_zero_sleep_is_immediate() {
        let clock = MockClock::new(0);
        let start = clock.now_ms();
        clock.sleep(0).await;
        assert_eq!(clock.now_ms(), start);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mock_clock_advance_wakes_sleeper() {
        let clock = Arc::new(MockClock::new(0));
        let c = Arc::clone(&clock);
        // lint:spawn-bare-ok cfg(test) scaffolding — no parent request span to propagate.
        let handle = tokio::spawn(async move {
            c.sleep(100).await;
            c.now_ms()
        });

        // Poll until the sleeper is registered. Can't rely on a fixed
        // tokio::time sleep since MockClock is not wired to tokio's timer.
        for _ in 0..100 {
            if clock.pending_sleeps() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(clock.pending_sleeps(), 1);

        clock.advance(100);
        let ts = handle.await.unwrap();
        assert_eq!(ts, 100);
    }

    #[test]
    fn system_clock_now_ms_is_positive() {
        let c = SystemClock::new();
        assert!(c.now_ms() > 1_600_000_000_000);
    }
}
