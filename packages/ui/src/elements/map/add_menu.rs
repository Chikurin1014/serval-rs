//! The bar of menus the maps are added from: a menu of the kinds of map for
//! each output type, their presets and variants in submenus beside them.

use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::dropdown_menu::DropdownMenuItem;
use crate::data::{DataType, MapContext, MapKind};
use crate::elements::{HoverMenu, HoverMenus, submenu};

use super::map_card::{MapTypes, type_names};

/// The add menus, each of the maps to the types it names.
const ADD_MENUS: [(&str, &[DataType]); 4] = [
    ("Bytes", &[DataType::Bytes]),
    ("Log", &[DataType::Log]),
    ("Number", &[DataType::Number]),
    ("Vector", &[DataType::Vector2d, DataType::Vector3d]),
];

/// As an add menu's item is pointed at or focused: its submenu to the left,
/// should it go past the window to the right (or, in a submenu to the left, as
/// that is, should it fit). Once for the page.
const SUBMENU_SIDE_JS: &str = r#"
if (!window.servalSubmenuSide) {
  window.servalSubmenuSide = (event) => {
    const item = event.target.closest?.('.add-map-content [data-presets="true"]');
    const submenu = item?.querySelector(":scope > .add-map-presets");
    if (!submenu) {
      return;
    }
    // Measured shown to the right, unseen
    delete submenu.dataset.side;
    const { display, visibility } = submenu.style;
    Object.assign(submenu.style, { display: "block", visibility: "hidden" });
    const { right, width } = submenu.getBoundingClientRect();
    Object.assign(submenu.style, { display, visibility });
    const fitsLeft = item.getBoundingClientRect().left - width >= 0;
    const inLeft = item.parentElement?.closest(".add-map-presets")?.dataset.side === "left";
    if ((inLeft && fitsLeft) || right > document.documentElement.clientWidth) {
      submenu.dataset.side = "left";
    }
  };
  document.addEventListener("mouseover", window.servalSubmenuSide);
  document.addEventListener("focusin", window.servalSubmenuSide);
}
"#;

/// A menu of the kinds of map for each output type.
#[component]
pub(super) fn AddMapBar() -> Element {
    let kinds = use_context::<MapContext>().kinds();
    let menus = use_hook(HoverMenus::new);
    use_effect(|| {
        document::eval(SUBMENU_SIDE_JS);
    });

    rsx! {
        div {
            class: "add-map-bar",
            span {
                class: "add-map-label",
                "Map to"
            }
            div {
                class: "add-map-menus",
                for (index, (name, types)) in ADD_MENUS.into_iter().enumerate() {
                    if index > 0 {
                        span { class: "add-map-separator", "/" }
                    }
                    AddMapMenu {
                        name,
                        kinds: kinds
                            .iter()
                            .filter(|kind| types.contains(&kind.to))
                            .copied()
                            .collect::<Vec<_>>(),
                        menus,
                        menu: index,
                    }
                }
            }
        }
    }
}

