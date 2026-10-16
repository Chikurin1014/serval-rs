use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, ConversionOutput, Data, DataContext, DataEntry, Input, MapRunner,
    MapSettings, NumberData, Operand, Segment, Vector, format_number, keep_taken, take_newest_pair,
};
use crate::helper::set_if_changed;

/// What a vector map works out: on two vectors, or a number and one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorOperation {
    Add,
    Subtract,
    /// A number times a vector, the number first.
    Multiply,
    /// A number, of vectors of either dimensions.
    Dot,
    /// A 3D vector of 3D vectors; of 2D ones, a number (its z).
    Cross,
}

impl VectorOperation {
    pub fn symbol(self) -> &'static str {
        match self {
            VectorOperation::Add => "+",
            VectorOperation::Subtract => "−",
            VectorOperation::Multiply => "",
            VectorOperation::Dot => "·",
            VectorOperation::Cross => "×",
        }
    }

    /// Its formula in LaTeX.
    pub fn latex(self) -> &'static str {
        match self {
            VectorOperation::Add => "\\vec{a} + \\vec{b}",
            VectorOperation::Subtract => "\\vec{a} - \\vec{b}",
            VectorOperation::Multiply => "a \\vec{b}",
            VectorOperation::Dot => "\\vec{a} \\cdot \\vec{b}",
            VectorOperation::Cross => "\\vec{a} \\times \\vec{b}",
        }
    }

    /// On two vectors; `None` for one with a number.
    fn of_vectors<V: Vector>(self, first: &V, second: &V) -> Option<Product<V>> {
        let pairs = || first.components().iter().zip(second.components());
        let each = |apply: fn(f64, f64) -> f64| {
            let components: Vec<f64> = pairs()
                .map(|(&first, &second)| apply(first, second))
                .collect();
            V::from_components(&components).map(Product::Vector)
        };
        match self {
            VectorOperation::Add => each(|first, second| first + second),
            VectorOperation::Subtract => each(|first, second| first - second),
            VectorOperation::Multiply => None,
            VectorOperation::Dot => Some(Product::Number(
                pairs().map(|(&first, &second)| first * second).sum(),
            )),
            VectorOperation::Cross => match (first.components(), second.components()) {
                (&[ax, ay], &[bx, by]) => Some(Product::Number(ax * by - ay * bx)),
                _ => first.cross(second).map(Product::Vector),
            },
        }
    }
}

/// What a vector map gives: a vector, or a number (as a dot product).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Product<V> {
    Vector(V),
    Number(f64),
}

impl<V: Vector> Product<V>
where
    Data<V>: DataEntry,
{
    fn text(&self) -> String {
        match self {
            Product::Vector(vector) => vector_text(vector),
            Product::Number(number) => format_number(*number),
        }
    }

    fn push_to(self, data: &mut DataContext, label: &str, timestamp: i64) {
        match self {
            Product::Vector(vector) => data.push(label, Data::new(timestamp, vector)),
            Product::Number(number) => data.push(label, NumberData::new(timestamp, number)),
        }
    }
}

/// `vector` times `number`.
fn scale<V: Vector>(number: f64, vector: &V) -> Option<Product<V>> {
    let components: Vec<f64> = vector
        .components()
        .iter()
        .map(|&component| number * component)
        .collect();
    V::from_components(&components).map(Product::Vector)
}

#[derive(Clone, Copy, PartialEq)]
pub struct VectorArithmeticSettings {
    pub operation: VectorOperation,
    /// A label, or else a constant: a number to multiply by, or a vector.
    pub first: Signal<String>,
    /// A vector label, or else a vector.
    pub second: Signal<String>,
    pub to_label: Signal<String>,
    pub error: Signal<Option<String>>,
}

super::saved_settings!(VectorArithmeticSettings {
    first,
    second,
    to_label
});

