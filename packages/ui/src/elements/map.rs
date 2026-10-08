mod arithmetic;
mod calculus;
mod concat;
mod decode;
mod encode;
mod field;
mod map_list;
mod regex;
mod replace;

use std::any::Any;

use dioxus::prelude::*;

use crate::data::{Decode, InitialMap, MapKind, RAW_BYTES_LABEL, RegexMatch, RegexOutput};

pub use arithmetic::{ADD, ArithmeticForm, DIVIDE, MULTIPLY, SUBTRACT};
pub use calculus::{CalculusForm, DIFFERENTIATE, INTEGRATE};
pub use concat::{CONCAT, ConcatForm};
pub use decode::{DECODE, DecodeForm};
pub use encode::{ENCODE, EncodeForm};
pub use map_list::MapList;
pub use regex::{
    ANONYMOUS_LABEL, NAME_COLON_NUMBER, NUMBER_ONLY, REGEX_TO_NUMBER, REGEX_TO_STRING,
};
pub use replace::{REPLACE, ReplaceForm};

/// A map's settings form for `MapKind::form`: `form` given `settings` as the
/// kind's own type `S`, or nothing if they are of another kind.
fn form_of<S: Copy + 'static>(settings: &dyn Any, form: impl FnOnce(S) -> Element) -> Element {
    settings
        .downcast_ref::<S>()
        .map_or_else(VNode::empty, |&settings| form(settings))
}

/// The label the initial maps decode the raw bytes into, as text.
pub const MESSAGE_LABEL: &str = "message";

/// The built-in kinds, for `MapProvider`'s `kinds`.
pub fn builtin_map_kinds() -> Vec<MapKind> {
    vec![
        DECODE,
        ENCODE,
        REGEX_TO_NUMBER,
        REGEX_TO_STRING,
        REPLACE,
        CONCAT,
        ADD,
        SUBTRACT,
        MULTIPLY,
        DIVIDE,
        DIFFERENTIATE,
        INTEGRATE,
    ]
}

/// For `MapProvider`'s `initial`: decodes the raw serial bytes into lines,
/// so there is text to work with from the start, and reads the numbers with
/// no label and the `name: value` ones from them (as the `Regex` presets).
pub fn initial_maps() -> Vec<InitialMap> {
    vec![
        InitialMap {
            kind: DECODE,
            enabled: true,
            create: || Box::new(Decode::new(RAW_BYTES_LABEL, MESSAGE_LABEL)),
        },
        InitialMap {
            kind: REGEX_TO_NUMBER,
            enabled: true,
            create: || {
                Box::new(RegexMatch::with(
                    RegexOutput::Number,
                    MESSAGE_LABEL,
                    NUMBER_ONLY,
                    ANONYMOUS_LABEL,
                    "$0",
                ))
            },
        },
        InitialMap {
            kind: REGEX_TO_NUMBER,
            enabled: true,
            create: || {
                Box::new(RegexMatch::with(
                    RegexOutput::Number,
                    MESSAGE_LABEL,
                    NAME_COLON_NUMBER,
                    "$1",
                    "$2",
                ))
            },
        },
    ]
}
