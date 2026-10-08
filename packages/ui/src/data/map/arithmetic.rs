use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionInput, DataContext, DataType, Input, MapRunner, NumberData, Segment,
    format_number, keep_taken, take_newest_pair,
};
use crate::helper::set_if_changed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    pub fn symbol(self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "−",
            Operation::Multiply => "×",
            Operation::Divide => "/",
        }
    }

    /// Its formula in LaTeX.
    pub fn latex(self) -> &'static str {
        match self {
            Operation::Add => "a + b",
            Operation::Subtract => "a - b",
            Operation::Multiply => "a \\times b",
            Operation::Divide => "a / b",
        }
    }

    fn apply(self, first: f64, second: f64) -> Result<f64, String> {
        match self {
            Operation::Add => Ok(first + second),
            Operation::Subtract => Ok(first - second),
            Operation::Multiply => Ok(first * second),
            Operation::Divide if second == 0.0 => Err("Division by zero".to_string()),
            Operation::Divide => Ok(first / second),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct ArithmeticSettings {
    pub operation: Operation,
    /// A Number label, or else a number.
    pub first: Signal<String>,
    pub second: Signal<String>,
    pub to_label: Signal<String>,
    pub error: Signal<Option<String>>,
}

/// Adds, subtracts, multiplies or divides two numbers, each from a label or a
/// constant. With two labels, a result comes once both have a new number.
pub struct Arithmetic {
    settings: ArithmeticSettings,
    taken: Option<[String; 3]>,
    operands: Operands,
    failure: Option<String>,
}

#[derive(Default)]
struct Operands {
    first: Input<f64>,
    second: Input<f64>,
    /// The last pair of constants used, so it gives one result.
    last_constants: Option<(f64, f64)>,
}

/// Where an operand comes from.
#[derive(Clone, Debug, PartialEq)]
enum Operand {
    Label(String),
    Constant(f64),
}

impl Operand {
    fn resolve(data: &DataContext, text: &str) -> Result<Self, String> {
        let text = text.trim();
        let is_label = data.data_type_of(text) == Some(DataType::Number);
        if is_label {
            Ok(Operand::Label(text.to_string()))
        } else {
            text.parse()
                .map(Operand::Constant)
                .map_err(|_| format!("No Number label or number '{text}'"))
        }
    }

    fn input(&self, value: f64) -> ConversionInput {
        match self {
            Operand::Label(label) => ConversionInput::new(label.as_str(), format_number(value)),
            Operand::Constant(_) => ConversionInput::new("", format_number(value)),
        }
    }
}

impl Arithmetic {
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
            operands: Operands::default(),
            failure: None,
        }
    }
}

impl Operands {
    fn pairs(&mut self, data: &DataContext, first: &Operand, second: &Operand) -> Vec<(f64, f64)> {
        if !matches!(
            (first, second),
            (Operand::Constant(_), Operand::Constant(_))
        ) {
            self.last_constants = None;
        }
        match (first, second) {
            (Operand::Label(first), Operand::Label(second)) => {
                self.first.read_newest(data, first);
                self.second.read_newest(data, second);
                take_newest_pair(&mut self.first, &mut self.second)
                    .into_iter()
                    .collect()
            }
            (Operand::Label(first), &Operand::Constant(second)) => {
                self.second.forget();
                let values = self.first.read(data, first);
                values.into_iter().map(|first| (first, second)).collect()
            }
            (&Operand::Constant(first), Operand::Label(second)) => {
                self.first.forget();
                let values = self.second.read(data, second);
                values.into_iter().map(|second| (first, second)).collect()
            }
            (&Operand::Constant(first), &Operand::Constant(second)) => {
                self.first.forget();
                self.second.forget();
                if self.last_constants == Some((first, second)) {
                    return Vec::new();
                }
                self.last_constants = Some((first, second));
                vec![(first, second)]
            }
        }
    }
}

impl MapRunner for Arithmetic {
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
            self.operands = Operands::default();
            self.failure = None;
        }
        set_if_changed(&mut error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let ArithmeticSettings {
            operation,
            mut error,
            ..
        } = self.settings;
        let [first, second, to] = self.taken.as_ref()?;

        // On each run: a label may come after the map is turned on
        let operands = Operand::resolve(data, first)
            .and_then(|first| Ok((first, Operand::resolve(data, second)?)));
        let (first, second) = match operands {
            Ok(operands) => operands,
            Err(message) => {
                set_if_changed(&mut error, Some(message));
                return None;
            }
        };
        if [&first, &second].contains(&&Operand::Label(to.clone())) {
            return None;
        }

        let mut latest = None;
        for (first_value, second_value) in self.operands.pairs(data, &first, &second) {
            match operation.apply(first_value, second_value) {
                Ok(result) => {
                    data.push(to, NumberData::new(timestamp, result));
                    latest = Some((first_value, second_value, result));
                    self.failure = None;
                }
                Err(message) => self.failure = Some(message),
            }
        }
        set_if_changed(&mut error, self.failure.clone());
        let (first_value, second_value, result) = latest?;
        Some(Conversion {
            from: vec![first.input(first_value), second.input(second_value)],
            to_label: vec![Segment::fixed(to)],
            to_value: vec![Segment::from_input(format_number(result))],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Operation;

    #[test]
    fn operations_apply_in_order() {
        assert_eq!(Operation::Add.apply(6.0, 3.0), Ok(9.0));
        assert_eq!(Operation::Subtract.apply(6.0, 3.0), Ok(3.0));
        assert_eq!(Operation::Multiply.apply(6.0, 3.0), Ok(18.0));
        assert_eq!(Operation::Divide.apply(6.0, 3.0), Ok(2.0));
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert!(Operation::Divide.apply(1.0, 0.0).is_err());
    }
}
