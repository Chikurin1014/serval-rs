use dioxus::prelude::*;

use crate::data::{
    Conversion, ConversionOutput, DataContext, Input, MapRunner, MapSettings, NumberData, Operand,
    Segment, format_number, keep_taken, pairs,
};
use crate::helper::set_if_changed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    /// `a` to the power of `b`.
    Pow,
}

impl Operation {
    pub fn symbol(self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "−",
            Operation::Multiply => "×",
            Operation::Divide => "/",
            Operation::Pow => "^",
        }
    }

    /// Its formula in LaTeX.
    pub fn latex(self) -> &'static str {
        match self {
            Operation::Add => "a + b",
            Operation::Subtract => "a - b",
            Operation::Multiply => "a \\times b",
            Operation::Divide => "a / b",
            Operation::Pow => "a^b",
        }
    }

    /// As Rust's `f64` has it: by 0, infinite; 0 / 0, NaN.
    fn apply(self, first: f64, second: f64) -> f64 {
        match self {
            Operation::Add => first + second,
            Operation::Subtract => first - second,
            Operation::Multiply => first * second,
            Operation::Divide => first / second,
            Operation::Pow => first.powf(second),
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

super::saved_settings!(ArithmeticSettings {
    first,
    second,
    to_label
});

/// Adds, subtracts, multiplies, divides or raises to a power two numbers, each
/// from a label or a constant. With two labels, a result comes once both have a new number.
pub struct Arithmetic {
    settings: ArithmeticSettings,
    taken: Option<[String; 3]>,
    operands: Operands,
}

#[derive(Default)]
struct Operands {
    first: Input<f64>,
    second: Input<f64>,
    /// The constants have given their result, with no label.
    constants_used: bool,
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
        }
    }
}

impl MapRunner for Arithmetic {
    fn settings(&self) -> &dyn MapSettings {
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

        set_if_changed(&mut error, None);

        let mut latest = None;
        let Operands {
            first: first_input,
            second: second_input,
            constants_used,
        } = &mut self.operands;
        let pairs = pairs(
            data,
            (&first, &second),
            (first_input, second_input),
            constants_used,
        );
        for (first_value, second_value) in pairs {
            let result = operation.apply(first_value, second_value);
            data.push(to, NumberData::new(timestamp, result));
            latest = Some((first_value, second_value, result));
        }
        let (first_value, second_value, result) = latest?;
        Some(Conversion {
            from: vec![first.input(first_value), second.input(second_value)],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(to)],
                vec![Segment::from_input(format_number(result))],
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use super::{Arithmetic, ArithmeticSettings, Operation};
    use crate::data::map::testing::{Maps, number};

    #[test]
    fn operations_apply_in_order() {
        assert_eq!(Operation::Add.apply(6.0, 3.0), 9.0);
        assert_eq!(Operation::Subtract.apply(6.0, 3.0), 3.0);
        assert_eq!(Operation::Multiply.apply(6.0, 3.0), 18.0);
        assert_eq!(Operation::Divide.apply(6.0, 3.0), 2.0);
        assert_eq!(Operation::Pow.apply(6.0, 3.0), 216.0);
    }

    fn arithmetic(
        maps: &mut Maps,
        operation: Operation,
        [first, second, to]: [&str; 3],
    ) -> ArithmeticSettings {
        maps.add(
            || Arithmetic::new(operation),
            |mut settings: ArithmeticSettings| {
                settings.first.set(first.to_string());
                settings.second.set(second.to_string());
                settings.to_label.set(to.to_string());
            },
        )
    }

    #[test]
    fn a_constant_gives_a_result_for_each_number() {
        let mut maps = Maps::new();
        arithmetic(&mut maps, Operation::Multiply, ["temp", "2", "doubled"]);
        maps.push("temp", number(1.5));
        maps.push("temp", number(-3.0));
        assert_eq!(maps.values("doubled"), ["3.0000", "-6.0000"]);
    }

    #[test]
    fn constants_are_named_after_a_hash() {
        let mut maps = Maps::new();
        arithmetic(&mut maps, Operation::Multiply, ["#pi", "temp", "turned"]);
        let unknown = arithmetic(&mut maps, Operation::Add, ["#tau", "temp", "sum"]);
        maps.push("temp", number(2.0));
        assert_eq!(maps.values("turned"), ["6.2832"]);
        assert_eq!(
            maps.read(unknown.error).as_deref(),
            Some("No constant '#tau'")
        );
    }

    #[test]
    fn a_label_wins_over_a_number() {
        let mut maps = Maps::new();
        maps.push("10", number(3.0));
        arithmetic(&mut maps, Operation::Add, ["temp", "10", "sum"]);
        maps.push("temp", number(2.0));
        assert_eq!(maps.values("sum"), ["5.0000"]);
    }

    #[test]
    fn an_unknown_operand_is_an_error() {
        let mut maps = Maps::new();
        let unknown = arithmetic(
            &mut maps,
            Operation::Subtract,
            ["nothing", "1", "difference"],
        );
        assert_eq!(
            maps.read(unknown.error).as_deref(),
            Some("No Number label or number 'nothing'")
        );
        assert!(maps.values("difference").is_empty());
    }

    #[test]
    fn division_by_zero_is_no_error() {
        let mut maps = Maps::new();
        let divided = arithmetic(&mut maps, Operation::Divide, ["temp", "0", "quotient"]);
        maps.push("temp", number(1.0));
        maps.push("temp", number(-1.0));
        maps.push("temp", number(0.0));
        assert_eq!(maps.values("quotient"), ["inf", "-inf", "NaN"]);
        assert_eq!(maps.read(divided.error), None);
    }
}
