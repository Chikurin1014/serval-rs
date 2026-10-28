use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_primitives::{ContentAlign, ContentSide};

use crate::{
    components::{
        input::Input,
        popover::{PopoverContent, PopoverRoot, PopoverTrigger},
        toggle_group::{ToggleGroup, ToggleItem},
    },
    serial::{Baudrate, ExtraSettings, FlowControl, Parity, SerialContext},
};

const PORT_CONFIGURATOR_CSS: Asset = asset!("/assets/styling/port-configurator.css");

const BAUDRATE_PRESETS: [u32; 4] = [9600_u32, 19200, 57600, 115200];
const BAUDRATE_PRESETS_LIST: &str = "baudrate-presets";

/// The selected port's baud rate, and its other settings in the menu beside it.
#[component]
pub fn PortConfigurator() -> Element {
    let serial = use_context::<SerialContext>();
    let selected_baudrate = serial
        .selected_port()
        .and_then(|port| port.settings.baudrate);

    rsx! {
        document::Link { rel: "stylesheet", href: PORT_CONFIGURATOR_CSS }

        div {
            class: "port-configurator joined-controls",
            label {
                class: "field baudrate-field",
                Input {
                    type: "number",
                    min: "1",
                    step: "1",
                    placeholder: "Baud rate (e.g. 9600)",
                    list: BAUDRATE_PRESETS_LIST,
                    value: selected_baudrate.map(|baudrate| baudrate.bps().to_string()),
                    oninput: move |event: FormEvent| {
                        if let Some(baudrate) = parse_baudrate(&event.value()) {
                            serial.set_baudrate(baudrate);
                        }
                    }
                }
                span { class: "field-label baudrate-unit", "bps" }
            }
            ExtraSettingsMenu {}
        }
        datalist {
            id: BAUDRATE_PRESETS_LIST,
            {BAUDRATE_PRESETS.iter().map(|baudrate_preset| {
                rsx!(
                    option {
                        value: "{baudrate_preset}"
                    }
                )
            })}
        }
    }
}

/// Data bits, parity, stop bits and flow control: its frame (e.g. "8N1") on
/// the button.
#[component]
fn ExtraSettingsMenu() -> Element {
    let serial = use_context::<SerialContext>();
    let port = serial.selected_port();
    let extra = port
        .as_ref()
        .map(|port| port.settings.extra)
        .unwrap_or_default();

    rsx! {
        PopoverRoot {
            class: "extra-settings-menu",
            PopoverTrigger {
                class: "extra-settings-trigger",
                aria_label: "Port settings",
                title: "Port settings: taken as the port opens",
                disabled: port.is_none(),
                span { class: "extra-settings-frame", "{extra.frame()}" }
            }
            PopoverContent {
                class: "extra-settings-content",
                side: ContentSide::Bottom,
                align: ContentAlign::End,
                SettingChoices {
                    name: "Data bits",
                    choices: ExtraSettings::DATA_BITS.map(|bits| bits.to_string()).to_vec(),
                    chosen: ExtraSettings::DATA_BITS.iter().position(|&bits| bits == extra.data_bits),
                    on_choose: move |index| serial.set_extra_settings(ExtraSettings { data_bits: ExtraSettings::DATA_BITS[index], ..extra }),
                }
                SettingChoices {
                    name: "Parity",
                    choices: Parity::ALL.map(|parity| parity.name().to_string()).to_vec(),
                    chosen: Parity::ALL.iter().position(|&parity| parity == extra.parity),
                    on_choose: move |index| serial.set_extra_settings(ExtraSettings { parity: Parity::ALL[index], ..extra }),
                }
                SettingChoices {
                    name: "Stop bits",
                    choices: ExtraSettings::STOP_BITS.map(|bits| bits.to_string()).to_vec(),
                    chosen: ExtraSettings::STOP_BITS.iter().position(|&bits| bits == extra.stop_bits),
                    on_choose: move |index| serial.set_extra_settings(ExtraSettings { stop_bits: ExtraSettings::STOP_BITS[index], ..extra }),
                }
                SettingChoices {
                    name: "Flow control",
                    choices: FlowControl::ALL.map(|flow| flow.name().to_string()).to_vec(),
                    chosen: FlowControl::ALL.iter().position(|&flow| flow == extra.flow_control),
                    on_choose: move |index| serial.set_extra_settings(ExtraSettings { flow_control: FlowControl::ALL[index], ..extra }),
                }
            }
        }
    }
}

/// One setting: its name, and a toggle for each of its choices, one pressed.
#[component]
fn SettingChoices(
    name: &'static str,
    choices: Vec<String>,
    chosen: Option<usize>,
    on_choose: EventHandler<usize>,
) -> Element {
    rsx! {
        div {
            class: "field field-choices extra-setting",
            role: "group",
            aria_label: name,
            span { class: "field-label", "{name}" }
            ToggleGroup {
                horizontal: true,
                pressed: Some(chosen.into_iter().collect::<HashSet<_>>()),
                on_pressed_change: move |pressed: HashSet<usize>| {
                    // Pressing the one chosen again leaves it so
                    if let Some(&index) = pressed.iter().next() {
                        on_choose.call(index);
                    }
                },
                for (index, choice) in choices.iter().enumerate() {
                    ToggleItem { index, "{choice}" }
                }
            }
        }
    }
}

fn parse_baudrate(value: &str) -> Option<Baudrate> {
    value.trim().parse::<u32>().ok().and_then(Baudrate::new)
}