/// Adds, subtracts, dots or crosses two vectors, or multiplies one by a number:
/// each from a label or a constant. With two labels, a result comes once both have
/// a new value.
pub struct VectorArithmetic<V> {
    settings: VectorArithmeticSettings,
    taken: Option<[String; 3]>,
    vectors: Input<V>,
    /// The first vector, of two.
    first_vectors: Input<V>,
    /// The number, to multiply by.
    numbers: Input<f64>,
    /// The constants have given their result, with no label.
    constants_used: bool,
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
pub(super) fn vector_text<V: Vector>(vector: &V) -> String {
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

/// A result, after its operands as text.
type Worked<V> = (String, String, Product<V>);

impl<V: Vector> VectorArithmetic<V>
where
    Data<V>: DataEntry,
{
    pub fn new(operation: VectorOperation) -> Self {
        Self {
            settings: VectorArithmeticSettings {
                operation,
                first: Signal::new(String::new()),
                second: Signal::new(String::new()),
                to_label: Signal::new(String::new()),
                error: Signal::new(None),
            },
            taken: None,
            vectors: Input::default(),
            first_vectors: Input::default(),
            numbers: Input::default(),
            constants_used: false,
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
        let vector = Source::<V>::resolve_vector(data, second)?;
        if operation == VectorOperation::Multiply {
            let number = Source::resolve_number(data, first)?;
            let labels = [number.label().to_string(), vector.label().to_string()];
            let results = pairs::<f64, V>(
                data,
                (&number, &vector),
                (&mut self.numbers, &mut self.vectors),
                &mut self.constants_used,
            )
            .into_iter()
            .filter_map(|(number, vector)| {
                let result = scale(number, &vector)?;
                Some((format_number(number), vector_text(&vector), result))
            })
            .collect();
            return Ok((results, labels));
        }
        let first = Source::<V>::resolve_vector(data, first)?;
        let labels = [first.label().to_string(), vector.label().to_string()];
        let results = pairs::<V, V>(
            data,
            (&first, &vector),
            (&mut self.first_vectors, &mut self.vectors),
            &mut self.constants_used,
        )
        .into_iter()
        .filter_map(|(first, second)| {
            let result = operation.of_vectors(&first, &second)?;
            Some((vector_text(&first), vector_text(&second), result))
        })
        .collect();
        Ok((results, labels))
    }
}

impl<V: Vector> MapRunner for VectorArithmetic<V>
where
    Data<V>: DataEntry,
{
    fn settings(&self) -> &dyn MapSettings {
        &self.settings
    }

    fn start(&mut self) {
        let VectorArithmeticSettings {
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
            self.first_vectors = Input::default();
            self.numbers = Input::default();
            self.constants_used = false;
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
        set_if_changed(&mut error, None);

        for &(_, _, result) in &results {
            result.push_to(data, &to, timestamp);
        }
        let (first_value, second_value, result) = results.into_iter().last()?;
        Some(Conversion {
            from: vec![
                ConversionInput::new(first_label, first_value),
                ConversionInput::new(second_label, second_value),
            ],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(&to)],
                vec![Segment::from_input(result.text())],
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Product, VectorOperation, parse_vector, scale, vector_text};
    use crate::data::{Vector2d, Vector3d};

    #[test]
    fn vectors_parse_with_or_without_parentheses() {
        assert_eq!(parse_vector("(1, -2.5)"), Some(Vector2d::new(1.0, -2.5)));
        assert_eq!(parse_vector("1,2,3"), Some(Vector3d::new(1.0, 2.0, 3.0)));
        assert_eq!(parse_vector::<Vector2d>("(1, 2, 3)"), None);
        assert_eq!(parse_vector::<Vector2d>("(1, x)"), None);
    }

    #[test]
    fn operations_apply_to_vectors() {
        let a = Vector2d::new(6.0, 4.0);
        let b = Vector2d::new(3.0, 2.0);
        let of = |operation: VectorOperation| operation.of_vectors(&a, &b);
        assert_eq!(
            of(VectorOperation::Add),
            Some(Product::Vector(Vector2d::new(9.0, 6.0)))
        );
        assert_eq!(
            of(VectorOperation::Subtract),
            Some(Product::Vector(Vector2d::new(3.0, 2.0)))
        );
        assert_eq!(
            scale(0.5, &a),
            Some(Product::Vector(Vector2d::new(3.0, 2.0)))
        );
        assert_eq!(of(VectorOperation::Multiply), None);
        assert_eq!(vector_text(&Vector2d::new(1.0, 0.5)), "(1.0000, 0.50000)");
    }

    #[test]
    fn dot_products_are_numbers() {
        let dot = VectorOperation::Dot;
        assert_eq!(
            dot.of_vectors(&Vector2d::new(1.0, 2.0), &Vector2d::new(3.0, 4.0)),
            Some(Product::Number(11.0))
        );
        assert_eq!(
            dot.of_vectors(
                &Vector3d::new(1.0, 2.0, 3.0),
                &Vector3d::new(4.0, -5.0, 6.0)
            ),
            Some(Product::Number(12.0))
        );
    }

    #[test]
    fn cross_products_follow_the_right_hand_rule() {
        let x = Vector3d::new(1.0, 0.0, 0.0);
        let y = Vector3d::new(0.0, 1.0, 0.0);
        let cross = |a, b| VectorOperation::Cross.of_vectors(&a, &b);
        assert_eq!(
            cross(x, y),
            Some(Product::Vector(Vector3d::new(0.0, 0.0, 1.0)))
        );
        assert_eq!(
            cross(y, x),
            Some(Product::Vector(Vector3d::new(0.0, 0.0, -1.0)))
        );
        assert_eq!(
            cross(Vector3d::new(1.0, 2.0, 3.0), Vector3d::new(4.0, 5.0, 6.0)),
            Some(Product::Vector(Vector3d::new(-3.0, 6.0, -3.0)))
        );
        // Of 2D vectors: the z of theirs as 3D ones
        let cross = |a, b| VectorOperation::Cross.of_vectors(&a, &b);
        assert_eq!(
            cross(Vector2d::new(1.0, 0.0), Vector2d::new(0.0, 1.0)),
            Some(Product::Number(1.0))
        );
        assert_eq!(
            cross(Vector2d::new(2.0, 3.0), Vector2d::new(4.0, 5.0)),
            Some(Product::Number(-2.0))
        );
    }
}
