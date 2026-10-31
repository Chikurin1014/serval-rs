use crate::data::{
    CONSTANT_PREFIX, ConversionInput, Data, DataContext, DataEntry, DataType, SourceCursor,
    format_number,
};

/// `from` and `to`, trimmed, if both are set and differ.
pub(crate) fn endpoints<'a>(from: &'a str, to: &'a str) -> Option<(&'a str, &'a str)> {
    let (from, to) = (from.trim(), to.trim());
    (!from.is_empty() && !to.is_empty() && from != to).then_some((from, to))
}

/// A map's input and output labels, checked by [`endpoints`].
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Endpoints {
    pub(crate) from: String,
    pub(crate) to: String,
}

impl Endpoints {
    pub(crate) fn new(from: &str, to: &str) -> Option<Self> {
        endpoints(from, to).map(|(from, to)| Self {
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}

/// One of a map's inputs: how far it is read, and its newest unused value.
pub(crate) struct Input<T> {
    cursor: SourceCursor,
    newest: Option<T>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            cursor: SourceCursor::default(),
            newest: None,
        }
    }
}

impl<T: Clone> Input<T> {
    /// The values added under `label` since the last read.
    pub(crate) fn read(&mut self, data: &DataContext, label: &str) -> Vec<T>
    where
        Data<T>: DataEntry,
    {
        match self.cursor.new_entries::<Data<T>>(data, label) {
            Some(read) => {
                if read.restarted {
                    self.newest = None;
                }
                read.entries
                    .iter()
                    .map(|entry| entry.value().clone())
                    .collect()
            }
            None => {
                self.forget();
                Vec::new()
            }
        }
    }

    pub(crate) fn read_newest(&mut self, data: &DataContext, label: &str)
    where
        Data<T>: DataEntry,
    {
        if let Some(last) = self.read(data, label).pop() {
            self.newest = Some(last);
        }
    }

    /// As [`Self::read_newest`], of those `taken` alone.
    pub(crate) fn read_newest_where(
        &mut self,
        data: &DataContext,
        label: &str,
        taken: impl Fn(&T) -> bool,
    ) where
        Data<T>: DataEntry,
    {
        if let Some(last) = self.read(data, label).into_iter().filter(taken).last() {
            self.newest = Some(last);
        }
    }

    pub(crate) fn forget(&mut self) {
        self.cursor.reset();
        self.newest = None;
    }
}

/// Where a value a map takes comes from: a label, or a constant.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source<T> {
    Label(String),
    Constant(T),
}

impl<T> Source<T> {
    /// Its label; none for a constant.
    pub(crate) fn label(&self) -> &str {
        match self {
            Source::Label(label) => label,
            Source::Constant(_) => "",
        }
    }
}

/// Where a number a map takes comes from: a Number label, or a constant.
pub(crate) type Operand = Source<f64>;

/// The constant `text` names after [`CONSTANT_PREFIX`] (as `#pi`), if any.
fn named_constant(text: &str) -> Option<f64> {
    match text.strip_prefix(CONSTANT_PREFIX)? {
        "pi" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        _ => None,
    }
}

/// `text` as a named constant (as `#pi`), or else a number.
pub(crate) fn constant_or_number(text: &str) -> Option<f64> {
    if text.starts_with(CONSTANT_PREFIX) {
        named_constant(text)
    } else {
        text.parse().ok()
    }
}

impl Operand {
    /// `text` as a named constant (as `#pi`), a Number label, or a number: a
    /// label over a number of its name.
    pub(crate) fn resolve(data: &DataContext, text: &str) -> Result<Self, String> {
        let text = text.trim();
        // No label of its name: none starts so
        if text.starts_with(CONSTANT_PREFIX) {
            return named_constant(text)
                .map(Operand::Constant)
                .ok_or_else(|| format!("No constant '{text}'"));
        }
        let is_label = data.data_type_of(text) == Some(DataType::Number);
        if is_label {
            Ok(Operand::Label(text.to_string()))
        } else {
            text.parse()
                .map(Operand::Constant)
                .map_err(|_| format!("No Number label or number '{text}'"))
        }
    }

    pub(crate) fn input(&self, value: f64) -> ConversionInput {
        ConversionInput::new(self.label(), format_number(value))
    }
}

