use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::STRINGS_LABELS_LIST_ID;
use crate::{
    components::input::Input,
    data::{DataType, MapKind, Replace, ReplaceSettings},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

pub const REPLACE: MapKind = MapKind {
    name: "Replace",
    from: &[DataType::String],
    to: DataType::String,
    create: || Box::new(Replace::new()),
    form: |settings: &dyn Any| match settings.downcast_ref::<ReplaceSettings>() {
        Some(&settings) => rsx! { ReplaceForm { settings } },
        None => VNode::empty(),
    },
};

/// Settings form of a `Replace` map, laid out as the `Regex` one.
#[component]
pub fn ReplaceForm(settings: ReplaceSettings) -> Element {
    let ReplaceSettings {
        mut from_label,
        mut to_label,
        mut pattern,
        mut replacement,
        mut pattern_error,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_FORM_CSS }

        div {
            class: "map-row",
            div {
                class: "field-stack",
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
                label {
                    class: "field",
                    span { class: "field-label", "From" }
                    lucide::Regex {}
                    Input {
                        placeholder: "Text to be replaced",
                        value: "{pattern}",
                        oninput: move |event: FormEvent| {
                            pattern.set(event.value());
                            pattern_error.set(None);
                        },
                    }
                }
                if let Some(error) = pattern_error() {
                    span { class: "field-error", "{error}" }
                }
            }
            lucide::MoveRight { size: 20 }
            div {
                class: "field-stack",
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        placeholder: "Output label",
                        list: STRINGS_LABELS_LIST_ID,
                        value: "{to_label}",
                        oninput: move |event: FormEvent| to_label.set(event.value()),
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "To" }
                    Input {
                        value: "{replacement}",
                        oninput: move |event: FormEvent| replacement.set(event.value()),
                    }
                }
            }
        }
    }
}
