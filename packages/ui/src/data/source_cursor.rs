use crate::data::{DataContext, DataEntry, Queue};

/// What a [`SourceCursor`] read.
#[derive(Clone, Debug, PartialEq)]
pub struct NewEntries<T> {
    /// Entries the reader does not have yet.
    pub entries: Vec<T>,
    /// The read started over from the front of the queue, so the reader
    /// should drop what it built from earlier reads: on the first read, after
    /// [`SourceCursor::reset`] or a label change, or when the queue was
    /// cleared or replaced since.
    pub restarted: bool,
    /// How many entries were dropped from the queue before this reader got to
    /// them, so it never saw them: it fell more than `MAX_ENTRIES_PER_LABEL`
    /// behind. They come before `entries`.
    pub missed: u64,
}

/// Where the last read ended, in which queue.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    queue: u64,
    end: u64,
}

/// Tracks how far a reader has read a queue in `DataContext`, so each read
/// only returns what was added since the previous one.
#[derive(Debug, Default)]
pub struct SourceCursor {
    label: String,
    read: Option<Position>,
}

impl SourceCursor {
    /// Starts over from the front of the queue on the next read.
    pub fn reset(&mut self) {
        self.read = None;
    }

    /// Entries added to `queue` since the previous read.
    ///
    /// Positions count every entry ever pushed, so the oldest being dropped
    /// does not move them. Another queue (the label was cleared or replaced)
    /// is read again in full. Entries dropped before the reader got to them
    /// are skipped, and counted in [`NewEntries::missed`].
    pub fn read<T: Clone>(&mut self, queue: &Queue<T>) -> NewEntries<T> {
        let current = Position {
            queue: queue.id(),
            end: queue.end(),
        };
        let resume_at = match self.read.replace(current) {
            Some(previous) if previous.queue == current.queue => Some(previous.end),
            _ => None,
        };
        let (skip, missed) = match resume_at {
            Some(end) => (
                end.saturating_sub(queue.dropped()) as usize,
                queue.dropped().saturating_sub(end),
            ),
            None => (0, 0),
        };
        NewEntries {
            entries: queue.iter().skip(skip).cloned().collect(),
            restarted: resume_at.is_none(),
            missed,
        }
    }

    /// Entries of type `T` added under `label` since the previous read, or
    /// `None` if `label` holds no entries of that type.
    pub fn new_entries<T: DataEntry>(
        &mut self,
        data: &DataContext,
        label: &str,
    ) -> Option<NewEntries<T>> {
        if self.label != label {
            self.label = label.to_string();
            self.reset();
        }
        // Only this label's writes (and labels coming and going) run the
        // reader again
        data.with_label(label, |data| T::queue(data).map(|queue| self.read(queue)))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::{NewEntries, SourceCursor};
    use crate::data::Queue;

    fn read(cursor: &mut SourceCursor, queue: &Queue<i32>) -> (Vec<i32>, bool) {
        let NewEntries {
            entries,
            restarted,
            missed,
        } = cursor.read(queue);
        assert_eq!(
            missed, 0,
            "only `skips_what_was_dropped_before_it_was_read` misses"
        );
        (entries, restarted)
    }

    fn queue_of(entries: &[i32]) -> Queue<i32> {
        entries.iter().copied().collect()
    }

    #[test]
    fn reads_everything_first_then_only_new_entries() {
        let mut cursor = SourceCursor::default();
        let mut queue = queue_of(&[1, 2]);
        assert_eq!(read(&mut cursor, &queue), (vec![1, 2], true));
        queue.push(3);
        assert_eq!(read(&mut cursor, &queue), (vec![3], false));
        assert_eq!(read(&mut cursor, &queue), (vec![], false));
    }

    #[test]
    fn keeps_its_place_when_the_oldest_are_dropped() {
        let mut cursor = SourceCursor::default();
        let mut queue = queue_of(&[1, 2, 3]);
        read(&mut cursor, &queue);
        queue.push_within(4, 3);
        queue.push_within(5, 3);
        assert_eq!(read(&mut cursor, &queue), (vec![4, 5], false));
    }

    #[test]
    fn skips_what_was_dropped_before_it_was_read() {
        let mut cursor = SourceCursor::default();
        let mut queue = queue_of(&[1]);
        read(&mut cursor, &queue);
        for entry in 2..=6 {
            queue.push_within(entry, 3);
        }
        // 2 and 3 were dropped unread
        let read = cursor.read(&queue);
        assert_eq!(read.entries, [4, 5, 6]);
        assert!(!read.restarted);
        assert_eq!(read.missed, 2);

        // Counted once: the next read misses nothing
        queue.push_within(7, 3);
        assert_eq!(cursor.read(&queue).missed, 0);
    }

    #[test]
    fn restarts_on_another_queue() {
        let mut cursor = SourceCursor::default();
        read(&mut cursor, &queue_of(&[1, 2]));
        // Even one as long, e.g. the label cleared and refilled
        assert_eq!(read(&mut cursor, &queue_of(&[5, 6])), (vec![5, 6], true));
    }

    #[test]
    fn an_empty_queue_filling_up_is_not_a_restart() {
        let mut cursor = SourceCursor::default();
        let mut queue = Queue::new();
        assert_eq!(read(&mut cursor, &queue), (vec![], true));
        assert_eq!(read(&mut cursor, &queue), (vec![], false));
        queue.push(1);
        assert_eq!(read(&mut cursor, &queue), (vec![1], false));
    }

    #[test]
    fn reset_reads_everything_again() {
        let mut cursor = SourceCursor::default();
        let queue = queue_of(&[1]);
        read(&mut cursor, &queue);
        cursor.reset();
        assert_eq!(read(&mut cursor, &queue), (vec![1], true));
    }
}
