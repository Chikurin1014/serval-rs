use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdTrash, LdX},
    Icon,
};

use crate::data::{DataContext, TypedData};

#[component]
pub fn DataList() -> Element {
    let data_context = use_context::<DataContext>();
    let mut all_clear_context = data_context.clone();
    let rows = data_context
        .data_with_labels
        .read()
        .iter()
        .map(|(label, data)| {
            let label = label.clone();
            let mut row_context = data_context.clone();
            let type_name = data.type_name();
            let preview = latest_value_preview(data);
            let timestamp = latest_timestamp(data);
            rsx! {
                tr {
                    th { class: "font-mono text-xs", "{label}" }
                    td { class: "font-mono text-xs", "{type_name}" }
                    td { class: "font-mono text-xs", "{preview}" }
                    td { class: "font-mono text-xs", "{timestamp}" }
                    td {
                        button {
                            class: "btn btn-xs btn-ghost btn-error btn-square",
                            aria_label: "Delete label",
                            onclick: {
                                let label = label.clone();
                                move |_| {
                                    row_context.remove(&label);
                                }
                            },
                            Icon { icon: LdX {} }
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    rsx! {
        div {
            class: "card bg-base-100 shadow-sm w-full",
            div {
                class: "card-body overflow-auto",
                h3 {
                    class: "card-title text-sm",
                    "Data List"
                }
                div {
                    class: "overflow-auto",
                    table {
                        class: "table table-xs table-zebra table-pin-rows table-pin-cols",
                        thead {
                            tr {
                                th { "Label" }
                                th { "Type" }
                                th { "Latest" }
                                th { "Timestamp" }
                                th {
                                    div {
                                        class: "tooltip tooltip-right tooltip-error",
                                       "data-tip": "Clear all",
                                       button {
                                           class: "btn btn-xs btn-error btn-ghost btn-square",
                                           onclick: move |_| all_clear_context.clear_all(),
                                           Icon { icon: LdTrash {} }
                                       }
                                    }
                                }
                            }
                        }
                        tbody { {rows.into_iter()} }
                    }
                }
            }
        }
    }
}

fn latest_value_preview(data: &TypedData) -> String {
    match data {
        TypedData::Number(queue) => queue
            .back()
            .map(|entry| format!("{}", entry.value()))
            .unwrap_or_else(|| "empty".to_string()),
        TypedData::String(queue) => queue
            .back()
            .map(|entry| entry.value().clone())
            .unwrap_or_else(|| "empty".to_string()),
        TypedData::Bytes(queue) => queue
            .back()
            .map(|entry| String::from_utf8_lossy(entry.value()).into_owned())
            .unwrap_or_else(|| "empty".to_string()),
    }
}

fn latest_timestamp(data: &TypedData) -> String {
    match data {
        TypedData::Number(queue) => queue
            .back()
            .map(|entry| format!("{}", entry.timestamp()))
            .unwrap_or_else(|| "N/A".to_string()),
        TypedData::String(queue) => queue
            .back()
            .map(|entry| format!("{}", entry.timestamp()))
            .unwrap_or_else(|| "N/A".to_string()),
        TypedData::Bytes(queue) => queue
            .back()
            .map(|entry| format!("{}", entry.timestamp()))
            .unwrap_or_else(|| "N/A".to_string()),
    }
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
