use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum TypedData {
    Number(VecDeque<NumberData>),
    String(VecDeque<StringData>),
    Bytes(VecDeque<ByteData>),
}

impl TypedData {
    pub fn type_name(&self) -> &'static str {
        match self {
            TypedData::Number(_) => "Number",
            TypedData::String(_) => "String",
            TypedData::Bytes(_) => "Bytes",
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
