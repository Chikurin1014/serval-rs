use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::field::{LabelField, TextField};
use super::map_list::STRINGS_LABELS_LIST_ID;
use crate::data::{Concat, ConcatSettings, DataType, MapKind};

pub const CONCAT: MapKind = MapKind {
    name: "Concat",
    from: &[DataType::String, DataType::String],
    to: DataType::String,
    create: || Box::new(Concat::new()),
    form: |settings: &dyn Any| match settings.downcast_ref::<ConcatSettings>() {
        Some(&settings) => rsx! { ConcatForm { settings } },
        None => VNode::empty(),
    },
    presets: &[],
};

/// Settings form of a `Concat` map: the two inputs on the left, the
/// output and what goes between them on the right.
#[component]
pub fn ConcatForm(settings: ConcatSettings) -> Element {
    let ConcatSettings {
        first_label,
        second_label,
        to_label,
        separator,
    } = settings;

    rsx! {
        div {
            class: "map-row",
            div {
                class: "field-stack",
                LabelField {
                    value: first_label,
                    list: STRINGS_LABELS_LIST_ID,
                    placeholder: "First input label",
                }
                LabelField {
                    value: second_label,
                    list: STRINGS_LABELS_LIST_ID,
                    placeholder: "Second input label",
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
                TextField { name: "Separator", value: separator, placeholder: "None" }
            }
        }
    }
}
