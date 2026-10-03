use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdMoon, LdSun},
    Icon,
};

use crate::components::button::{Button, ButtonSize, ButtonVariant};

const DX_COMPONENTS_THEME_CSS: Asset = asset!("/assets/dx-components-theme.css");
const THEME_CSS: Asset = asset!("/assets/styling/theme.css");
const THEME_SWITCH_CSS: Asset = asset!("/assets/styling/theme-switch.css");

/// Re-applies the theme chosen with `ThemeSwitch` in a previous session.
const RESTORE_THEME_JS: &str = r#"
try {
    const theme = localStorage.getItem("theme");
    if (theme === "light" || theme === "dark") {
        document.documentElement.dataset.theme = theme;
    }
} catch (_) {}
"#;

/// Flips `data-theme` on `<html>`, which `dx-components-theme.css` switches on.
/// Without an explicit choice the current theme is the system preference.
const TOGGLE_THEME_JS: &str = r#"
const root = document.documentElement;
const system = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
const next = (root.dataset.theme ?? system) === "dark" ? "light" : "dark";
root.dataset.theme = next;
try {
    localStorage.setItem("theme", next);
} catch (_) {}
"#;

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    use_effect(|| {
        document::eval(RESTORE_THEME_JS);
    });

    rsx! {
        document::Link { rel: "stylesheet", href: DX_COMPONENTS_THEME_CSS }
        document::Link { rel: "stylesheet", href: THEME_CSS }
        {children}
    }
}

/// Button that toggles between the light and dark theme.
/// Both icons are rendered; CSS shows the one matching the active theme.
#[component]
pub fn ThemeSwitch() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: THEME_SWITCH_CSS }

        Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconSm,
            class: "theme-switch",
            aria_label: "Toggle theme",
            onclick: move |_| {
                document::eval(TOGGLE_THEME_JS);
            },
            span {
                class: "theme-switch-sun",
                Icon { icon: LdSun {} }
            }
            span {
                class: "theme-switch-moon",
                Icon { icon: LdMoon {} }
            }
        }
    }
}
