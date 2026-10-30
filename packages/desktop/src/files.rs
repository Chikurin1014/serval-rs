//! Files the user picks through the OS's dialogs: record files, written on a
//! thread of their own, and the files the app saves.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
    sync::mpsc,
    thread,
};

use chrono::Local;
use dioxus::desktop::window;
use rfd::AsyncFileDialog;
use tokio::sync::{oneshot, watch};

use ui::{
    save::FileSaver,
    serial::{
        LocalFuture, SerialResult,
        record::{RecordFile, RecordSink, TAIL_BYTES},
    },
};

use crate::serial::blocking;

/// A dialog over the app's window.
fn dialog() -> AsyncFileDialog {
    AsyncFileDialog::new().set_parent(&window().window)
}

/// A record file the user picks; see `SerialBackend::open_record_file`.
pub async fn open_record(extension: &str, append: bool) -> SerialResult<Option<RecordFile>> {
    let handle = if append {
        dialog().set_title("Continue a record").pick_file().await
    } else {
        // As the web app names it
        let name = format!(
            "serval-{}.{extension}",
            Local::now().format("%Y%m%d-%H%M%S")
        );
        dialog()
            .set_title("Save a record")
            .set_file_name(name)
            .add_filter("Record", &[extension])
            .save_file()
            .await
    };
    let Some(handle) = handle else {
        return Ok(None);
    };
    let path = handle.path().to_path_buf();
    let (file, tail) = blocking(move || open_at_end(&path, append))
        .await
        .map_err(|error| format!("{}: {error}", handle.path().display()))?;
    Ok(Some(RecordFile {
        name: handle.file_name(),
        tail,
        sink: Rc::new(FileSink::new(file)),
    }))
}

/// The file at `path` to write to: emptied, or with `append`, at its end, with
/// its last [`TAIL_BYTES`] bytes.
fn open_at_end(path: &Path, append: bool) -> io::Result<(File, Vec<u8>)> {
    if !append {
        return Ok((File::create(path)?, Vec::new()));
    }
    let mut file = OpenOptions::new().read(true).append(true).open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(size.saturating_sub(TAIL_BYTES as u64)))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    Ok((file, tail))
}

enum Command {
    Write(Vec<u8>),
    Close(oneshot::Sender<SerialResult<()>>),
}

/// Writes in order on a thread, each straight to the file: so the OS has all
/// that is written, even if the app ends.
struct FileSink {
    commands: mpsc::Sender<Command>,
    /// `None` till the first write fails (`Some(Some(why))`) or it is closed
    /// without one (`Some(None)`).
    failure: watch::Receiver<Option<Option<String>>>,
}

impl FileSink {
    fn new(mut file: File) -> Self {
        let (commands, received) = mpsc::channel();
        let (failed, failure) = watch::channel(None);
        thread::spawn(move || {
            let mut error = None;
            for command in received {
                match command {
                    Command::Write(bytes) if error.is_none() => {
                        if let Err(why) = file.write_all(&bytes) {
                            let why = why.to_string();
                            failed.send_replace(Some(Some(why.clone())));
                            error = Some(why);
                        }
                    }
                    Command::Write(_) => {}
                    Command::Close(closed) => {
                        let result = match error.take() {
                            Some(why) => Err(why),
                            None => file.flush().map_err(|why| why.to_string()),
                        };
                        if result.is_ok() {
                            failed.send_replace(Some(None));
                        }
                        let _ = closed.send(result);
                        return;
                    }
                }
            }
        });
        Self { commands, failure }
    }
}

impl RecordSink for FileSink {
    fn write(&self, bytes: Vec<u8>) {
        let _ = self.commands.send(Command::Write(bytes));
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let (closed, result) = oneshot::channel();
        let sent = self.commands.send(Command::Close(closed));
        Box::pin(async move {
            sent.map_err(|_| "The record is closed already".to_string())?;
            result
                .await
                .map_err(|_| "The record's writer stopped".to_string())?
        })
    }

    fn failure(&self) -> LocalFuture<Option<String>> {
        let mut failure = self.failure.clone();
        Box::pin(async move {
            let failure = failure.wait_for(Option::is_some).await.ok()?;
            failure.clone().flatten()
        })
    }
}

/// Saves the app's files where the user picks, through a save dialog.
pub fn saver() -> FileSaver {
    FileSaver::new(|name, text| {
        Box::pin(async move {
            let extension = Path::new(&name)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_string);
            let mut dialog = dialog().set_file_name(&name);
            if let Some(extension) = &extension {
                dialog = dialog.add_filter(extension.to_uppercase(), &[extension]);
            }
            let Some(handle) = dialog.save_file().await else {
                return Ok(());
            };
            handle
                .write(text.as_bytes())
                .await
                .map_err(|error| format!("{}: {error}", handle.path().display()))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[tokio::test]
    async fn writes_in_order_and_appends_after_the_tail() {
        let path = std::env::temp_dir().join(format!("serval-record-{}", std::process::id()));
        let (file, tail) = open_at_end(&path, false).unwrap();
        assert!(tail.is_empty());
        let sink = FileSink::new(file);
        sink.write(b"one\n".to_vec());
        sink.write(b"two\n".to_vec());
        sink.close().await.unwrap();
        assert_eq!(sink.failure().await, None);

        let (file, tail) = open_at_end(&path, true).unwrap();
        assert_eq!(tail, b"one\ntwo\n");
        let sink = FileSink::new(file);
        sink.write(b"three\n".to_vec());
        sink.close().await.unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"one\ntwo\nthree\n");
        fs::remove_file(&path).unwrap();
    }
}
