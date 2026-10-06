use std::any::Any;

use dioxus::prelude::*;

use crate::data::{
    ByteData, Conversion, DataContext, MapRunner, Segment, SourceCursor, StringData, set_if_changed,
};

#[derive(Clone, Copy, PartialEq)]
pub struct EncodeSettings {
    pub from_label: Signal<String>,
    pub to_label: Signal<String>,
}

/// Turns each string into its UTF-8 bytes: the reverse of [`Decode`](super::Decode)
/// without a delimiter.
pub struct Encode {
    settings: EncodeSettings,
    latest: Signal<Option<Conversion>>,
    cursor: SourceCursor,
}

impl Encode {
    pub fn new(from_label: &str, to_label: &str) -> Self {
        Self {
            settings: EncodeSettings {
                from_label: Signal::new(from_label.to_string()),
                to_label: Signal::new(to_label.to_string()),
            },
            latest: Signal::new(None),
            cursor: SourceCursor::default(),
        }
    }
}

impl MapRunner for Encode {
    fn settings(&self) -> &dyn Any {
        &self.settings
    }

    fn latest(&self) -> Signal<Option<Conversion>> {
        self.latest
    }

    fn run(&mut self, data: &mut DataContext, timestamp: i64) {
        let EncodeSettings {
            from_label,
            to_label,
        } = self.settings;
        let (source, target) = (from_label(), to_label());
        let (source, target) = (source.trim(), target.trim());
        if source.is_empty() || target.is_empty() || source == target {
            return;
        }

        let Some(read) = self.cursor.new_entries::<StringData>(data, source) else {
            self.cursor.reset();
            return;
        };
        let Some(last) = read.entries.last() else {
            return;
        };

        let conversion = Conversion {
            from_label: source.to_string(),
            from_value: last.value().clone(),
            to_label: vec![Segment::fixed(target)],
            to_value: vec![Segment::from_input(last.value().as_str())],
        };
        set_if_changed(&mut self.latest, Some(conversion));
        for entry in &read.entries {
            data.push(
                target,
                ByteData::new(timestamp, entry.value().as_bytes().to_vec()),
            );
        }
    }
}
