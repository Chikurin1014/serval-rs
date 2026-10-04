use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdPlus, Icon};

use super::{GraphContext, GraphProperty};
use crate::{
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
        sidebar::{
            Sidebar, SidebarCollapsible, SidebarContent, SidebarInset, SidebarProvider,
            SidebarSide, SidebarTrigger,
        },
    },
    elements::DataList,
};

const GRAPH_BOARD_CSS: Asset = asset!("/assets/styling/graph-board.css");

/// The graphs in `GraphContext`, with the data list in a sidebar on the left.
///
/// The sidebar is the dx `Sidebar`, kept inside the board by `graph-board.css`
/// (it is made for the page edge). Below 768px wide it opens as a sheet instead.
#[component]
pub fn GraphBoard() -> Element {
    let mut context = use_context::<GraphContext>();
    // Graphs compare by id and property; each graph view reads its own property
    let graphs = use_memo(move || context.list());
    let kinds = context.kinds();
    let columns = columns(graphs.read().len());

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
                    // One kind needs no menu; with several, choose like conversions
                    if let [kind] = kinds[..] {
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                                context.add(kind, GraphProperty::default());
                            },
                            Icon { icon: LdPlus }
                            "Add graph"
                        }
                    } else {
                        DropdownMenu {
                            DropdownMenuTrigger {
                                Icon { icon: LdPlus }
                                "Add graph"
                            }
                            DropdownMenuContent {
                                for (index, kind) in kinds.into_iter().enumerate() {
                                    DropdownMenuItem {
                                        value: kind,
                                        index,
                                        on_select: move |kind| {
                                            context.add(kind, GraphProperty::default());
                                        },
                                        "{kind.name}"
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    class: "graph-grid",
                    "data-columns": columns,
                    {graphs().into_iter().map(|graph| graph.view())}
                    if graphs.read().is_empty() {
                        p {
                            class: "graph-grid-empty",
                            "No graphs. Use \"Add graph\" to create one."
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
