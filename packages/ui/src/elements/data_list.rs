use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardAction, CardContent, CardHeader, CardTitle},
};
use crate::data::{DataContext, TypedData};

const DATA_LIST_CSS: Asset = asset!("/assets/styling/data-list.css");

#[component]
pub fn DataList() -> Element {
    let mut data_context = use_context::<DataContext>();
    let rows = data_context
        .data_with_labels()
        .iter()
        .map(|(label, data)| {
            let label = label.clone();
            let mut row_context = data_context;
            let type_name = data.data_type().name();
            let preview = latest_value_preview(data);
            let timestamp = data
                .latest_timestamp()
                .map_or_else(|| "N/A".to_string(), |timestamp| timestamp.to_string());
            rsx! {
                tr {
                    th { class: "data-cell", "{label}" }
                    td { class: "data-cell", "{type_name}" }
                    td { class: "data-cell", "{preview}" }
                    td { class: "data-cell", "{timestamp}" }
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
        .collect::<Vec<_>>();

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
                                th { "Timestamp" }
                                th {}
                            }
                        }
                        tbody { {rows.into_iter()} }
                    }
                }
            }
        }
    }
}

/// The newest entry as text, or "empty".
fn latest_value_preview(data: &TypedData) -> String {
    let latest = match data {
        TypedData::Number(queue) => queue.back().map(|entry| entry.value().to_string()),
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
    use std::collections::VecDeque;

    #[test]
    fn latest_value_preview_uses_latest_queue_entry() {
        let data = TypedData::Number(VecDeque::from([
            crate::data::NumberData::new(1, 10.0),
            crate::data::NumberData::new(2, 20.0),
        ]));

        assert_eq!(latest_value_preview(&data), "20");
    }

    #[test]
    fn latest_value_preview_handles_string_queue() {
        let data = TypedData::String(VecDeque::from([
            crate::data::StringData::new(1, "first".to_string()),
            crate::data::StringData::new(2, "second".to_string()),
        ]));

        assert_eq!(latest_value_preview(&data), "second");
    }
}
