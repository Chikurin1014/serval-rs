use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, ConversionInput, ConversionOutput, DataContext, Endpoints, LogData,
    LogLevel, MapRunner, Segment, SourceCursor, keep_taken,
};

#[derive(Clone, Copy, PartialEq)]
pub struct EncodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
    /// The weakest level of the input taken: those weaker left out.
    pub weakest_level: Signal<LogLevel>,
}

/// Turns each log message into its UTF-8 bytes.
pub struct Encode {
    settings: EncodeSettings,
    taken: Option<(Endpoints, LogLevel)>,
    cursor: SourceCursor,
}

impl Encode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: EncodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
                weakest_level: Signal::new(LogLevel::None),
            },
            taken: None,
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for Encode {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn start(&mut self) {
        let EncodeSettings {
            from_label,
            to_label,
            weakest_level,
        } = self.settings;
        let taken = Endpoints::new(&from_label.peek(), &to_label.peek())
            .map(|endpoints| (endpoints, *weakest_level.peek()));
        if keep_taken(&mut self.taken, taken) {
            self.cursor.reset();
        }
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) -> Option<Conversion> {
        let (Endpoints { from, to }, weakest) = self.taken.as_ref()?;
        let weakest = *weakest;
        let Some(read) = self.cursor.new_entries::<LogData>(data, from) else {
            self.cursor.reset();
            return None;
        };
        let messages: Vec<&str> = read
            .entries
            .iter()
            .filter(|entry| entry.value().level() <= weakest)
            .map(|entry| entry.value().message())
            .collect();
        let last = *messages.last()?;

        let conversion = Conversion {
            from: vec![ConversionInput::new(from, last)],
            to: vec![ConversionOutput::new(
                vec![Segment::fixed(to)],
                vec![Segment::from_input(last)],
            )],
        };
        for message in messages {
            data.push(to, ByteData::new(timestamp, message.as_bytes().to_vec()));
        }
        Some(conversion)
    }
}
