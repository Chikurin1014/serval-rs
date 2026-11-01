use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    input::Input,
    tag_group::{Tag, TagGroup, TagList},
};

pub use logic::filter::{FilterContext, FilterKind};

#[component]
pub(super) fn LabelFilter() -> Element {
    let mut filter_context = use_context::<FilterContext>();
    let mut kind = use_signal(|| FilterKind::Show);
    let mut pattern = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut add = move || {
        let value = pattern();
        if value.is_empty() {
            return;
        }
        match filter_context.add(kind(), &value) {
            Ok(()) => pattern.set(String::new()),
            Err(message) => error.set(Some(message)),
        }
    };

    rsx! {
        div {
            class: "label-filter",
            div {
                class: "label-filter-input",
                label {
                    class: "field label-filter-field",
                    lucide::Funnel {}
                    Input {
                        placeholder: "Label filter",
                        value: "{pattern}",
                        oninput: move |event: FormEvent| {
                            pattern.set(event.value());
                            error.set(None);
                        },
                        onkeydown: move |event: KeyboardEvent| {
                            if event.key() == Key::Enter {
                                add();
                            }
                        },
                    }
                }
                Button {
                    class: "label-filter-kind",
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    "data-kind": kind().name(),
                    title: "Labels matching it are shown or hidden",
                    onclick: move |_| {
                        kind.set(match kind() {
                            FilterKind::Show => FilterKind::Hide,
                            FilterKind::Hide => FilterKind::Show,
                        });
                    },
                    FilterKindIcon { kind: kind() }
                    "{kind().name()}"
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::IconSm,
                    aria_label: "Add filter",
                    title: "Add filter",
                    onclick: move |_| add(),
                    lucide::Plus {}
                }
            }
            // The tag group takes no class
            div {
                class: "label-filter-tags",
                TagGroup {
                    selectable: false,
                    aria_label: "Label filters",
                    TagList {
                        for (index, (kind, pattern)) in filter_context.filters().into_iter().enumerate() {
                            Tag {
                                key: "{kind.name()}-{pattern}",
                                index,
                                value: format!("{}-{pattern}", kind.name()),
                                "data-kind": kind.name(),
                                FilterKindIcon { kind }
                                span { "{pattern}" }
                                Button {
                                    class: "label-filter-remove reveal-on-hover",
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::IconXs,
                                    aria_label: "Remove {kind.name()} filter {pattern}",
                                    title: "Remove filter",
                                    onclick: move |event: MouseEvent| {
                                        event.stop_propagation();
                                        filter_context.remove(kind, &pattern);
                                    },
                                    lucide::X {}
                                }
                            }
                        }
                    }
                }
            }
            if let Some(error) = error() {
                p { class: "label-filter-error", "{error}" }
            }
        }
    }
}

#[component]
fn FilterKindIcon(kind: FilterKind) -> Element {
    match kind {
        FilterKind::Show => rsx! { lucide::Eye {} },
        FilterKind::Hide => rsx! { lucide::EyeOff {} },
    }
}
