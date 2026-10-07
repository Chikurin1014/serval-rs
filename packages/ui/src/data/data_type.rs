use crate::data::Queue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Number,
    String,
    Bytes,
}

impl DataType {
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Number => "Number",
            DataType::String => "String",
            DataType::Bytes => "Bytes",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypedData {
    Number(Queue<NumberData>),
    String(Queue<StringData>),
    Bytes(Queue<ByteData>),
}

impl TypedData {
    pub fn data_type(&self) -> DataType {
        match self {
            TypedData::Number(_) => DataType::Number,
            TypedData::String(_) => DataType::String,
            TypedData::Bytes(_) => DataType::Bytes,
        }
    }

    /// When the newest entry was received, if there is one.
    pub fn latest_timestamp(&self) -> Option<i64> {
        match self {
            TypedData::Number(queue) => queue.back().map(Data::timestamp),
            TypedData::String(queue) => queue.back().map(Data::timestamp),
            TypedData::Bytes(queue) => queue.back().map(Data::timestamp),
        }
    }

    /// The newest `count` entries, oldest first, with when they came and their
    /// values as text: numbers as `numbers` says, bytes decoded as UTF-8.
    pub fn newest_as_text(&self, count: usize, numbers: NumberText) -> Vec<(i64, String)> {
        fn newest<T: Clone>(
            queue: &Queue<Data<T>>,
            count: usize,
            text: impl Fn(&T) -> String,
        ) -> Vec<(i64, String)> {
            queue
                .iter()
                .skip(queue.len().saturating_sub(count))
                .map(|entry| (entry.timestamp(), text(entry.value())))
                .collect()
        }
        match self {
            TypedData::Number(queue) => newest(queue, count, |value| match numbers {
                NumberText::Rounded => format_number(*value),
                NumberText::Exact => value.to_string(),
            }),
            TypedData::String(queue) => newest(queue, count, String::clone),
            TypedData::Bytes(queue) => newest(queue, count, |value| {
                String::from_utf8_lossy(value).into_owned()
            }),
        }
    }
}

/// How [`TypedData::newest_as_text`] writes numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberText {
    /// With [`format_number`], for display.
    Rounded,
    /// In full, e.g. for export.
    Exact,
}

/// How many significant digits [`format_number`] shows.
const SIGNIFICANT_DIGITS: i32 = 5;

/// `value` to five significant digits for display, e.g.
/// `4.8950`, or as `1.2346e5` when its integer part has more digits.
pub fn format_number(value: f64) -> String {
    // The digits before the point (`log10` has none for 0)
    let integer_digits = if value == 0.0 {
        1.0
    } else {
        value.abs().log10().floor() + 1.0
    };
    if integer_digits > SIGNIFICANT_DIGITS as f64 {
        return format!("{value:.*e}", SIGNIFICANT_DIGITS as usize - 1);
    }
    let decimals = (SIGNIFICANT_DIGITS as f64 - integer_digits).clamp(0.0, 15.0) as usize;
    format!("{value:.decimals$}")
}

pub type NumberData = Data<f64>;
pub type StringData = Data<String>;
pub type ByteData = Data<Vec<u8>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Data<T: Clone> {
    timestamp: i64,
    value: T,
}

impl<T: Clone> Data<T> {
    pub fn new(timestamp: i64, value: T) -> Self {
        Self { timestamp, value }
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

/// An entry type of [`TypedData`]: each one is stored in its own variant.
pub trait DataEntry: Clone + Into<TypedData> {
    fn timestamp(&self) -> i64;

    /// The queue of entries of this type in `data`, if it holds this type.
    fn queue(data: &TypedData) -> Option<&Queue<Self>>;

    /// Like [`DataEntry::queue`], to change the queue.
    fn queue_mut(data: &mut TypedData) -> Option<&mut Queue<Self>>;
}

macro_rules! data_entry {
    ($entry:ty, $variant:ident) => {
        impl From<$entry> for TypedData {
            fn from(data: $entry) -> Self {
                TypedData::$variant(Queue::from_iter([data]))
            }
        }

        impl DataEntry for $entry {
            fn timestamp(&self) -> i64 {
                Data::timestamp(self)
            }

            fn queue(data: &TypedData) -> Option<&Queue<Self>> {
                match data {
                    TypedData::$variant(queue) => Some(queue),
                    _ => None,
                }
            }

            fn queue_mut(data: &mut TypedData) -> Option<&mut Queue<Self>> {
                match data {
                    TypedData::$variant(queue) => Some(queue),
                    _ => None,
                }
            }
        }
    };
}

data_entry!(NumberData, Number);
data_entry!(StringData, String);
data_entry!(ByteData, Bytes);

#[cfg(test)]
mod tests {
    use super::{
        ByteData, DataEntry, NumberData, NumberText, StringData, TypedData, format_number,
    };

    #[test]
    fn format_number_shows_five_significant_digits() {
        assert_eq!(format_number(4.8949695964621345), "4.8950");
        assert_eq!(format_number(1234.56789), "1234.6");
        assert_eq!(format_number(0.000123456), "0.00012346");
        assert_eq!(format_number(-20.0), "-20.000");
        assert_eq!(format_number(0.0), "0.0000");
        assert_eq!(format_number(99999.4), "99999");
        assert_eq!(format_number(123456.7), "1.2346e5");
        assert_eq!(format_number(-0.5e9), "-5.0000e8");
    }
    use crate::data::Queue;

    #[test]
    fn each_entry_type_has_its_own_queue() {
        let mut numbers: TypedData = NumberData::new(1, 1.5).into();
        assert_eq!(NumberData::queue(&numbers).map(Queue::len), Some(1));
        assert!(StringData::queue(&numbers).is_none());
        assert!(ByteData::queue(&numbers).is_none());

        NumberData::queue_mut(&mut numbers)
            .unwrap()
            .push(NumberData::new(2, 2.5));
        assert_eq!(NumberData::queue(&numbers).map(Queue::len), Some(2));
        assert!(StringData::queue_mut(&mut numbers).is_none());
    }

    #[test]
    fn latest_timestamp_is_the_newest_entry() {
        let mut strings: TypedData = StringData::new(10, "a".to_string()).into();
        StringData::queue_mut(&mut strings)
            .unwrap()
            .push(StringData::new(20, "b".to_string()));
        assert_eq!(strings.latest_timestamp(), Some(20));
        assert_eq!(TypedData::Bytes(Queue::new()).latest_timestamp(), None);
    }

    #[test]
    fn newest_as_text_takes_the_newest_oldest_first() {
        let numbers = TypedData::Number(Queue::from_iter(
            (1..=3).map(|n| NumberData::new(n, n as f64 / 3.0)),
        ));
        assert_eq!(
            numbers.newest_as_text(2, NumberText::Rounded),
            vec![(2, "0.66667".to_string()), (3, "1.0000".to_string())]
        );
        assert_eq!(
            numbers.newest_as_text(1, NumberText::Exact),
            vec![(3, "1".to_string())]
        );
        assert_eq!(
            numbers.newest_as_text(usize::MAX, NumberText::Exact).len(),
            3
        );

        let bytes = TypedData::Bytes(Queue::from_iter([ByteData::new(1, b"ok\xff".to_vec())]));
        assert_eq!(
            bytes.newest_as_text(1, NumberText::Exact),
            vec![(1, "ok\u{fffd}".to_string())]
        );
    }
}
