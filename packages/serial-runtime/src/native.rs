//! Serial ports of the OS, through `serialport`. Blocking: it is for those who
//! open them to do so on threads of their own.

use std::{
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use serialport::{SerialPort, SerialPortInfo, SerialPortType};

use crate::{Baudrate, ExtraSettings, FlowControl, Parity, PortInfo, UsbDevice};

/// How long a read waits for bytes, before it looks whether the port is closed.
pub const READ_TIMEOUT: Duration = Duration::from_millis(50);
const READ_BUFFER_BYTES: usize = 4096;

/// The ports to list. On macOS, each device is both `/dev/cu.*` and
/// `/dev/tty.*`: the latter waits for a carrier as it opens, so only the former.
pub fn listed_ports() -> serialport::Result<Vec<SerialPortInfo>> {
    let mut ports = serialport::available_ports()?;
    ports.retain(|port| !(cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty.")));
    Ok(ports)
}

pub fn port_info(port: &SerialPortInfo) -> PortInfo {
    let usb = match &port.port_type {
        SerialPortType::UsbPort(usb) => Some(UsbDevice {
            vendor_id: usb.vid,
            product_id: usb.pid,
            manufacturer: usb.manufacturer.clone(),
            product: usb.product.clone(),
        }),
        _ => None,
    };
    PortInfo::of_device(port.port_name.clone(), usb)
}

pub fn open(
    path: &str,
    baudrate: Baudrate,
    extra: ExtraSettings,
) -> Result<Box<dyn SerialPort>, String> {
    let mut port = serialport::new(path, baudrate.bps())
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
        .open()
        .map_err(open_error)?;
    // On, as Web Serial opens a port (the app takes them so); not RTS under
    // hardware flow control, which it is for
    let _ = port.write_data_terminal_ready(true);
    if extra.flow_control == FlowControl::None {
        let _ = port.write_request_to_send(true);
    }
    Ok(port)
}

/// Passes each chunk read to `on_read`, until `closing` is set (and ends), it
/// fails (and passes why), or `on_read` returns false as no one reads any more.
pub fn read_loop(
    mut reader: Box<dyn SerialPort>,
    closing: &AtomicBool,
    mut on_read: impl FnMut(Result<Vec<u8>, String>) -> bool,
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
        if !on_read(read) || failed {
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
