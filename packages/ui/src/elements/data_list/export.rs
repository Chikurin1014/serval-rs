use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::FilterContext;
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::data::{DataContext, NumberText};
use crate::helper::csv_field;
use crate::time::TimeContext;

/// Saves `[file name, text]` sent from Rust as a CSV file, by a link to it.
const DOWNLOAD_JS: &str = r#"
const [name, text] = await dioxus.recv();
const url = URL.createObjectURL(new Blob([text], { type: "text/csv" }));
const link = document.createElement("a");
link.href = url;
link.download = name;
document.body.append(link);
link.click();
link.remove();
URL.revokeObjectURL(url);
"#;

/// Saves the data of the labels the data list shows (by `FilterContext`) as
/// a CSV file.
#[component]
pub(super) fn ExportCsvButton() -> Element {
    let data_context = use_context::<DataContext>();
    let filter_context = use_context::<FilterContext>();
    let time_context = use_context::<TimeContext>();

    let export = move |_| {
        let series = data_context.with_each(None, |data| {
            let mut series = data
                .iter()
                .filter(|(label, _)| filter_context.shows(label))
                .map(|&(label, data)| {
                    (
                        label.to_string(),
                        data.newest_as_text(usize::MAX, NumberText::Exact),
                    )
                })
                .collect::<Vec<_>>();
            series.sort_by(|(a, _), (b, _)| a.cmp(b));
            series
        });
        let csv = to_csv(&series);
        let name = format!("serval-{}.csv", time_context.current());
        let download = document::eval(DOWNLOAD_JS);
        let _ = download.send((name, csv));
    };

    rsx! {
        Button {
            class: "data-export",
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconSm,
            aria_label: "Export as CSV",
            title: "Export as a CSV file",
            onclick: export,
            lucide::Download {}
        }
    }
}

/// Two columns per label, `timestamp-<label>` and `value-<label>`, side by side:
/// row by row, each label's entries in the order they came, those of a label
/// with fewer left empty below its last.
fn to_csv(series: &[(String, Vec<(i64, String)>)]) -> String {
    let header = series
        .iter()
        .flat_map(|(label, _)| [format!("timestamp-{label}"), format!("value-{label}")])
        .map(|field| csv_field(&field))
        .collect::<Vec<_>>();
    let mut csv = header.join(",");
    csv.push('\n');

    let rows = series
        .iter()
        .map(|(_, entries)| entries.len())
        .max()
        .unwrap_or(0);
    for row in 0..rows {
        let fields = series
            .iter()
            .flat_map(|(_, entries)| match entries.get(row) {
                Some((timestamp, value)) => [timestamp.to_string(), csv_field(value)],
                None => [String::new(), String::new()],
            })
            .collect::<Vec<_>>();
        csv.push_str(&fields.join(","));
        csv.push('\n');
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::to_csv;

    #[test]
    fn to_csv_puts_each_label_in_two_columns() {
        let series = vec![
            (
                "temp".to_string(),
                vec![(10, "20.5".to_string()), (30, "21".to_string())],
            ),
            ("led".to_string(), vec![(20, "on, then off".to_string())]),
        ];
        assert_eq!(
            to_csv(&series),
            "timestamp-temp,value-temp,timestamp-led,value-led\n\
             10,20.5,20,\"on, then off\"\n\
             30,21,,\n"
        );
    }

    #[test]
    fn to_csv_with_no_labels_is_an_empty_header() {
        assert_eq!(to_csv(&[]), "\n");
    }
}
