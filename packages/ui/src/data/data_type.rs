use std::collections::VecDeque;

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
    Number(VecDeque<NumberData>),
    String(VecDeque<StringData>),
    Bytes(VecDeque<ByteData>),
}

impl TypedData {
    pub fn new(data_type: DataType) -> Self {
        match data_type {
            DataType::Number => TypedData::Number(VecDeque::new()),
            DataType::String => TypedData::String(VecDeque::new()),
            DataType::Bytes => TypedData::Bytes(VecDeque::new()),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            TypedData::Number(_) => DataType::Number,
            TypedData::String(_) => DataType::String,
            TypedData::Bytes(_) => DataType::Bytes,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TypedData::Number(queue) => queue.len(),
            TypedData::String(queue) => queue.len(),
            TypedData::Bytes(queue) => queue.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            TypedData::Number(queue) => queue.is_empty(),
            TypedData::String(queue) => queue.is_empty(),
            TypedData::Bytes(queue) => queue.is_empty(),
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

impl From<NumberData> for TypedData {
    fn from(data: NumberData) -> Self {
        TypedData::Number(VecDeque::from([data]))
    }
}

impl From<StringData> for TypedData {
    fn from(data: StringData) -> Self {
        TypedData::String(VecDeque::from([data]))
    }
}

impl From<ByteData> for TypedData {
    fn from(data: ByteData) -> Self {
        TypedData::Bytes(VecDeque::from([data]))
    }
}
