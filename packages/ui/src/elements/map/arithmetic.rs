use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::LabelField;
use super::map_list::NUMBERS_LABELS_LIST_ID;
use crate::{
    data::{Arithmetic, ArithmeticSettings, DataType, MapKind, Operation},
    elements::Formula,
};

pub const ADD: MapKind = arithmetic_kind("Add", || Box::new(Arithmetic::new(Operation::Add)));
pub const SUBTRACT: MapKind = arithmetic_kind("Subtract", || {
    Box::new(Arithmetic::new(Operation::Subtract))
});
pub const MULTIPLY: MapKind = arithmetic_kind("Multiply", || {
    Box::new(Arithmetic::new(Operation::Multiply))
});
pub const DIVIDE: MapKind =
    arithmetic_kind("Divide", || Box::new(Arithmetic::new(Operation::Divide)));

const fn arithmetic_kind(
    name: &'static str,
    create: fn() -> Box<dyn crate::data::MapRunner>,
) -> MapKind {
    MapKind {
        name,
        from: &[DataType::Number, DataType::Number],
        to: DataType::Number,
        create,
        form: |settings: &dyn Any| match settings.downcast_ref::<ArithmeticSettings>() {
            Some(&settings) => rsx! { ArithmeticForm { settings } },
            None => VNode::empty(),
        },
        presets: &[],
    }
}

/// Settings form of an `Arithmetic` map: the formula (`a + b`) over the
/// form, then the operands `a` and `b` one above the other.
#[component]
pub fn ArithmeticForm(settings: ArithmeticSettings) -> Element {
    let ArithmeticSettings {
        operation,
        first,
        second,
        to_label,
        error,
    } = settings;

    rsx! {
        div {
            class: "map-form",
            // Centred over the whole form
            Formula {
                class: "map-formula",
                latex: operation.latex().to_string(),
                fallback: format!("a {} b", operation.symbol()),
            }
            div {
                class: "map-row",
                div {
                    class: "field-stack",
                    LabelField {
                        name: "a",
                        value: first,
                        list: NUMBERS_LABELS_LIST_ID,
                        placeholder: "Label or number",
                    }
                    LabelField {
                        name: "b",
                        value: second,
                        list: NUMBERS_LABELS_LIST_ID,
                        placeholder: "Label or number",
                    }
                    if let Some(error) = error() {
                        span { class: "field-error", "{error}" }
                    }
                }
                lucide::MoveRight { size: 20 }
                LabelField {
                    value: to_label,
                    list: NUMBERS_LABELS_LIST_ID,
                    placeholder: "Output label",
                }
            }
        }
    }
}
