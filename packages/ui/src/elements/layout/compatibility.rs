//! Layout files of the versions before, as of this one: by the changes of the
//! layout file since, in turn.

use std::{fmt, str::FromStr};

use serde_json::Value;

/// A version of Serval, as `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u32, u32, u32);

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let parts: Vec<u32> = text
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| format!("No version '{text}'"))?;
        match parts[..] {
            [major, minor, patch] => Ok(Self(major, minor, patch)),
            _ => Err(format!("No version '{text}'")),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The first version with layout files.
const FIRST: Version = Version(0, 4, 0);

/// A change of the layout file: a layout of its version or before, made as of
/// the one after.
type Step = fn(&mut Value) -> Result<(), String>;

/// Each change, after the last version of the format before it, in order.
const STEPS: &[(Version, Step)] = &[(Version(0, 4, 1), lower_case_and_time_ranges)];

/// `layout` (a layout file's JSON) as of `current`, the version running: of its
/// `version` (none for `current`), by each change since. An error for a version
/// that is none, before layout files, or after `current`.
pub(super) fn resolve_version(mut layout: Value, current: &str) -> Result<Value, String> {
    let current: Version = current.parse().expect("this version is one");
    let version = match layout.get("version") {
        None => current,
        Some(Value::String(version)) => version.parse()?,
        Some(other) => return Err(format!("No version {other}")),
    };
    if version < FIRST {
        return Err(format!(
            "No layout file of version {version}: they came in {FIRST}"
        ));
    }
    if version > current {
        return Err(format!(
            "Of a newer Serval, {version}: update this one ({current}) to import it"
        ));
    }
    for (last, step) in STEPS {
        if version <= *last {
            step(&mut layout)?;
        }
    }
    if let Value::Object(fields) = &mut layout {
        fields.insert("version".to_string(), Value::from(current.to_string()));
    }
    Ok(layout)
}

/// After v0.4.1: the graphs' scales and draw styles in lower case; a time
/// series's `time_window` as its `time_range`, and Box plot and Scatter (of all
/// the values then) of `"all"`. Those of the change already (as of a build of
/// the version yet to come, still 0.4.1) left as they are.
fn lower_case_and_time_ranges(layout: &mut Value) -> Result<(), String> {
    let Some(Value::Array(graphs)) = layout.get_mut("graphs") else {
        return Ok(());
    };
    for graph in graphs {
        let kind = graph
            .get("kind")
            .and_then(Value::as_str)
            .map(str::to_string);
        let Some(Value::Object(settings)) = graph.get_mut("settings") else {
            continue;
        };
        for key in ["value_scale", "x_scale", "draw_style"] {
            if let Some(Value::String(value)) = settings.get_mut(key)
                && matches!(value.as_str(), "Linear" | "Log" | "Points" | "Stepped")
            {
                *value = value.to_lowercase();
            }
        }
        let Some(window) = settings.remove("time_window") else {
            continue;
        };
        let range = match kind.as_deref() {
            Some("Time series") => window,
            Some("Box plot" | "Scatter") => Value::from("all"),
            _ => continue,
        };
        settings.insert("time_range".to_string(), range);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::resolve_version;

    #[test]
    fn a_layout_of_v0_4_1_is_made_as_of_this_version() {
        let graph = |kind: &str, settings| json!({ "kind": kind, "settings": settings });
        let old = json!({
            "version": "0.4.1",
            "maps": [],
            "graphs": [
                graph("Time series", json!({ "draw_style": "Stepped", "time_window": 30 })),
                graph("Box plot", json!({ "value_scale": "Log", "time_window": 10 })),
                graph("Scatter", json!({ "x_scale": "Linear", "time_window": 10 })),
                graph("Log", json!({ "time_window": 10 })),
            ],
        });
        assert_eq!(
            resolve_version(old, "0.5.0").unwrap(),
            json!({
                "version": "0.5.0",
                "maps": [],
                "graphs": [
                    graph("Time series", json!({ "draw_style": "stepped", "time_range": 30 })),
                    graph("Box plot", json!({ "value_scale": "log", "time_range": "all" })),
                    graph("Scatter", json!({ "x_scale": "linear", "time_range": "all" })),
                    graph("Log", json!({})),
                ],
            })
        );
    }

    #[test]
    fn a_layout_of_this_version_is_left_as_it_is() {
        let new = json!({
            "version": "0.4.1",
            "maps": [],
            "graphs": [{ "kind": "Box plot", "settings": { "value_scale": "log", "time_range": 20 } }],
        });
        assert_eq!(resolve_version(new.clone(), "0.4.1").unwrap(), new);
        // None: of this one
        let unversioned = json!({ "maps": [], "graphs": [] });
        assert_eq!(
            resolve_version(unversioned, "0.4.1").unwrap(),
            json!({ "version": "0.4.1", "maps": [], "graphs": [] })
        );
    }

    #[test]
    fn versions_it_cannot_take_are_errors() {
        let of = |version| json!({ "version": version, "maps": [], "graphs": [] });
        assert_eq!(
            resolve_version(of(json!("0.5.0")), "0.4.1").unwrap_err(),
            "Of a newer Serval, 0.5.0: update this one (0.4.1) to import it"
        );
        assert_eq!(
            resolve_version(of(json!("0.3.1")), "0.4.1").unwrap_err(),
            "No layout file of version 0.3.1: they came in 0.4.0"
        );
        assert_eq!(
            resolve_version(of(json!("next")), "0.4.1").unwrap_err(),
            "No version 'next'"
        );
        assert_eq!(
            resolve_version(of(json!("0.4")), "0.4.1").unwrap_err(),
            "No version '0.4'"
        );
        assert_eq!(
            resolve_version(of(json!(4)), "0.4.1").unwrap_err(),
            "No version 4"
        );
    }
}
