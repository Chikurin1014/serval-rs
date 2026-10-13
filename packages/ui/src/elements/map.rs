mod arithmetic;
mod calculus;
mod compose;
mod concat;
mod decode;
mod decompose;
mod encode;
mod field;
mod magnitude;
mod map_list;
mod matching;
mod replace;

use std::any::Any;

use dioxus::prelude::*;

use crate::data::{Decode, InitialMap, MapKind, Match, MatchOutput, RAW_BYTES_LABEL};

pub use arithmetic::{
    ADD, ADD_2D, ADD_3D, ArithmeticForm, CROSS_2D, CROSS_3D, DIVIDE, DOT_2D, DOT_3D, MULTIPLY,
    MULTIPLY_2D, MULTIPLY_3D, SUBTRACT, SUBTRACT_2D, SUBTRACT_3D, VectorArithmeticForm,
};
pub use calculus::{
    CalculusForm, DIFFERENTIATE, DIFFERENTIATE_2D, DIFFERENTIATE_3D, INTEGRATE, INTEGRATE_2D,
    INTEGRATE_3D,
};
pub use compose::{COMPOSE_2D, COMPOSE_3D, ComposeForm};
pub use concat::{CONCAT, ConcatForm};
pub use decode::{DECODE, DecodeForm};
pub use decompose::{DECOMPOSE_2D, DECOMPOSE_3D, DecomposeForm};
pub use encode::{ENCODE, EncodeForm};
pub use magnitude::{MAGNITUDE_2D, MAGNITUDE_3D, MagnitudeForm};
pub use map_list::MapList;
pub use matching::{
    ANONYMOUS_LABEL, MATCH_TO_NUMBER, MATCH_TO_STRING, MATCH_TO_VECTOR2D, MATCH_TO_VECTOR3D,
    NAME_COLON_NUMBER, NUMBER_ONLY,
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
        MATCH_TO_VECTOR2D,
        MATCH_TO_VECTOR3D,
        REPLACE,
        CONCAT,
        COMPOSE_2D,
        COMPOSE_3D,
        ADD,
        SUBTRACT,
        MULTIPLY,
        DIVIDE,
        ADD_2D,
        ADD_3D,
        SUBTRACT_2D,
        SUBTRACT_3D,
        MULTIPLY_2D,
        MULTIPLY_3D,
        CROSS_3D,
        DIFFERENTIATE_2D,
        DIFFERENTIATE_3D,
        INTEGRATE_2D,
        INTEGRATE_3D,
        DOT_2D,
        DOT_3D,
        CROSS_2D,
        MAGNITUDE_2D,
        MAGNITUDE_3D,
        DECOMPOSE_2D,
        DECOMPOSE_3D,
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
