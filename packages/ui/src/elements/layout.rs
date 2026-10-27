//! The layout: the maps and graphs, of their kinds and settings, kept in a JSON
//! file to be taken in again in place of those there are.

use dioxus::prelude::*;
use dioxus_icons::lucide;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    components::dropdown_menu::{
        DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
    },
    data::{DataType, InitialMap, MapContext, MapKind},
    elements::graph::{GraphContext, GraphKind, GraphProperty},
    helper::{clear_file, download_text, pick_file},
    toast::use_toaster,
};

const LAYOUT_MENU_CSS: Asset = asset!("/assets/styling/layout-menu.css");
const INITIAL_LAYOUT: &str = include_str!("initial_layout.json");

/// The version of Serval a layout file is of.
pub const LAYOUT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LAYOUT_FILE_NAME: &str = "serval-layout.json";
const FILE_INPUT_ID: &str = "layout-file-input";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct LayoutFile {
    /// Of this version, if missing (as in `initial_layout.json`, so a release
    /// need not write it, as it would in a format of its own).
    #[serde(default = "this_version")]
    version: String,
    maps: Vec<SavedMap>,
    graphs: Vec<SavedGraph>,
}

/// A map, its kind by its name and types.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SavedMap {
    kind: String,
    from: Vec<DataType>,
    to: DataType,
    enabled: bool,
    /// Each setting missing as the kind makes it.
    #[serde(default = "no_settings")]
    settings: Value,
}

fn this_version() -> String {
    LAYOUT_VERSION.to_string()
}

fn no_settings() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SavedGraph {
    kind: String,
    #[serde(default)]
    settings: GraphProperty,
}

