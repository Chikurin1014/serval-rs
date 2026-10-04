// Draws the `Number` data sent from `TimeSeriesGraph` (time_series.rs) with uPlot.
//
// Messages from Rust:
//   1. the container element id (once, after mount)
//   2. `[labels, updates]` whenever data changes
//      - labels:  the selected labels that hold numbers; others are dropped
//      - updates: `[label, reset, [[timestamp_ms, value], ...]]`, only new points
//                 unless `reset` is set
//   3. `null` when the component unmounts

const MAX_POINTS = 10000;

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
    return;
}

// `uPlot.iife.min.js` is loaded by a <script> tag that may not have run yet
while (!window.uPlot) {
    await new Promise((resolve) => setTimeout(resolve, 20));
}

/** @type {Map<string, {t: number[], v: number[]}>} */
const series = new Map();
let plot = null;
let plotLabels = "";
let frame = 0;

// Resolve a CSS custom property (which may use the `--light`/`--dark` switch) to a color
const probe = document.createElement("span");
probe.style.display = "none";
container.appendChild(probe);
function cssColor(name) {
    probe.style.color = `var(${name})`;
    return getComputedStyle(probe).color;
}

function withAlpha(color, alpha) {
    return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
}

const PALETTE = [
    "--dc-accent",
    "--focused-border-color",
    "--secondary-warning-color",
    "--dc-danger",
    "--secondary-color-5",
];

function sortedLabels() {
    return [...series.keys()].sort();
}

// uPlot wants one shared x array; series without a point at some x get `null`
function alignedData(labels) {
    const xs = [...new Set(labels.flatMap((label) => series.get(label).t))].sort((a, b) => a - b);
    const index = new Map(xs.map((x, i) => [x, i]));
    const ys = labels.map((label) => {
        const { t, v } = series.get(label);
        const y = new Array(xs.length).fill(null);
        t.forEach((x, i) => {
            y[index.get(x)] = v[i];
        });
        return y;
    });
    return [xs, ...ys];
}

function fit() {
    if (!plot) {
        return;
    }
    const legend = plot.root.querySelector(".u-legend");
    const height = container.clientHeight - (legend ? legend.offsetHeight : 0);
    plot.setSize({ width: container.clientWidth, height: Math.max(height, 50) });
}

function create() {
    plot?.destroy();
    const labels = sortedLabels();
    plotLabels = labels.join("\n");

    const text = cssColor("--dc-text-muted");
    const grid = withAlpha(cssColor("--dc-border"), 0.35);
    const axis = { stroke: text, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid, width: 1 } };

    plot = new uPlot(
        {
            width: container.clientWidth,
            height: Math.max(container.clientHeight, 50),
            scales: { x: { time: true } },
            axes: [axis, { ...axis }],
            series: [
                {},
                ...labels.map((label, i) => ({
                    label,
                    stroke: cssColor(PALETTE[i % PALETTE.length]),
                    width: 1.5,
                    spanGaps: true,
                    points: { show: false },
                })),
            ],
        },
        alignedData(labels),
        container,
    );
    fit();
    // The new legend only gets its final height after layout, so measure again
    requestAnimationFrame(fit);
}

function draw() {
    frame = 0;
    const labels = sortedLabels();
    if (!plot || labels.join("\n") !== plotLabels) {
        create();
    } else {
        plot.setData(alignedData(labels));
    }
}

// Coalesce bursts of updates into one redraw per animation frame
function schedule() {
    if (!frame) {
        frame = requestAnimationFrame(draw);
    }
}

const resize = new ResizeObserver(fit);
resize.observe(container);

// Colors are baked in at creation, so rebuild when the theme changes
const themeObserver = new MutationObserver(create);
themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
const systemTheme = matchMedia("(prefers-color-scheme: dark)");
systemTheme.addEventListener("change", create);

create();

while (true) {
    const message = await dioxus.recv();
    if (message === null) {
        break;
    }

    const [labels, updates] = message;
    for (const label of [...series.keys()]) {
        if (!labels.includes(label)) {
            series.delete(label);
        }
    }
    for (const label of labels) {
        if (!series.has(label)) {
            series.set(label, { t: [], v: [] });
        }
    }
    for (const [label, reset, points] of updates) {
        const s = reset ? { t: [], v: [] } : series.get(label);
        series.set(label, s);
        for (const [timestamp, value] of points) {
            s.t.push(timestamp / 1000);
            s.v.push(value);
        }
        const excess = s.t.length - MAX_POINTS;
        if (excess > 0) {
            s.t.splice(0, excess);
            s.v.splice(0, excess);
        }
    }
    schedule();
}

cancelAnimationFrame(frame);
resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", create);
plot?.destroy();
