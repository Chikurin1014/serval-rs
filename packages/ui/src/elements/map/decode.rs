use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{DataType, Decode, DecodeSettings, MapKind};

pub const DECODE: MapKind = MapKind {
    name: "Decode",
    from: &[DataType::Bytes],
    to: DataType::String,
    create: || Box::new(Decode::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<DecodeSettings>() {
        Some(&settings) => rsx! { DecodeForm { settings } },
        None => VNode::empty(),
    },
    presets: &[],
};

/// Settings form of a `Decode` map.
#[component]
pub fn DecodeForm(settings: DecodeSettings) -> Element {
    let DecodeSettings {
        from_label,
        to_label,
        delimiter,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: from_label,
                    list: BYTES_LABELS_LIST_ID,
                    placeholder: "Input label",
                }
                // Without one, each entry becomes a string as it is
                TextField { name: "Delimiter", value: delimiter, placeholder: "None" }
            }
            lucide::MoveRight { size: 20 }
            LabelField {
                value: to_label,
                list: STRINGS_LABELS_LIST_ID,
                placeholder: "Output label",
            }
        }
    }
}
