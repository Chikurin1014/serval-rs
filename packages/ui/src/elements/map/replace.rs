use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::labels_list;
use crate::data::{DataType, MapKind, Replace, ReplaceSettings};

pub const REPLACE: MapKind = MapKind {
    name: "Replace",
    from: &[DataType::Log],
    to: DataType::Log,
    create: || Box::new(Replace::new()),
    form: |settings| {
        form_of(
            settings,
            |settings: ReplaceSettings| rsx! { ReplaceForm { settings } },
        )
    },
    presets: &[],
};

#[component]
pub fn ReplaceForm(settings: ReplaceSettings) -> Element {
    let ReplaceSettings {
        from_label,
        to_label,
        pattern,
        replacement,
        pattern_error,
        weakest_level,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: labels_list(DataType::Log),
                    placeholder: "Input label",
                    weakest_level,
                }
                TextField {
                    name: "From",
                    icon: rsx! { lucide::Regex {} },
                    value: pattern,
                    placeholder: "Text to be replaced",
                    error: pattern_error,
                }
                if let Some(error) = pattern_error() {
                    span { class: "field-error", "{error}" }
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
                TextField { name: "To", value: replacement }
            }
        }
    }
}
