use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::labels_list;
use crate::data::{DataType, Encode, EncodeSettings, MapKind};

pub const ENCODE: MapKind = MapKind {
    name: "Encode",
    from: &[DataType::Log],
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
        weakest_level,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            LabelField {
                value: from_label,
                list: labels_list(DataType::Log),
                placeholder: "Input label",
                weakest_level,
            }
            lucide::MoveRight { size: 20 }
            LabelField {
                value: to_label,
                list: labels_list(DataType::Bytes),
                placeholder: "Output label",
            }
        }
    }
}
