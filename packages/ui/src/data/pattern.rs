//! The regexes typed in maps and filters, which may use aliases for the parts
//! common in serial data.

use dioxus::prelude::*;
use fancy_regex::Regex;

use crate::helper::set_if_changed;

/// The aliases a pattern may use, by name: `{name}` stands for its regex.
pub const PATTERN_ALIASES: &[(&str, &str)] = &[
    // e.g. `20`, `-0.5`, `.5`, `+1.` or `1.5e-3`
    ("number", r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?"),
    // A run of anything but spaces and line breaks, so symbols (`-`, `.`) too;
    // as short as it can be, so it ends at the first separator after it
    ("word", r"[^\s]+?"),
];

/// Compiles `pattern`, its aliases put in place (see [`expand_aliases`]).
pub fn compile_pattern(pattern: &str) -> Result<Regex, fancy_regex::Error> {
    Regex::new(&expand_aliases(pattern))
}

/// Compiles a map's `pattern` as the map is turned on (see
/// `MapRunner::start`): `None` for an empty one, or one that does not compile,
/// with why in `error` (cleared otherwise).
pub(crate) fn compile_for_map(pattern: &str, error: &mut Signal<Option<String>>) -> Option<Regex> {
    let compiled = if pattern.is_empty() {
        Ok(None)
    } else {
        compile_pattern(pattern).map(Some)
    };
    match compiled {
        Ok(regex) => {
            set_if_changed(error, None);
            regex
        }
        Err(failed) => {
            set_if_changed(error, Some(format!("Invalid regex: {failed}")));
            None
        }
    }
}

/// `pattern` with each alias (`{number}`) put in place by its regex, as a group
/// of its own with no capture (so `{word}?` makes the whole word optional).
/// One escaped (`\{number}`) or in a character class is left as it is.
pub fn expand_aliases(pattern: &str) -> String {
    let mut expanded = String::with_capacity(pattern.len());
    let mut rest = pattern;
    // How deep in character classes, which may nest (`[a-z&&[^x]]`)
    let mut class_depth = 0usize;
    while let Some(c) = rest.chars().next() {
        let mut taken = c.len_utf8();
        match c {
            // Escapes are copied whole, so `\{` and `\[` stay literal
            '\\' => taken += rest[1..].chars().next().map_or(0, char::len_utf8),
            '[' => {
                class_depth += 1;
                // A `]` first in a class is literal (`[]a]`, `[^]a]`)
                let after = &rest[1..];
                let negated = usize::from(after.starts_with('^'));
                if after[negated..].starts_with(']') {
                    taken += negated + 1;
                }
            }
            ']' if class_depth > 0 => class_depth -= 1,
            '{' if class_depth == 0 => {
                let alias = PATTERN_ALIASES.iter().find(|(name, _)| {
                    rest[1..]
                        .strip_prefix(name)
                        .is_some_and(|after| after.starts_with('}'))
                });
                if let Some((name, regex)) = alias {
                    expanded.push_str("(?:");
                    expanded.push_str(regex);
                    expanded.push(')');
                    rest = &rest[name.len() + 2..];
                    continue;
                }
            }
            _ => {}
        }
        expanded.push_str(&rest[..taken]);
        rest = &rest[taken..];
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::{compile_pattern, expand_aliases};

    fn captures(pattern: &str, input: &str) -> Option<Vec<String>> {
        let regex = compile_pattern(pattern).unwrap();
        let captures = regex.captures(input).unwrap()?;
        Some(
            captures
                .iter()
                .map(|group| group.map_or(String::new(), |m| m.as_str().to_string()))
                .collect(),
        )
    }

    #[test]
    fn number_matches_numbers_whole() {
        for number in ["20", "+20", "-0.5", ".5", "1.", "1.5E-3", "-2e+10"] {
            assert_eq!(
                captures("^{number}$", number),
                Some(vec![number.to_string()]),
                "{number}"
            );
        }
        assert_eq!(captures("^{number}$", "."), None);
        assert_eq!(captures("^{number}$", "1e"), None);
    }

    #[test]
    fn word_matches_a_name() {
        assert_eq!(
            captures("({word}): ({number})", "> temp_1: 20.5"),
            Some(vec![
                "temp_1: 20.5".to_string(),
                "temp_1".to_string(),
                "20.5".to_string()
            ])
        );
    }

    #[test]
    fn word_takes_symbols_but_not_spaces() {
        assert_eq!(
            captures("({word}): ({number})", "[log] temp-1.a: 20.5"),
            Some(vec![
                "temp-1.a: 20.5".to_string(),
                "temp-1.a".to_string(),
                "20.5".to_string()
            ])
        );
        assert_eq!(captures("^{word}$", "led\u{2028}on"), None);
    }

    #[test]
    fn word_ends_at_the_first_separator() {
        assert_eq!(
            captures("({word}):(.+)", "url:http://x"),
            Some(vec![
                "url:http://x".to_string(),
                "url".to_string(),
                "http://x".to_string()
            ])
        );
    }

    #[test]
    fn aliases_take_no_group_number_of_their_own() {
        // `{word}` before does not move `$1` along
        assert_eq!(
            captures("{word}=({number})", "volt=3.3"),
            Some(vec!["volt=3.3".to_string(), "3.3".to_string()])
        );
        // And a quantifier applies to the whole alias
        assert_eq!(expand_aliases("{word}?"), r"(?:[^\s]+?)?");
    }

    #[test]
    fn escaped_or_in_a_class_aliases_are_left_alone() {
        assert_eq!(expand_aliases(r"\{word}"), r"\{word}");
        assert_eq!(expand_aliases(r"\\{word}"), r"\\(?:[^\s]+?)");
        assert_eq!(expand_aliases("[{word}]"), "[{word}]");
        assert_eq!(expand_aliases("[]{word}]{word}"), r"[]{word}](?:[^\s]+?)");
        assert_eq!(expand_aliases(r"[\]{word}]"), r"[\]{word}]");
    }

    #[test]
    fn other_braces_are_left_alone() {
        assert_eq!(expand_aliases(r"\d{2}{words}"), r"\d{2}{words}");
        assert_eq!(expand_aliases("{number"), "{number");
    }
}
