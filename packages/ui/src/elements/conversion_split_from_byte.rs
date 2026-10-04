use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoveRight, LdTag},
    Icon,
};

use std::any::Any;

use crate::{
    components::input::Input,
    data::{ConversionKind, DataType, SplitFromByte, SplitFromByteSettings},
};

const CONVERSION_CSS: Asset = asset!("/assets/styling/conversion.css");

pub const SPLIT_FROM_BYTE: ConversionKind = ConversionKind {
    name: "Split (from Byte)",
    from: DataType::Bytes,
    to: DataType::String,
    create: || Box::new(SplitFromByte::new("", "")),
    form: |settings: &dyn Any| match settings.downcast_ref::<SplitFromByteSettings>() {
        Some(&settings) => rsx! { SplitFromByteForm { settings } },
        None => VNode::empty(),
    },
};

/// Settings form of a `SplitFromByte` conversion.
#[component]
pub fn SplitFromByteForm(settings: SplitFromByteSettings) -> Element {
    let SplitFromByteSettings {
        mut from_label,
        mut to_label,
        mut delimiter,
    } = settings;

    rsx! {
        document::Link { rel: "stylesheet", href: CONVERSION_CSS }

        div {
            class: "conversion-form",
            div {
                class: "conversion-row",
                div {
                    class: "field-stack",
                    label {
                        class: "field",
                        Icon { icon: LdTag {} }
                        Input {
                            list: "conversion-bytes-labels", // Defined in `ConversionList` component
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
                Icon { icon: LdMoveRight {} }
                label {
                    class: "field",
                    Icon { icon: LdTag {} }
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
