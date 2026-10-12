use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ArithmeticSettings, Conversion, ConversionInput, Data, DataContext, DataEntry, Input,
    MapRunner, Operand, Operation, Segment, Vector, format_number, keep_taken, take_newest_pair,
};
use crate::helper::set_if_changed;

/// Adds or subtracts two vectors, or multiplies one by a number: each from a
/// label or a constant. With two labels, a result comes once both have a new
/// value.
pub struct VectorArithmetic<V> {
    settings: ArithmeticSettings,
    taken: Option<[String; 3]>,
    vectors: Input<V>,
    /// The second vector, to add or subtract.
    second_vectors: Input<V>,
    /// The number, to multiply by.
    numbers: Input<f64>,
    /// The constants have given their result, with no label.
    constants_used: bool,
    failure: Option<String>,
}

/// Where a value comes from.
#[derive(Clone, Debug, PartialEq)]
enum Source<T> {
    Label(String),
    Constant(T),
}

impl<T> Source<T> {
    fn label(&self) -> &str {
        match self {
            Source::Label(label) => label,
            Source::Constant(_) => "",
        }
    }
}

impl<V: Vector> Source<V> {
    /// A label of vectors of `V`, or else a vector as `(x, y)` or `x, y`.
    fn resolve_vector(data: &DataContext, text: &str) -> Result<Self, String> {
        let text = text.trim();
        if data.data_type_of(text) == Some(V::DATA_TYPE) {
            return Ok(Source::Label(text.to_string()));
        }
        parse_vector(text)
            .map(Source::Constant)
            .ok_or_else(|| format!("No {} label or vector '{text}'", V::DATA_TYPE.name()))
    }
}

impl Source<f64> {
    fn resolve_number(data: &DataContext, text: &str) -> Result<Self, String> {
        Ok(match Operand::resolve(data, text)? {
            Operand::Label(label) => Source::Label(label),
            Operand::Constant(value) => Source::Constant(value),
        })
    }
}

/// `(x, y)` or `x, y` (and z), if each is a number.
fn parse_vector<V: Vector>(text: &str) -> Option<V> {
    let inner = text
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
        .unwrap_or(text);
    let components = inner
        .split(',')
        .map(|component| component.trim().parse().ok())
        .collect::<Option<Vec<f64>>>()?;
    V::from_components(&components)
}

/// `(x, y)` or `(x, y, z)`, rounded as numbers are shown.
fn vector_text<V: Vector>(vector: &V) -> String {
    let components: Vec<String> = vector
        .components()
        .iter()
        .map(|&component| format_number(component))
        .collect();
    format!("({})", components.join(", "))
}

/// The pairs of values to work out: with two labels, their newest once both
/// have one; with a label and a constant, each new value; with two constants,
/// once.
fn pairs<A: Clone, B: Clone>(
    data: &DataContext,
    sources: (&Source<A>, &Source<B>),
    inputs: (&mut Input<A>, &mut Input<B>),
    constants_used: &mut bool,
) -> Vec<(A, B)>
where
    Data<A>: DataEntry,
    Data<B>: DataEntry,
{
    let (first_input, second_input) = inputs;
    if !matches!(sources, (Source::Constant(_), Source::Constant(_))) {
        *constants_used = false;
    }
    match sources {
        (Source::Label(first), Source::Label(second)) => {
            first_input.read_newest(data, first);
            second_input.read_newest(data, second);
            take_newest_pair(first_input, second_input)
                .into_iter()
                .collect()
        }
        (Source::Label(first), Source::Constant(second)) => {
            second_input.forget();
            let values = first_input.read(data, first);
            values
                .into_iter()
                .map(|first| (first, second.clone()))
                .collect()
        }
        (Source::Constant(first), Source::Label(second)) => {
            first_input.forget();
            let values = second_input.read(data, second);
            values
                .into_iter()
                .map(|second| (first.clone(), second))
                .collect()
        }
        (Source::Constant(first), Source::Constant(second)) => {
            first_input.forget();
            second_input.forget();
            if std::mem::replace(constants_used, true) {
                return Vec::new();
            }
            vec![(first.clone(), second.clone())]
        }
    }
}

/// `operation` on each component of `first`, with `second`'s or a number.
fn apply<V: Vector>(
    operation: Operation,
    first: &V,
    second: impl Fn(usize) -> f64,
) -> Result<V, String> {
    let components = first
        .components()
        .iter()
        .enumerate()
        .map(|(index, &component)| operation.apply(component, second(index)))
        .collect::<Result<Vec<_>, _>>()?;
    V::from_components(&components).ok_or_else(|| "The wrong number of components".to_string())
}

/// A result, after its operands as text.
type Worked<V> = (String, String, V);

