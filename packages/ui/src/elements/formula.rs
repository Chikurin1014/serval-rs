use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

/// A folder, so its CSS finds the fonts beside it
const KATEX: Asset = asset!("/assets/vendor/katex", AssetOptions::folder());
const FORMULA_CSS: Asset = asset!("/assets/styling/formula.css");
const FORMULA_JS: &str = include_str!("formula.js");

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A LaTeX formula rendered by KaTeX; `fallback` shows until it loads.
///
/// `latex` is a plain value, not a signal: a signal made of each value its
/// parent renders with would be kept until the parent is gone.
#[component]
pub fn Formula(
    latex: String,
    fallback: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let id = use_hook(|| format!("formula-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)));

    // Rendered again as `latex` changes
    use_effect({
        let id = id.clone();
        use_reactive!(|latex| {
            let eval = document::eval(FORMULA_JS);
            let _ = eval.send((id.clone(), latex));
        })
    });

    rsx! {
        document::Link { rel: "stylesheet", href: "{KATEX}/katex.min.css" }
        document::Link { rel: "stylesheet", href: FORMULA_CSS }
        document::Script { src: "{KATEX}/katex.min.js" }

        span {
            ..attributes,
            span {
                class: "formula",
                id: "{id}",
                span { class: "formula-fallback", "{fallback}" }
                // Filled by KaTeX
                span { class: "formula-math" }
            }
        }
    }
}
