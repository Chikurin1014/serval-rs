//! A value of a fixed set, each of its own name: as a setting is chosen.

/// One of a fixed set of values, each named: taken by its name too.
pub trait Choice: Copy + PartialEq + 'static {
    /// What they are, in errors: e.g. "parity".
    const KIND: &'static str;

    /// All of them, in the order they are shown.
    const ALL: &'static [Self];

    /// As it is shown and taken.
    fn name(self) -> &'static str;

    /// Of its name, as [`Self::name`].
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|choice| choice.name() == name)
    }

    /// Of its name, or an error naming those there are: e.g. "Not a parity:
    /// mark (none, even, or odd)".
    fn parse(name: &str) -> Result<Self, String> {
        Self::from_name(name).ok_or_else(|| {
            let names: Vec<_> = Self::ALL.iter().map(|choice| choice.name()).collect();
            let names = match names.as_slice() {
                [one, other] => format!("{one} or {other}"),
                [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
                [] => String::new(),
            };
            format!("Not a {}: {name} ({names})", Self::KIND)
        })
    }

    /// Where it is in [`Self::ALL`].
    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|&choice| choice == self)
            .expect("each is in ALL")
    }
}

#[cfg(test)]
mod tests {
    use super::Choice;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Size {
        Small,
        Medium,
        Large,
    }

    impl Choice for Size {
        const KIND: &'static str = "size";
        const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large];

        fn name(self) -> &'static str {
            match self {
                Self::Small => "small",
                Self::Medium => "medium",
                Self::Large => "large",
            }
        }
    }

    #[test]
    fn is_taken_by_its_name() {
        assert_eq!(Size::parse("medium"), Ok(Size::Medium));
        assert_eq!(Size::Large.index(), 2);
    }

    #[test]
    fn an_unknown_name_is_told_with_those_there_are() {
        assert_eq!(
            Size::parse("huge"),
            Err("Not a size: huge (small, medium, or large)".to_string())
        );
    }
}
