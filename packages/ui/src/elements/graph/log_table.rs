use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    GraphContext, GraphFrame, GraphKind, GraphPreset, GraphProperty, GraphSettings,
    context::{until_paused, use_graph_setting, use_variant_setting},
    plot::{PlotAssets, series_color},
};
use crate::components::{toggle::Toggle, virtual_list::VirtualList};
use crate::data::{DataContext, DataType, LogData, LogLevel, Queue, TypedData};
use crate::elements::{FilterContext, LogLevelTag};
use crate::helper::use_script;
use crate::time::TimeContext;

const LOG_TABLE_CSS: Asset = asset!("/assets/styling/log-table.css");
const LOG_TABLE_JS: &str = include_str!("log_table.js");

/// A row's height in CSS pixels, as `log-table.css` sets it: each the same, so
/// none needs measuring before it is shown.
const ROW_HEIGHT: u32 = 24;

pub const LOG_TABLE: GraphKind = GraphKind {
    name: "Log",
    menu: "Text",
    view: |id| rsx! { LogTable { id } },
    settings: || GraphSettings::LogTable(LogTableSettings::default()),
    // Itself, its logs of INFO and those that matter more from the start
    presets: &[GraphPreset {
        name: "Log",
        property: || {
            GraphProperty::new(GraphSettings::LogTable(LogTableSettings {
                weakest_level: LogLevel::Info,
            }))
        },
    }],
};

/// A log table's own settings, each missing in a layout file as by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogTableSettings {
    /// The weakest level it shows: those weaker left out.
    pub weakest_level: LogLevel,
}

/// A log of the table, by where it is: its time, its label's place among those
/// shown, and its place in that label's queue (its message read only as it is
/// shown).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    timestamp: i64,
    label: usize,
    index: usize,
}

/// How many logs of each level, in the order of [`LogLevel::ALL`].
type Counts = [usize; LogLevel::ALL.len()];

/// The rows of a table, and how many logs of each level are of its labels
/// (those of levels left out too).
#[derive(Clone, Debug, Default, PartialEq)]
struct LogView {
    rows: Vec<Row>,
    counts: Counts,
}

/// The logs of `queues` (each a label's place and its logs, oldest first) of
/// `weakest` or a level that matters more, up to `paused_at` if paused: by
/// time, newest first (of one time, the last label's first); and how many of
/// each level there are up to then, all in one pass.
fn merged_rows(
    queues: &[(usize, &Queue<LogData>)],
    weakest: LogLevel,
    paused_at: Option<i64>,
) -> LogView {
    let mut counts = Counts::default();
    let mut each: Vec<_> = queues
        .iter()
        .map(|&(label, queue)| {
            let counts = &mut counts;
            queue
                .iter()
                .enumerate()
                .filter(move |(_, entry)| {
                    if !until_paused(paused_at, entry.timestamp()) {
                        return false;
                    }
                    let level = entry.value().level();
                    if let Some(at) = LogLevel::ALL.iter().position(|each| *each == level) {
                        counts[at] += 1;
                    }
                    level <= weakest
                })
                .map(move |(index, entry)| Row {
                    timestamp: entry.timestamp(),
                    label,
                    index,
                })
                .collect::<Vec<_>>()
                .into_iter()
                .peekable()
        })
        .collect();
    let mut rows = Vec::new();
    // A label's newest after the others' always: as few labels as there are,
    // the oldest of their next ones is taken each time
    loop {
        let next = each
            .iter_mut()
            .enumerate()
            .filter_map(|(at, rows)| Some((rows.peek()?.timestamp, at)))
            .min();
        let Some((_, at)) = next else {
            break;
        };
        rows.extend(each[at].next());
    }
    rows.reverse();
    LogView { rows, counts }
}

/// How many of `rows` (newest first) came on top of those there were `before`:
/// the newest time then and how many rows were of it. With the same of `rows`,
/// for the next time: as each label's oldest go (past as many as it keeps),
/// their number need not change as they come.
fn added_on_top(rows: &[Row], before: Option<(i64, usize)>) -> (usize, Option<(i64, usize)>) {
    let of_time =
        |rows: &[Row], time: i64| rows.iter().take_while(|row| row.timestamp == time).count();
    let newest = rows
        .first()
        .map(|row| (row.timestamp, of_time(rows, row.timestamp)));
    let Some((time, were)) = before else {
        return (0, newest);
    };
    let newer = rows.iter().take_while(|row| row.timestamp > time).count();
    let added = newer + of_time(&rows[newer..], time).saturating_sub(were);
    (added, newest)
}

/// The rows of a table and its labels shown, for its rows to read.
#[derive(Clone, Copy)]
struct LogRows {
    view: Memo<LogView>,
    labels: Memo<Vec<String>>,
}

