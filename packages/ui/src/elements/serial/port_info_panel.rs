use dioxus::prelude::*;

use crate::helper::{Choice, format_bytes};
use crate::{
    components::card::{Card, CardContent, CardHeader, CardTitle},
    serial::{NotificationKind, SerialContext},
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
    // As the port opens next, or as it opened
    let settings = port.as_ref().map(|port| port.settings).unwrap_or_default();
    let baudrate = settings
        .baudrate
        .map_or_else(unknown, |baudrate| baudrate.to_string());
    let extra = settings.extra;
    let frame = extra.frame();
    let (parity, flow_control) = (extra.parity.name(), extra.flow_control.name());
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
                        span { "{baudrate} {frame}" }
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
                        if let Some(path) = port.as_ref().and_then(|port| port.info.path.clone()) {
                            dt { "Path" }
                            dd { "{path}" }
                        }
                        dt { "Baud rate" }
                        dd { "{baudrate}" }
                        dt { "Data bits" }
                        dd { "{extra.data_bits}" }
                        dt { "Parity" }
                        dd { "{parity}" }
                        dt { "Stop bits" }
                        dd { "{extra.stop_bits}" }
                        dt { "Flow control" }
                        dd { "{flow_control}" }
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

fn kind_name(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Success => "success",
        NotificationKind::Info => "info",
        NotificationKind::Error => "error",
    }
}