/// The layout as a layout file has it.
pub fn export_layout(maps: MapContext, graphs: GraphContext) -> String {
    let file = LayoutFile {
        version: LAYOUT_VERSION.to_string(),
        maps: maps
            .list()
            .into_iter()
            .map(|map| SavedMap {
                kind: map.kind.name.to_string(),
                from: map.kind.from.to_vec(),
                to: map.kind.to,
                enabled: *map.enabled.peek(),
                settings: maps.saved_settings(map.id).unwrap_or_else(no_settings),
            })
            .collect(),
        graphs: graphs
            .list()
            .into_iter()
            .map(|graph| SavedGraph {
                kind: graph.kind.name.to_string(),
                settings: graph.property,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).expect("a layout is JSON")
}

/// A layout of known kinds, as a layout file has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Their settings are checked only as they are made.
    pub maps: Vec<InitialMap>,
    pub graphs: Vec<(GraphKind, GraphProperty)>,
}

/// The layout of the file `text`, of `map_kinds` and `graph_kinds`; else what is
/// wrong with it.
pub fn parse_layout(
    text: &str,
    map_kinds: &[MapKind],
    graph_kinds: &[GraphKind],
) -> Result<Layout, String> {
    let file: LayoutFile =
        serde_json::from_str(text).map_err(|error| format!("Not a layout file: {error}"))?;
    let maps = file
        .maps
        .into_iter()
        .map(|map| {
            let kind = map_kinds
                .iter()
                .find(|kind| kind.name == map.kind && kind.from == map.from && kind.to == map.to)
                .ok_or_else(|| {
                    let from = map.from.iter().map(DataType::name).collect::<Vec<_>>();
                    format!(
                        "No map {} from {} to {}",
                        map.kind,
                        from.join(", "),
                        map.to.name()
                    )
                })?;
            Ok(InitialMap {
                kind: *kind,
                enabled: map.enabled,
                settings: map.settings,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let graphs = file
        .graphs
        .into_iter()
        .map(|graph| {
            let kind = graph_kinds
                .iter()
                .find(|kind| kind.name == graph.kind)
                .ok_or_else(|| format!("No graph {}", graph.kind))?;
            Ok((*kind, graph.settings))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Layout { maps, graphs })
}

/// The layout the app starts with, of `initial_layout.json`.
pub fn initial_layout(map_kinds: &[MapKind], graph_kinds: &[GraphKind]) -> Layout {
    parse_layout(INITIAL_LAYOUT, map_kinds, graph_kinds)
        .unwrap_or_else(|error| panic!("initial_layout.json is wrong: {error}"))
}

/// Puts the layout of the file `text` in place of all there is; else what is
/// wrong with it, with nothing changed.
pub fn import_layout(
    maps: &mut MapContext,
    graphs: &mut GraphContext,
    text: &str,
) -> Result<(), String> {
    let layout = parse_layout(text, &maps.kinds(), &graphs.kinds())?;
    // The graphs after the maps, as only those may fail now
    maps.replace_all(layout.maps)?;
    graphs.replace_all(layout.graphs);
    Ok(())
}

/// Exports the layout to a file, or imports one in place of it.
#[component]
pub fn LayoutMenu() -> Element {
    let mut maps = use_context::<MapContext>();
    let mut graphs = use_context::<GraphContext>();
    let toaster = use_toaster();
    let mut open = use_signal(|| false);

    rsx! {
        document::Link { rel: "stylesheet", href: LAYOUT_MENU_CSS }

        DropdownMenu {
            class: "layout-menu",
            open: Some(open()),
            on_open_change: move |value| open.set(value),
            DropdownMenuTrigger {
                class: "layout-menu-trigger",
                aria_label: "Layout",
                "Layout"
                if open() {
                    lucide::ChevronUp {}
                } else {
                    lucide::ChevronDown {}
                }
            }
            DropdownMenuContent {
                class: "layout-menu-content",
                DropdownMenuItem {
                    value: "import",
                    index: 0usize,
                    on_select: move |_: &str| {
                        open.set(false);
                        pick_file(FILE_INPUT_ID);
                    },
                    lucide::FileUp {}
                    "Import from JSON"
                }
                DropdownMenuItem {
                    value: "export",
                    index: 1usize,
                    on_select: move |_: &str| {
                        open.set(false);
                        download_text(
                            LAYOUT_FILE_NAME,
                            "application/json",
                            export_layout(maps, graphs),
                        );
                    },
                    lucide::FileDown {}
                    "Export to JSON"
                }
            }
        }
        input {
            id: FILE_INPUT_ID,
            class: "layout-file-input",
            r#type: "file",
            accept: ".json,application/json",
            onchange: move |event| async move {
                let Some(file) = event.files().into_iter().next() else {
                    return;
                };
                let imported = match file.read_string().await {
                    Ok(text) => import_layout(&mut maps, &mut graphs, &text),
                    Err(error) => Err(error.to_string()),
                };
                // So the same file picked again is a change
                clear_file(FILE_INPUT_ID);
                match imported {
                    Ok(()) => toaster.success("Layout imported", &file.name()),
                    Err(error) => toaster.error("Failed to import the layout", &error),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use dioxus::{
        core::{Runtime, ScopeId},
        prelude::*,
    };
    use serde_json::{Value, json};

    use super::{INITIAL_LAYOUT, LAYOUT_VERSION, export_layout, import_layout, initial_layout};
    use crate::{
        TimeContext,
        data::{DataProvider, MapContext, MapProvider},
        elements::{
            graph::{GraphContext, GraphProvider, builtin_graph_kinds},
            map::builtin_map_kinds,
        },
    };

    thread_local! {
        static CONTEXTS: RefCell<Option<(MapContext, GraphContext)>> = const { RefCell::new(None) };
    }

    #[component]
    fn Grab() -> Element {
        let maps = use_context::<MapContext>();
        let graphs = use_context::<GraphContext>();
        CONTEXTS.with(|cell| *cell.borrow_mut() = Some((maps, graphs)));
        rsx! {}
    }

    #[component]
    fn Root() -> Element {
        use_context_provider(|| TimeContext::new(|| 0));
        let layout = initial_layout(&builtin_map_kinds(), &builtin_graph_kinds());
        rsx! {
            DataProvider {
                MapProvider {
                    kinds: builtin_map_kinds(),
                    initial: layout.maps,
                    GraphProvider {
                        kinds: builtin_graph_kinds(),
                        initial: layout.graphs,
                        Grab {}
                    }
                }
            }
        }
    }

    /// The app's contexts as it starts, to run `f` on.
    fn with_layout(f: impl FnOnce(MapContext, GraphContext)) {
        let mut dom = VirtualDom::new(Root);
        dom.rebuild_in_place();
        let (maps, graphs) = CONTEXTS.with(|cell| cell.borrow().expect("grabbed"));
        dom.in_runtime(|| Runtime::current().in_scope(ScopeId::ROOT, || f(maps, graphs)));
    }

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).expect("JSON")
    }

    #[test]
    fn the_app_starts_with_the_initial_layout() {
        with_layout(|maps, graphs| {
            let initial = parsed(INITIAL_LAYOUT);
            // Of the version it is loaded with: none to go out of date
            assert!(initial.get("version").is_none());
            let layout = parsed(&export_layout(maps, graphs));
            assert_eq!(layout["maps"], initial["maps"]);
            assert_eq!(
                graphs
                    .list()
                    .iter()
                    .map(|graph| graph.kind.name)
                    .collect::<Vec<_>>(),
                ["Time series"]
            );
        });
    }

    #[test]
    fn a_layout_is_of_this_version_with_the_maps_and_graphs() {
        with_layout(|maps, graphs| {
            let layout = parsed(&export_layout(maps, graphs));
            assert_eq!(layout["version"], LAYOUT_VERSION);
            assert_eq!(layout["maps"].as_array().unwrap().len(), maps.list().len());
            let decode = &layout["maps"][0];
            assert_eq!(decode["kind"], "Decode");
            assert_eq!(decode["from"], json!(["Bytes"]));
            assert_eq!(decode["to"], "Log");
            assert_eq!(decode["enabled"], true);
            assert_eq!(
                decode["settings"]["delimiters"],
                json!(["CR", "LF", "CRLF"])
            );
            assert!(decode["settings"].get("pattern_error").is_none());
            let graph = &layout["graphs"][0];
            assert_eq!(graph["kind"], "Time series");
            assert_eq!(graph["settings"]["time_range"], 10);
            assert_eq!(graph["settings"]["weakest_level"], "NONE");
        });
    }

    #[test]
    fn an_imported_layout_is_as_exported() {
        with_layout(|mut maps, mut graphs| {
            let mut layout = parsed(&export_layout(maps, graphs));
            layout["maps"][0]["enabled"] = json!(false);
            layout["maps"][0]["settings"]["pattern"] = json!("^(.*)$");
            layout["maps"].as_array_mut().unwrap().truncate(1);
            layout["graphs"] = json!([
                { "kind": "Log", "settings": { "weakest_level": "WARN", "hidden": ["log"] } },
                { "kind": "Scatter", "settings": { "x_label": "x", "time_range": 64 } },
            ]);
            import_layout(&mut maps, &mut graphs, &layout.to_string()).unwrap();

            let imported = parsed(&export_layout(maps, graphs));
            assert_eq!(imported["maps"], layout["maps"]);
            let [log, scatter] = &graphs.list()[..] else {
                panic!("two graphs");
            };
            assert_eq!(log.kind.name, "Log");
            assert_eq!(log.property.weakest_level, crate::data::LogLevel::Warn);
            assert_eq!(log.property.hidden, ["log"]);
            assert_eq!(scatter.property.x_label, "x");
            // Clamped to its steps
            assert_eq!(scatter.property.time_range.seconds(), Some(60));
        });
    }

    #[test]
    fn maps_of_each_kind_are_imported_as_exported() {
        with_layout(|mut maps, mut graphs| {
            for kind in builtin_map_kinds() {
                maps.add(kind);
            }
            let mut layout = parsed(&export_layout(maps, graphs));
            for map in layout["maps"].as_array_mut().unwrap() {
                for value in map["settings"].as_object_mut().unwrap().values_mut() {
                    if value == "" {
                        *value = json!("imported");
                    }
                }
            }
            import_layout(&mut maps, &mut graphs, &layout.to_string()).unwrap();
            assert_eq!(parsed(&export_layout(maps, graphs)), layout);
        });
    }

    #[test]
    fn a_layout_of_no_version_is_of_this_one() {
        with_layout(|mut maps, mut graphs| {
            let mut layout = parsed(&export_layout(maps, graphs));
            layout.as_object_mut().unwrap().remove("version");
            layout["graphs"] = json!([{ "kind": "Scatter" }]);
            import_layout(&mut maps, &mut graphs, &layout.to_string()).unwrap();
            assert_eq!(graphs.list()[0].kind.name, "Scatter");
        });
    }

    #[test]
    fn a_wrong_layout_changes_nothing() {
        with_layout(|mut maps, mut graphs| {
            let before = export_layout(maps, graphs);
            let mut wrong = |layout: Value| {
                let error = import_layout(&mut maps, &mut graphs, &layout.to_string());
                assert_eq!(export_layout(maps, graphs), before);
                error.unwrap_err()
            };
            let layout = parsed(&before);

            let mut unknown_map = layout.clone();
            unknown_map["maps"][1]["kind"] = json!("Unknown");
            assert!(wrong(unknown_map).starts_with("No map Unknown"));

            let mut unknown_graph = layout.clone();
            unknown_graph["graphs"][0]["kind"] = json!("Unknown");
            assert_eq!(wrong(unknown_graph), "No graph Unknown");

            let mut wrong_setting = layout.clone();
            wrong_setting["maps"][0]["settings"]["pattern"] = json!(1);
            assert!(wrong(wrong_setting).starts_with("Decode: 'pattern'"));

            let mut wrong_level = layout;
            wrong_level["graphs"][0]["settings"]["weakest_level"] = json!("LOUD");
            assert!(wrong(wrong_level).starts_with("Not a layout file"));

            assert!(wrong(json!({ "maps": [] })).starts_with("Not a layout file"));
        });
    }
}
