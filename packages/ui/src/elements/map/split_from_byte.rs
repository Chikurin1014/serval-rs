use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::BYTES_LABELS_LIST_ID;
use crate::{
    components::input::Input,
    data::{DataType, MapKind, SplitFromByte, SplitFromByteSettings},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const SPLIT_FROM_BYTE: MapKind = MapKind {
    name: "Split",
    from: DataType::Bytes,
    to: DataType::String,
    create: || Box::new(SplitFromByte::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<SplitFromByteSettings>() {
        Some(&settings) => rsx! { SplitFromByteForm { settings } },
        None => VNode::empty(),
    },
};

/// Settings form of a `SplitFromByte` map.
#[component]
pub fn SplitFromByteForm(settings: SplitFromByteSettings) -> Element {
    let SplitFromByteSettings {
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
                            placeholder: "Source label",
                            autocomplete: "on",
                            value: "{from_label}",
                            oninput: move |event: FormEvent| from_label.set(event.value()),
                        }
                    }
                    label {
                        class: "field",
                        span { class: "field-label", "Delimiter" }
                        Input {
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
                        placeholder: "Target label",
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
            }
        }
    }
}
