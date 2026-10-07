use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField, use_map_enabled};
use super::map_list::STRINGS_LABELS_LIST_ID;
use crate::data::{DataType, MapKind, Replace, ReplaceSettings};

pub const REPLACE: MapKind = MapKind {
    name: "Replace",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(Replace::new()),
    form: |settings: &dyn Any| match settings.downcast_ref::<ReplaceSettings>() {
        Some(&settings) => rsx! { ReplaceForm { settings } },
        None => VNode::empty(),
    },
    presets: &[],
};

/// Settings form of a `Replace` map, laid out as the `Regex` one.
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
                    // Compiled when the map is turned on, so kept as it is while on
                    locked: use_map_enabled(),
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
