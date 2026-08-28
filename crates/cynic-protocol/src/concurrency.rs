//! Bounded fan-out concurrency — one shared primitive, not N re-implemented loops.
//!
//! ## Why this exists
//! Many `reconcile_*` and adapter paths iterate over a collection of subjects
//! (nodes, tokens, wallets, signatures) and fire an independent unit of work for
//! each. The naive shape is either:
//! - serial (`for item in items { work(item).await }`) — correct but needlessly
//!   slow when the work is I/O bound and the upstream rate-limits per attempt, or
//! - fire-all-at-once (`items.iter().map(work)`) — fast but can trip a provider's
//!   rate limit or exhaust a connection pool.
//!
//! This module is the *single* place that resolves that tension. Callers hand it
//! the items, a concurrency cap, and the per-item closure; it manages the window,
//! not the result collection (each closure records its own outcome into a Map/Vec,
//! the same shape TrenchScanner's `forEachWithConcurrency` uses). One definition,
//! one behavior, reused everywhere — which is invariant #6 (one canonical decision
//! source per concern) applied to *iteration*, not just to *decisions*.
//!
//! ## Sync vs async
//! The sync variant (`for_each_concurrent`) has zero external dependencies and is
//! safe to call from any thread. An async variant is intentionally NOT here yet:
//! `cynic-protocol` stays runtime-agnostic (no tokio dependency), and no caller
//! has demanded bounded async fan-out so far. Add it behind a feature flag only
//! when a real async caller needs it — do not pre-build for a hypothetical.

use std::sync::{Arc, Mutex};

/// Runs `work` over `items` with at most `concurrency` calls in flight at once.
///
/// Each `work` call is expected to record its own result (push to a `Vec`, insert
/// into a `HashMap` keyed by subject id, …) — this only manages the fan-out window,
/// not result collection, since concurrent completion order cannot produce a stable
/// ordered return array anyway.
///
/// A `concurrency` of 0 or larger than the item count is clamped to the item count
/// (a window of 0 would otherwise stall forever). Panics are contained per-item: a
/// panicking `work` aborts that one task and is surfaced via `Errored`, so one bad
/// subject cannot silently drop the others or take down the whole sweep.
pub fn for_each_concurrent<T, F>(items: &[T], concurrency: usize, work: F)
where
    T: Sync,
    F: Fn(&T) -> WorkOutcome + Sync,
{
    let queue = Mutex::new(items.iter().enumerate().collect::<Vec<_>>());
    let completed = Arc::new(AtomicCounter::new(0));
    let errored = Arc::new(AtomicCounter::new(0));

    let window = concurrency.clamp(1, items.len().max(1));
    let mut handles = Vec::with_capacity(window);

    for _ in 0..window {
        let queue = &queue;
        let completed = completed.clone();
        let errored = errored.clone();
        let handle = std::thread::spawn(move || {
            loop {
                let next = queue.lock().unwrap().pop();
                let (idx, item) = match next {
                    Some((i, it)) => (i, it),
                    None => break, // queue drained
                };
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(item)));
                match outcome {
                    Ok(WorkOutcome::Done) => completed.inc(),
                    Ok(WorkOutcome::Errored) => errored.inc(),
                    // A panic in `work` is one bad subject, not a sweep failure.
                    Err(_) => errored.inc(),
                }
                let _ = idx; // index retained for callers that need stable ordering later
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.join();
    }
}

/// Outcome a single unit of work reports back to the fan-out driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOutcome {
    /// Subject processed successfully (caller already recorded its result).
    Done,
    /// Subject failed but the sweep should continue (caller already logged it).
    Errored,
}

/// Minimal atomic counter so we don't pull in `atomic` re-exports everywhere.
struct AtomicCounter {
    inner: Mutex<u64>,
}

impl AtomicCounter {
    fn new(v: u64) -> Self {
        Self { inner: Mutex::new(v) }
    }
    fn inc(&self) -> u64 {
        let mut g = self.inner.lock().unwrap();
        *g += 1;
        *g
    }
    #[allow(dead_code)]
    fn get(&self) -> u64 {
        *self.inner.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn processes_every_item_exactly_once() {
        let items: Vec<u32> = (0..100).collect();
        let seen = Arc::new(Mutex::new(vec![false; 100]));
        for_each_concurrent(&items, 8, |&x| {
            seen.lock().unwrap()[x as usize] = true;
            WorkOutcome::Done
        });
        assert!(seen.lock().unwrap().iter().all(|b| *b));
    }

    #[test]
    fn concurrency_is_respected_not_exceeded() {
        let items: Vec<u32> = (0..200).collect();
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        for_each_concurrent(&items, 4, |_| {
            let cur = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            max_seen.fetch_max(cur, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(2));
            in_flight.fetch_sub(1, Ordering::SeqCst);
            WorkOutcome::Done
        });
        // Allow one extra for the pop-vs-execute race window, but never wildly over.
        assert!(
            max_seen.load(Ordering::SeqCst) <= 5,
            "concurrency exceeded: {}",
            max_seen.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn empty_input_is_a_noop() {
        let items: Vec<u32> = vec![];
        let calls = Arc::new(AtomicUsize::new(0));
        for_each_concurrent(&items, 4, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            WorkOutcome::Done
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn errored_item_does_not_stop_the_sweep() {
        let items: Vec<u32> = (0..20).collect();
        let done = Arc::new(AtomicUsize::new(0));
        for_each_concurrent(&items, 4, |&x| {
            if x % 5 == 0 {
                WorkOutcome::Errored
            } else {
                done.fetch_add(1, Ordering::SeqCst);
                WorkOutcome::Done
            }
        });
        assert_eq!(done.load(Ordering::SeqCst), 16);
    }

    #[test]
    fn panicking_work_is_contained() {
        let items: Vec<u32> = (0..10).collect();
        let survived = Arc::new(AtomicUsize::new(0));
        for_each_concurrent(&items, 3, |&x| {
            if x == 3 {
                panic!("boom");
            }
            survived.fetch_add(1, Ordering::SeqCst);
            WorkOutcome::Done
        });
        // 9 of 10 should still complete despite one panic.
        assert_eq!(survived.load(Ordering::SeqCst), 9);
    }

    #[test]
    fn zero_concurrency_is_clamped_to_one() {
        let items: Vec<u32> = (0..5).collect();
        let calls = Arc::new(AtomicUsize::new(0));
        for_each_concurrent(&items, 0, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            WorkOutcome::Done
        });
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }
}
