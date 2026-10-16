use std::any::Any;

use dioxus::prelude::*;

use crate::data::SavedSettings;
use crate::data::{
    Conversion, ConversionOutput, DataContext, Input, MapRunner, Operand, Segment, Vector2d,
    Vector2dData, Vector3d, Vector3dData, format_number, keep_taken, take_newest_all,
};
use crate::helper::set_if_changed;

/// Each component a Number label or a number.
#[derive(Clone, Copy, PartialEq)]
pub struct ComposeSettings {
    pub x: Signal<String>,
    pub y: Signal<String>,
    /// For a [`Vector3d`] only.
    pub z: Option<Signal<String>>,
    pub to_label: Signal<String>,
    pub error: Signal<Option<String>>,
}

super::saved_settings!(ComposeSettings { x, y, z, to_label });

/// Makes a vector of two or three components, each the newest number of a
/// label or a constant: once each label has a new one (or, with constants
/// alone, once).
pub struct Compose {
    settings: ComposeSettings,
    taken: Option<Taken>,
    components: Vec<Input<f64>>,
    /// The constants last used, with no label, so they give one vector.
    last_constants: Option<Vec<f64>>,
}

#[derive(PartialEq)]
struct Taken {
    from: Vec<String>,
    to: String,
}

impl Compose {
    pub fn vector2d() -> Self {
        Self::new(None)
    }

    pub fn vector3d() -> Self {
        Self::new(Some(Signal::new(String::new())))
    }

    fn new(z: Option<Signal<String>>) -> Self {
        Self {
            settings: ComposeSettings {
                x: Signal::new(String::new()),
                y: Signal::new(String::new()),
                z,
                to_label: Signal::new(String::new()),
                error: Signal::new(None),
            },
            taken: None,
            components: Vec::new(),
            last_constants: None,
        }
    }

    /// The components of a new vector, if there is one.
    fn values(&mut self, data: &DataContext, operands: &[Operand]) -> Option<Vec<f64>> {
        let constants: Option<Vec<f64>> = operands
            .iter()
            .map(|operand| match operand {
                Operand::Constant(value) => Some(*value),
                Operand::Label(_) => None,
            })
            .collect();
        if let Some(constants) = constants {
            for component in &mut self.components {
                component.forget();
            }
            if self.last_constants.as_ref() == Some(&constants) {
                return None;
            }
            self.last_constants = Some(constants.clone());
            return Some(constants);
        }
        self.last_constants = None;

        let mut read = Vec::new();
        for (component, operand) in self.components.iter_mut().zip(operands) {
            match operand {
                Operand::Label(label) => {
                    component.read_newest(data, label);
                    read.push(component);
                }
                Operand::Constant(_) => component.forget(),
            }
        }
        let mut newest = take_newest_all(read)?.into_iter();
        operands
            .iter()
            .map(|operand| match operand {
                Operand::Label(_) => newest.next(),
                Operand::Constant(value) => Some(*value),
            })
            .collect()
    }
}

impl MapRunner for Compose {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn saved(&self) -> &dyn SavedSettings {
        &self.settings
    }

    fn start(&mut self) {
        let ComposeSettings {
            x,
            y,
            z,
            to_label,
            mut error,
        } = self.settings;
        let to = to_label.peek().trim().to_string();
        let from: Vec<String> = [Some(x), Some(y), z]
            .into_iter()
            .flatten()
            .map(|text| text.peek().trim().to_string())
            .collect();
        let taken = (!to.is_empty() && from.iter().all(|from| !from.is_empty()))
            .then_some(Taken { from, to });
        if keep_taken(&mut self.taken, taken) {
            self.components = self
                .taken
                .iter()
                .flat_map(|taken| &taken.from)
                .map(|_| Input::default())
                .collect();
            self.last_constants = None;
        }
        set_if_changed(&mut error, None);
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let mut error = self.settings.error;
        let Taken { from, to } = self.taken.as_ref()?;
        let (from, to) = (from.clone(), to.clone());

        // On each run: a label may come after the map is turned on
        let operands = match from
            .iter()
            .map(|text| Operand::resolve(data, text))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(operands) => operands,
            Err(message) => {
                set_if_changed(&mut error, Some(message));
                return None;
            }
        };
        set_if_changed(&mut error, None);
        // The same label may be two components, but not the output
        if operands.contains(&Operand::Label(to.clone())) {
            return None;
        }

        let values = self.values(data, &operands)?;
        match *values.as_slice() {
            [x, y] => data.push(&to, Vector2dData::new(timestamp, Vector2d::new(x, y))),
            [x, y, z] => data.push(&to, Vector3dData::new(timestamp, Vector3d::new(x, y, z))),
            _ => return None,
        }

        Some(Conversion {
            from: operands
                .iter()
                .zip(&values)
                .map(|(operand, &value)| operand.input(value))
                .collect(),
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(&to)],
                vector_segments(&values),
            )],
        })
    }
}

/// `(x, y)` or `(x, y, z)`, the components from the inputs.
fn vector_segments(values: &[f64]) -> Vec<Segment> {
    let mut segments = vec![Segment::fixed("(")];
    for (index, &value) in values.iter().enumerate() {
        if index > 0 {
            segments.push(Segment::fixed(", "));
        }
        segments.push(Segment::from_input(format_number(value)));
    }
    segments.push(Segment::fixed(")"));
    segments
}

#[cfg(test)]
mod tests {
    use super::vector_segments;
    use crate::data::Segment;

    #[test]
    fn vector_segments_mark_the_components_from_the_inputs() {
        assert_eq!(
            vector_segments(&[1.0, -2.5]),
            vec![
                Segment::fixed("("),
                Segment::from_input("1.0000"),
                Segment::fixed(", "),
                Segment::from_input("-2.5000"),
                Segment::fixed(")"),
            ]
        );
    }
}
