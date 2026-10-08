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
                        DataRows {}
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

/// The rows, one for each label shown. Its own component: it renders again with
/// every new value, and the list around it, with its buttons, must not.
#[component]
fn DataRows() -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let time_context = use_context::<TimeContext>();
    let expanded = use_signal(HashSet::<String>::new);
    // Plain values only, so a row renders again only when they change
    let rows = data_context.with_each(None, |data| {
        data.iter()
            .filter(|(label, _)| filter_context.shows(label))
            .map(|&(label, data)| DataRowProps {
                label: label.to_string(),
                type_name: data.data_type().name(),
                preview: latest_value_preview(data),
                time: data.latest_timestamp().map_or_else(
                    || "N/A".to_string(),
                    |timestamp| time_context.format_millis(timestamp),
                ),
                // Even while collapsed, so they can slide open
                history: values_before_latest(data, HISTORY_LENGTH)
                    .into_iter()
                    .map(|(timestamp, value)| (time_context.format_millis(timestamp), value))
                    .collect(),
                expanded,
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        for row in rows {
            DataRow { key: "{row.label}", ..row }
        }
    }
}

/// A label's row, and its earlier values below it.
#[component]
fn DataRow(
    label: String,
    type_name: &'static str,
    preview: String,
    time: String,
    /// `(time, value)`, newest first.
    history: Vec<(String, String)>,
    expanded: Signal<HashSet<String>>,
) -> Element {
    let is_expanded = expanded.read().contains(&label);
    let toggle = {
        let label = label.clone();
        move |_| {
            let mut expanded = expanded.write();
            if !expanded.remove(&label) {
                expanded.insert(label.clone());
            }
        }
    };

    rsx! {
        tbody {
            class: "data-group",
            "data-expanded": is_expanded,
            tr {
                class: "data-row reveals",
                "data-expanded": is_expanded,
                onclick: toggle,
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
                    DeleteLabel { label: label.clone() }
                }
            }
            for (time, value) in history {
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
                        HistorySlide { "{time}" }
                    }
                    td {}
                }
            }
        }
    }
}

/// Its own component, so it renders again only for another label: a row
/// renders again with each new value, and each render would leave behind a new
/// callback for `Button`.
#[component]
fn DeleteLabel(label: String) -> Element {
    let mut data_context = use_context::<DataContext>();

    rsx! {
        Button {
            class: "data-row-delete reveal-on-hover",
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconXs,
            aria_label: "Delete label",
            onclick: move |event: MouseEvent| {
                event.stop_propagation();
                data_context.remove(&label);
            },
            lucide::X {}
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
