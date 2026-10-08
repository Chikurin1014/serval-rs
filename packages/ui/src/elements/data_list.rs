use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_icons::lucide;

mod export;
mod filter;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardAction, CardContent, CardFooter, CardHeader, CardTitle},
};
use crate::data::{DataContext, NumberText, TypedData};
use crate::time::TimeContext;

use export::ExportCsvButton;
use filter::LabelFilter;
pub use filter::{FilterContext, FilterKind};

const DATA_LIST_CSS: Asset = asset!("/assets/styling/data-list.css");

/// How many earlier values an expanded row shows.
const HISTORY_LENGTH: usize = 4;

/// Requires `DataContext`, `FilterContext` and `TimeContext`.
#[component]
pub fn DataList() -> Element {
    let mut data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let time_context = use_context::<TimeContext>();
    let mut expanded = use_signal(HashSet::<String>::new);
    let rows = data_context.with_each(None, |data| {
        data.iter()
            .filter(|(label, _)| filter_context.shows(label))
            .map(|&(label, data)| {
                let label = label.to_string();
                let mut row_context = data_context;
                let type_name = data.data_type().name();
                let preview = latest_value_preview(data);
                let time = data.latest_timestamp().map_or_else(
                    || "N/A".to_string(),
                    |timestamp| time_context.format_millis(timestamp),
                );
                let is_expanded = expanded.read().contains(&label);
                // Even while collapsed, so they can slide open
                let history = values_before_latest(data, HISTORY_LENGTH);
                let mut toggle = {
                    let label = label.clone();
                    move || {
                        let mut expanded = expanded.write();
                        if !expanded.remove(&label) {
                            expanded.insert(label.clone());
                        }
                    }
                };
                rsx! {
                    tbody {
                        key: "{label}",
                        class: "data-group",
                        "data-expanded": is_expanded,
                        tr {
                            class: "data-row reveals",
                            "data-expanded": is_expanded,
                            onclick: move |_| toggle(),
                            th {
                                class: "data-cell",
                                button {
                                    class: "data-row-toggle",
                                    r#type: "button",
                                    aria_expanded: is_expanded,
                                    title: "{label}",
                                    if is_expanded {
                                        lucide::ChevronDown {}
                                    } else {
                                        lucide::ChevronRight {}
                                    }
                                    span { "{label}" }
                                }
                            }
                            td { class: "data-cell", "{type_name}" }
                            td { class: "data-cell", title: "{preview}", "{preview}" }
                            td { class: "data-cell", "{time}" }
                            td {
                                Button {
                                    class: "data-row-delete reveal-on-hover",
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::IconXs,
                                    aria_label: "Delete label",
                                    onclick: {
                                        let label = label.clone();
                                        move |event: MouseEvent| {
                                            event.stop_propagation();
                                            row_context.remove(&label);
                                        }
                                    },
                                    lucide::X {}
                                }
                            }
                        }
                        for (timestamp, value) in history {
                            tr {
                                class: "data-history",
                                aria_hidden: !is_expanded,
                                th {}
                                td {}
                                td {
                                    class: "data-cell",
                                    title: "{value}",
                                    HistorySlide { "{value}" }
                                }
                                td {
                                    class: "data-cell",
                                    HistorySlide { "{time_context.format_millis(timestamp)}" }
                                }
                                td {}
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
                            onclick: move |_| {
                                data_context.clear_all();
                            },
                            lucide::Trash {}
                        }
                    }
                }
                CardContent {
                    table {
                        class: "data-table",
                        // Fixed widths, so columns stay put as values change
                        colgroup {
                            col { class: "data-col-label" }
                            col { class: "data-col-type" }
                            col { class: "data-col-latest" }
                            col { class: "data-col-time" }
                            col { class: "data-col-delete" }
                        }
                        thead {
                            tr {
                                th { "Label" }
                                th { "Type" }
                                th { "Latest" }
                                th { "Time" }
                                th {}
                            }
                        }
                        {rows.into_iter()}
                    }
                }
                CardFooter {
                    LabelFilter {}
                    ExportCsvButton {}
                }
            }
        }
    }
}

#[component]
fn HistorySlide(children: Element) -> Element {
    rsx! {
        div {
            class: "data-history-slide",
            div { {children} }
        }
    }
}

fn latest_value_preview(data: &TypedData) -> String {
    data.newest_as_text(1, NumberText::Rounded)
        .pop()
        .map_or_else(|| "empty".to_string(), |(_, text)| text)
}

/// The `count` entries before the newest, newest first.
fn values_before_latest(data: &TypedData, count: usize) -> Vec<(i64, String)> {
    let mut values = data.newest_as_text(count + 1, NumberText::Rounded);
    values.pop();
    values.reverse();
    values
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

    #[test]
    fn values_before_latest_are_the_newest_first() {
        let data = TypedData::String(Queue::from_iter(
            (1..=7).map(|n| crate::data::StringData::new(n, n.to_string())),
        ));

        let values = values_before_latest(&data, 4);
        let expected = (3..=6)
            .rev()
            .map(|n| (n, n.to_string()))
            .collect::<Vec<_>>();
        assert_eq!(values, expected);
        assert_eq!(values_before_latest(&data, 10).len(), 6);
    }
}
