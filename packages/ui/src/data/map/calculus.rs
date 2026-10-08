use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, DataContext, Endpoints, MapRunner, NumberData, Segment,
    SourceCursor, format_number, keep_taken,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calculus {
    /// `df/dt`, from each number and the one before it.
    Differentiate,
    /// `∫ f(t) dt` from the first number, by the trapezoidal rule.
    Integrate,
}

impl Calculus {
    /// Its formula in LaTeX.
    pub fn latex(self) -> &'static str {
        match self {
            Calculus::Differentiate => r"\frac{d}{dt} f(t)",
            Calculus::Integrate => r"\int f(t)\,dt",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Calculus::Differentiate => "d/dt f(t)",
            Calculus::Integrate => "∫ f(t) dt",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CalculusSettings {
    pub calculus: Calculus,
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

/// Differentiates or integrates a Number label over time, in seconds.
pub struct CalculusMap {
    settings: CalculusSettings,
    endpoints: Option<Endpoints>,
    cursor: SourceCursor,
    state: State,
}

#[derive(Debug, Default, PartialEq)]
struct State {
    /// The last number and its time in ms.
    previous: Option<(i64, f64)>,
    integral: f64,
}

impl State {
    fn next(&mut self, calculus: Calculus, timestamp: i64, value: f64) -> Option<f64> {
        let elapsed = self
            .previous
            .map(|(previous, _)| (timestamp - previous) as f64 / 1000.0)
            .filter(|&elapsed| elapsed > 0.0);
        let result = match (calculus, self.previous, elapsed) {
            (Calculus::Differentiate, Some((_, previous)), Some(elapsed)) => {
                Some((value - previous) / elapsed)
            }
            (Calculus::Differentiate, ..) => None,
            (Calculus::Integrate, Some((_, previous)), Some(elapsed)) => {
                self.integral += (previous + value) / 2.0 * elapsed;
                Some(self.integral)
            }
            (Calculus::Integrate, ..) => Some(self.integral),
        };
        self.previous = Some((timestamp, value));
        result
    }
}

impl CalculusMap {
    pub fn new(calculus: Calculus) -> Self {
        Self {
            settings: CalculusSettings {
                calculus,
                from_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
            },
            endpoints: None,
            cursor: SourceCursor::default(),
            state: State::default(),
        }
    }
}

impl MapRunner for CalculusMap {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let CalculusSettings {
            from_label,
            to_label,
            ..
        } = self.settings;
        let endpoints = Endpoints::new(&from_label.peek(), &to_label.peek());
        if keep_taken(&mut self.endpoints, endpoints) {
            self.cursor.reset();
            self.state = State::default();
        }
    }

    fn run(&mut self, data: &mut DataContext, _timestamp: i64) -> Option<Conversion> {
        let calculus = self.settings.calculus;
        let Endpoints { from, to } = self.endpoints.as_ref()?;
        let Some(read) = self.cursor.new_entries::<NumberData>(data, from) else {
            self.cursor.reset();
            self.state = State::default();
            return None;
        };
        if read.restarted {
            self.state = State::default();
        }

        let mut latest = None;
        for entry in &read.entries {
            let value = *entry.value();
            if let Some(result) = self.state.next(calculus, entry.timestamp(), value) {
                data.push(to, NumberData::new(entry.timestamp(), result));
                latest = Some((value, result));
            }
        }
        let (value, result) = latest?;
        Some(Conversion {
            from: vec![ConversionInput::new(from, format_number(value))],
            to_label: vec![Segment::fixed(to)],
            to_value: vec![Segment::from_input(format_number(result))],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Calculus, State};

    fn results(calculus: Calculus, points: &[(i64, f64)]) -> Vec<Option<f64>> {
        let mut state = State::default();
        points
            .iter()
            .map(|&(timestamp, value)| state.next(calculus, timestamp, value))
            .collect()
    }

    #[test]
    fn differentiates_per_second() {
        assert_eq!(
            results(
                Calculus::Differentiate,
                &[(0, 1.0), (500, 2.0), (1500, 0.0)]
            ),
            vec![None, Some(2.0), Some(-2.0)]
        );
    }

    #[test]
    fn integrates_by_trapezoids_from_zero() {
        assert_eq!(
            results(Calculus::Integrate, &[(0, 1.0), (1000, 3.0), (3000, 3.0)]),
            vec![Some(0.0), Some(2.0), Some(8.0)]
        );
    }

    #[test]
    fn numbers_at_the_same_time_add_nothing() {
        assert_eq!(
            results(Calculus::Differentiate, &[(0, 1.0), (0, 5.0), (1000, 6.0)]),
            vec![None, None, Some(1.0)]
        );
        assert_eq!(
            results(Calculus::Integrate, &[(0, 1.0), (0, 5.0)]),
            vec![Some(0.0), Some(0.0)]
        );
    }
}
