//! Which labels the app shows, of the filters the user adds.

use dioxus::prelude::*;
use fancy_regex::Regex;

use crate::data::compile_pattern;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Show,
    Hide,
}

impl FilterKind {
    pub fn name(self) -> &'static str {
        match self {
            FilterKind::Show => "Show",
            FilterKind::Hide => "Hide",
        }
    }
}

/// Which labels the data list shows: those a `Show` filter matches (if any) and
/// no `Hide` filter does.
#[derive(Clone, Copy, PartialEq)]
pub struct FilterContext {
    filters: Signal<Vec<Filter>>,
}

#[derive(Clone, Debug)]
struct Filter {
    kind: FilterKind,
    pattern: String,
    regex: Regex,
}

impl FilterContext {
    pub fn new() -> Self {
        Self {
            filters: Signal::new(Vec::new()),
        }
    }

    pub fn with(filters: &[(FilterKind, &str)]) -> Self {
        let mut context = Self::new();
        for &(kind, pattern) in filters {
            let _ = context.add(kind, pattern);
        }
        context
    }

    pub fn filters(&self) -> Vec<(FilterKind, String)> {
        self.filters
            .read()
            .iter()
            .map(|filter| (filter.kind, filter.pattern.clone()))
            .collect()
    }

    /// Adds a filter unless there is one already; an error for an invalid regex.
    pub fn add(&mut self, kind: FilterKind, pattern: &str) -> Result<(), String> {
        let regex = whole_match(pattern).map_err(|error| format!("Invalid regex: {error}"))?;
        if !self
            .filters
            .read()
            .iter()
            .any(|filter| filter.kind == kind && filter.pattern == pattern)
        {
            self.filters.write().push(Filter {
                kind,
                pattern: pattern.to_string(),
                regex,
            });
        }
        Ok(())
    }

    pub fn remove(&mut self, kind: FilterKind, pattern: &str) {
        self.filters
            .write()
            .retain(|filter| !(filter.kind == kind && filter.pattern == pattern));
    }

    pub fn shows(&self, label: &str) -> bool {
        shows(&self.filters.read(), label)
    }
}

impl Default for FilterContext {
    fn default() -> Self {
        Self::new()
    }
}

fn whole_match(pattern: &str) -> Result<Regex, fancy_regex::Error> {
    compile_pattern(&format!("^(?:{pattern})$"))
}

fn shows(filters: &[Filter], label: &str) -> bool {
    let mut show = filters
        .iter()
        .filter(|filter| filter.kind == FilterKind::Show)
        .peekable();
    let shown = show.peek().is_none() || show.any(|filter| matches(filter, label));
    let hidden = filters
        .iter()
        .any(|filter| filter.kind == FilterKind::Hide && matches(filter, label));
    shown && !hidden
}

fn matches(filter: &Filter, label: &str) -> bool {
    filter.regex.is_match(label).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{Filter, FilterKind, shows, whole_match};

    fn filters(filters: &[(FilterKind, &str)]) -> Vec<Filter> {
        filters
            .iter()
            .map(|&(kind, pattern)| Filter {
                kind,
                pattern: pattern.to_string(),
                regex: whole_match(pattern).unwrap(),
            })
            .collect()
    }

    #[test]
    fn no_filters_show_every_label() {
        assert!(shows(&[], "raw_bytes"));
    }

    #[test]
    fn a_label_shows_if_any_show_filter_matches() {
        let filters = filters(&[(FilterKind::Show, "temp.*"), (FilterKind::Show, "volt")]);
        assert!(shows(&filters, "temp_rate"));
        assert!(shows(&filters, "volt"));
        assert!(!shows(&filters, "message"));
    }

    #[test]
    fn filters_match_whole_labels() {
        let filters = filters(&[(FilterKind::Show, "temp|volt")]);
        assert!(shows(&filters, "volt"));
        assert!(!shows(&filters, "temp_rate"));
        assert!(!shows(&filters, "my_volt"));
    }

    #[test]
    fn filters_can_look_around() {
        let filters = filters(&[(FilterKind::Show, r"(?!.*_rate$).*")]);
        assert!(shows(&filters, "temp"));
        assert!(!shows(&filters, "temp_rate"));
    }

    #[test]
    fn filters_can_use_aliases() {
        let filters = filters(&[(FilterKind::Hide, "{word}_rate")]);
        assert!(!shows(&filters, "temp_rate"));
        assert!(shows(&filters, "temp"));
    }

    #[test]
    fn hide_filters_hide_what_they_match() {
        let filters = filters(&[(FilterKind::Hide, "raw_.*")]);
        assert!(!shows(&filters, "raw_bytes"));
        assert!(shows(&filters, "temp"));
    }

    #[test]
    fn hide_filters_win_over_show_filters() {
        let filters = filters(&[(FilterKind::Show, "temp.*"), (FilterKind::Hide, ".*_rate")]);
        assert!(shows(&filters, "temp"));
        assert!(!shows(&filters, "temp_rate"));
        assert!(!shows(&filters, "volt"));
    }
}
