use dioxus::prelude::*;
use dioxus_icons::lucide;

mod filter;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardAction, CardContent, CardFooter, CardHeader, CardTitle},
    input::Input,
    tag_group::{Tag, TagGroup, TagList},
};
use crate::data::{DataContext, TypedData, format_number};
use crate::time::TimeContext;

pub use filter::{FilterContext, FilterKind};

const DATA_LIST_CSS: Asset = asset!("/assets/styling/data-list.css");

/// Requires `DataContext`, `FilterContext` and `TimeContext` to be provided by an
/// ancestor.
#[component]
pub fn DataList() -> Element {
    let mut data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let time_context = use_context::<TimeContext>();
    // Read in place: only what is shown is copied out of each queue
    let rows = data_context.with_data(|data| {
        let mut entries = data
            .iter()
            .filter(|(label, _)| filter_context.shows(label))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(label, _)| *label);
        entries
            .into_iter()
            .map(|(label, data)| {
                let label = label.clone();
                let mut row_context = data_context;
                let type_name = data.data_type().name();
                let preview = latest_value_preview(data);
                let time = data.latest_timestamp().map_or_else(
                    || "N/A".to_string(),
                    |timestamp| time_context.format_millis(timestamp),
                );
                rsx! {
                    tr {
                        key: "{label}",
                        th { class: "data-cell", "{label}" }
                        td { class: "data-cell", "{type_name}" }
                        td { class: "data-cell", "{preview}" }
                        td { class: "data-cell", "{time}" }
                        td {
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::IconXs,
                                aria_label: "Delete label",
                                onclick: {
                                    let label = label.clone();
                                    move |_| {
                                        row_context.remove(&label);
                                    }
                                },
                                lucide::X {}
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        document::Link { rel: "stylesheet", href: DATA_LIST_CSS }

        div {
            class: "data-list",
            Card {
                CardHeader {
                    CardTitle { "Data List" }
                    CardAction {
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::IconSm,
                            aria_label: "Clear all",
                            title: "Clear all",
                            onclick: move |_| data_context.clear_all(),
                            lucide::Trash {}
                        }
                    }
                }
                CardContent {
                    table {
                        class: "data-table",
                        thead {
                            tr {
                                th { "Label" }
                                th { "Type" }
                                th { "Latest" }
                                th { "Time" }
                                th {}
                            }
                        }
                        tbody { {rows.into_iter()} }
                    }
                }
                CardFooter {
                    LabelFilter {}
                }
            }
        }
    }
}

/// A regex to show or hide labels by, and the filters as tags beside it.
#[component]
fn LabelFilter() -> Element {
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
                    class: "label-filter-field",
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
                // Which kind of filter the input adds
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
            // In a div of its own, as the tag group takes no class
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
                                // Shown while the tag is hovered
                                Button {
                                    class: "label-filter-remove",
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

/// An open eye for a filter that shows labels, a closed one for one that hides them.
#[component]
fn FilterKindIcon(kind: FilterKind) -> Element {
    match kind {
        FilterKind::Show => rsx! { lucide::Eye {} },
        FilterKind::Hide => rsx! { lucide::EyeOff {} },
    }
}

/// The newest entry as text, or "empty".
fn latest_value_preview(data: &TypedData) -> String {
    let latest = match data {
        TypedData::Number(queue) => queue.back().map(|entry| format_number(*entry.value())),
        TypedData::String(queue) => queue.back().map(|entry| entry.value().clone()),
        TypedData::Bytes(queue) => queue
            .back()
            .map(|entry| String::from_utf8_lossy(entry.value()).into_owned()),
    };
    latest.unwrap_or_else(|| "empty".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Queue;

    #[test]
    fn latest_value_preview_uses_latest_queue_entry() {
        let data = TypedData::Number(Queue::from_iter([
            crate::data::NumberData::new(1, 10.0),
            crate::data::NumberData::new(2, 20.0),
        ]));

        assert_eq!(latest_value_preview(&data), "20.000");
    }

    #[test]
    fn latest_value_preview_handles_string_queue() {
        let data = TypedData::String(Queue::from_iter([
            crate::data::StringData::new(1, "first".to_string()),
            crate::data::StringData::new(2, "second".to_string()),
        ]));

        assert_eq!(latest_value_preview(&data), "second");
    }
}
