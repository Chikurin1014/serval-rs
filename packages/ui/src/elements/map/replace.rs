use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::STRINGS_LABELS_LIST_ID;
use crate::data::{DataType, MapKind, Replace, ReplaceSettings};

pub const REPLACE: MapKind = MapKind {
    name: "Replace",
    from: &[DataType::String],
    to: DataType::String,
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
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: STRINGS_LABELS_LIST_ID,
                    placeholder: "Input label",
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
                    list: STRINGS_LABELS_LIST_ID,
                    placeholder: "Output label",
                }
                TextField { name: "To", value: replacement }
            }
        }
    }
}
