use std::collections::VecDeque;

use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoveRight, LdRegex, LdTag},
    Icon,
};
use regex::Regex;

use crate::{
    data::{DataContext, StringData, TypedData},
    time::TimeContext,
};

#[component]
pub fn ConversionStringToString(
    #[props(default)] initial_source_label: String,
    #[props(default)] initial_target_label: String,
    #[props(default)] initial_from: String,
    #[props(default)] initial_to: String,
    enabled: ReadSignal<bool>,
) -> Element {
    let mut data_context = use_context::<DataContext>();
    let time_context = use_context::<TimeContext>();
    let mut source_label = use_signal(|| initial_source_label);
    let mut target_label = use_signal(|| initial_target_label);
    let mut from = use_signal(|| initial_from);
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
        let pattern = from();
        let replacement = to();
        let config = format!("{source}\0{target}\0{pattern}\0{replacement}");
        if previous_config() != config {
            previous_config.set(config);
            processed_count.set(0);
            from_error.set(String::new());
            to_error.set(String::new());
        }
        if source.trim().is_empty()
            || target.trim().is_empty()
            || source.trim() == target.trim()
            || pattern.is_empty()
        {
            return;
        }

        let regex = match Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(value) => {
                from_error.set(format!("Invalid regex: {value}"));
                return;
            }
        };

        let Some(queue) = data_context
            .data_with_labels
            .read()
            .get(&source)
            .and_then(|data| match data {
                TypedData::String(queue) => Some(queue.clone()),
                _ => None,
            })
        else {
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
        let mut map = data_context.data_with_labels.write();
        for entry in &new_entries {
            if regex.captures(entry.value()).is_none() {
                continue;
            }
            let target_label = regex.replace(entry.value(), target.as_str()).into_owned();
            if target_label.trim().is_empty() {
                continue;
            }
            let converted = StringData::new(
                timestamp,
                regex
                    .replace(entry.value(), replacement.as_str())
                    .into_owned(),
            );
            let target_queue = map
                .entry(target_label)
                .or_insert_with(|| TypedData::String(VecDeque::new()));
            match target_queue {
                TypedData::String(queue) => queue.push_back(converted),
                other => *other = TypedData::String(VecDeque::from([converted])),
            }
        }
        from_error.set(String::new());
        to_error.set(String::new());
    });

    rsx! {
        div {
            class: "flex items-center gap-2",
            div {
                class: "join join-vertical",
                label {
                    class: "input input-sm input-bordered",
                    Icon { icon: LdTag {} }
                    input {
                        class: "grow",
                        placeholder: "Source label",
                        list: "conversion-strings-labels", // Defined in `ConversionList` component
                        autocomplete: "on",
                        value: "{source_label()}",
                        oninput: move |event| source_label.set(event.value()),
                    }
                }
                label {
                    class: "input input-sm input-bordered",
                    span { class: "label", "From" }
                    Icon { icon: LdRegex {} }
                    input {
                        class: "grow",
                        placeholder: "Text to be replaced",
                        value: "{from()}",
                        oninput: move |event| {
                            from.set(event.value());
                            from_error.set(String::new());
                        },
                    }
                }
                if !from_error().is_empty() {
                    span { class: "text-error text-xs", "{from_error()}" }
                }
            }
            Icon { icon: LdMoveRight {} }
            div {
                class: "join join-vertical",
                label {
                    class: "input input-sm input-bordered",
                    Icon { icon: LdTag {} }
                    input {
                        class: "grow",
                        placeholder: "Target label",
                        list: "conversion-strings-labels",
                        value: "{target_label()}",
                        oninput: move |event| target_label.set(event.value()),
                    }
                }
                label {
                    class: "input input-sm input-bordered",
                    span { class: "label", "To" }
                    input {
                        class: "grow",
                        value: "{to()}",
                        oninput: move |event| {
                            to.set(event.value());
                            to_error.set(String::new());
                        }
                    }
                }
                if !to_error().is_empty() {
                    span { class: "text-error text-xs", "{to_error()}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_all_matches_with_format() {
        let regex = Regex::new(r"led: (on|off)").unwrap();
        assert_eq!(regex.replace_all("led: on", "state=$1"), "state=on");
    }

    #[test]
    fn target_label_can_use_match_groups() {
        let regex = Regex::new(r"led: (on|off)").unwrap();
        assert_eq!(regex.replace("led: on", "led_$1"), "led_on");
    }
}
