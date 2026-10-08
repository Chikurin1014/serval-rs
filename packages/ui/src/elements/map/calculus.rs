use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::NUMBERS_LABELS_LIST_ID;
use crate::{
    data::{Calculus, CalculusMap, CalculusSettings, DataType, MapKind, MapRunner},
    elements::Formula,
};

pub const DIFFERENTIATE: MapKind = calculus_kind("Differentiate", || {
    Box::new(CalculusMap::new(Calculus::Differentiate))
});
pub const INTEGRATE: MapKind = calculus_kind("Integrate", || {
    Box::new(CalculusMap::new(Calculus::Integrate))
});

const fn calculus_kind(name: &'static str, create: fn() -> Box<dyn MapRunner>) -> MapKind {
    MapKind {
        name,
        from: &[DataType::Number],
        to: DataType::Number,
        create,
        form: |settings| {
            form_of(
                settings,
                |settings: CalculusSettings| rsx! { CalculusForm { settings } },
            )
        },
        presets: &[],
    }
}

/// Settings form of a `CalculusMap`: its formula over the form, as
/// `Arithmetic`'s, then the input `f(t)` and the output.
#[component]
pub fn CalculusForm(settings: CalculusSettings) -> Element {
    let CalculusSettings {
        calculus,
        from_label,
        to_label,
    } = settings;

    rsx! {
        div {
            class: "map-form",
            Formula {
                class: "map-formula",
                latex: calculus.latex().to_string(),
                fallback: calculus.text().to_string(),
            }
            div {
                class: "map-row",
                LabelField {
                    name: "f(t)",
                    value: from_label,
                    list: NUMBERS_LABELS_LIST_ID,
                    placeholder: "Input label",
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
