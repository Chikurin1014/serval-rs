mod data_context;
mod data_type;
mod map;
mod pattern;
mod queue;
mod source_cursor;

pub use data_context::{DataContext, DataProvider, RAW_BYTES_LABEL};
pub use data_type::{
    ByteData, Data, DataEntry, DataType, NumberData, NumberText, StringData, TypedData,
    format_number,
};
pub use map::{
    Arithmetic, ArithmeticSettings, Calculus, CalculusMap, CalculusSettings, Concat,
    ConcatSettings, Conversion, ConversionInput, Decode, DecodeSettings, Delimiter, Encode,
    EncodeSettings, InitialMap, Map, MapContext, MapKind, MapPreset, MapProvider, MapRunner,
    Operation, RegexMatch, RegexOutput, RegexSettings, Replace, ReplaceSettings, Segment,
};
pub(crate) use map::{Endpoints, Input, endpoints, keep_taken, take_newest_pair, trim_segments};
pub(crate) use pattern::compile_for_map;
pub use pattern::{PATTERN_ALIASES, compile_pattern, expand_aliases};
pub use queue::{MAX_ENTRIES_PER_LABEL, Queue};
pub use source_cursor::{NewEntries, SourceCursor};
