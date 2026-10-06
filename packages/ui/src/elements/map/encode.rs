use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::{BYTES_LABELS_LIST_ID, STRINGS_LABELS_LIST_ID};
use crate::{
    components::input::Input,
    data::{DataType, Encode, EncodeSettings, MapKind},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const ENCODE: MapKind = MapKind {
    name: "Encode",
    from: &[DataType::String],
    to: DataType::Bytes,
    create: || Box::new(Encode::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<EncodeSettings>() {
        Some(&settings) => rsx! { EncodeForm { settings } },
        None => VNode::empty(),
    },
};

/// Settings form of an `Encode` map.
#[component]
pub fn EncodeForm(settings: EncodeSettings) -> Element {
    let EncodeSettings {
        mut from_label,
        mut to_label,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-form",
            div {
                class: "map-row",
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        list: STRINGS_LABELS_LIST_ID,
                        placeholder: "Input label",
                        autocomplete: "on",
                        value: "{from_label}",
                        oninput: move |event: FormEvent| from_label.set(event.value()),
                    }
                }
                lucide::MoveRight { size: 20 }
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        list: BYTES_LABELS_LIST_ID,
                        placeholder: "Output label",
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
            }
        }
    }
}
