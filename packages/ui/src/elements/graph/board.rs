use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::{GraphContext, GraphKind, GraphProperty};
use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::DropdownMenuItem,
        sidebar::{
            Sidebar, SidebarCollapsible, SidebarContent, SidebarInset, SidebarProvider,
            SidebarSide, SidebarTrigger,
        },
    },
    elements::{DataList, HoverMenu, HoverMenus},
};

const GRAPH_BOARD_CSS: Asset = asset!("/assets/styling/graph-board.css");

/// The graph being dragged, and the one it is over.
#[derive(Clone, Copy)]
pub(super) struct GraphDrag {
    pub dragging: Signal<Option<usize>>,
    pub target: Signal<Option<usize>>,
}

/// The graphs, with the data list in a sidebar.
#[component]
pub fn GraphBoard() -> Element {
    let context = use_context::<GraphContext>();
    let graphs = use_memo(move || context.list());
    let kinds = context.kinds();
    let columns = columns(graphs.read().len());
    use_context_provider(|| GraphDrag {
        dragging: Signal::new(None),
        target: Signal::new(None),
    });

    rsx! {
        document::Link { rel: "stylesheet", href: GRAPH_BOARD_CSS }

        SidebarProvider {
            class: "graph-board",
            default_open: false,
            Sidebar {
                side: SidebarSide::Left,
                collapsible: SidebarCollapsible::Offcanvas,
                SidebarContent {
                    DataList {}
                }
            }
            SidebarInset {
                class: "graph-main",
                div {
                    class: "graph-bar",
                    SidebarTrigger {
                        aria_label: "Toggle data list",
                        title: "Toggle data list",
                    }
                    AddGraphBar { kinds }
                    PauseButton {}
                }
                div {
                    class: "graph-grid",
                    "data-columns": columns,
                    {graphs().into_iter().map(|graph| graph.view())}
                    if graphs.read().is_empty() {
                        p {
                            class: "graph-grid-empty",
                            "No graphs. Add one from the menu above."
                        }
                    }
                }
            }
        }
    }
}

fn columns(graphs: usize) -> usize {
    match graphs {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::{columns, menu_items};
    use crate::elements::graph::{ARROWS, ARROWS_3D, BOX_PLOT, SCATTER, TIME_SERIES};

    #[test]
    fn columns_follow_the_number_of_graphs() {
        let by_count: Vec<_> = (0..=7).map(columns).collect();
        assert_eq!(by_count, [1, 1, 2, 2, 2, 3, 3, 3]);
    }

    #[test]
    fn menus_take_the_presets_or_the_kinds_with_none() {
        let menus = menu_items(&[TIME_SERIES, BOX_PLOT, SCATTER, ARROWS, ARROWS_3D]);
        let names: Vec<_> = menus
            .iter()
            .map(|(menu, items)| {
                (
                    *menu,
                    items.iter().map(|item| item.name).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            names,
            [
                ("Time series", vec!["Points", "Linear", "Stepped"]),
                ("Statistics", vec!["Box plot", "Scatter"]),
                ("Vector", vec!["Arrows (2D)", "Arrows (3D)"]),
            ]
        );
    }
}

#[component]
fn AddGraphBar(kinds: Vec<GraphKind>) -> Element {
    let mut context = use_context::<GraphContext>();
    let menus = use_hook(HoverMenus::new);

    rsx! {
        div {
            class: "add-graph-bar",
            for (menu, (name, items)) in menu_items(&kinds).into_iter().enumerate() {
                HoverMenu {
                    menus,
                    menu,
                    trigger: rsx! { "{name}" },
                    for (index, item) in items.iter().enumerate() {
                        DropdownMenuItem {
                            value: index,
                            index,
                            on_select: {
                                let items = items.clone();
                                move |index: usize| {
                                    let item: &MenuItem = &items[index];
                                    context.add(item.kind, (item.property)());
                                    menus.close(menu);
                                }
                            },
                            "{item.name}"
                        }
                    }
                }
            }
        }
    }
}

/// What a menu item adds.
#[derive(Clone, Copy)]
struct MenuItem {
    name: &'static str,
    kind: GraphKind,
    property: fn() -> GraphProperty,
}

/// Each menu, in the order its first kind comes, with the items of its kinds:
/// their presets, or a kind itself if it has none.
fn menu_items(kinds: &[GraphKind]) -> Vec<(&'static str, Vec<MenuItem>)> {
    let mut menus: Vec<(&'static str, Vec<MenuItem>)> = Vec::new();
    for &kind in kinds {
        let items = if kind.presets.is_empty() {
            vec![MenuItem {
                name: kind.name,
                kind,
                property: GraphProperty::default,
            }]
        } else {
            kind.presets
                .iter()
                .map(|preset| MenuItem {
                    name: preset.name,
                    kind,
                    property: preset.property,
                })
                .collect()
        };
        match menus.iter_mut().find(|(menu, _)| *menu == kind.menu) {
            Some((_, menu_items)) => menu_items.extend(items),
            None => menus.push((kind.menu, items)),
        }
    }
    menus
}

/// Pauses the graphs, the port read on all the same; or plays them again,
/// with all that came meanwhile.
#[component]
fn PauseButton() -> Element {
    let mut context = use_context::<GraphContext>();
    let paused = context.is_paused();
    let action = if paused {
        "Play graphs"
    } else {
        "Pause graphs"
    };

    rsx! {
        Button {
            class: "graph-pause",
            variant: ButtonVariant::Outline,
            size: ButtonSize::IconSm,
            aria_label: action,
            aria_pressed: paused,
            title: action,
            onclick: move |_| context.set_paused(!paused),
            if paused {
                lucide::Play {}
            } else {
                lucide::Pause {}
            }
        }
    }
}
