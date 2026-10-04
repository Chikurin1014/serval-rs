mod conversion_list;
mod conversion_regex_match;
mod conversion_split_from_byte;
mod data_list;
mod port_baudrate_configurator;
mod port_io_console;
mod port_open_close_button;
mod port_selector;

pub use conversion_list::ConversionList;
pub use conversion_regex_match::{RegexMatchForm, REGEX_TO_NUMBER, REGEX_TO_STRING};
pub use conversion_split_from_byte::{SplitFromByteForm, SPLIT_FROM_BYTE};
pub use data_list::DataList;
pub use port_baudrate_configurator::PortBaudrateConfigurator;
pub use port_io_console::PortIoConsole;
pub use port_open_close_button::PortOpenCloseButton;
pub use port_selector::PortSelector;

use crate::data::{ConversionKind, InitialConversion, SplitFromByte};

/// The built-in kinds, for `ConversionProvider`'s `kinds`.
pub fn builtin_conversion_kinds() -> Vec<ConversionKind> {
    vec![SPLIT_FROM_BYTE, REGEX_TO_NUMBER, REGEX_TO_STRING]
}

/// For `ConversionProvider`'s `initial`: splits the raw serial bytes into
/// lines, so there is text to work with from the start.
pub fn initial_conversions() -> Vec<InitialConversion> {
    vec![InitialConversion {
        kind: SPLIT_FROM_BYTE,
        enabled: true,
        create: || Box::new(SplitFromByte::new("raw_data", "raw_str")),
    }]
}
