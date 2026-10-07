// Draws the `Number` data sent from `TimeSeriesGraph` (time_series.rs) with uPlot.
// Runs after `time_series_data.js`, whose functions it uses.
//
// Messages from Rust:
//   1. the container element id (once, after mount)
//   2. `[labels, updates, colors, log, style, hidden, window]` whenever data or
//      the settings change
//      - labels:  the labels shown (by the Data list's filters) that hold numbers;
//                 others are dropped
//      - updates: `[label, reset, [[timestamp_ms, value], ...]]`, only new points
//                 unless `reset` is set
//      - colors:  `{label: css custom property}`, each line's color
//      - log:     whether the value axis is logarithmic, else linear
//      - style:   how the values are drawn: "points", "linear" or "stepped"
//      - hidden:  the labels turned off in the legend
//      - window:  `[seconds, fit]`, how much of the newest data the time axis
//                 shows: always that long, or with `fit`, the data up to that long
//   3. `null` when the component unmounts
//
// Message to Rust: the labels turned off, whenever one is turned on or off in
// the legend

// Points kept per label: as many as a label keeps in Rust (`MAX_ENTRIES_PER_LABEL`)
const MAX_POINTS = 10000;
// Shared by every graph, so their cursors move together
const CURSOR_SYNC_KEY = "serval-graphs";

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
// Whether the value axis is logarithmic (base 10), else linear
let logScale = false;
// How each label's values are drawn: "points", "linear" or "stepped"
let drawStyle = "linear";
// How many seconds of the newest data the time axis shows, and whether it fits
// the data (up to that long) rather than always being that wide
let windowSeconds = 10;
let fitData = true;
// The labels turned off in the legend (kept in Rust's `GraphContext`)
let hiddenLabels = new Set();
// The list last told to Rust, until Rust sends it back: messages before that
// still have the old one
let hiddenSent = null;
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

// A label's series options in `drawStyle`, for its line `color`
function seriesStyle(color) {
  if (drawStyle === "points") {
    // Only the points: no line between them, nothing filled
    return {
      paths: () => null,
      points: { show: true, size: 4, width: 1, stroke: color, fill: color },
    };
  }
  return {
    // Each value held until the next for "stepped"
    ...(drawStyle === "stepped" && {
      paths: uPlot.paths.stepped({ align: 1 }),
    }),
    // An area chart: filled down to the bottom of the plot
    fill: areaFill(color),
    spanGaps: true,
    points: { show: false },
  };
}

// A series turned on or off in the legend: tells Rust, to keep it
function onSeriesToggle(u, index, options) {
  if (index === null || options.show === undefined) {
    return;
  }
  const label = u.series[index].label;
  if (options.show) {
    hiddenLabels.delete(label);
  } else {
    hiddenLabels.add(label);
  }
  const hidden = [...hiddenLabels].sort();
  hiddenSent = JSON.stringify(hidden);
  dioxus.send(hidden);
}

// The series off in Rust, turned off here too, without telling Rust back
function applyHidden(hidden) {
  if (hiddenSent !== null) {
    if (JSON.stringify([...hidden].sort()) !== hiddenSent) {
      return;
    }
    hiddenSent = null;
  }
  hiddenLabels = new Set(hidden);
  plot?.series.forEach((s, index) => {
    if (index > 0 && s.show === hiddenLabels.has(s.label)) {
      plot.setSeries(index, { show: !hiddenLabels.has(s.label) }, false);
    }
  });
}

// The points to plot, as the value axis can show them
function plotData(labels) {
  const data = alignedData(series, labels);
  return logScale ? forLogScale(data) : data;
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
  // What the plot was made with, for the page (and its tests) to read
  container.dataset.valueScale = logScale ? "log" : "linear";
  container.dataset.drawStyle = drawStyle;
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
      scales: {
        // Read at each redraw, so a new window needs no new plot
        x: {
          time: true,
          range: (u, min, max) => timeRange(min, max, windowSeconds, fitData),
        },
        y: logScale ? { distr: 3, log: 10 } : {},
      },
      // One cursor across the graphs: each follows the time pointed at in another
      // (Only the cursor: each graph turns its own series on and off)
      cursor: { sync: { key: CURSOR_SYNC_KEY, setSeries: false } },
      hooks: {
        setSeries: [onSeriesToggle],
        // The time axis's span in seconds, for the page (and its tests) to read
        setScale: [
          (u, key) => {
            if (key === "x" && u.scales.x.min != null) {
              container.dataset.timeSpan = String(
                u.scales.x.max - u.scales.x.min,
              );
            }
          },
        ],
      },
      axes: [axis, { ...axis }],
      series: [
        // The time pointed at, `HH:MM:SS.SSS` as the Data list shows it
        {
          value: (u, seconds) =>
            seconds == null ? "--" : timeOfDay(seconds * 1000),
        },
        ...labels.map((label) => {
          const color = cssColor(colors[label] ?? "--secondary-color-5");
          return {
            label,
            show: !hiddenLabels.has(label),
            stroke: color,
            width: 1.5,
            ...seriesStyle(color),
          };
        }),
      ],
    },
    plotData(labels),
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
    plot.setData(plotData(labels));
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

  const [labels, updates, lineColors, log, style, hidden, [seconds, fit]] =
    message;
  windowSeconds = seconds;
  fitData = fit;
  colors = lineColors;
  applyHidden(hidden);
  if (log !== logScale || style !== drawStyle) {
    logScale = log;
    drawStyle = style;
    // These are set at creation, so make the plot again
    plotLines = null;
  }
  applyMessage(series, [labels, updates], MAX_POINTS);
  schedule();
}

cancelAnimationFrame(frame);
resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", create);
plot?.destroy();
