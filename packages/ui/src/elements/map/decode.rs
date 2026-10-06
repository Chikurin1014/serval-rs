use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::BYTES_LABELS_LIST_ID;
use crate::{
    components::input::Input,
    data::{DataType, Decode, DecodeSettings, MapKind},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const DECODE: MapKind = MapKind {
    name: "Decode",
    from: &[DataType::Bytes],
    to: DataType::String,
    create: || Box::new(Decode::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<DecodeSettings>() {
        Some(&settings) => rsx! { DecodeForm { settings } },
        None => VNode::empty(),
    },
};

/// Settings form of a `Decode` map.
#[component]
pub fn DecodeForm(settings: DecodeSettings) -> Element {
    let DecodeSettings {
        mut from_label,
        mut to_label,
        mut delimiter,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-form",
            div {
                class: "map-row",
                div {
                    class: "field-stack",
                    label {
                        class: "field",
                        lucide::Tag {}
                        Input {
                            list: BYTES_LABELS_LIST_ID,
                            placeholder: "Input label",
                            autocomplete: "on",
                            value: "{from_label}",
                            oninput: move |event: FormEvent| from_label.set(event.value()),
                        }
                    }
                    label {
                        class: "field",
                        span { class: "field-label", "Delimiter" }
                        Input {
                            // Without one, each entry becomes a string as it is
                            placeholder: "None",
                            value: "{delimiter}",
                            oninput: move |event: FormEvent| delimiter.set(event.value()),
                        }
                    }
                }
                lucide::MoveRight { size: 20 }
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        placeholder: "Output label",
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
            }
        }
    }
}
