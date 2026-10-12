use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::{NUMBERS_LABELS_LIST_ID, VECTOR2D_LABELS_LIST_ID, VECTOR3D_LABELS_LIST_ID};
use crate::{
    data::{
        Arithmetic, ArithmeticSettings, DataType, MapKind, MapRunner, Operation, Vector2d,
        Vector3d, VectorArithmetic,
    },
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
        form: |settings| {
            form_of(
                settings,
                |settings: ArithmeticSettings| rsx! { ArithmeticForm { settings } },
            )
        },
        presets: &[],
    }
}

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

/// A vector map of `operation` on `$from` (vectors of `$vector`, or a number
/// and one).
macro_rules! vector_kind {
    ($name:literal, $operation:ident, $vector:ident, [$first:ident, $second:ident], $list:expr) => {
        MapKind {
            name: $name,
            from: &[DataType::$first, DataType::$second],
            to: DataType::$vector,
            create: || -> Box<dyn MapRunner> {
                Box::new(VectorArithmetic::<$vector>::new(Operation::$operation))
            },
            form: |settings| {
                form_of(settings, |settings: ArithmeticSettings| {
                    rsx! {
                        VectorArithmeticForm { settings, vector_list: $list.to_string() }
                    }
                })
            },
            presets: &[],
        }
    };
}

pub const ADD_2D: MapKind = vector_kind!(
    "Add",
    Add,
    Vector2d,
    [Vector2d, Vector2d],
    VECTOR2D_LABELS_LIST_ID
);
pub const ADD_3D: MapKind = vector_kind!(
    "Add",
    Add,
    Vector3d,
    [Vector3d, Vector3d],
    VECTOR3D_LABELS_LIST_ID
);
pub const SUBTRACT_2D: MapKind = vector_kind!(
    "Subtract",
    Subtract,
    Vector2d,
    [Vector2d, Vector2d],
    VECTOR2D_LABELS_LIST_ID
);
pub const SUBTRACT_3D: MapKind = vector_kind!(
    "Subtract",
    Subtract,
    Vector3d,
    [Vector3d, Vector3d],
    VECTOR3D_LABELS_LIST_ID
);
/// A number times a vector, the number first.
pub const MULTIPLY_2D: MapKind = vector_kind!(
    "Multiply",
    Multiply,
    Vector2d,
    [Number, Vector2d],
    VECTOR2D_LABELS_LIST_ID
);
pub const MULTIPLY_3D: MapKind = vector_kind!(
    "Multiply",
    Multiply,
    Vector3d,
    [Number, Vector3d],
    VECTOR3D_LABELS_LIST_ID
);

/// As [`ArithmeticForm`], on vectors: `a` and `b` both, or `a` a number to
/// multiply `b` by; their labels in the `datalist` `vector_list`.
#[component]
pub fn VectorArithmeticForm(settings: ArithmeticSettings, vector_list: String) -> Element {
    let ArithmeticSettings {
        operation,
        first,
        second,
        to_label,
        error,
    } = settings;
    let by_number = operation == Operation::Multiply;

    rsx! {
        div {
            class: "map-form",
            Formula {
                class: "map-formula",
                latex: operation.vector_latex().to_string(),
                fallback: format!("a {} b", operation.symbol()),
            }
            div {
                class: "map-row",
                div {
                    class: "field-stack",
                    LabelField {
                        name: "a",
                        value: first,
                        list: if by_number { NUMBERS_LABELS_LIST_ID.to_string() } else { vector_list.clone() },
                        placeholder: if by_number { "Label or number" } else { "Label or vector" },
                    }
                    LabelField {
                        name: "b",
                        value: second,
                        list: vector_list.clone(),
                        placeholder: "Label or vector",
                    }
                    if let Some(error) = error() {
                        span { class: "field-error", "{error}" }
                    }
                }
                lucide::MoveRight { size: 20 }
                LabelField {
                    value: to_label,
                    list: vector_list,
                    placeholder: "Output label",
                }
            }
        }
    }
}