impl<V: Vector> VectorArithmetic<V>
where
    Data<V>: DataEntry,
{
    /// Adds or subtracts two vectors, or multiplies one by a number (before
    /// it, as `b` in `a × b`): others are not on vectors.
    pub fn new(operation: Operation) -> Self {
        Self {
            settings: ArithmeticSettings {
                operation,
                first: Signal::new(String::new()),
                second: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
                error: Signal::new(None),
            },
            taken: None,
            vectors: Input::default(),
            second_vectors: Input::default(),
            numbers: Input::default(),
            constants_used: false,
            failure: None,
        }
    }

    /// The results of the new values, each with the operands as text and
    /// their labels (none for constants), or why the operands are not set.
    fn results(
        &mut self,
        data: &DataContext,
        first: &str,
        second: &str,
    ) -> Result<(Vec<Worked<V>>, [String; 2]), String> {
        let operation = self.settings.operation;
        let mut results = Vec::new();
        let mut failure = None;
        let labels;
        if operation == Operation::Multiply {
            let number = Source::resolve_number(data, first)?;
            let vector = Source::<V>::resolve_vector(data, second)?;
            labels = [number.label().to_string(), vector.label().to_string()];
            let pairs = pairs::<f64, V>(
                data,
                (&number, &vector),
                (&mut self.numbers, &mut self.vectors),
                &mut self.constants_used,
            );
            for (number, vector) in pairs {
                match apply(operation, &vector, |_| number) {
                    Ok(result) => {
                        results.push((format_number(number), vector_text(&vector), result))
                    }
                    Err(message) => failure = Some(message),
                }
            }
        } else {
            let first = Source::<V>::resolve_vector(data, first)?;
            let second = Source::<V>::resolve_vector(data, second)?;
            labels = [first.label().to_string(), second.label().to_string()];
            let pairs = pairs::<V, V>(
                data,
                (&first, &second),
                (&mut self.vectors, &mut self.second_vectors),
                &mut self.constants_used,
            );
            for (first, second) in pairs {
                match apply(operation, &first, |index| second.components()[index]) {
                    Ok(result) => results.push((vector_text(&first), vector_text(&second), result)),
                    Err(message) => failure = Some(message),
                }
            }
        }
        self.failure = failure;
        Ok((results, labels))
    }
}

impl<V: Vector> MapRunner for VectorArithmetic<V>
where
    Data<V>: DataEntry,
{
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let ArithmeticSettings {
            first,
            second,
            to_label,
            mut error,
            ..
        } = self.settings;
        let texts = [first, second, to_label].map(|text| text.peek().trim().to_string());
        let taken = (!texts.iter().any(String::is_empty)).then_some(texts);
        if keep_taken(&mut self.taken, taken) {
            self.vectors = Input::default();
            self.second_vectors = Input::default();
            self.numbers = Input::default();
            self.constants_used = false;
            self.failure = None;
        }
        set_if_changed(&mut error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let mut error = self.settings.error;
        let [first, second, to] = self.taken.clone()?;
        // Not its own output
        if first == to || second == to {
            return None;
        }

        // On each run: a label may come after the map is turned on
        let (results, [first_label, second_label]) = match self.results(data, &first, &second) {
            Ok(results) => results,
            Err(message) => {
                set_if_changed(&mut error, Some(message));
                return None;
            }
        };
        set_if_changed(&mut error, self.failure.clone());

        for &(_, _, result) in &results {
            data.push(&to, Data::new(timestamp, result));
        }
        let (first_value, second_value, result) = results.into_iter().last()?;
        Some(Conversion {
            from: vec![
                ConversionInput::new(first_label, first_value),
                ConversionInput::new(second_label, second_value),
            ],
            to_label: vec![Segment::fixed(&to)],
            to_value: vec![Segment::from_input(vector_text(&result))],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{apply, parse_vector, vector_text};
    use crate::data::{Operation, Vector2d, Vector3d};

    #[test]
    fn vectors_parse_with_or_without_parentheses() {
        assert_eq!(parse_vector("(1, -2.5)"), Some(Vector2d::new(1.0, -2.5)));
        assert_eq!(parse_vector("1,2,3"), Some(Vector3d::new(1.0, 2.0, 3.0)));
        assert_eq!(parse_vector::<Vector2d>("(1, 2, 3)"), None);
        assert_eq!(parse_vector::<Vector2d>("(1, x)"), None);
    }

    #[test]
    fn operations_apply_to_each_component() {
        let a = Vector2d::new(6.0, 4.0);
        let b = Vector2d::new(3.0, 2.0);
        let component = |index| b.components()[index];
        assert_eq!(
            apply(Operation::Add, &a, component),
            Ok(Vector2d::new(9.0, 6.0))
        );
        assert_eq!(
            apply(Operation::Subtract, &a, component),
            Ok(Vector2d::new(3.0, 2.0))
        );
        assert_eq!(
            apply(Operation::Multiply, &a, |_| 0.5),
            Ok(Vector2d::new(3.0, 2.0))
        );
        assert_eq!(vector_text(&Vector2d::new(1.0, 0.5)), "(1.0000, 0.50000)");
    }
}
