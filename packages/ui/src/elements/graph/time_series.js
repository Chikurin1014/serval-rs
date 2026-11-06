// Plots the data sent from `TimeSeriesGraph` with uPlot.
//
// From Rust: the container id, then `[labels, updates, colors, log, style,
// hidden, rangeSeconds, paused]` on each change (updates: `[label, reset,
// [[ms, value]]]`; rangeSeconds: the seconds of its time range, none for
// all the time), then `null` on unmount. To Rust: `["hidden", labels]`, those
// turned off in the legend; `["zoomed", []]` as it is zoomed into.

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
let rangeSeconds = 10;
const hiddenLabels = trackHidden();

const themed = themeColors(container);
const cssColor = themed.color;

// The values at the pointer's time, beside it: in the plot the pointer is
// over, not in those whose cursors follow it
const hover = plotTooltip("time-series-hover");
let pointerOver = false;

function showHover(u) {
  const { left, top, idx, idxs } = u.cursor;
  const over = pointerOver && idx != null && left != null && left >= 0;
  hover.hidden = !over;
  if (!over) {
    return;
  }
  // Each label shown and its value nearest the time, as uPlot's legend had
  const rows = [];
  u.series.forEach((s, index) => {
    const at = idxs[index];
    const value = at == null ? null : u.data[index][at];
    if (index > 0 && s.show && value != null) {
      const swatch = document.createElement("span");
      swatch.className = "plot-tooltip-swatch";
      swatch.style.background = seriesColor(s.label);
      rows.push([[swatch, s.label], s.value(u, value, index, at)]);
    }
  });
  fillTooltip(hover, u.series[0].value(u, u.data[0][idx], 0, idx), rows);
  placeTooltip(hover, [left, top], [u.over.clientWidth, u.over.clientHeight]);
}

function seriesColor(label) {
  return cssColor(colors[label] ?? "--secondary-color-5");
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

// The times and values zoomed to by dragging a rectangle over the plot, if it
// is: kept until a double click, or played again (the values, until another
// scale)
let zoomed = null;

function onSelect(u) {
  const { left, top, width, height } = u.select;
  if (width < 2 || height < 2) {
    return;
  }
  zoomed = {
    x: [u.posToVal(left, "x"), u.posToVal(left + width, "x")],
    y: [u.posToVal(top + height, "y"), u.posToVal(top, "y")],
  };
  u.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
  zoomIn(u);
  // For tests
  container.dataset.zoomed = "true";
  // Paused, so the data stays as zoomed into
  dioxus.send(["zoomed", []]);
}

/** Sets the plot to what it is zoomed to. */
function zoomIn(u) {
  u.batch(() => {
    u.setScale("x", { min: zoomed.x[0], max: zoomed.x[1] });
    if (zoomed.y) {
      u.setScale("y", { min: zoomed.y[0], max: zoomed.y[1] });
    }
  });
}

function unzoom() {
  if (!zoomed) {
    return;
  }
  zoomed = null;
  container.dataset.zoomed = "false";
  plot?.setData(plot.data);
}

function onSeriesToggle(u, index, options) {
  if (index === null || options.show === undefined) {
    return;
  }
  const label = u.series[index].label;
  hiddenLabels.set(label, !options.show);
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
          range: (u, min, max) =>
            zoomed?.x ?? timeRange(min, max, rangeSeconds),
        },
        y: {
          ...(logScale && { distr: 3, log: 10 }),
          // Kept as zoomed to, as the data is set
          auto: () => !zoomed?.y,
        },
      },
      cursor: {
        sync: { key: CURSOR_SYNC_KEY, setSeries: false },
        // Zoomed by `onSelect`, so it is told
        drag: { x: true, y: true, setScale: false },
        // Each label's nearest value, as each has times of its own
        hover: { skip: [null, undefined] },
      },
      // Its labels alone: the values are in `hover`
      legend: { live: false },
      hooks: {
        setCursor: [showHover],
        setSeries: [onSeriesToggle],
        setSelect: [onSelect],
        // For tests
        setScale: [
          (u, key) => {
            if (key === "x" && u.scales.x.min != null) {
              container.dataset.timeSpan = String(
                u.scales.x.max - u.scales.x.min,
              );
            }
            if (key === "y" && u.scales.y.min != null) {
              container.dataset.valueSpan = String(
                u.scales.y.max - u.scales.y.min,
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
          const color = seriesColor(label);
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
  // Unzoomed by a double click, before uPlot takes it
  plot.root.addEventListener("dblclick", unzoom, true);
  plot.over.appendChild(hover);
  plot.over.addEventListener("mouseenter", () => {
    pointerOver = true;
  });
  plot.over.addEventListener("mouseleave", () => {
    pointerOver = false;
    hover.hidden = true;
  });
  if (zoomed) {
    zoomIn(plot);
  }
  fit();
  // The legend only gets its height after layout
  requestAnimationFrame(fit);
}

function draw() {
  const labels = sortedLabels();
  if (!plot || lines(labels) !== plotLines) {
    create();
  } else {
    plot.setData(plotData(labels));
  }
}

// One redraw per frame, fewer as the page is busy
const draws = pacedDraws(draw);
function schedule() {
  draws.schedule();
}

const resize = new ResizeObserver(fit);
resize.observe(container);

// Colors are baked in at creation, as taken anew
function onTheme() {
  themed.clear();
  create();
}
const stopWatchingTheme = watchTheme(onTheme);

create();

for await (const message of messages()) {
  const [
    labels,
    updates,
    lineColors,
    log,
    style,
    hidden,
    seconds,
    paused,
  ] = message;
  // Played again: the newest as it comes
  if (!paused) {
    unzoom();
  }
  rangeSeconds = seconds;
  colors = lineColors;
  hiddenLabels.apply(hidden);
  showSeries(plot, hiddenLabels);
  if (log !== logScale || style !== drawStyle) {
    // The values zoomed to, not of another scale
    if (log !== logScale && zoomed) {
      zoomed.y = null;
    }
    logScale = log;
    drawStyle = style;
    // Set at creation
    plotLines = null;
  }
  applyMessage(series, [labels, updates], MAX_POINTS);
  schedule();
}

draws.cancel();
resize.disconnect();
stopWatchingTheme();
plot?.destroy();
