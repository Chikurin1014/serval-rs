use dioxus::prelude::*;
use regex::Regex;

/// What a filter does to the labels it matches.
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

/// The regexes the data list shows labels by: a label shows if any `Show`
/// filter matches the whole of it (or there is none), and no `Hide` filter does.
/// Provided above the tabs (see `Home`), so they stay when the data list is
/// left and shown again.
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
    /// No filters yet; call in the providing component, as it makes a signal.
    pub fn new() -> Self {
        Self {
            filters: Signal::new(Vec::new()),
        }
    }

    /// As [`Self::new`], with the filters to start with (valid patterns).
    pub fn with(filters: &[(FilterKind, &str)]) -> Self {
        let mut context = Self::new();
        for &(kind, pattern) in filters {
            let _ = context.add(kind, pattern);
        }
        context
    }

    /// The filters' kinds and patterns, in the order they were added.
    pub fn filters(&self) -> Vec<(FilterKind, String)> {
        self.filters
            .read()
            .iter()
            .map(|filter| (filter.kind, filter.pattern.clone()))
            .collect()
    }

    /// Adds a `kind` filter by `pattern`, unless there is one already; an
    /// error if `pattern` is not a regex.
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

    /// Whether the data list shows `label`.
    pub fn shows(&self, label: &str) -> bool {
        shows(&self.filters.read(), label)
    }
}

impl Default for FilterContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A regex matching only the whole of a label, not a part of it.
fn whole_match(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&format!("^(?:{pattern})$"))
}

fn shows(filters: &[Filter], label: &str) -> bool {
    let mut show = filters
        .iter()
        .filter(|filter| filter.kind == FilterKind::Show)
        .peekable();
    let shown = show.peek().is_none() || show.any(|filter| filter.regex.is_match(label));
    let hidden = filters
        .iter()
        .any(|filter| filter.kind == FilterKind::Hide && filter.regex.is_match(label));
    shown && !hidden
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
