// Plots the points sent from `ScatterGraph` with uPlot.
//
// From Rust: the container id, then `[reset, points, [xLabel, yLabel],
// [xLog, yLog], [pointColor, lineColor], line, statistics]` on each change
// (points: `[[x, y]]`, after those before unless `reset`; line: `[slope,
// intercept]` or `null`; statistics: `[covariance, correlation]` as text or
// `null`), then `null` on unmount.

// As `MAX_ENTRIES_PER_LABEL`
const MAX_POINTS = 10000;

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Its <script> tag may not have run yet
while (!window.uPlot) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

let xs = [];
let ys = [];
let labels = ["", ""];
let logs = [false, false];
let colors = ["--dc-accent", "--secondary-warning-color"];
// `[slope, intercept]` of the line drawn, if any
let line = null;
// `[covariance, correlation]` as shown, if any
let statistics = null;
let plot = null;
// What the plot was made with
let made = "";
let frame = 0;

const probe = document.createElement("span");
probe.style.display = "none";
container.appendChild(probe);
function cssColor(name) {
  probe.style.color = `var(${name})`;
  return getComputedStyle(probe).color;
}

// The statistics, as a tooltip by the pointer over the plot
const statisticsBox = document.createElement("div");
statisticsBox.className = "scatter-statistics plot-tooltip";
statisticsBox.hidden = true;
const statisticsList = document.createElement("dl");
statisticsBox.append(statisticsList);
const statisticsValues = ["Covariance", "Correlation"].map((name) => {
  const term = document.createElement("dt");
  term.textContent = name;
  const value = document.createElement("dd");
  statisticsList.append(term, value);
  return value;
});

function showStatistics(u) {
  statistics?.forEach((text, index) => {
    statisticsValues[index].textContent = text;
  });
  const { left, top } = u.cursor;
  const over = statistics && left != null && left >= 0;
  statisticsBox.hidden = !over;
  if (!over) {
    return;
  }
  // Below right of the pointer, or on the side with room
  const gap = 12;
  const { clientWidth: width, clientHeight: height } = u.over;
  const x =
    left + gap + statisticsBox.offsetWidth <= width
      ? left + gap
      : left - gap - statisticsBox.offsetWidth;
  const y =
    top + gap + statisticsBox.offsetHeight <= height
      ? top + gap
      : top - gap - statisticsBox.offsetHeight;
  statisticsBox.style.left = `${Math.max(0, x)}px`;
  statisticsBox.style.top = `${Math.max(0, y)}px`;
}

function withAlpha(color, alpha) {
  return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
}

/** uPlot's data: the points, those of 0 or less left out of a log axis. */
function plotData() {
  if (!logs[0] && !logs[1]) {
    return [null, [xs, ys]];
  }
  const x = [];
  const y = [];
  xs.forEach((value, index) => {
    if ((!logs[0] || value > 0) && (!logs[1] || ys[index] > 0)) {
      x.push(value);
      y.push(ys[index]);
    }
  });
  return [null, [x, y]];
}

// Padded, so one point (or all of one value) has room around it: as no
// points do
function scale(log) {
  return {
    time: false,
    ...(log && { distr: 3, log: 10 }),
    range: (u, min, max) => {
      if (min == null || max == null) {
        return log ? [1, 10] : [0, 1];
      }
      return log
        ? uPlot.rangeLog(min, max, 10, false)
        : uPlot.rangeNum(min, max, 0.1, true);
    },
  };
}

// Each point as a dot: uPlot draws none of its own in `mode: 2`
function drawPoints(fill, stroke) {
  return (u, seriesIndex) => {
    const size = 5 * devicePixelRatio;
    uPlot.orient(
      u,
      seriesIndex,
      (
        series,
        dataX,
        dataY,
        scaleX,
        scaleY,
        valToPosX,
        valToPosY,
        xOff,
        yOff,
        xDim,
        yDim,
        moveTo,
        lineTo,
        rect,
        arc,
      ) => {
        const [xs, ys] = u.data[seriesIndex];
        const path = new Path2D();
        for (let i = 0; i < xs.length; i++) {
          const [x, y] = [xs[i], ys[i]];
          if (
            x < scaleX.min ||
            x > scaleX.max ||
            y < scaleY.min ||
            y > scaleY.max
          ) {
            continue;
          }
          const cx = valToPosX(x, scaleX, xDim, xOff);
          const cy = valToPosY(y, scaleY, yDim, yOff);
          path.moveTo(cx + size / 2, cy);
          arc(path, cx, cy, size / 2, 0, 2 * Math.PI);
        }
        u.ctx.save();
        u.ctx.fillStyle = fill;
        u.ctx.strokeStyle = stroke;
        u.ctx.lineWidth = devicePixelRatio;
        u.ctx.fill(path);
        u.ctx.stroke(path);
        // As it was, for the axes drawn after
        u.ctx.restore();
      },
    );
    return null;
  };
}

