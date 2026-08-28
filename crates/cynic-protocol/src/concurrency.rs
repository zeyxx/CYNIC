use std::sync::{Arc, Mutex};

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
                let (_idx, item) = match next {
                    Some((i, it)) => (i, it),
                    None => break,
                };
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(item)));
                match outcome {
                    Ok(WorkOutcome::Done) => {
                        completed.inc();
                    }
                    Ok(WorkOutcome::Errored) | Err(_) => {
                        errored.inc();
                    }
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOutcome {
    Done,
    Errored,
}

struct AtomicCounter {
    inner: Mutex<u64>,
}

impl AtomicCounter {
    fn new(v: u64) -> Self {
        Self {
            inner: Mutex::new(v),
        }
    }
    fn inc(&self) -> u64 {
        let mut g = self.inner.lock().unwrap();
        *g += 1;
        *g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_all_items_concurrently() {
        let items = vec![1, 2, 3, 4, 5];
        let counter = Arc::new(AtomicCounter::new(0));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 2, move |_| {
            counter_clone.inc();
            WorkOutcome::Done
        });
        assert_eq!(counter.value(), 5);
    }

    #[test]
    fn clamps_concurrency_to_item_count() {
        let items = vec![1, 2];
        let counter = Arc::new(AtomicCounter::new(0));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 10, move |_| {
            counter_clone.inc();
            WorkOutcome::Done
        });
        assert_eq!(counter.value(), 2);
    }

    #[test]
    fn captures_errored_outcomes() {
        let items = vec![1, 2, 3];
        let counter = Arc::new(AtomicCounter::new(0));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 2, move |item| {
            if *item == 2 {
                counter_clone.inc();
                WorkOutcome::Errored
            } else {
                counter_clone.inc();
                WorkOutcome::Done
            }
        });
        assert_eq!(counter.value(), 3);
    }
}

impl AtomicCounter {
    #[allow(dead_code)]
    fn value(&self) -> u64 {
        *self.inner.lock().unwrap()
    }
}
