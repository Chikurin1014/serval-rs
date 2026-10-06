use dioxus::prelude::*;
use dioxus_icons::lucide;

use std::any::Any;

use super::map_list::STRINGS_LABELS_LIST_ID;
use crate::{
    components::input::Input,
    data::{Concat, ConcatSettings, DataType, MapKind},
};

const MAP_FORM_CSS: Asset = asset!("/assets/styling/map-form.css");

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
        mut first_label,
        mut second_label,
        mut to_label,
        mut separator,
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
                        placeholder: "First input label",
                        autocomplete: "on",
                        value: "{first_label}",
                        oninput: move |event: FormEvent| first_label.set(event.value()),
                    }
                }
                label {
                    class: "field",
                    lucide::Tag {}
                    Input {
                        list: STRINGS_LABELS_LIST_ID,
                        placeholder: "Second input label",
                        autocomplete: "on",
                        value: "{second_label}",
                        oninput: move |event: FormEvent| second_label.set(event.value()),
                    }
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
                    span { class: "field-label", "Separator" }
                    Input {
                        placeholder: "None",
                        value: "{separator}",
                        oninput: move |event: FormEvent| separator.set(event.value()),
                    }
                }
            }
        }
    }
}
