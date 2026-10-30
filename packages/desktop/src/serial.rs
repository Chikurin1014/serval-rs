//! Serial ports of the OS, through `serialport`: each read on a thread of its
//! own, the rest on tokio's blocking threads.

use std::{
    cell::RefCell,
    io::{self, Read},
    path::Path,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use serialport::{SerialPortInfo, SerialPortType};
use tokio::sync::mpsc;

use ui::serial::{
    Baudrate, ExtraSettings, FlowControl, LocalFuture, Parity, PortInfo, SerialBackend, SerialPort,
    SerialResult, record::RecordFile,
};

use crate::files;

/// How long a read waits for bytes, before it looks whether the port is closed.
const READ_TIMEOUT: Duration = Duration::from_millis(50);
const READ_BUFFER_BYTES: usize = 4096;

/// All the serial ports of the OS.
pub fn backend() -> Rc<dyn SerialBackend> {
    Rc::new(NativeSerial)
}

struct NativeSerial;

impl SerialBackend for NativeSerial {
    // All are listed: none is requested
    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>> {
        Box::pin(async { Ok(None) })
    }

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>> {
        Box::pin(async {
            let ports = blocking(serialport::available_ports)
                .await
                .map_err(|error| error.to_string())?;
            Ok(ports
                .iter()
                .filter(|port| is_listed(port))
                .map(|port| Rc::new(NativePort::new(port)) as Rc<dyn SerialPort>)
                .collect())
        })
    }

    fn open_record_file(
        &self,
        extension: &str,
        append: bool,
    ) -> LocalFuture<SerialResult<Option<RecordFile>>> {
        let extension = extension.to_string();
        Box::pin(async move { files::open_record(&extension, append).await })
    }
}

/// On macOS, each device is both `/dev/cu.*` and `/dev/tty.*`: the latter
/// waits for a carrier as it opens, so only the former is listed.
fn is_listed(port: &SerialPortInfo) -> bool {
    !(cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty."))
}

struct NativePort {
    info: PortInfo,
    path: String,
    /// While open.
    opened: Rc<RefCell<Option<Opened>>>,
}

#[derive(Clone)]
struct Opened {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    /// Tells the read thread to end.
    closing: Arc<AtomicBool>,
    /// The read thread is on, its handle open.
    reading: Arc<AtomicBool>,
}

impl NativePort {
    fn new(port: &SerialPortInfo) -> Self {
        Self {
            info: port_info(port),
            path: port.port_name.clone(),
            opened: Rc::new(RefCell::new(None)),
        }
    }

    /// Runs `task` on the open port, on a blocking thread.
    fn with_port(
        &self,
        task: impl FnOnce(&mut dyn serialport::SerialPort) -> io::Result<()> + Send + 'static,
    ) -> LocalFuture<SerialResult<()>> {
        let opened = self.opened.borrow().clone();
        Box::pin(async move {
            let opened = opened.ok_or("The port is not open")?;
            blocking(move || {
                let mut port = opened
                    .port
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                task(port.as_mut())
            })
            .await
            .map_err(|error| error.to_string())
        })
    }
}

impl SerialPort for NativePort {
    fn info(&self) -> PortInfo {
        self.info.clone()
    }

    fn open(&self, baudrate: Baudrate, extra: ExtraSettings) -> LocalFuture<SerialResult<()>> {
        let path = self.path.clone();
        let opened = self.opened.clone();
        Box::pin(async move {
            let port = blocking(move || {
                let mut port = serialport::new(&path, baudrate.bps())
                    .data_bits(match extra.data_bits {
                        7 => serialport::DataBits::Seven,
                        _ => serialport::DataBits::Eight,
                    })
                    .parity(match extra.parity {
                        Parity::None => serialport::Parity::None,
                        Parity::Even => serialport::Parity::Even,
                        Parity::Odd => serialport::Parity::Odd,
                    })
                    .stop_bits(match extra.stop_bits {
                        2 => serialport::StopBits::Two,
                        _ => serialport::StopBits::One,
                    })
                    .flow_control(match extra.flow_control {
                        FlowControl::None => serialport::FlowControl::None,
                        FlowControl::Hardware => serialport::FlowControl::Hardware,
                    })
                    .timeout(READ_TIMEOUT)
                    .open()?;
                // On, as Web Serial opens a port (the app takes them so); not
                // RTS under hardware flow control, which it is for
                let _ = port.write_data_terminal_ready(true);
                if extra.flow_control == FlowControl::None {
                    let _ = port.write_request_to_send(true);
                }
                Ok(port)
            })
            .await
            .map_err(open_error)?;
            *opened.borrow_mut() = Some(Opened {
                port: Arc::new(Mutex::new(port)),
                closing: Arc::new(AtomicBool::new(false)),
                reading: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
    }

    fn read(&self, mut on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>> {
        let opened = self.opened.borrow().clone();
        Box::pin(async move {
            let opened = opened.ok_or("The port is not open")?;
            // A handle of its own, so writes need not wait for a read
            let reader = (opened
                .port
                .lock()
                .unwrap_or_else(|error| error.into_inner()))
            .try_clone()
            .map_err(|error| error.to_string())?;
            let (sender, mut receiver) = mpsc::unbounded_channel();
            opened.reading.store(true, Ordering::Relaxed);
            thread::spawn(move || {
                read_loop(reader, &opened.closing, &sender);
                opened.reading.store(false, Ordering::Relaxed);
            });
            while let Some(read) = receiver.recv().await {
                match read {
                    Ok(bytes) => on_chunk(bytes),
                    Err(error) => return Err(error),
                }
            }
            // Ended as the port closes
            Ok(())
        })
    }

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>> {
        self.with_port(move |port| {
            port.write_all(&bytes)?;
            port.flush()
        })
    }

    fn set_signals(&self, dtr: Option<bool>, rts: Option<bool>) -> LocalFuture<SerialResult<()>> {
        self.with_port(move |port| {
            if let Some(dtr) = dtr {
                port.write_data_terminal_ready(dtr)?;
            }
            if let Some(rts) = rts {
                port.write_request_to_send(rts)?;
            }
            Ok(())
        })
    }

    fn set_break(&self, on: bool) -> LocalFuture<SerialResult<()>> {
        self.with_port(move |port| {
            if on {
                port.set_break()?;
            } else {
                port.clear_break()?;
            }
            Ok(())
        })
    }

    fn close(&self) -> LocalFuture<SerialResult<()>> {
        let opened = self.opened.borrow_mut().take();
        Box::pin(async move {
            let Some(opened) = opened else {
                return Ok(());
            };
            opened.closing.store(true, Ordering::Relaxed);
            // Closed once the read thread lets its handle go too: till then, it
            // may not open again (as it is opened only once at a time)
            while opened.reading.load(Ordering::Relaxed) {
                tokio::time::sleep(READ_TIMEOUT / 5).await;
            }
            Ok(())
        })
    }
}

/// Sends each chunk read, until the port is closed (and ends) or fails (and
/// sends why).
fn read_loop(
    mut reader: Box<dyn serialport::SerialPort>,
    closing: &AtomicBool,
    sender: &mpsc::UnboundedSender<SerialResult<Vec<u8>>>,
) {
    let mut buffer = vec![0; READ_BUFFER_BYTES];
    while !closing.load(Ordering::Relaxed) {
        let read = match reader.read(&mut buffer) {
            Ok(0) => continue,
            Ok(read) => Ok(buffer[..read].to_vec()),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(error) => Err(error.to_string()),
        };
        let failed = read.is_err();
        // Or no one reads them any more
        if sender.send(read).is_err() || failed {
            return;
        }
    }
}

/// Why it did not open: on Linux, how to be let to, if not.
fn open_error(error: serialport::Error) -> String {
    let denied = error.kind() == serialport::ErrorKind::Io(io::ErrorKind::PermissionDenied);
    if denied && cfg!(target_os = "linux") {
        format!(
            "{error}: join the port's group (e.g. `sudo usermod -aG dialout $USER`), then log in again"
        )
    } else {
        error.to_string()
    }
}

fn port_info(port: &SerialPortInfo) -> PortInfo {
    let usb = match &port.port_type {
        SerialPortType::UsbPort(usb) => Some(usb),
        _ => None,
    };
    // As Web Serial's are named (of the USB ID database), else as the device
    // tells
    let vendor = usb.and_then(|usb| {
        <usb_ids::Vendor as usb_ids::FromId<u16>>::from_id(usb.vid)
            .map(|vendor| vendor.name().to_string())
            .or_else(|| usb.manufacturer.clone())
    });
    let product = usb.and_then(|usb| {
        usb_ids::Device::from_vid_pid(usb.vid, usb.pid)
            .map(|device| device.name().to_string())
            .or_else(|| usb.product.clone())
    });
    let path = port.port_name.clone();
    // Its device's name, and which it is if more are alike
    let name = match &product {
        Some(product) => {
            let file = Path::new(&path)
                .file_name()
                .map(|file| file.to_string_lossy());
            format!("{product} · {}", file.as_deref().unwrap_or(&path))
        }
        None => path.clone(),
    };
    PortInfo {
        name,
        vendor,
        product,
        path: Some(path),
    }
}

pub(crate) async fn blocking<T: Send + 'static>(task: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(task)
        .await
        .expect("a blocking task is not cancelled")
}

#[cfg(all(test, unix))]
mod tests {
    use std::{cell::RefCell, io::Write, rc::Rc, time::Duration};

    use serialport::{SerialPort as _, SerialPortInfo, SerialPortType};
    use ui::serial::{Baudrate, ExtraSettings, SerialPort};

    use super::NativePort;

    /// Of a pseudo terminal: its other side, as the device, and the port.
    fn device_and_port() -> (serialport::TTYPort, NativePort) {
        let (device, port) = serialport::TTYPort::pair().expect("a pseudo terminal");
        let path = port.name().expect("its path");
        // Let go, for the app to open
        drop(port);
        let info = SerialPortInfo {
            port_name: path,
            port_type: SerialPortType::Unknown,
        };
        (device, NativePort::new(&info))
    }

    #[tokio::test]
    async fn sends_receives_and_opens_again_once_closed() {
        let (mut device, port) = device_and_port();
        let baudrate = Baudrate::new(115200).unwrap();
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async move {
                port.open(baudrate, ExtraSettings::default()).await.unwrap();
                let received = Rc::new(RefCell::new(Vec::new()));
                let reading = tokio::task::spawn_local(port.read(Box::new({
                    let received = received.clone();
                    move |chunk| received.borrow_mut().extend(chunk)
                })));

                device.write_all(b"hello").unwrap();
                for _ in 0..100 {
                    if received.borrow().len() >= 5 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                assert_eq!(*received.borrow(), b"hello");

                port.write(b"led on".to_vec()).await.unwrap();
                let mut sent = [0; 6];
                device.set_timeout(Duration::from_secs(1)).unwrap();
                std::io::Read::read_exact(&mut device, &mut sent).unwrap();
                assert_eq!(&sent, b"led on");

                port.close().await.unwrap();
                assert_eq!(reading.await.unwrap(), Ok(()));
                port.open(baudrate, ExtraSettings::default()).await.unwrap();
                port.close().await.unwrap();
            })
            .await;
    }

    #[tokio::test]
    async fn is_named_by_its_path_without_a_product() {
        let (_device, port) = device_and_port();
        let info = port.info();
        assert_eq!(Some(&info.name), info.path.as_ref());
        assert_eq!(info.product, None);
    }
}
