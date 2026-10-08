use dioxus::prelude::*;

use crate::helper::format_bytes;
use crate::{
    components::card::{Card, CardContent, CardHeader, CardTitle},
    serial::{DATA_BITS, FLOW_CONTROL, NotificationKind, PARITY, STOP_BITS, SerialContext},
    time::TimeContext,
};

const PORT_INFO_PANEL_CSS: Asset = asset!("/assets/styling/port-info-panel.css");

/// The selected port's details, byte counts and notifications.
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
    let rx_bytes = format_bytes(serial.rx_bytes());
    let tx_bytes = format_bytes(serial.tx_bytes());
    let notifications = serial.notifications();

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_INFO_PANEL_CSS }

        div {
            class: "port-info-panel",
            Card {
                CardHeader { CardTitle { "Port Info" } }
                CardContent {
                    // Shown instead when the panel is too short
                    div {
                        class: "port-summary",
                        span { "{product} · {vendor}" }
                        span { "{baudrate} {frame()}" }
                        span { "RX {rx_bytes} · TX {tx_bytes}" }
                        if let Some(entry) = notifications.first() {
                            span {
                                class: "port-summary-notification",
                                "data-kind": kind_name(entry.kind),
                                "{time.format(entry.time_ms)} {entry.title}"
                            }
                        }
                    }
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
                        dd { "{rx_bytes}" }
                        dt { "Sent" }
                        dd { "{tx_bytes}" }
                    }
                    ol {
                        class: "port-notifications",
                        aria_label: "Log",
                        for entry in notifications {
                            li {
                                "data-kind": kind_name(entry.kind),
                                time { "{time.format(entry.time_ms)}" }
                                span { class: "port-notification-title", "{entry.title}" }
                                if !entry.detail.is_empty() {
                                    span { class: "port-notification-detail", "{entry.detail}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// e.g. "8N1"
fn frame() -> String {
    let parity = PARITY.chars().next().unwrap_or('N').to_ascii_uppercase();
    format!("{DATA_BITS}{parity}{STOP_BITS}")
}

fn kind_name(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Success => "success",
        NotificationKind::Info => "info",
        NotificationKind::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::frame;

    #[test]
    fn frame_is_8n1() {
        assert_eq!(frame(), "8N1");
    }
}
