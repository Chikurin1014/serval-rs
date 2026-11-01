//! Recording what the port receives (and sends) to a file the user picks:
//! one record at a time, on while ports open and close.

use std::rc::Rc;

use dioxus::prelude::*;

use super::{
    LocalFuture, NotificationKind, SerialContext, SerialResult,
    record::{self, RecordFile, RecordFormat, RecordSink, Recorder},
};

impl SerialContext {
    /// The format of the record file being written, if one is.
    pub fn record_format(&self) -> Option<RecordFormat> {
        (self.record_format)()
    }

    /// Writes what the port receives (and sends, if `format` keeps it) to
    /// `sink`, until [`Self::stop_record`]; a record already being written ends first.
    pub fn start_record(&self, format: RecordFormat, sink: Rc<dyn RecordSink>) {
        self.stop_record();
        let (mut recorder, mut record_format, mut record_id) =
            (self.recorder, self.record_format, self.record_id);
        let id = *record_id.peek() + 1;
        record_id.set(id);
        let failure = sink.failure();
        recorder.set(Some(Recorder::new(format, sink)));
        record_format.set(Some(format));
        // Ended at once by a failed write, not to go on unwritten unseen
        let context = *self;
        self.spawn(async move {
            if let Some(error) = failure.await {
                context.fail_record(id, &error);
            }
        });
    }

    /// Ends record `id`, if it is still the one being written, as a write failed.
    fn fail_record(&self, id: u64, error: &str) {
        let (mut recorder, mut record_format) = (self.recorder, self.record_format);
        if *self.record_id.peek() != id {
            return;
        }
        let Some(recorder) = recorder.write().take() else {
            return;
        };
        record_format.set(None);
        // Closed to let go of the file; its failure is the one told here
        let closed = recorder.close();
        self.spawn(async move {
            let _ = closed.await;
        });
        self.report(
            NotificationKind::Error,
            "Recording stopped",
            &format!("Failed to write the record: {error}"),
        );
    }

    /// Asks for a new file (see [`SerialBackend::open_record_file`]) and records to
    /// it in `format`. Call it as the user asks: browsers pick files only then.
    pub fn open_record(&self, format: RecordFormat) {
        self.pick_record_file(Some(format));
    }

    /// As [`Self::open_record`], to an existing file, in the format it was
    /// written in (see [`record::detect_format`]).
    pub fn continue_record(&self) {
        self.pick_record_file(None);
    }

    /// A new file for `format`, or an existing one for `None`.
    fn pick_record_file(&self, format: Option<RecordFormat>) {
        let context = *self;
        self.spawn(async move {
            let backend = context.backend.cloned();
            let extension = format.map_or("log", record::extension);
            match backend.open_record_file(extension, format.is_none()).await {
                Ok(Some(RecordFile { sink, name, tail })) => {
                    let (format, title) = match format {
                        Some(format) => (format, "Recording started"),
                        None => (record::detect_format(&name, &tail), "Recording continued"),
                    };
                    context.start_record(format, sink);
                    let detail = format!("{name} ({})", format.name());
                    context.report(NotificationKind::Info, title, &detail);
                }
                Ok(None) => {}
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to open the record", &error)
                }
            }
        });
    }

    /// As [`Self::stop_record`], telling nothing: the file is done once the
    /// future is (e.g. for the app to close then). `None` with no record.
    pub fn finish_record(&self) -> Option<LocalFuture<SerialResult<()>>> {
        let (mut recorder, mut record_format) = (self.recorder, self.record_format);
        let recorder = recorder.write().take()?;
        record_format.set(None);
        Some(recorder.close())
    }

    /// Ends the record file, with its unfinished lines.
    pub fn stop_record(&self) {
        let Some(closed) = self.finish_record() else {
            return;
        };
        let context = *self;
        self.spawn(async move {
            match closed.await {
                Ok(()) => context.report(NotificationKind::Info, "Recording stopped", ""),
                Err(error) => {
                    context.report(NotificationKind::Error, "Failed to save the record", &error)
                }
            }
        });
    }
}
