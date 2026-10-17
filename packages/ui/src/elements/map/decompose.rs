use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::labels_list;
use crate::data::{DataType, Decompose, DecomposeSettings, MapKind, Vector2d, Vector3d};

pub const DECOMPOSE_2D: MapKind = MapKind {
    name: "Decompose",
    from: &[DataType::Vector2d],
    to: DataType::Number,
    create: || Box::new(Decompose::<Vector2d>::vector2d()),
    form: |settings| decompose_form(settings, labels_list(DataType::Vector2d)),
    presets: &[],
};

pub const DECOMPOSE_3D: MapKind = MapKind {
    name: "Decompose",
    from: &[DataType::Vector3d],
    to: DataType::Number,
    create: || Box::new(Decompose::<Vector3d>::vector3d()),
    form: |settings| decompose_form(settings, labels_list(DataType::Vector3d)),
    presets: &[],
};

fn decompose_form(settings: &dyn std::any::Any, input_list: &'static str) -> Element {
    form_of(settings, |settings: DecomposeSettings| {
        rsx! {
            DecomposeForm { settings, input_list: input_list.to_string() }
        }
    })
}

/// Its input's labels in the `datalist` `input_list`; an output label for each
/// component, left empty for none.
#[component]
pub fn DecomposeForm(settings: DecomposeSettings, input_list: String) -> Element {
    let DecomposeSettings {
        from_label,
        x_label,
        y_label,
        z_label,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            LabelField {
                value: from_label,
                list: input_list,
                placeholder: "Input label",
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                LabelField {
                    name: "x",
                    value: x_label,
                    list: labels_list(DataType::Number),
                    placeholder: "Output label",
                }
                LabelField {
                    name: "y",
                    value: y_label,
                    list: labels_list(DataType::Number),
                    placeholder: "Output label",
                }
                if let Some(z_label) = z_label {
                    LabelField {
                        name: "z",
                        value: z_label,
                        list: labels_list(DataType::Number),
                        placeholder: "Output label",
                    }
                }
            }
        }
    }
}
