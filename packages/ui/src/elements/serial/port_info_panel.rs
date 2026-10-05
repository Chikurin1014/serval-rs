use dioxus::prelude::*;

use crate::{
    components::card::{Card, CardContent, CardHeader, CardTitle},
    serial::{DATA_BITS, FLOW_CONTROL, LogKind, PARITY, STOP_BITS, SerialContext},
    time::TimeContext,
};

const PORT_INFO_PANEL_CSS: Asset = asset!("/assets/styling/port-info-panel.css");

/// The selected port's device and settings, the bytes received and sent, and
/// the log of what happened with the ports.
#[component]
pub fn PortInfoPanel() -> Element {
    let serial = use_context::<SerialContext>();
    let time = use_context::<TimeContext>();
    let port = serial.selected_port();
    let unknown = || "-".to_string();
    let vendor = port
        .as_ref()
        .and_then(|port| port.info.vendor.clone())
        .unwrap_or_else(unknown);
    let product = port
        .as_ref()
        .and_then(|port| port.info.product.clone())
        .unwrap_or_else(unknown);
    let baudrate = port
        .as_ref()
        .and_then(|port| port.baudrate)
        .map_or_else(unknown, |baudrate| format!("{baudrate} bps"));

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_INFO_PANEL_CSS }

        div {
            class: "port-info-panel",
            Card {
                CardHeader { CardTitle { "Port Info" } }
                CardContent {
                    dl {
                        class: "port-info",
                        dt { "Vendor" }
                        dd { "{vendor}" }
                        dt { "Product" }
                        dd { "{product}" }
                        dt { "Baudrate" }
                        dd { "{baudrate}" }
                        dt { "Data bits" }
                        dd { "{DATA_BITS}" }
                        dt { "Parity" }
                        dd { "{PARITY}" }
                        dt { "Stop bits" }
                        dd { "{STOP_BITS}" }
                        dt { "Flow control" }
                        dd { "{FLOW_CONTROL}" }
                        dt { "Received" }
                        dd { {format_bytes(serial.rx_bytes())} }
                        dt { "Sent" }
                        dd { {format_bytes(serial.tx_bytes())} }
                    }
                    ol {
                        class: "port-log",
                        aria_label: "Log",
                        for entry in serial.log() {
                            li {
                                "data-kind": match entry.kind {
                                    LogKind::Success => "success",
                                    LogKind::Info => "info",
                                    LogKind::Error => "error",
                                },
                                time { "{time.format(entry.time_ms)}" }
                                span { class: "port-log-title", "{entry.title}" }
                                if !entry.detail.is_empty() {
                                    span { class: "port-log-detail", "{entry.detail}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `bytes` with thousands separated, e.g. "12,345 B".
fn format_bytes(bytes: u64) -> String {
    let digits = bytes.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    format!("{grouped} B")
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn format_bytes_separates_thousands() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1_000), "1,000 B");
        assert_eq!(format_bytes(1_234_567), "1,234,567 B");
    }
}
