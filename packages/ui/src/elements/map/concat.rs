use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::labels_list;
use crate::data::{Concat, ConcatSettings, DataType, MapKind};

pub const CONCAT: MapKind = MapKind {
    name: "Concat",
    from: &[DataType::Log, DataType::Log],
    to: DataType::Log,
    create: || Box::new(Concat::new()),
    form: |settings| {
        form_of(
            settings,
            |settings: ConcatSettings| rsx! { ConcatForm { settings } },
        )
    },
    presets: &[],
};

#[component]
pub fn ConcatForm(settings: ConcatSettings) -> Element {
    let ConcatSettings {
        first_label,
        second_label,
        first_weakest_level,
        second_weakest_level,
        to_label,
        separator,
        level,
        error,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: first_label,
                    list: labels_list(DataType::Log),
                    placeholder: "First input label",
                    weakest_level: first_weakest_level,
                }
                LabelField {
                    value: second_label,
                    list: labels_list(DataType::Log),
                    placeholder: "Second input label",
                    weakest_level: second_weakest_level,
                }
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                LabelField {
                    value: to_label,
                    list: labels_list(DataType::Log),
                    placeholder: "Output label",
                }
                TextField { name: "Separator", value: separator, placeholder: "None" }
                TextField { name: "Level", value: level, placeholder: "None", error }
                if let Some(error) = error() {
                    span { class: "field-error", "{error}" }
                }
            }
        }
    }
}
