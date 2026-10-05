use dioxus::prelude::*;

use crate::theme::ThemeSwitch;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div {
            id: "navbar",
            {children}
            ThemeSwitch {}
        }
    }
}
