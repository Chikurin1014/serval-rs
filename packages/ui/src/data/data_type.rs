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
    use super::{ByteData, DataEntry, NumberData, StringData, TypedData};
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
}
