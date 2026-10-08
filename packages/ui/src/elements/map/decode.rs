use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::{ChoicesField, LabelField};
use super::form_of;
use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::{
    data::{DataType, Decode, DecodeSettings, MapKind},
    helper::{Delimiter, single_line},
};

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
        mut delimiters,
    } = settings;
    let chosen = delimiters
        .read()
        .iter()
        .filter_map(|delimiter| Delimiter::ALL.iter().position(|all| all == delimiter))
        .collect::<Vec<_>>();

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
                ChoicesField {
                    name: "Delimiter",
                    options: Delimiter::ALL.map(|delimiter| single_line(delimiter.text())).to_vec(),
                    chosen,
                    on_change: move |chosen: Vec<usize>| {
                        delimiters.set(chosen.into_iter().map(|index| Delimiter::ALL[index]).collect());
                    },
                }
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
