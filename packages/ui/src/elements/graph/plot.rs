//! What the graphs' plots share: uPlot and its styles, the colors of their
//! series, and the script each runs.

use dioxus::{document::Eval, prelude::*};

use super::GraphContext;
use crate::elements::FilterContext;
use crate::helper::use_script;
use crate::time::TimeContext;

const UPLOT_CSS: Asset = asset!("/assets/vendor/uplot/uPlot.min.css");
const UPLOT_JS: Asset = asset!(
    "/assets/vendor/uplot/uPlot.iife.min.js",
    AssetOptions::js().with_minify(false)
);
const TIME_SERIES_CSS: Asset = asset!("/assets/styling/time-series.css");

/// A plot's script, of `$files` in this folder after the scripts the plots
/// share.
macro_rules! plot_script {
    ($($file:literal),+ $(,)?) => {
        concat!(
            include_str!("paced_draws.js"),
            include_str!("plot_common.js"),
            $(include_str!($file)),+
        )
    };
}
pub(super) use plot_script;

/// Series colors, by a label's place among those of its type.
const SERIES_COLORS: [&str; 5] = [
    "--dc-accent",
    "--focused-border-color",
    "--secondary-warning-color",
    "--dc-danger",
    "--secondary-color-5",
];

pub(super) fn series_color(index: usize) -> &'static str {
    SERIES_COLORS[index % SERIES_COLORS.len()]
}

/// The color of `label`, by its place among `labels`.
pub(super) fn label_color(labels: &[String], label: &str) -> &'static str {
    series_color(labels.iter().position(|each| each == label).unwrap_or(0))
}

/// The styles of the plots and their legends (uPlot's, for those not of it
/// too), and uPlot if `uplot`.
#[component]
pub(super) fn PlotAssets(uplot: bool) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: UPLOT_CSS }
        document::Link { rel: "stylesheet", href: TIME_SERIES_CSS }
        if uplot {
            document::Script { src: UPLOT_JS }
        }
    }
}

/// Runs the plot `script` of graph `id`, taking what it tells: the labels
/// turned off in its legend (`["hidden", labels]`), or that it was zoomed into
/// (`["zoomed", []]`), so the graphs are paused.
pub(super) fn use_plot(id: usize, script: &'static str) -> Eval {
    let mut graph_context = use_context::<GraphContext>();
    let time_context = use_context::<TimeContext>();
    let plot = use_script(script);
    use_future(move || {
        let time_context = time_context.clone();
        async move {
            let mut plot = plot;
            while let Ok((kind, labels)) = plot.recv::<(String, Vec<String>)>().await {
                match kind.as_str() {
                    "hidden" => graph_context.set_hidden(id, labels),
                    "zoomed" => graph_context.pause(time_context.current()),
                    _ => {}
                }
            }
        }
    });
    plot
}

/// How many of `labels` the data list's filters show.
pub(super) fn use_shown_count(labels: Memo<Vec<String>>) -> Memo<usize> {
    let filter_context = use_context::<FilterContext>();
    use_memo(move || {
        labels
            .read()
            .iter()
            .filter(|label| filter_context.shows(label))
            .count()
    })
}