/// The logs of each label the data list shows, as they come: by time, of the
/// level chosen or one that matters more; each label turned on or off in the
/// legend. Only the rows in view are made.
#[component]
pub fn LogTable(id: usize) -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let mut graph_context = use_context::<GraphContext>();
    let hidden = use_graph_setting(graph_context, id, |property| property.hidden.clone());
    let weakest = use_variant_setting(graph_context, id, |settings: &LogTableSettings| {
        settings.weakest_level
    });

    let log_labels = use_memo(move || data_context.labels_of(DataType::Log));
    let labels = use_memo(move || {
        log_labels
            .read()
            .iter()
            .filter(|label| filter_context.shows(label))
            .cloned()
            .collect::<Vec<_>>()
    });
    let view = use_memo(move || {
        let paused_at = graph_context.paused_at();
        let weakest = weakest();
        let hidden = hidden.read();
        let labels = labels.read();
        data_context.with_each(Some(DataType::Log), |data| {
            let queues: Vec<_> = data
                .iter()
                .filter(|(label, _)| !hidden.iter().any(|each| each == label))
                .filter_map(|&(label, data)| {
                    let TypedData::Log(queue) = data else {
                        return None;
                    };
                    Some((labels.iter().position(|each| each == label)?, queue))
                })
                .collect();
            merged_rows(&queues, weakest, paused_at)
        })
    });
    // Not read here, so the table is not made anew as each log comes: only the
    // rows in view are, and the counts
    let count = use_memo(move || view.read().rows.len());
    use_context_provider(|| LogRows { view, labels });

    let container_id = format!("log-table-{id}");
    // At the newest as they come, while it is scrolled to them; else kept where
    // it is, as they come on top of it
    let follow = use_script(LOG_TABLE_JS);
    let newest = use_hook(|| Rc::new(Cell::new(None)));
    use_effect(move || {
        let (added, now) = added_on_top(&view.read().rows, newest.get());
        newest.set(now);
        if added > 0 {
            let _ = follow.send(added);
        }
    });

    let empty = if log_labels.read().is_empty() {
        Some("No log data yet. Add a map to a log to show it.")
    } else if labels.read().is_empty() {
        Some("No log data shown. Change the Data list's filters to show some.")
    } else {
        None
    };

    rsx! {
        // For its legend, as the other graphs'
        PlotAssets { uplot: false }
        document::Link { rel: "stylesheet", href: LOG_TABLE_CSS }

        GraphFrame {
            id,
            settings: rsx! {
                div {
                    class: "graph-setting",
                    span { class: "graph-setting-label", "Level" }
                    div {
                        class: "graph-scale",
                        role: "group",
                        aria_label: "Weakest level",
                        title: "The weakest level shown: those weaker are left out",
                        for level in LogLevel::ALL {
                            Toggle {
                                pressed: Some(weakest() == level),
                                on_pressed_change: move |_| {
                                    graph_context.update_variant(id, |settings: &mut LogTableSettings| settings.weakest_level = level);
                                },
                                if level == LogLevel::None { "ALL" } else { "{level}" }
                            }
                        }
                    }
                }
            },
            div {
                class: "graph-plot log-table",
                id: "{container_id}",
                onmounted: {
                    move |_| {
                        let _ = follow.send(container_id.clone());
                    }
                },
                LogCounts { weakest: weakest() }
                div {
                    class: "log-table-head log-table-row",
                    role: "row",
                    span { role: "columnheader", "Time" }
                    span { role: "columnheader", "Label" }
                    span { role: "columnheader", "Level" }
                    span { role: "columnheader", "Message" }
                }
                VirtualList {
                    class: "log-table-body",
                    aria_label: "Logs",
                    count,
                    buffer: 8usize,
                    estimate_size: |_| ROW_HEIGHT,
                    render_item: |index: usize| rsx! {
                        LogRow { key: "{index}", index }
                    },
                }
                // Each label turned on or off by clicking it, as the other
                // graphs' legends
                table {
                    class: "u-legend u-inline",
                    tbody {
                        for (index, label) in labels().into_iter().enumerate() {
                            tr {
                                key: "{label}",
                                class: if hidden.read().contains(&label) { "u-series u-off" } else { "u-series" },
                                onclick: {
                                    move |_| {
                                        let label = label.clone();
                                        graph_context.update(id, move |property| {
                                            if property.hidden.contains(&label) {
                                                property.hidden.retain(|each| *each != label);
                                            } else {
                                                property.hidden.push(label);
                                                property.hidden.sort();
                                            }
                                        });
                                    }
                                },
                                th {
                                    div {
                                        class: "u-marker",
                                        style: "border: 2px solid var({series_color(index)}); background: color-mix(in srgb, var({series_color(index)}) 35%, transparent);",
                                    }
                                    div { class: "u-label", "{label}" }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(empty) = empty {
                p { class: "graph-empty", "{empty}" }
            }
        }
    }
}

/// How many logs of each level have come, of the labels shown: those weaker
/// than `weakest` (left out of the table) fainter. Apart from the table, so
/// only it is made anew as they change.
#[component]
fn LogCounts(weakest: LogLevel) -> Element {
    let LogRows { view, .. } = use_context::<LogRows>();
    let counts = use_memo(move || view.read().counts);

    rsx! {
        div {
            class: "log-table-counts",
            role: "group",
            aria_label: "Logs of each level",
            for (level, count) in LogLevel::ALL.into_iter().zip(counts()) {
                span {
                    class: "log-table-count",
                    "data-shown": level <= weakest,
                    title: "{level}",
                    if level == LogLevel::None {
                        span { class: "log-table-count-none", "NONE" }
                    } else {
                        LogLevelTag { level }
                    }
                    span { class: "log-table-count-value", "{count}" }
                }
            }
        }
    }
}

/// A row of the table, by its place: made only while it is in view, again as
/// the rows change.
#[component]
fn LogRow(index: usize) -> Element {
    let LogRows { view, labels } = use_context::<LogRows>();
    let data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    let Some(row) = view.read().rows.get(index).copied() else {
        return rsx! {};
    };
    let Some(label) = labels.read().get(row.label).cloned() else {
        return rsx! {};
    };
    let log = data_context
        .with_label(&label, |data| match data {
            TypedData::Log(queue) => queue.get(row.index).map(|entry| entry.value().clone()),
            _ => None,
        })
        .flatten();
    let Some(log) = log else {
        return rsx! {};
    };
    let time = time_context.format_millis(row.timestamp);
    let color = series_color(row.label);
    let message = log.message().to_string();

    rsx! {
        div {
            class: "log-table-row",
            role: "row",
            span { class: "log-table-time", role: "cell", "{time}" }
            span {
                class: "log-table-label",
                role: "cell",
                style: "color: var({color});",
                title: "{label}",
                "{label}"
            }
            span {
                class: "log-table-level",
                role: "cell",
                if log.level() != LogLevel::None {
                    LogLevelTag { level: log.level() }
                }
            }
            span { class: "log-table-message", role: "cell", title: "{message}", "{message}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Row, added_on_top, merged_rows};
    use crate::data::{Log, LogData, LogLevel, Queue};

    fn logs(entries: &[(i64, LogLevel)]) -> Queue<LogData> {
        entries
            .iter()
            .map(|&(timestamp, level)| LogData::new(timestamp, Log::new(level, "x")))
            .collect()
    }

    fn places(rows: &[Row]) -> Vec<(i64, usize, usize)> {
        rows.iter()
            .map(|row| (row.timestamp, row.label, row.index))
            .collect()
    }

    #[test]
    fn rows_are_by_time_across_the_labels() {
        let first = logs(&[
            (1, LogLevel::Info),
            (4, LogLevel::Info),
            (5, LogLevel::Info),
        ]);
        let second = logs(&[(2, LogLevel::Warn), (4, LogLevel::Error)]);
        let rows = merged_rows(&[(0, &first), (1, &second)], LogLevel::None, None).rows;
        // Newest first; of one time, the last label's first
        assert_eq!(
            places(&rows),
            [(5, 0, 2), (4, 1, 1), (4, 0, 1), (2, 1, 0), (1, 0, 0)]
        );
    }

    #[test]
    fn weaker_levels_are_left_out() {
        let queue = logs(&[
            (1, LogLevel::Error),
            (2, LogLevel::Info),
            (3, LogLevel::Debug),
            (4, LogLevel::None),
            (5, LogLevel::Warn),
        ]);
        let view = merged_rows(&[(0, &queue)], LogLevel::Info, None);
        // Their places in the queue kept, to read them by
        assert_eq!(places(&view.rows), [(5, 0, 4), (2, 0, 1), (1, 0, 0)]);
        // Counted all the same
        assert_eq!(view.counts, [1, 1, 1, 1, 0, 1]);
        assert_eq!(
            merged_rows(&[(0, &queue)], LogLevel::None, None).rows.len(),
            5
        );
    }

    #[test]
    fn paused_those_after_are_left_out() {
        let queue = logs(&[
            (1, LogLevel::Info),
            (2, LogLevel::Info),
            (3, LogLevel::Info),
        ]);
        let view = merged_rows(&[(0, &queue)], LogLevel::None, Some(2));
        assert_eq!(places(&view.rows), [(2, 0, 1), (1, 0, 0)]);
        // Of those up to then
        assert_eq!(view.counts, [0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn rows_added_on_top_are_counted_by_the_newest_before() {
        let row = |timestamp, index| Row {
            timestamp,
            label: 0,
            index,
        };
        // None before: none added, the first time
        let rows = [row(2, 1), row(1, 0)];
        assert_eq!(added_on_top(&rows, None), (0, Some((2, 1))));
        // Two newer, and one more of the time that was newest
        let rows = [row(4, 3), row(3, 2), row(2, 9), row(2, 1), row(1, 0)];
        assert_eq!(added_on_top(&rows, Some((2, 1))), (3, Some((4, 1))));
        // As many as before, the oldest gone: those on top counted all the same
        let rows = [row(5, 3), row(4, 2), row(3, 1)];
        assert_eq!(added_on_top(&rows, Some((4, 1))), (1, Some((5, 1))));
        assert_eq!(added_on_top(&[], Some((4, 1))), (0, None));
    }
}
