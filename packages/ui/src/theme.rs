use dioxus::prelude::*;
use dioxus_icons::lucide;

use crate::components::button::{Button, ButtonSize, ButtonVariant};

const DX_COMPONENTS_THEME_CSS: Asset = asset!("/assets/dx-components-theme.css");
const THEME_CSS: Asset = asset!("/assets/styling/theme.css");
const THEME_SWITCH_CSS: Asset = asset!("/assets/styling/theme-switch.css");

const RESTORE_THEME_JS: &str = concat!(
    include_str!("theme.js"),
    "\nrestoreTheme(document.documentElement, browserStorage());\n",
);

const TOGGLE_THEME_JS: &str = concat!(
    include_str!("theme.js"),
    "\ntoggleTheme(\n",
    "    document.documentElement,\n",
    "    browserStorage(),\n",
    "    matchMedia(\"(prefers-color-scheme: dark)\").matches,\n",
    ");\n",
);

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

/// Toggles the light and dark theme.
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
                lucide::Sun {}
            }
            span {
                class: "theme-switch-moon",
                lucide::Moon {}
            }
        }
    }
}
