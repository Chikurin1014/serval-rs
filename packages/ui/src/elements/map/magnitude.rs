use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::{NUMBERS_LABELS_LIST_ID, VECTOR2D_LABELS_LIST_ID, VECTOR3D_LABELS_LIST_ID};
use crate::{
    data::{DataType, Magnitude, MagnitudeSettings, MapKind, Vector2d, Vector3d},
    elements::Formula,
};

pub const MAGNITUDE_2D: MapKind = MapKind {
    name: "Magnitude",
    from: &[DataType::Vector2d],
    to: DataType::Number,
    create: || Box::new(Magnitude::<Vector2d>::new()),
    form: |settings| magnitude_form(settings, VECTOR2D_LABELS_LIST_ID),
    presets: &[],
};

pub const MAGNITUDE_3D: MapKind = MapKind {
    name: "Magnitude",
    from: &[DataType::Vector3d],
    to: DataType::Number,
    create: || Box::new(Magnitude::<Vector3d>::new()),
    form: |settings| magnitude_form(settings, VECTOR3D_LABELS_LIST_ID),
    presets: &[],
};

fn magnitude_form(settings: &dyn std::any::Any, input_list: &'static str) -> Element {
    form_of(settings, |settings: MagnitudeSettings| {
        rsx! {
            MagnitudeForm { settings, input_list: input_list.to_string() }
        }
    })
}

/// Its input's labels in the `datalist` `input_list`.
#[component]
pub fn MagnitudeForm(settings: MagnitudeSettings, input_list: String) -> Element {
    let MagnitudeSettings {
        from_label,
        to_label,
    } = settings;

    rsx! {
        div {
            class: "map-form",
            Formula {
                class: "map-formula",
                latex: "\\lVert \\vec{a} \\rVert".to_string(),
                fallback: "‖a‖".to_string(),
            }
            div {
                class: "map-row",
                LabelField {
                    name: "a",
                    value: from_label,
                    list: input_list,
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