/// The pairs of values to work out, of `sources` read by `inputs`: with two
/// labels, their newest once both have one; with a label and a constant, each
/// new value; with two constants, once (`constants_used` after).
pub(crate) fn pairs<A: Clone, B: Clone>(
    data: &DataContext,
    sources: (&Source<A>, &Source<B>),
    inputs: (&mut Input<A>, &mut Input<B>),
    constants_used: &mut bool,
) -> Vec<(A, B)>
where
    Data<A>: DataEntry,
    Data<B>: DataEntry,
{
    let (first_input, second_input) = inputs;
    if !matches!(sources, (Source::Constant(_), Source::Constant(_))) {
        *constants_used = false;
    }
    match sources {
        (Source::Label(first), Source::Label(second)) => {
            first_input.read_newest(data, first);
            second_input.read_newest(data, second);
            take_newest_pair(first_input, second_input)
                .into_iter()
                .collect()
        }
        (Source::Label(first), Source::Constant(second)) => {
            second_input.forget();
            let values = first_input.read(data, first);
            values
                .into_iter()
                .map(|first| (first, second.clone()))
                .collect()
        }
        (Source::Constant(first), Source::Label(second)) => {
            first_input.forget();
            let values = second_input.read(data, second);
            values
                .into_iter()
                .map(|second| (first.clone(), second))
                .collect()
        }
        (Source::Constant(first), Source::Constant(second)) => {
            first_input.forget();
            second_input.forget();
            if std::mem::replace(constants_used, true) {
                return Vec::new();
            }
            vec![(first.clone(), second.clone())]
        }
    }
}

/// The newest values of both inputs, once each has one.
pub(crate) fn take_newest_pair<A, B>(
    first: &mut Input<A>,
    second: &mut Input<B>,
) -> Option<(A, B)> {
    if first.newest.is_none() || second.newest.is_none() {
        return None;
    }
    first.newest.take().zip(second.newest.take())
}

/// The newest values of all `inputs`, once each has one.
pub(crate) fn take_newest_all<'a, T: 'a>(
    inputs: impl IntoIterator<Item = &'a mut Input<T>>,
) -> Option<Vec<T>> {
    let mut inputs: Vec<_> = inputs.into_iter().collect();
    if inputs.is_empty() || inputs.iter().any(|input| input.newest.is_none()) {
        return None;
    }
    inputs.iter_mut().map(|input| input.newest.take()).collect()
}

#[cfg(test)]
mod tests {
    use super::{Input, endpoints, named_constant, take_newest_all, take_newest_pair};

    #[test]
    fn constants_are_named_after_a_hash() {
        assert_eq!(named_constant("#pi"), Some(std::f64::consts::PI));
        assert_eq!(named_constant("#e"), Some(std::f64::consts::E));
        assert_eq!(named_constant("pi"), None);
        assert_eq!(named_constant("#tau"), None);
        assert_eq!(named_constant("#PI"), None);
    }

    #[test]
    fn endpoints_are_set_and_apart() {
        assert_eq!(endpoints(" raw ", "text"), Some(("raw", "text")));
        assert_eq!(endpoints("raw", " "), None);
        assert_eq!(endpoints("", "text"), None);
        assert_eq!(endpoints("raw", " raw"), None);
    }

    #[test]
    fn a_pair_needs_both_and_uses_them_up() {
        let mut first = Input::<f64>::default();
        let mut second = Input::<String>::default();
        first.newest = Some(1.0);
        assert_eq!(take_newest_pair(&mut first, &mut second), None);
        assert_eq!(first.newest, Some(1.0));

        second.newest = Some("a".to_string());
        assert_eq!(
            take_newest_pair(&mut first, &mut second),
            Some((1.0, "a".to_string()))
        );
        assert_eq!(take_newest_pair(&mut first, &mut second), None);
    }

    #[test]
    fn all_need_each_and_use_them_up() {
        let mut inputs: [Input<f64>; 3] = Default::default();
        inputs[0].newest = Some(1.0);
        inputs[2].newest = Some(3.0);
        assert_eq!(take_newest_all(&mut inputs), None);
        assert_eq!(inputs[0].newest, Some(1.0));

        inputs[1].newest = Some(2.0);
        assert_eq!(take_newest_all(&mut inputs), Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(take_newest_all(&mut inputs), None);
        assert_eq!(take_newest_all(&mut [] as &mut [Input<f64>]), None);
    }
}
