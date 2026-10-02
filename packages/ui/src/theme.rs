use dioxus::prelude::*;

const DX_COMPONENTS_THEME_CSS: Asset = asset!("/assets/dx-components-theme.css");
const THEME_CSS: Asset = asset!("/assets/styling/theme.css");

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: DX_COMPONENTS_THEME_CSS }
        document::Link { rel: "stylesheet", href: THEME_CSS }
        {children}
    }
}