// The line across the plot, dashed: a curve on a log axis
function drawLine(u) {
  if (!line) {
    return;
  }
  const [slope, intercept] = line;
  const { left, top, width, height } = u.bbox;
  const ctx = u.ctx;
  ctx.save();
  ctx.beginPath();
  ctx.rect(left, top, width, height);
  ctx.clip();
  ctx.beginPath();
  let drawing = false;
  // Each 2 CSS pixels across
  for (let pos = 0; pos <= width / devicePixelRatio; pos += 2) {
    const x = u.posToVal(pos, "x");
    const y = slope * x + intercept;
    if (!Number.isFinite(y) || (logs[1] && y <= 0)) {
      drawing = false;
      continue;
    }
    const cx = u.valToPos(x, "x", true);
    // Far past the plot is as good as past it
    const cy = Math.max(-1e6, Math.min(1e6, u.valToPos(y, "y", true)));
    if (drawing) {
      ctx.lineTo(cx, cy);
    } else {
      ctx.moveTo(cx, cy);
      drawing = true;
    }
  }
  ctx.strokeStyle = cssColor(colors[1]);
  ctx.lineWidth = 1.5 * devicePixelRatio;
  // Dashed, apart from the lines of data
  ctx.setLineDash([6 * devicePixelRatio, 4 * devicePixelRatio]);
  ctx.stroke();
  ctx.restore();
}

function fit() {
  plot?.setSize({
    width: container.clientWidth,
    height: Math.max(container.clientHeight, 50),
  });
}

function create() {
  plot?.destroy();
  made = JSON.stringify([labels, logs, colors]);
  // For tests
  container.dataset.xScale = logs[0] ? "log" : "linear";
  container.dataset.yScale = logs[1] ? "log" : "linear";

  const text = cssColor("--dc-text-muted");
  const grid = withAlpha(cssColor("--dc-border"), 0.35);
  const axis = (label) => ({
    label,
    stroke: text,
    grid: { stroke: grid, width: 1 },
    ticks: { stroke: grid, width: 1 },
  });
  const point = cssColor(colors[0]);

  plot = new uPlot(
    {
      mode: 2,
      width: container.clientWidth,
      height: Math.max(container.clientHeight, 50),
      legend: { show: false },
      hooks: { draw: [drawLine], setCursor: [showStatistics] },
      scales: { x: scale(logs[0]), y: scale(logs[1]) },
      axes: [axis(labels[0]), axis(labels[1])],
      series: [
        {},
        {
          label: `${labels[1]} / ${labels[0]}`,
          facets: [
            { scale: "x", auto: true },
            { scale: "y", auto: true },
          ],
          paths: drawPoints(withAlpha(point, 0.6), point),
          points: { show: false },
        },
      ],
    },
    plotData(),
    container,
  );
  // Over the points, in the plot's own area
  plot.over.appendChild(statisticsBox);
  statisticsBox.hidden = true;
}

function draw() {
  frame = 0;
  if (!plot || JSON.stringify([labels, logs, colors]) !== made) {
    create();
  } else {
    plot.setData(plotData());
  }
  // For tests
  container.dataset.points = String(plotData()[1][0].length);
  container.dataset.line = line ? line.join(" ") : "";
  showStatistics(plot);
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

  const [
    reset,
    points,
    pointLabels,
    pointLogs,
    pointColors,
    pointLine,
    pointStatistics,
  ] = message;
  labels = pointLabels;
  logs = pointLogs;
  colors = pointColors;
  line = pointLine;
  statistics = pointStatistics;
  if (reset) {
    xs = [];
    ys = [];
  }
  for (const [x, y] of points) {
    xs.push(x);
    ys.push(y);
  }
  const excess = xs.length - MAX_POINTS;
  if (excess > 0) {
    xs.splice(0, excess);
    ys.splice(0, excess);
  }
  schedule();
}

cancelAnimationFrame(frame);
resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", create);
plot?.destroy();
