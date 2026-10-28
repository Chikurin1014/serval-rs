use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_primitives::{ContentAlign, ContentSide};

use crate::{
    components::{
        input::Input,
        popover::{PopoverContent, PopoverRoot, PopoverTrigger},
        toggle_group::{ToggleGroup, ToggleItem},
    },
    helper::Choice,
    serial::{Baudrate, ExtraSettings, FlowControl, Parity, PortSettings, SerialContext},
};

const PORT_CONFIGURATOR_CSS: Asset = asset!("/assets/styling/port-configurator.css");

const BAUDRATE_PRESETS: [u32; 4] = [9600_u32, 19200, 57600, 115200];
const BAUDRATE_PRESETS_LIST: &str = "baudrate-presets";

/// The selected port's baud rate, and its other settings in the menu beside it.
#[component]
pub fn PortConfigurator() -> Element {
    let serial = use_context::<SerialContext>();
    let settings = serial.settings().unwrap_or_default();

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
                    disabled: !serial.settings_editable(),
                    value: settings.baudrate.map(|baudrate| baudrate.bps().to_string()),
                    oninput: move |event: FormEvent| {
                        if let Some(baudrate) = parse_baudrate(&event.value()) {
                            // Refused as the field is disabled: no port, or open
                            let _ = serial.set_settings(PortSettings { baudrate: Some(baudrate), ..settings });
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
    let settings = serial.settings().unwrap_or_default();
    let extra = settings.extra;
    // Refused as the menu is disabled: no port, or open
    let set = move |extra| {
        let _ = serial.set_settings(PortSettings { extra, ..settings });
    };

    rsx! {
        PopoverRoot {
            class: "extra-settings-menu",
            PopoverTrigger {
                class: "extra-settings-trigger",
                aria_label: "Port settings",
                title: if serial.is_open() { "Close the port to change its settings" } else { "Port settings: taken as the port opens" },
                disabled: !serial.settings_editable(),
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
                    on_choose: move |index| set(ExtraSettings { data_bits: ExtraSettings::DATA_BITS[index], ..extra }),
                }
                SettingChoices {
                    name: "Parity",
                    choices: names::<Parity>(),
                    chosen: Some(extra.parity.index()),
                    on_choose: move |index| set(ExtraSettings { parity: Parity::ALL[index], ..extra }),
                }
                SettingChoices {
                    name: "Stop bits",
                    choices: ExtraSettings::STOP_BITS.map(|bits| bits.to_string()).to_vec(),
                    chosen: ExtraSettings::STOP_BITS.iter().position(|&bits| bits == extra.stop_bits),
                    on_choose: move |index| set(ExtraSettings { stop_bits: ExtraSettings::STOP_BITS[index], ..extra }),
                }
                SettingChoices {
                    name: "Flow control",
                    choices: names::<FlowControl>(),
                    chosen: Some(extra.flow_control.index()),
                    on_choose: move |index| set(ExtraSettings { flow_control: FlowControl::ALL[index], ..extra }),
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

/// The names of all `T`, as its toggles show them.
fn names<T: Choice>() -> Vec<String> {
    T::ALL
        .iter()
        .map(|choice| choice.name().to_string())
        .collect()
}

fn parse_baudrate(value: &str) -> Option<Baudrate> {
    value.trim().parse::<u32>().ok().and_then(Baudrate::new)
}
