mod arithmetic;
mod calculus;
mod concat;
mod decode;
mod encode;
mod field;
mod map_list;
mod matching;
mod replace;

use std::any::Any;

use dioxus::prelude::*;

use crate::data::{Decode, InitialMap, MapKind, Match, MatchOutput, RAW_BYTES_LABEL};

pub use arithmetic::{ADD, ArithmeticForm, DIVIDE, MULTIPLY, SUBTRACT};
pub use calculus::{CalculusForm, DIFFERENTIATE, INTEGRATE};
pub use concat::{CONCAT, ConcatForm};
pub use decode::{DECODE, DecodeForm};
pub use encode::{ENCODE, EncodeForm};
pub use map_list::MapList;
pub use matching::{
    ANONYMOUS_LABEL, MATCH_TO_NUMBER, MATCH_TO_STRING, NAME_COLON_NUMBER, NUMBER_ONLY,
};
pub use replace::{REPLACE, ReplaceForm};

/// For `MapKind::form`: `form` given `settings` downcast to `S`.
fn form_of<S: Copy + 'static>(settings: &dyn Any, form: impl FnOnce(S) -> Element) -> Element {
    settings
        .downcast_ref::<S>()
        .map_or_else(VNode::empty, |&settings| form(settings))
}

pub const MESSAGE_LABEL: &str = "message";

pub fn builtin_map_kinds() -> Vec<MapKind> {
    vec![
        DECODE,
        ENCODE,
        MATCH_TO_NUMBER,
        MATCH_TO_STRING,
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

/// Decodes the raw bytes into lines, and reads numbers from them.
pub fn initial_maps() -> Vec<InitialMap> {
    vec![
        InitialMap {
            kind: DECODE,
            enabled: true,
            create: || Box::new(Decode::new(RAW_BYTES_LABEL, MESSAGE_LABEL)),
        },
        InitialMap {
            kind: MATCH_TO_NUMBER,
            enabled: true,
            create: || {
                Box::new(Match::with(
                    MatchOutput::Number,
                    MESSAGE_LABEL,
                    NUMBER_ONLY,
                    ANONYMOUS_LABEL,
                    "$0",
                ))
            },
        },
        InitialMap {
            kind: MATCH_TO_NUMBER,
            enabled: true,
            create: || {
                Box::new(Match::with(
                    MatchOutput::Number,
                    MESSAGE_LABEL,
                    NAME_COLON_NUMBER,
                    "$1",
                    "$2",
                ))
            },
        },
    ]
}
