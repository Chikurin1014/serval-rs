//! A label's values within a time range, kept in order as they come and go, so
//! a box is summed up without sorting them all again at each new value.

use std::collections::VecDeque;

use sorted_vec::partial::SortedVec;

use crate::data::{NumberData, Queue, SourceCursor};

use super::super::context::until_paused;

/// The values of one label at or after `since` (and up to a pause), sorted.
#[derive(Debug, Default)]
pub(super) struct SortedWindow {
    cursor: SourceCursor,
    /// Those within, oldest first: the newest of the queue, of their times.
    within: VecDeque<(i64, f64)>,
    /// The finite of them, in order (none NaN, which it could not compare).
    sorted: SortedVec<f64>,
    /// What they were taken for last.
    taken_for: Option<(i64, Option<i64>)>,
}

impl SortedWindow {
    /// Brings it up to `queue` (oldest first, in order of time), within `since`
    /// and up to `paused_at`; the finite values within, sorted.
    ///
    /// Only the new values are read and those gone dropped, but taken anew as
    /// it reaches back further (as `since` goes back) or while paused.
    pub(super) fn update(
        &mut self,
        queue: &Queue<NumberData>,
        since: i64,
        paused_at: Option<i64>,
    ) -> &[f64] {
        let taken_for = Some((since, paused_at));
        let reaches_back = self.taken_for.is_none_or(|(taken_since, taken_paused)| {
            since < taken_since || paused_at != taken_paused
        });
        if reaches_back || paused_at.is_some() {
            self.cursor.reset();
        }
        self.taken_for = taken_for;

        let read = self.cursor.read(queue);
        if read.restarted {
            self.within.clear();
            self.sorted.clear();
        }
        for entry in &read.entries {
            let timestamp = entry.timestamp();
            if timestamp >= since && until_paused(paused_at, timestamp) {
                let value = *entry.value();
                self.within.push_back((timestamp, value));
                if value.is_finite() {
                    self.sorted.insert(value);
                }
            }
        }
        // Those the queue dropped, then those too old: the oldest, either way
        while self.within.len() > queue.len()
            || self
                .within
                .front()
                .is_some_and(|&(timestamp, _)| timestamp < since)
        {
            if let Some((_, value)) = self.within.pop_front()
                && value.is_finite()
            {
                self.sorted.remove_item(&value);
            }
        }
        &self.sorted
    }
}

#[cfg(test)]
mod tests {
    use super::SortedWindow;
    use crate::data::{NumberData, Queue};

    fn queue_of(values: &[(i64, f64)], max_entries: usize) -> Queue<NumberData> {
        let mut queue = Queue::new();
        for &(timestamp, value) in values {
            queue.push_within(NumberData::new(timestamp, value), max_entries);
        }
        queue
    }

    /// What a sort of them all gives.
    fn sorted_anew(queue: &Queue<NumberData>, since: i64, paused_at: Option<i64>) -> Vec<f64> {
        let mut values: Vec<f64> = queue
            .iter()
            .filter(|entry| {
                entry.timestamp() >= since && paused_at.is_none_or(|at| entry.timestamp() <= at)
            })
            .map(|entry| *entry.value())
            .filter(|value| value.is_finite())
            .collect();
        values.sort_by(f64::total_cmp);
        values
    }

    #[test]
    fn values_come_and_go_as_a_sort_of_them_all_has_them() {
        let mut window = SortedWindow::default();
        let mut queue = Queue::new();
        // Of a few values over and over, some not finite; 50 kept
        let values = [
            3.0,
            -1.0,
            7.5,
            3.0,
            f64::NAN,
            0.0,
            -0.0,
            12.0,
            f64::INFINITY,
        ];
        for (step, value) in values.iter().cycle().take(200).enumerate() {
            let timestamp = step as i64 * 100;
            queue.push_within(NumberData::new(timestamp, *value), 50);
            // A range of 2 s, then all
            let since = if step < 150 {
                timestamp - 2_000
            } else {
                i64::MIN
            };
            assert_eq!(
                window.update(&queue, since, None),
                sorted_anew(&queue, since, None),
                "at step {step}"
            );
        }
    }

    #[test]
    fn a_pause_leaves_out_the_newer_until_it_ends() {
        let mut window = SortedWindow::default();
        let queue = queue_of(&[(0, 1.0), (100, 2.0), (200, 3.0), (300, 4.0)], 10);
        assert_eq!(window.update(&queue, i64::MIN, Some(150)), [1.0, 2.0]);
        assert_eq!(window.update(&queue, 50, Some(250)), [2.0, 3.0]);
        assert_eq!(window.update(&queue, 50, None), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_queue_cleared_or_replaced_is_taken_anew() {
        let mut window = SortedWindow::default();
        let queue = queue_of(&[(0, 5.0), (100, 1.0)], 10);
        assert_eq!(window.update(&queue, i64::MIN, None), [1.0, 5.0]);
        let replaced = queue_of(&[(200, 3.0)], 10);
        assert_eq!(window.update(&replaced, i64::MIN, None), [3.0]);
    }
}
