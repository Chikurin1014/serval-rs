use crate::data::Queue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Number,
    String,
    Bytes,
    Vector2d,
    Vector3d,
}

impl DataType {
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Number => "Number",
            DataType::String => "String",
            DataType::Bytes => "Bytes",
            DataType::Vector2d => "Vector2d",
            DataType::Vector3d => "Vector3d",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypedData {
    Number(Queue<NumberData>),
    String(Queue<StringData>),
    Bytes(Queue<ByteData>),
    Vector2d(Queue<Vector2dData>),
    Vector3d(Queue<Vector3dData>),
}

impl TypedData {
    pub fn data_type(&self) -> DataType {
        match self {
            TypedData::Number(_) => DataType::Number,
            TypedData::String(_) => DataType::String,
            TypedData::Bytes(_) => DataType::Bytes,
            TypedData::Vector2d(_) => DataType::Vector2d,
            TypedData::Vector3d(_) => DataType::Vector3d,
        }
    }

    /// When the newest entry was received, if there is one.
    pub fn latest_timestamp(&self) -> Option<i64> {
        match self {
            TypedData::Number(queue) => queue.back().map(Data::timestamp),
            TypedData::String(queue) => queue.back().map(Data::timestamp),
            TypedData::Bytes(queue) => queue.back().map(Data::timestamp),
            TypedData::Vector2d(queue) => queue.back().map(Data::timestamp),
            TypedData::Vector3d(queue) => queue.back().map(Data::timestamp),
        }
    }

    /// The newest `count` entries as `(timestamp, text)`, oldest first.
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
            TypedData::Number(queue) => newest(queue, count, |value| number_text(*value, numbers)),
            TypedData::String(queue) => newest(queue, count, String::clone),
            TypedData::Bytes(queue) => newest(queue, count, |value| {
                String::from_utf8_lossy(value).into_owned()
            }),
            TypedData::Vector2d(queue) => newest(queue, count, |value| {
                vector_text(value.components(), numbers)
            }),
            TypedData::Vector3d(queue) => newest(queue, count, |value| {
                vector_text(value.components(), numbers)
            }),
        }
    }
}

/// How [`TypedData::newest_as_text`] writes numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberText {
    Rounded,
    Exact,
}

/// A number as `numbers` says.
fn number_text(value: f64, numbers: NumberText) -> String {
    match numbers {
        NumberText::Rounded => format_number(value),
        NumberText::Exact => value.to_string(),
    }
}

/// A vector as `(x, y)` or `(x, y, z)`, each component as [`number_text`] writes it.
fn vector_text(components: &[f64], numbers: NumberText) -> String {
    let components: Vec<String> = components
        .iter()
        .map(|&component| number_text(component, numbers))
        .collect();
    format!("({})", components.join(", "))
}

/// How many significant digits [`format_number`] shows.
const SIGNIFICANT_DIGITS: i32 = 5;

/// `value` to five significant digits, e.g. `4.8950` or `1.2346e5`.
pub fn format_number(value: f64) -> String {
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
pub type Vector2dData = Data<Vector2d>;
pub type Vector3dData = Data<Vector3d>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector2d([f64; 2]);

impl Vector2d {
    pub fn new(x: f64, y: f64) -> Self {
        Self([x, y])
    }

    pub fn x(&self) -> f64 {
        self.0[0]
    }

    pub fn y(&self) -> f64 {
        self.0[1]
    }

    /// `[x, y]`.
    pub fn components(&self) -> &[f64] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3d([f64; 3]);

impl Vector3d {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self([x, y, z])
    }

    pub fn x(&self) -> f64 {
        self.0[0]
    }

    pub fn y(&self) -> f64 {
        self.0[1]
    }

    pub fn z(&self) -> f64 {
        self.0[2]
    }

    /// `[x, y, z]`.
    pub fn components(&self) -> &[f64] {
        &self.0
    }
}

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

/// An entry type of [`TypedData`], stored in a variant of its own.
pub trait DataEntry: Clone + Into<TypedData> {
    fn timestamp(&self) -> i64;

    fn queue(data: &TypedData) -> Option<&Queue<Self>>;

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
data_entry!(Vector2dData, Vector2d);
data_entry!(Vector3dData, Vector3d);

#[cfg(test)]
mod tests {
    use super::{
        ByteData, DataEntry, DataType, NumberData, NumberText, StringData, TypedData, Vector2d,
        Vector2dData, Vector3d, Vector3dData, format_number,
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

    #[test]
    fn vectors_have_their_own_queues_and_types() {
        let mut vectors: TypedData = Vector2dData::new(1, Vector2d::new(1.0, 2.0)).into();
        assert_eq!(vectors.data_type(), DataType::Vector2d);
        assert!(Vector3dData::queue(&vectors).is_none());
        Vector2dData::queue_mut(&mut vectors)
            .unwrap()
            .push(Vector2dData::new(2, Vector2d::new(3.0, 4.0)));
        assert_eq!(vectors.latest_timestamp(), Some(2));

        let vectors: TypedData = Vector3dData::new(1, Vector3d::new(1.0, 2.0, 3.0)).into();
        assert_eq!(vectors.data_type(), DataType::Vector3d);
        assert_eq!(DataType::Vector3d.name(), "Vector3d");
    }

    #[test]
    fn vectors_tell_their_components() {
        let vector = Vector2d::new(1.0, 2.0);
        assert_eq!((vector.x(), vector.y()), (1.0, 2.0));
        let vector = Vector3d::new(1.0, 2.0, 3.0);
        assert_eq!((vector.x(), vector.y(), vector.z()), (1.0, 2.0, 3.0));
        assert_eq!(vector.components(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn vectors_as_text_list_their_components() {
        let vectors = TypedData::Vector2d(Queue::from_iter([Vector2dData::new(
            1,
            Vector2d::new(1.0 / 3.0, -2.0),
        )]));
        assert_eq!(
            vectors.newest_as_text(1, NumberText::Rounded),
            vec![(1, "(0.33333, -2.0000)".to_string())]
        );
        let vectors = TypedData::Vector3d(Queue::from_iter([Vector3dData::new(
            1,
            Vector3d::new(0.5, 1.0, 1e6),
        )]));
        assert_eq!(
            vectors.newest_as_text(1, NumberText::Exact),
            vec![(1, "(0.5, 1, 1000000)".to_string())]
        );
    }
}
