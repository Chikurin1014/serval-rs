use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{LabelField, TextField};
use super::form_of;
use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{DataType, Decode, DecodeSettings, MapKind};

pub const DECODE: MapKind = MapKind {
    name: "Decode",
    from: &[DataType::Bytes],
    to: DataType::String,
    create: || Box::new(Decode::new("", "")),
    form: |settings| {
        form_of(
            settings,
            |settings: DecodeSettings| rsx! { DecodeForm { settings } },
        )
    },
    presets: &[],
};

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
