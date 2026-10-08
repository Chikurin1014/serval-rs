// Plots the data sent from `TimeSeriesGraph` with uPlot.
//
// From Rust: the container id, then `[labels, updates, colors, log, style,
// hidden, window]` on each change (updates: `[label, reset, [[ms, value]]]`),
// then `null` on unmount. To Rust: the labels turned off in the legend.

// As `MAX_ENTRIES_PER_LABEL`
const MAX_POINTS = 10000;
const CURSOR_SYNC_KEY = "serval-graphs";

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Its <script> tag may not have run yet
while (!window.uPlot) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

/** @type {Map<string, {t: number[], v: number[]}>} */
const series = new Map();
let plot = null;
let plotLines = "";
/** @type {Record<string, string>} */
let colors = {};
let logScale = false;
let drawStyle = "linear";
let windowSeconds = 10;
let fitData = true;
let hiddenLabels = new Set();
// Sent to Rust and not yet echoed back
let hiddenSent = null;
let frame = 0;

const probe = document.createElement("span");
probe.style.display = "none";
container.appendChild(probe);
function cssColor(name) {
  probe.style.color = `var(${name})`;
  return getComputedStyle(probe).color;
}

function areaFill(color) {
  return (u) => {
    const { top, height } = u.bbox;
    // No size before the first layout
    if (!Number.isFinite(top) || !Number.isFinite(height) || height <= 0) {
      return withAlpha(color, 0.15);
    }
    const gradient = u.ctx.createLinearGradient(0, top, 0, top + height);
    gradient.addColorStop(0, withAlpha(color, 0.35));
    gradient.addColorStop(1, withAlpha(color, 0.02));
    return gradient;
  };
}

function seriesStyle(color) {
  if (drawStyle === "points") {
    return {
      paths: () => null,
      points: { show: true, size: 4, width: 1, stroke: color, fill: color },
    };
  }
  return {
    ...(drawStyle === "stepped" && {
      paths: uPlot.paths.stepped({ align: 1 }),
    }),
    fill: areaFill(color),
    spanGaps: true,
    points: { show: false },
  };
}

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
  // For tests
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
        x: {
          time: true,
          range: (u, min, max) => timeRange(min, max, windowSeconds, fitData),
        },
        y: logScale ? { distr: 3, log: 10 } : {},
      },
      cursor: { sync: { key: CURSOR_SYNC_KEY, setSeries: false } },
      hooks: {
        setSeries: [onSeriesToggle],
        // For tests
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
  // The legend only gets its height after layout
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

// One redraw per frame
function schedule() {
  if (!frame) {
    frame = requestAnimationFrame(draw);
  }
}

const resize = new ResizeObserver(fit);
resize.observe(container);

// Colors are baked in at creation
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
    // Set at creation
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
