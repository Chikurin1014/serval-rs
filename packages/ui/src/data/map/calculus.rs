use std::any::Any;
use std::marker::PhantomData;

use dioxus::prelude::*;

use super::vector_arithmetic::vector_text;
use crate::data::SavedSettings;
use crate::data::{
    Conversion, ConversionInput, ConversionOutput, Data, DataContext, DataEntry, Endpoints,
    MapRunner, NumberData, Segment, SourceCursor, Vector, format_number, keep_taken,
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

    /// Its formula in LaTeX, of a vector.
    pub fn vector_latex(self) -> &'static str {
        match self {
            Calculus::Differentiate => r"\frac{d}{dt} \vec{f}(t)",
            Calculus::Integrate => r"\int \vec{f}(t)\,dt",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CalculusSettings {
    pub calculus: Calculus,
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

super::saved_settings!(CalculusSettings {
    from_label,
    to_label
});

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

    fn saved(&self) -> &dyn SavedSettings {
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
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(to)],
                vec![Segment::from_input(format_number(result))],
            )],
        })
    }
}

/// Differentiates or integrates a vector label over time, in seconds: each
/// component as a number.
pub struct VectorCalculus<V> {
    settings: CalculusSettings,
    endpoints: Option<Endpoints>,
    cursor: SourceCursor,
    /// One for each component.
    states: Vec<State>,
    _vector: PhantomData<V>,
}

impl<V> VectorCalculus<V> {
    pub fn new(calculus: Calculus) -> Self {
        Self {
            settings: CalculusSettings {
                calculus,
                from_label: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
            },
            endpoints: None,
            cursor: SourceCursor::default(),
            states: Vec::new(),
            _vector: PhantomData,
        }
    }
}

/// The result of the next vector, once each component has one: by `states`,
/// one for each.
fn next_vector<V: Vector>(
    states: &mut Vec<State>,
    calculus: Calculus,
    timestamp: i64,
    vector: &V,
) -> Option<V> {
    let components = vector.components();
    states.resize_with(components.len(), State::default);
    let results: Vec<Option<f64>> = states
        .iter_mut()
        .zip(components)
        .map(|(state, &value)| state.next(calculus, timestamp, value))
        .collect();
    V::from_components(&results.into_iter().collect::<Option<Vec<_>>>()?)
}

impl<V: Vector> MapRunner for VectorCalculus<V>
where
    Data<V>: DataEntry,
{
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn saved(&self) -> &dyn SavedSettings {
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
            self.states.clear();
        }
    }

    fn run(&mut self, data: &mut DataContext, _timestamp: i64) -> Option<Conversion> {
        let Endpoints { from, to } = self.endpoints.clone()?;
        let Some(read) = self.cursor.new_entries::<Data<V>>(data, &from) else {
            self.cursor.reset();
            self.states.clear();
            return None;
        };
        if read.restarted {
            self.states.clear();
        }

        let mut latest = None;
        for entry in &read.entries {
            let calculus = self.settings.calculus;
            if let Some(result) =
                next_vector(&mut self.states, calculus, entry.timestamp(), entry.value())
            {
                data.push(&to, Data::new(entry.timestamp(), result));
                latest = Some((*entry.value(), result));
            }
        }
        let (vector, result) = latest?;
        Some(Conversion {
            from: vec![ConversionInput::new(&from, vector_text(&vector))],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(&to)],
                vec![Segment::from_input(vector_text(&result))],
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Calculus, State, next_vector};
    use crate::data::Vector2d;

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

    #[test]
    fn vectors_go_by_each_component() {
        let mut states = Vec::new();
        let mut next = |timestamp, x, y| {
            next_vector(
                &mut states,
                Calculus::Differentiate,
                timestamp,
                &Vector2d::new(x, y),
            )
        };
        assert_eq!(next(0, 1.0, 0.0), None);
        assert_eq!(next(500, 2.0, -1.0), Some(Vector2d::new(2.0, -2.0)));

        let mut states = Vec::new();
        let mut next = |timestamp, x, y| {
            next_vector(
                &mut states,
                Calculus::Integrate,
                timestamp,
                &Vector2d::new(x, y),
            )
        };
        assert_eq!(next(0, 1.0, 2.0), Some(Vector2d::new(0.0, 0.0)));
        assert_eq!(next(1000, 3.0, 2.0), Some(Vector2d::new(2.0, 2.0)));
    }
}
