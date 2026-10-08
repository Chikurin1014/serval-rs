use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{DataType, Encode, EncodeSettings, MapKind};

pub const ENCODE: MapKind = MapKind {
    name: "Encode",
    from: &[DataType::String],
    to: DataType::Bytes,
    create: || Box::new(Encode::new("", "")),
    form: |settings| {
        form_of(
            settings,
            |settings: EncodeSettings| rsx! { EncodeForm { settings } },
        )
    },
    presets: &[],
};

#[component]
pub fn EncodeForm(settings: EncodeSettings) -> Element {
    let EncodeSettings {
        from_label,
        to_label,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            LabelField {
                value: from_label,
                list: STRINGS_LABELS_LIST_ID,
                placeholder: "Input label",
            }
            lucide::MoveRight { size: 20 }
            LabelField {
                value: to_label,
                list: BYTES_LABELS_LIST_ID,
                placeholder: "Output label",
            }
        }
    }
}
