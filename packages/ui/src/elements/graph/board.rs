use dioxus::prelude::*;

use super::{GraphContext, GraphKind};
use crate::{
    components::{
        dropdown_menu::DropdownMenuItem,
        sidebar::{
            Sidebar, SidebarCollapsible, SidebarContent, SidebarInset, SidebarProvider,
            SidebarSide, SidebarTrigger,
        },
    },
    elements::{DataList, HoverMenu, HoverMenus},
};

const GRAPH_BOARD_CSS: Asset = asset!("/assets/styling/graph-board.css");

/// A graph being dragged to another place on the board, and the graph it is
/// over (where it would land), for `GraphFrame`.
#[derive(Clone, Copy)]
pub(super) struct GraphDrag {
    pub dragging: Signal<Option<usize>>,
    pub target: Signal<Option<usize>>,
}

/// The graphs in `GraphContext`, with the data list in a sidebar on the left.
///
/// The sidebar is the dx `Sidebar`, kept inside the board by `graph-board.css`
/// (it is made for the page edge). Below 768px wide it opens as a sheet instead.
#[component]
pub fn GraphBoard() -> Element {
    let context = use_context::<GraphContext>();
    // Graphs compare by id and property; each graph view reads its own property
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

/// Columns of the graph grid: one graph alone, up to four in two columns,
/// more in three (see `graph-board.css`).
fn columns(graphs: usize) -> usize {
    match graphs {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::columns;

    #[test]
    fn columns_follow_the_number_of_graphs() {
        let by_count: Vec<_> = (0..=7).map(columns).collect();
        assert_eq!(by_count, [1, 1, 2, 2, 2, 3, 3, 3]);
    }
}

/// A menu for each kind of graph, opening on hover as the maps' do, of the
/// presets it can be added with.
#[component]
fn AddGraphBar(kinds: Vec<GraphKind>) -> Element {
    let mut context = use_context::<GraphContext>();
    let menus = use_hook(HoverMenus::new);

    rsx! {
        div {
            class: "add-graph-bar",
            for (menu, kind) in kinds.into_iter().enumerate() {
                HoverMenu {
                    menus,
                    menu,
                    trigger: rsx! { "{kind.name}" },
                    for (index, preset) in kind.presets.iter().enumerate() {
                        DropdownMenuItem {
                            value: index,
                            index,
                            on_select: move |index: usize| {
                                context.add(kind, (kind.presets[index].property)());
                                menus.close(menu);
                            },
                            "{preset.name}"
                        }
                    }
                }
            }
        }
    }
}
