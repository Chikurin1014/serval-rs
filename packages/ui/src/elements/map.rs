mod map_list;
mod regex;
mod split_from_byte;

use crate::data::{InitialMap, MapKind, SplitFromByte};

pub use map_list::MapList;
pub use regex::{REGEX_TO_NUMBER, REGEX_TO_STRING};
pub use split_from_byte::{SPLIT_FROM_BYTE, SplitFromByteForm};

/// The built-in kinds, for `MapProvider`'s `kinds`.
pub fn builtin_map_kinds() -> Vec<MapKind> {
    vec![SPLIT_FROM_BYTE, REGEX_TO_NUMBER, REGEX_TO_STRING]
}

/// For `MapProvider`'s `initial`: splits the raw serial bytes into
/// lines, so there is text to work with from the start.
pub fn initial_maps() -> Vec<InitialMap> {
    vec![InitialMap {
        kind: SPLIT_FROM_BYTE,
        enabled: true,
        create: || Box::new(SplitFromByte::new("raw_data", "raw_str")),
    }]
}
