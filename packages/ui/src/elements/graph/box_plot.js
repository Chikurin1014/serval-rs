// Draws the boxes sent from `BoxPlotGraph` with uPlot.
//
// From Rust: the container id, then `[boxes, log]` on each change (each box:
// `[label, color, [min, low, q1, median, q3, high, max], outliers, rows]`;
// low and high its whiskers' ends, rows `[[name, text]]` its hover's), then
// `null` on unmount.

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Its <script> tag may not have run yet
while (!window.uPlot) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

let boxes = [];
let logScale = false;
let plot = null;
// The box the pointer is over, if any
let hovered = null;
let frame = 0;

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

// The statistics of the box the pointer is over, beside it
const hover = document.createElement("div");
hover.className = "box-plot-hover plot-tooltip";
hover.hidden = true;

/** uPlot's data: each box's place, lowest and highest value. */
function plotData() {
  const shown = (value) => (logScale && value <= 0 ? null : value);
  return [
    boxes.map((_, index) => index),
    boxes.map(([, , values]) => shown(values[0])),
    boxes.map(([, , values]) => shown(values[6])),
  ];
}

/** Each box's half width, in canvas pixels. */
function halfWidth(u) {
  const column = u.bbox.width / Math.max(boxes.length, 1);
  return Math.min(column * 0.3, 40 * devicePixelRatio);
}

function drawBoxes(u) {
  // For tests: each box's label, outliers and place across the plot's area,
  // as of each drawing (as the plot is resized too)
  container.dataset.boxes = JSON.stringify(
    boxes.map(([label, , , outliers], index) => [
      label,
      outliers.length,
      Math.round(u.valToPos(index, "x")),
    ]),
  );
  const { left, top, width, height } = u.bbox;
  const ctx = u.ctx;
  // Values of 0 or less below a log axis's bottom
  const y = (value) =>
    logScale && value <= 0 ? top + height : u.valToPos(value, "y", true);
  const half = halfWidth(u);
  const line = devicePixelRatio;
  ctx.save();
  ctx.beginPath();
  ctx.rect(left, top, width, height);
  ctx.clip();
  boxes.forEach(([, color, values, outliers], index) => {
    const [, low, q1, median, q3, high] = values;
    const stroke = cssColor(color);
    const x = u.valToPos(index, "x", true);
    ctx.strokeStyle = stroke;
    ctx.lineWidth = line;

    // Whiskers, with their caps
    ctx.beginPath();
    for (const [from, to] of [
      [low, q1],
      [q3, high],
    ]) {
      ctx.moveTo(x, y(from));
      ctx.lineTo(x, y(to));
    }
    for (const end of [low, high]) {
      ctx.moveTo(x - half / 2, y(end));
      ctx.lineTo(x + half / 2, y(end));
    }
    ctx.stroke();

    // The box, the median across it
    const boxTop = y(q3);
    const boxHeight = y(q1) - boxTop;
    ctx.fillStyle = withAlpha(stroke, index === hovered ? 0.4 : 0.2);
    ctx.fillRect(x - half, boxTop, half * 2, boxHeight);
    ctx.strokeRect(x - half, boxTop, half * 2, boxHeight);
    ctx.lineWidth = 2 * line;
    ctx.beginPath();
    ctx.moveTo(x - half, y(median));
    ctx.lineTo(x + half, y(median));
    ctx.stroke();

    // Outliers, as dots
    ctx.lineWidth = line;
    ctx.fillStyle = withAlpha(stroke, 0.6);
    const radius = 2.5 * devicePixelRatio;
    const dots = new Path2D();
    for (const value of outliers) {
      if (logScale && value <= 0) {
        continue;
      }
      dots.moveTo(x + radius, y(value));
      dots.arc(x, y(value), radius, 0, 2 * Math.PI);
    }
    ctx.fill(dots);
    ctx.stroke(dots);
  });
  ctx.restore();
}

function showHover(u) {
  const left = u.cursor.left;
  const index =
    left == null || left < 0 ? null : Math.round(u.posToVal(left, "x"));
  const over = index !== null && index >= 0 && index < boxes.length;
  const next = over ? index : null;
  if (next !== hovered) {
    hovered = next;
    // Its box filled darker
    u.redraw(false);
  }
  hover.hidden = !over;
  if (!over) {
    return;
  }
  const [label, , , , rows] = boxes[index];
  hover.replaceChildren();
  const title = document.createElement("div");
  title.className = "plot-tooltip-label";
  title.textContent = label;
  const list = document.createElement("dl");
  for (const [name, text] of rows) {
    const term = document.createElement("dt");
    term.textContent = name;
    const value = document.createElement("dd");
    value.textContent = text;
    list.append(term, value);
  }
  hover.append(title, list);

  // Beside the box, on the side with room
  const x = u.valToPos(index, "x");
  const half = halfWidth(u) / devicePixelRatio;
  const area = u.over.clientWidth;
  const right = x + half + 8;
  hover.style.left =
    right + hover.offsetWidth <= area
      ? `${right}px`
      : `${Math.max(0, x - half - 8 - hover.offsetWidth)}px`;
  const top = Math.min(
    Math.max(0, u.cursor.top - hover.offsetHeight / 2),
    Math.max(0, u.over.clientHeight - hover.offsetHeight),
  );
  hover.style.top = `${top}px`;
}

// Padded, so a box of one value has room around it
function valueScale() {
  return {
    ...(logScale && { distr: 3, log: 10 }),
    range: (u, min, max) => {
      if (min == null || max == null) {
        return logScale ? [1, 10] : [0, 1];
      }
      return logScale
        ? uPlot.rangeLog(min, max, 10, false)
        : uPlot.rangeNum(min, max, 0.1, true);
    },
  };
}

function fit() {
  plot?.setSize({
    width: container.clientWidth,
    height: Math.max(container.clientHeight, 50),
  });
}

function create() {
  plot?.destroy();
  // For tests
  container.dataset.valueScale = logScale ? "log" : "linear";

  const text = cssColor("--dc-text-muted");
  const grid = withAlpha(cssColor("--dc-border"), 0.35);
  const axis = {
    stroke: text,
    grid: { stroke: grid, width: 1 },
    ticks: { stroke: grid, width: 1 },
  };
  // Not drawn: they only set the value axis's range
  const extent = { paths: () => null, points: { show: false } };

  plot = new uPlot(
    {
      width: container.clientWidth,
      height: Math.max(container.clientHeight, 50),
      legend: { show: false },
      cursor: {
        x: false,
        y: false,
        points: { show: false },
        drag: { x: false, y: false },
      },
      scales: {
        x: {
          time: false,
          range: () => [-0.5, Math.max(boxes.length, 1) - 0.5],
        },
        y: valueScale(),
      },
      axes: [
        {
          ...axis,
          grid: { show: false },
          // Each box's label below it
          splits: () => boxes.map((_, index) => index),
          values: (u, splits) => splits.map((index) => boxes[index]?.[0] ?? ""),
        },
        axis,
      ],
      hooks: { draw: [drawBoxes], setCursor: [showHover] },
      series: [{}, extent, extent],
    },
    plotData(),
    container,
  );
  plot.over.appendChild(hover);
}

function draw() {
  frame = 0;
  plot.setData(plotData());
  if (hovered !== null) {
    showHover(plot);
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

  const [nextBoxes, log] = message;
  boxes = nextBoxes;
  if (log !== logScale) {
    logScale = log;
    // Set at creation
    create();
  }
  schedule();
}

cancelAnimationFrame(frame);
resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", create);
plot?.destroy();
