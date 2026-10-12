use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::{NUMBERS_LABELS_LIST_ID, VECTOR2D_LABELS_LIST_ID, VECTOR3D_LABELS_LIST_ID};
use crate::data::{Compose, ComposeSettings, DataType, MapKind};

pub const COMPOSE_2D: MapKind = MapKind {
    name: "Compose",
    from: &[DataType::Number, DataType::Number],
    to: DataType::Vector2d,
    create: || Box::new(Compose::vector2d()),
    form: |settings| compose_form(settings, VECTOR2D_LABELS_LIST_ID),
    presets: &[],
};

pub const COMPOSE_3D: MapKind = MapKind {
    name: "Compose",
    from: &[DataType::Number, DataType::Number, DataType::Number],
    to: DataType::Vector3d,
    create: || Box::new(Compose::vector3d()),
    form: |settings| compose_form(settings, VECTOR3D_LABELS_LIST_ID),
    presets: &[],
};

fn compose_form(settings: &dyn std::any::Any, output_list: &'static str) -> Element {
    form_of(settings, |settings: ComposeSettings| {
        rsx! {
            ComposeForm { settings, output_list: output_list.to_string() }
        }
    })
}

#[component]
pub fn ComposeForm(settings: ComposeSettings, output_list: String) -> Element {
    let ComposeSettings {
        x,
        y,
        z,
        to_label,
        error,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    name: "x",
                    value: x,
                    list: NUMBERS_LABELS_LIST_ID,
                    placeholder: "Label or number",
                }
                LabelField {
                    name: "y",
                    value: y,
                    list: NUMBERS_LABELS_LIST_ID,
                    placeholder: "Label or number",
                }
                if let Some(z) = z {
                    LabelField {
                        name: "z",
                        value: z,
                        list: NUMBERS_LABELS_LIST_ID,
                        placeholder: "Label or number",
                    }
                }
                if let Some(error) = error() {
                    span { class: "field-error", "{error}" }
                }
            }
            lucide::MoveRight { size: 20 }
            LabelField {
                value: to_label,
                list: output_list,
                placeholder: "Output label",
            }
        }
    }
}
