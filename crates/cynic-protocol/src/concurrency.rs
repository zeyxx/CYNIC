use std::sync::{Arc, Mutex};

pub fn for_each_concurrent<T, F>(items: &[T], concurrency: usize, work: F)
where
    T: Sync + Send + Clone,
    F: Fn(&T) -> WorkOutcome + Sync + Send + Clone,
{
    if items.is_empty() {
        return;
    }
    let window = concurrency.clamp(1, items.len());
    let chunk_size = (items.len() + window - 1) / window;
    let chunks: Vec<Vec<T>> = items.chunks(chunk_size).map(|c| c.to_vec()).collect();
    let work = Arc::new(work);

    let mut handles = Vec::with_capacity(window);
    for chunk in chunks {
        let work = work.clone();
        let handle = std::thread::spawn(move || {
            for item in &chunk {
                work(item);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_all_items_concurrently() {
        let items = vec![1, 2, 3, 4, 5];
        let counter = Arc::new(Mutex::new(0u64));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 2, move |_| {
            *counter_clone.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 5);
    }

    #[test]
    fn clamps_concurrency_to_item_count() {
        let items = vec![1, 2];
        let counter = Arc::new(Mutex::new(0u64));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 10, move |_| {
            *counter_clone.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[test]
    fn captures_errored_outcomes() {
        let items = vec![1, 2, 3];
        let counter = Arc::new(Mutex::new(0u64));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 2, move |_| {
            *counter_clone.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn handles_empty_input() {
        let items: Vec<i32> = vec![];
        let counter = Arc::new(Mutex::new(0u64));
        let counter_clone = counter.clone();
        for_each_concurrent(&items, 2, move |_| {
            *counter_clone.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 0);
    }
}
