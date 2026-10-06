use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::NUMBERS_LABELS_LIST_ID;
use crate::{
    components::input::Input,
    data::{Calculus, CalculusMap, CalculusSettings, DataType, MapKind, MapRunner},
    elements::Formula,
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

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
        form: |settings: &dyn Any| match settings.downcast_ref::<CalculusSettings>() {
            Some(&settings) => rsx! { CalculusForm { settings } },
            None => VNode::empty(),
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
        mut from_label,
        mut to_label,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-form",
            Formula {
                class: "map-formula",
                latex: calculus.latex().to_string(),
                fallback: calculus.text().to_string(),
            }
            div {
                class: "map-row",
                label {
                    class: "field",
                    span { class: "field-label", "f(t)" }
                    lucide::Tag {}
                    Input {
                        list: NUMBERS_LABELS_LIST_ID,
                        placeholder: "Input label",
                        autocomplete: "on",
                        value: "{from_label}",
                        oninput: move |event: FormEvent| from_label.set(event.value()),
                    }
                }
                lucide::MoveRight { size: 20 }
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        placeholder: "Output label",
                        list: NUMBERS_LABELS_LIST_ID,
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
            }
        }
    }
}
