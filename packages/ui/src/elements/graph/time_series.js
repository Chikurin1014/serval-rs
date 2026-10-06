// Draws the `Number` data sent from `TimeSeriesGraph` (time_series.rs) with uPlot.
// Runs after `time_series_data.js`, whose functions it uses.
//
// Messages from Rust:
//   1. the container element id (once, after mount)
//   2. `[labels, updates, colors]` whenever data changes
//      - labels:  the selected labels that hold numbers; others are dropped
//      - updates: `[label, reset, [[timestamp_ms, value], ...]]`, only new points
//                 unless `reset` is set
//      - colors:  `{label: css custom property}`, each line's color (as its tag's)
//   3. `null` when the component unmounts

// Points kept per label: as many as a label keeps in Rust (`MAX_ENTRIES_PER_LABEL`)
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
// The labels and their colors the plot was created with
let plotLines = "";
/** @type {Record<string, string>} */
let colors = {};
let frame = 0;

// Resolve a CSS custom property (which may use the `--light`/`--dark` switch) to a color
const probe = document.createElement("span");
probe.style.display = "none";
container.appendChild(probe);
function cssColor(name) {
  probe.style.color = `var(${name})`;
  return getComputedStyle(probe).color;
}

// The area under a line, in its colour fading to clear towards the bottom
function areaFill(color) {
  return (u) => {
    const { top, height } = u.bbox;
    // Before the first layout the plot has no size to fade over yet
    if (!Number.isFinite(top) || !Number.isFinite(height) || height <= 0) {
      return withAlpha(color, 0.15);
    }
    const gradient = u.ctx.createLinearGradient(0, top, 0, top + height);
    gradient.addColorStop(0, withAlpha(color, 0.35));
    gradient.addColorStop(1, withAlpha(color, 0.02));
    return gradient;
  };
}

function sortedLabels() {
  return [...series.keys()].sort();
}

function lines(labels) {
  return labels.map((label) => `${label}=${colors[label]}`).join("\n");
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
  plotLines = lines(labels);

  const text = cssColor("--dc-text-muted");
  const grid = withAlpha(cssColor("--dc-border"), 0.35);
  const axis = {
    stroke: text,
    grid: { stroke: grid, width: 1 },
    ticks: { stroke: grid, width: 1 },
  };

  plot = new uPlot(
    {
      width: container.clientWidth,
      height: Math.max(container.clientHeight, 50),
      scales: { x: { time: true } },
      axes: [axis, { ...axis }],
      series: [
        {},
        ...labels.map((label) => {
          const color = cssColor(colors[label] ?? "--secondary-color-5");
          return {
            label,
            stroke: color,
            // An area chart: filled down to the bottom of the plot
            fill: areaFill(color),
            width: 1.5,
            spanGaps: true,
            points: { show: false },
          };
        }),
      ],
    },
    alignedData(series, labels),
    container,
  );
  fit();
  // The new legend only gets its final height after layout, so measure again
  requestAnimationFrame(fit);
}

function draw() {
  frame = 0;
  const labels = sortedLabels();
  if (!plot || lines(labels) !== plotLines) {
    create();
  } else {
    plot.setData(alignedData(series, labels));
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
themeObserver.observe(document.documentElement, {
  attributes: true,
  attributeFilter: ["data-theme"],
});
const systemTheme = matchMedia("(prefers-color-scheme: dark)");
systemTheme.addEventListener("change", create);

create();

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }

  const [labels, updates, lineColors] = message;
  colors = lineColors;
  applyMessage(series, [labels, updates], MAX_POINTS);
  schedule();
}

cancelAnimationFrame(frame);
resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", create);
plot?.destroy();
