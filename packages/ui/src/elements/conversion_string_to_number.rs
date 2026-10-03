use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoveRight, LdRegex, LdTag},
    Icon,
};
use regex::Regex;

use crate::{
    components::input::Input,
    data::{DataContext, NumberData},
    time::TimeContext,
};

const CONVERSION_CSS: Asset = asset!("/assets/styling/conversion.css");

#[component]
pub fn ConversionStringToNumber(
    #[props(default)] initial_source_label: String,
    #[props(default)] initial_target_label: String,
    #[props(default)] initial_regex_pattern: String,
    #[props(default)] initial_to: String,
    enabled: ReadSignal<bool>,
) -> Element {
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    let mut source_label = use_signal(|| initial_source_label);
    let mut target_label = use_signal(|| initial_target_label);
    let mut regex_pattern = use_signal(|| initial_regex_pattern);
    let mut to = use_signal(|| initial_to);
    let mut processed_count = use_signal(|| 0usize);
    let mut previous_config = use_signal(String::new);
    let mut from_error = use_signal(String::new);
    let mut to_error = use_signal(String::new);

    use_effect(move || {
        if !enabled() {
            return;
        }

        let source = source_label();
        let target = target_label();
        if source.trim().is_empty() || target.trim().is_empty() || source.trim() == target.trim() {
            return;
        }

        let pattern = regex_pattern();
        let replacement = to();
        let config = format!("{source}\0{target}\0{pattern}\0{replacement}");
        if previous_config() != config {
            previous_config.set(config);
            processed_count.set(0);
            from_error.set(String::new());
            to_error.set(String::new());
        }
        if pattern.trim().is_empty() {
            return;
        }

        let regex = match Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(value) => {
                from_error.set(format!("Invalid regex: {value}"));
                return;
            }
        };

        let Some(queue) = data_context.get_string(&source) else {
            return;
        };

        let processed = processed_count();
        if processed > queue.len() {
            processed_count.set(0);
        }
        let new_entries = queue
            .iter()
            .skip(processed.min(queue.len()))
            .cloned()
            .collect::<Vec<_>>();
        if new_entries.is_empty() {
            return;
        }
        processed_count.set(queue.len());

        let timestamp = time_context.current();
        let mut had_error = false;
        for entry in new_entries {
            let Some(captures) = regex.captures(entry.value()) else {
                continue;
            };
            let mut value = String::new();
            captures.expand(&replacement, &mut value);
            let Ok(number) = value.parse::<f64>() else {
                to_error.set(format!("The result '{value}' is not a number"));
                had_error = true;
                continue;
            };
            let target_label = regex
                .replace(entry.value(), target.as_str())
                .trim()
                .to_string();
            if target_label.is_empty() {
                continue;
            }
            data_context.push_number(&target_label, NumberData::new(timestamp, number));
        }
        if !had_error {
            from_error.set(String::new());
            to_error.set(String::new());
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: CONVERSION_CSS }

        div {
            class: "conversion-row",
            div {
                class: "field-stack",
                label {
                    class: "field",
                    Icon { icon: LdTag {} }
                    Input {
                        list: "conversion-strings-labels", // Defined in `ConversionList` component
                        placeholder: "Source label",
                        autocomplete: "on",
                        value: "{source_label()}",
                        oninput: move |event: FormEvent| {
                            source_label.set(event.value());
                        },
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "From" }
                    Icon { icon: LdRegex {} }
                    Input {
                        placeholder: "Text to be matched",
                        value: "{regex_pattern()}",
                        oninput: move |event: FormEvent| {
                            regex_pattern.set(event.value());
                            from_error.set(String::new());
                        },
                    }
                }
                if !from_error().is_empty() {
                    span { class: "field-error", "{from_error()}" }
                }
            }
            Icon { icon: LdMoveRight {} }
            div {
                class: "field-stack",
                label {
                    class: "field",
                    Icon { icon: LdTag {} }
                    Input {
                        placeholder: "Target label",
                        value: "{target_label()}",
                        oninput: move |event: FormEvent| {
                            target_label.set(event.value());
                        },
                    }
                }
                label {
                    class: "field",
                    span { class: "field-label", "To" }
                    Input {
                        value: "{to()}",
                        oninput: move |event: FormEvent| {
                            to.set(event.value());
                            to_error.set(String::new());
                        },
                    }
                }
                if !to_error().is_empty() {
                    span { class: "field-error", "{to_error()}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_extracts_digit_group() {
        let regex = Regex::new(r"(\d+)").unwrap();
        let capture = regex
            .captures("abc123def")
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        assert_eq!(capture, "123");
        assert!(capture.parse::<f64>().is_ok());
    }
}
