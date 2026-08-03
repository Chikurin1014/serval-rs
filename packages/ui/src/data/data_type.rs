use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub struct LabeledData {
    pub label: String,
    pub queue: TypedQueue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypedQueue {
    Timestamp(VecDeque<i64>),
    Number(VecDeque<f64>),
    String(VecDeque<String>),
    Bytes(VecDeque<Vec<u8>>),
}
