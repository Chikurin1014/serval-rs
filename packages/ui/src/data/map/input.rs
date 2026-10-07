//! What the maps share in reading their inputs.

use crate::data::{Data, DataContext, DataEntry, SourceCursor};

/// `from` and `to`, trimmed, if a map can run with them: both set, and not the
/// same (a map does not write to the label it reads).
pub(crate) fn endpoints<'a>(from: &'a str, to: &'a str) -> Option<(&'a str, &'a str)> {
    let (from, to) = (from.trim(), to.trim());
    (!from.is_empty() && !to.is_empty() && from != to).then_some((from, to))
}

/// One of a map's inputs: how far its label is read, and for maps joining two
/// inputs, its newest value not used yet (see [`take_newest_pair`]).
pub(crate) struct Input<T> {
    cursor: SourceCursor,
    newest: Option<T>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            cursor: SourceCursor::default(),
            newest: None,
        }
    }
}

impl<T: Clone> Input<T> {
    /// The values added under `label` since the last read. Read from its start
    /// again (the label changed, or its data was cleared or is of another type),
    /// it drops the newest value kept.
    pub(crate) fn read(&mut self, data: &DataContext, label: &str) -> Vec<T>
    where
        Data<T>: DataEntry,
    {
        match self.cursor.new_entries::<Data<T>>(data, label) {
            Some(read) => {
                if read.restarted {
                    self.newest = None;
                }
                read.entries
                    .iter()
                    .map(|entry| entry.value().clone())
                    .collect()
            }
            None => {
                self.forget();
                Vec::new()
            }
        }
    }

    /// As [`Self::read`], keeping the last value as the newest.
    pub(crate) fn read_newest(&mut self, data: &DataContext, label: &str)
    where
        Data<T>: DataEntry,
    {
        if let Some(last) = self.read(data, label).pop() {
            self.newest = Some(last);
        }
    }

    /// Reads from the start next time, with no newest value.
    pub(crate) fn forget(&mut self) {
        self.cursor.reset();
        self.newest = None;
    }
}

/// The newest values of both inputs once each has one, used up by this.
pub(crate) fn take_newest_pair<A, B>(
    first: &mut Input<A>,
    second: &mut Input<B>,
) -> Option<(A, B)> {
    if first.newest.is_none() || second.newest.is_none() {
        return None;
    }
    first.newest.take().zip(second.newest.take())
}

#[cfg(test)]
mod tests {
    use super::{Input, endpoints, take_newest_pair};

    #[test]
    fn endpoints_are_set_and_apart() {
        assert_eq!(endpoints(" raw ", "text"), Some(("raw", "text")));
        assert_eq!(endpoints("raw", " "), None);
        assert_eq!(endpoints("", "text"), None);
        assert_eq!(endpoints("raw", " raw"), None);
    }

    #[test]
    fn a_pair_needs_both_and_uses_them_up() {
        let mut first = Input::<f64>::default();
        let mut second = Input::<String>::default();
        first.newest = Some(1.0);
        assert_eq!(take_newest_pair(&mut first, &mut second), None);
        // The one waiting is kept for the other
        assert_eq!(first.newest, Some(1.0));

        second.newest = Some("a".to_string());
        assert_eq!(
            take_newest_pair(&mut first, &mut second),
            Some((1.0, "a".to_string()))
        );
        assert_eq!(take_newest_pair(&mut first, &mut second), None);
    }
}
