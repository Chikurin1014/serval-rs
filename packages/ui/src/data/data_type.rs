use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum TypedData {
    Number(VecDeque<NumberData>),
    String(VecDeque<StringData>),
    Bytes(VecDeque<ByteData>),
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
