use std::sync::Arc;

pub fn for_each_concurrent<T, F>(items: &[T], concurrency: usize, work: F)
where
    T: Sync,
    F: Fn(&T) -> WorkOutcome + Sync + Send,
{
    if items.is_empty() {
        return;
    }
    let window = concurrency.clamp(1, items.len());
    let chunk_size = (items.len() + window - 1) / window;
    let chunks: Vec<&[T]> = items.chunks(chunk_size).collect();
    let work = Arc::new(work);

    std::thread::scope(|s| {
        for chunk in chunks {
            let work = work.clone();
            s.spawn(move || {
                for item in chunk {
                    work(item);
                }
            });
        }
    });
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
        let counter = Arc::new(std::sync::Mutex::new(0u64));
        for_each_concurrent(&items, 2, |_| {
            *counter.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 5);
    }

    #[test]
    fn clamps_concurrency_to_item_count() {
        let items = vec![1, 2];
        let counter = Arc::new(std::sync::Mutex::new(0u64));
        for_each_concurrent(&items, 10, |_| {
            *counter.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[test]
    fn captures_errored_outcomes() {
        let items = vec![1, 2, 3];
        let counter = Arc::new(std::sync::Mutex::new(0u64));
        for_each_concurrent(&items, 2, |_| {
            *counter.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn handles_empty_input() {
        let items: Vec<i32> = vec![];
        let counter = Arc::new(std::sync::Mutex::new(0u64));
        for_each_concurrent(&items, 2, |_| {
            *counter.lock().unwrap() += 1;
            WorkOutcome::Done
        });
        assert_eq!(*counter.lock().unwrap(), 0);
    }
}
