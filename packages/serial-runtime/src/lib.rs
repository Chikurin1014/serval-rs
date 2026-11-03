//! What the app asks of a serial runtime: the ports of each platform, and what
//! they open with.

use std::{fmt, future::Future, pin::Pin, rc::Rc};

pub mod backend;
mod choice;
#[cfg(feature = "native")]
pub mod native;

pub use choice::Choice;

/// Not `Send`, as platform serial APIs are not.
pub type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub type SerialResult<T> = Result<T, String>;

pub trait SerialBackend {
    /// Whether a port must be requested (e.g. Web Serial's, each allowed by
    /// the user) to be known; else all are, and the list follows the ports
    /// plugged in and out.
    fn needs_request(&self) -> bool {
        false
    }

    fn request_port(&self) -> LocalFuture<SerialResult<Option<Rc<dyn SerialPort>>>>;

    fn known_ports(&self) -> LocalFuture<SerialResult<Vec<Rc<dyn SerialPort>>>>;
}

pub trait SerialPort {
    fn info(&self) -> PortInfo;

    fn open(&self, baudrate: Baudrate, extra: ExtraSettings) -> LocalFuture<SerialResult<()>>;

    /// Passes each chunk to `on_chunk` until the port is closed or lost.
    fn read(&self, on_chunk: Box<dyn FnMut(Vec<u8>)>) -> LocalFuture<SerialResult<()>>;

    fn write(&self, bytes: Vec<u8>) -> LocalFuture<SerialResult<()>>;

    /// Sets DTR and RTS, each if given.
    fn set_signals(&self, dtr: Option<bool>, rts: Option<bool>) -> LocalFuture<SerialResult<()>>;

    /// Holds the line low (a break) while on.
    fn set_break(&self, on: bool) -> LocalFuture<SerialResult<()>>;

    fn close(&self) -> LocalFuture<SerialResult<()>>;
}

/// A baud rate: above 0 bps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Baudrate(u32);

impl Baudrate {
    pub fn new(bps: u32) -> Option<Self> {
        (bps > 0).then_some(Self(bps))
    }

    pub fn bps(self) -> u32 {
        self.0
    }
}

/// e.g. "9600 bps"
impl fmt::Display for Baudrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bps", self.0)
    }
}

/// How a port frames its bytes, but the baud rate: 8N1 with no flow control
/// at first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraSettings {
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
    pub flow_control: FlowControl,
}

impl ExtraSettings {
    /// Those the platforms all take.
    pub const DATA_BITS: [u8; 2] = [7, 8];
    pub const STOP_BITS: [u8; 2] = [1, 2];

    /// e.g. "8N1"
    pub fn frame(&self) -> String {
        format!(
            "{}{}{}",
            self.data_bits,
            self.parity.letter(),
            self.stop_bits
        )
    }
}

impl Default for ExtraSettings {
    fn default() -> Self {
        Self {
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
            flow_control: FlowControl::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl Choice for Parity {
    const KIND: &'static str = "parity";
    const ALL: &'static [Self] = &[Self::None, Self::Even, Self::Odd];

    /// As Web Serial names it.
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Even => "even",
            Self::Odd => "odd",
        }
    }
}

impl Parity {
    fn letter(self) -> char {
        match self {
            Self::None => 'N',
            Self::Even => 'E',
            Self::Odd => 'O',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowControl {
    None,
    /// RTS and CTS.
    Hardware,
}

impl Choice for FlowControl {
    const KIND: &'static str = "flow control";
    const ALL: &'static [Self] = &[Self::None, Self::Hardware];

    /// As Web Serial names it.
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Hardware => "hardware",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortInfo {
    pub name: String,
    pub vendor: Option<String>,
    pub product: Option<String>,
    /// Where it is on the system (e.g. `/dev/ttyACM0` or `COM3`), if known.
    pub path: Option<String>,
}

/// A port's USB device, as the OS tells of it.
#[derive(Clone, Debug, PartialEq)]
pub struct UsbDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

impl PortInfo {
    /// Of a port the OS lists at `path`.
    pub fn of_device(path: String, usb: Option<UsbDevice>) -> Self {
        // As Web Serial's are named (of the USB ID database), else as the device
        // tells
        let vendor = usb.as_ref().and_then(|usb| {
            <usb_ids::Vendor as usb_ids::FromId<u16>>::from_id(usb.vendor_id)
                .map(|vendor| vendor.name().to_string())
                .or_else(|| usb.manufacturer.clone())
        });
        let product = usb.as_ref().and_then(|usb| {
            usb_ids::Device::from_vid_pid(usb.vendor_id, usb.product_id)
                .map(|device| device.name().to_string())
                .or_else(|| usb.product.clone())
        });
        // Its device's name, and which it is if more are alike
        let name = match &product {
            Some(product) => {
                let file = std::path::Path::new(&path)
                    .file_name()
                    .map(|file| file.to_string_lossy());
                format!("{product} · {}", file.as_deref().unwrap_or(&path))
            }
            None => path.clone(),
        };
        Self {
            name,
            vendor,
            product,
            path: Some(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Baudrate, ExtraSettings, Parity};

    #[test]
    fn frame_is_8n1_at_first() {
        assert_eq!(ExtraSettings::default().frame(), "8N1");
    }

    #[test]
    fn frame_names_the_parity() {
        let settings = ExtraSettings {
            data_bits: 7,
            parity: Parity::Even,
            stop_bits: 2,
            ..ExtraSettings::default()
        };
        assert_eq!(settings.frame(), "7E2");
    }

    #[test]
    fn a_baud_rate_is_above_0() {
        assert_eq!(Baudrate::new(0), None);
        assert_eq!(Baudrate::new(9600).map(Baudrate::bps), Some(9600));
        assert_eq!(Baudrate::new(9600).unwrap().to_string(), "9600 bps");
    }
}
