use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};

/// How many entries a label keeps (as Tera Term's default scroll buffer).
pub const MAX_ENTRIES_PER_LABEL: usize = 10_000;

/// A label's entries, oldest first, at most [`MAX_ENTRIES_PER_LABEL`]. Its id and
/// drop count let a `SourceCursor` tell new entries from a replaced queue.
#[derive(Clone, Debug, PartialEq)]
pub struct Queue<T> {
    id: u64,
    dropped: u64,
    entries: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            dropped: 0,
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, entry: T) {
        self.push_within(entry, MAX_ENTRIES_PER_LABEL);
    }

    pub(crate) fn push_within(&mut self, entry: T, max_entries: usize) {
        self.entries.push_back(entry);
        while self.entries.len() > max_entries {
            self.entries.pop_front();
            self.dropped += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn back(&self) -> Option<&T> {
        self.entries.back()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter()
    }

    /// Unique among queues, clones aside.
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// How many entries were ever pushed.
    pub fn end(&self) -> u64 {
        self.dropped + self.entries.len() as u64
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FromIterator<T> for Queue<T> {
    fn from_iter<I: IntoIterator<Item = T>>(entries: I) -> Self {
        let mut queue = Self::new();
        queue.entries.extend(entries);
        queue
    }
}

#[cfg(test)]
mod tests {
    use super::Queue;

    #[test]
    fn drops_the_oldest_past_the_limit() {
        let mut queue = Queue::new();
        for entry in 0..5 {
            queue.push_within(entry, 3);
        }
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(queue.dropped(), 2);
        assert_eq!(queue.end(), 5);
    }

    #[test]
    fn each_queue_has_its_own_id() {
        let queue = Queue::<i32>::new();
        assert_ne!(queue.id(), Queue::<i32>::new().id());
        assert_eq!(queue.id(), queue.clone().id());
    }
}
