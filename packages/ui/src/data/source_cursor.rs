use std::collections::VecDeque;

use super::data_type::Data;
use crate::data::{ByteData, DataContext, NumberData, StringData, TypedData};

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
}

/// How much of a queue was read last time.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    len: usize,
    first_timestamp: Option<i64>,
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
    /// Queues only grow at the back, so one that got shorter, or whose first
    /// entry changed, was cleared or replaced: it is read again in full.
    pub fn read<T: Clone>(&mut self, queue: &VecDeque<Data<T>>) -> NewEntries<Data<T>> {
        let current = Position {
            len: queue.len(),
            first_timestamp: queue.front().map(|data| data.timestamp()),
        };
        let resume_at = match self.read.replace(current) {
            Some(previous)
                if previous.len <= current.len
                    && (previous.len == 0
                        || previous.first_timestamp == current.first_timestamp) =>
            {
                Some(previous.len)
            }
            _ => None,
        };
        NewEntries {
            entries: queue.iter().skip(resume_at.unwrap_or(0)).cloned().collect(),
            restarted: resume_at.is_none(),
        }
    }

    /// Bytes added under `label` since the previous read, or `None` if
    /// `label` holds no bytes.
    pub fn new_bytes(&mut self, data: &DataContext, label: &str) -> Option<NewEntries<ByteData>> {
        self.read_label(data, label, |data| match data {
            TypedData::Bytes(queue) => Some(queue),
            _ => None,
        })
    }

    /// Strings added under `label` since the previous read, or `None` if
    /// `label` holds no strings.
    pub fn new_strings(
        &mut self,
        data: &DataContext,
        label: &str,
    ) -> Option<NewEntries<StringData>> {
        self.read_label(data, label, |data| match data {
            TypedData::String(queue) => Some(queue),
            _ => None,
        })
    }

    /// Numbers added under `label` since the previous read, or `None` if
    /// `label` holds no numbers.
    pub fn new_numbers(
        &mut self,
        data: &DataContext,
        label: &str,
    ) -> Option<NewEntries<NumberData>> {
        self.read_label(data, label, |data| match data {
            TypedData::Number(queue) => Some(queue),
            _ => None,
        })
    }

    fn read_label<T: Clone>(
        &mut self,
        data: &DataContext,
        label: &str,
        queue_of: impl FnOnce(&TypedData) -> Option<&VecDeque<Data<T>>>,
    ) -> Option<NewEntries<Data<T>>> {
        if self.label != label {
            self.label = label.to_string();
            self.reset();
        }
        data.with_data(|data| {
            let queue = data.get(label).and_then(queue_of)?;
            Some(self.read(queue))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::{NewEntries, SourceCursor};
    use crate::data::NumberData;

    fn queue(points: &[(i64, f64)]) -> VecDeque<NumberData> {
        points
            .iter()
            .map(|&(timestamp, value)| NumberData::new(timestamp, value))
            .collect()
    }

    fn read(cursor: &mut SourceCursor, points: &[(i64, f64)]) -> (Vec<(i64, f64)>, bool) {
        let NewEntries { entries, restarted } = cursor.read(&queue(points));
        let entries = entries
            .iter()
            .map(|data| (data.timestamp(), *data.value()))
            .collect();
        (entries, restarted)
    }

    #[test]
    fn reads_everything_first_then_only_new_entries() {
        let mut cursor = SourceCursor::default();
        assert_eq!(
            read(&mut cursor, &[(1, 1.0), (2, 2.0)]),
            (vec![(1, 1.0), (2, 2.0)], true)
        );
        assert_eq!(
            read(&mut cursor, &[(1, 1.0), (2, 2.0), (3, 3.0)]),
            (vec![(3, 3.0)], false)
        );
        assert_eq!(
            read(&mut cursor, &[(1, 1.0), (2, 2.0), (3, 3.0)]),
            (vec![], false)
        );
    }

    #[test]
    fn restarts_when_the_queue_got_shorter() {
        let mut cursor = SourceCursor::default();
        read(&mut cursor, &[(1, 1.0), (2, 2.0)]);
        assert_eq!(read(&mut cursor, &[(5, 5.0)]), (vec![(5, 5.0)], true));
    }

    #[test]
    fn restarts_when_the_queue_was_refilled_past_its_old_length() {
        let mut cursor = SourceCursor::default();
        read(&mut cursor, &[(1, 1.0), (2, 2.0)]);
        // Same or greater length, but a different first entry
        assert_eq!(
            read(&mut cursor, &[(5, 5.0), (6, 6.0), (7, 7.0)]),
            (vec![(5, 5.0), (6, 6.0), (7, 7.0)], true)
        );
    }

    #[test]
    fn an_empty_queue_filling_up_is_not_a_restart() {
        let mut cursor = SourceCursor::default();
        assert_eq!(read(&mut cursor, &[]), (vec![], true));
        assert_eq!(read(&mut cursor, &[]), (vec![], false));
        assert_eq!(read(&mut cursor, &[(1, 1.0)]), (vec![(1, 1.0)], false));
    }

    #[test]
    fn reset_reads_everything_again() {
        let mut cursor = SourceCursor::default();
        read(&mut cursor, &[(1, 1.0)]);
        cursor.reset();
        assert_eq!(read(&mut cursor, &[(1, 1.0)]), (vec![(1, 1.0)], true));
    }
}
