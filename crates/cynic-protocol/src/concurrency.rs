use std::sync::{Arc, Mutex};

pub fn for_each_concurrent<T, F>(items: &[T], concurrency: usize, work: F)
where
    T: Sync + Send,
    F: Fn(&T) -> WorkOutcome + Sync + Send,
{
    let _ = concurrency;
    let counter = Arc::new(Mutex::new(0u64));
    for item in items {
        let c = counter.clone();
        let _guard = c.lock().unwrap();
        let _ = work(item);
        *guard += 1;
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
    fn processes_all_items() {
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