/// The kinds of map to the types of the menu `name`: those of one name to
/// several of them (e.g. a 2D and a 3D vector) as one item, its variants
/// beside it, as presets are.
#[component]
fn AddMapMenu(
    name: &'static str,
    mut kinds: Vec<MapKind>,
    menus: HoverMenus,
    menu: usize,
) -> Element {
    let mut context = use_context::<MapContext>();
    // By their inputs' last menu: a number times a vector with the vector maps,
    // and those of 2D and 3D vectors together
    kinds.sort_by_key(|kind| {
        kind.from
            .iter()
            .filter_map(|&from| {
                ADD_MENUS
                    .iter()
                    .position(|(_, types)| types.contains(&from))
            })
            .max()
    });
    let mut items: Vec<Vec<MapKind>> = Vec::new();
    for kind in kinds {
        match items.iter_mut().find(|item| item[0].name == kind.name) {
            Some(item) => item.push(kind),
            None => items.push(vec![kind]),
        }
    }
    let mut in_presets = use_signal(|| false);

    rsx! {
        HoverMenu {
            menus,
            menu,
            disabled: items.is_empty(),
            content_class: "add-map-content",
            keep_open: in_presets,
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::ArrowRight {
                    event.prevent_default();
                    document::eval(submenu::FOCUS_FIRST);
                }
            },
            trigger: rsx! { "{name}" },
            for (index, variants) in items.into_iter().enumerate() {
                // The first, as its item is picked itself
                {
                    let kind = variants[0];
                    let has_variants = variants.len() > 1;
                    // Of the variants together: by their menus' names (e.g. Vector), once each
                    let mut from: Vec<&str> = Vec::new();
                    for &each in variants.iter().flat_map(|variant| variant.from) {
                        let each = if has_variants { menu_name(each) } else { each.name() };
                        if has_variants && from.contains(&each) {
                            continue;
                        }
                        from.push(each);
                    }
                    rsx! {
                        DropdownMenuItem {
                            "data-presets": has_variants || !kind.presets.is_empty(),
                            value: kind,
                            index,
                            on_select: move |kind| {
                                context.add(kind);
                                menus.close(menu);
                            },
                            span { class: "add-map-title", "{kind.name}" }
                            MapTypes {
                                from,
                                to: if has_variants { name } else { kind.to.name() },
                            }
                            if has_variants {
                                lucide::ChevronRight {}
                                AddMapSubmenu {
                                    label: "{kind.name} variants",
                                    open: in_presets,
                                    for variant in variants {
                                        // Its presets beside it, as the item's are
                                        div {
                                            class: "add-map-variant",
                                            "data-presets": !variant.presets.is_empty(),
                                            button {
                                                class: "add-map-preset",
                                                r#type: "button",
                                                role: "menuitem",
                                                aria_haspopup: if !variant.presets.is_empty() { "menu" },
                                                onclick: move |event: MouseEvent| {
                                                    event.stop_propagation();
                                                    context.add(variant);
                                                    in_presets.set(false);
                                                    menus.close(menu);
                                                },
                                                span { class: "add-map-preset-name", "{variant_name(variant)}" }
                                                MapTypes { from: type_names(variant.from), to: variant.to.name() }
                                                if !variant.presets.is_empty() {
                                                    lucide::ChevronRight {}
                                                }
                                            }
                                            if !variant.presets.is_empty() {
                                                AddMapSubmenu {
                                                    label: "{kind.name} {variant_name(variant)} presets",
                                                    open: in_presets,
                                                    for preset in variant.presets {
                                                        button {
                                                            class: "add-map-preset",
                                                            r#type: "button",
                                                            role: "menuitem",
                                                            onclick: move |event: MouseEvent| {
                                                                event.stop_propagation();
                                                                context.add_preset(variant, *preset);
                                                                in_presets.set(false);
                                                                menus.close(menu);
                                                            },
                                                            span { class: "add-map-preset-name", "{preset.name}" }
                                                            span { class: "add-map-preset-detail", "{preset.detail}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if !kind.presets.is_empty() {
                                lucide::ChevronRight {}
                                AddMapSubmenu {
                                    label: "{kind.name} presets",
                                    open: in_presets,
                                    for preset in kind.presets {
                                        button {
                                            class: "add-map-preset",
                                            r#type: "button",
                                            role: "menuitem",
                                            onclick: move |event: MouseEvent| {
                                                event.stop_propagation();
                                                context.add_preset(kind, *preset);
                                                in_presets.set(false);
                                                menus.close(menu);
                                            },
                                            span { class: "add-map-preset-name", "{preset.name}" }
                                            span { class: "add-map-preset-detail", "{preset.detail}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// In a submenu: the item `step` from the focused one, of its own (not those of
/// a submenu in it).
fn focus_sibling(step: i32) -> String {
    format!(
        r#"const current = document.activeElement;
const menu = current?.closest("[role=menu]");
if (menu) {{
  const items = [...menu.querySelectorAll("[role=menuitem]")]
    .filter((item) => item.closest("[role=menu]") === menu);
  items[items.indexOf(current) + ({step})]?.focus();
}}"#
    )
}

/// From a submenu's item into the submenu beside it, if it has one.
const FOCUS_NESTED: &str = r#"document.activeElement?.parentElement
  ?.querySelector(":scope > .add-map-presets [role=menuitem]")?.focus();"#;

/// Back to what the focused submenu is beside: the menu's item, or another
/// submenu's.
const FOCUS_OWNER: &str = r#"const owner = document.activeElement?.closest(".add-map-presets")?.parentElement;
(owner?.matches("[role=option]") ? owner : owner?.querySelector(":scope > [role=menuitem]"))?.focus();"#;

/// Beside an add menu's item: its presets or variants, the menu kept `open`
/// while one is focused.
#[component]
fn AddMapSubmenu(label: String, open: Signal<bool>, children: Element) -> Element {
    let mut open = open;

    rsx! {
        div {
            class: "add-map-presets",
            onfocusin: move |_| open.set(true),
            onfocusout: move |_| open.set(false),
            onkeydown: move |event: KeyboardEvent| {
                let script = match event.key() {
                    Key::ArrowDown => focus_sibling(1),
                    Key::ArrowUp => focus_sibling(-1),
                    Key::ArrowRight => FOCUS_NESTED.to_string(),
                    Key::ArrowLeft | Key::Escape => FOCUS_OWNER.to_string(),
                    Key::Enter => {
                        event.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                event.prevent_default();
                // Not for the submenu (or menu) it is in
                event.stop_propagation();
                document::eval(&script);
            },
            div {
                class: "add-map-presets-list",
                role: "menu",
                aria_label: label,
                {children}
            }
        }
    }
}

/// What tells a variant from the others of its kind: the dimensions of the
/// vectors it gives, or else takes (as a dot product); or what it gives.
fn variant_name(kind: MapKind) -> &'static str {
    let dimensions = |data_type| match data_type {
        DataType::Vector2d => Some("2D"),
        DataType::Vector3d => Some("3D"),
        _ => None,
    };
    dimensions(kind.to)
        .or_else(|| kind.from.iter().copied().find_map(dimensions))
        .unwrap_or(kind.to.name())
}

/// The name of the add menu of `data_type`, e.g. Vector for a 2D vector.
fn menu_name(data_type: DataType) -> &'static str {
    ADD_MENUS
        .iter()
        .find(|(_, types)| types.contains(&data_type))
        .map_or(data_type.name(), |(name, _)| name)
}
