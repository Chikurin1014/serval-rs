use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::LabelField;
use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::data::{DataType, Encode, EncodeSettings, MapKind};

pub const ENCODE: MapKind = MapKind {
    name: "Encode",
    from: &[DataType::String],
    to: DataType::Bytes,
    create: || Box::new(Encode::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<EncodeSettings>() {
        Some(&settings) => rsx! { EncodeForm { settings } },
        None => VNode::empty(),
    },
    presets: &[],
};

/// Settings form of an `Encode` map.
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
