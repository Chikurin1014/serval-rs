// Plots the points sent from `ScatterGraph` with uPlot.
//
// From Rust: the container id, then `[reset, dropped, points, [xLabel, yLabel],
// [xLog, yLog], [pointColor, lineColor], line, statistics]` on each change
// (points: `[[x, y]]`, after those before unless `reset`, then `dropped` of
// the oldest left out, so it has no more than Rust keeps; line: `[slope,
// intercept]` or `null`; statistics: `[covariance, correlation]` as text or
// `null`), then `null` on unmount. To Rust: `["zoomed", []]` as it is zoomed
// into.

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
// Redrawn at most this often, not as each pair comes: each redraw is of all
// its points, as many as 10000
const DRAW_EVERY_MS = 100;

const themed = themeColors(container);
const cssColor = themed.color;

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

// The ranges zoomed to by dragging across the plot, if it is: kept until a
// double click, played again, or another scale
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
  u.batch(() => {
    u.setScale("x", { min: zoomed.x[0], max: zoomed.x[1] });
    u.setScale("y", { min: zoomed.y[0], max: zoomed.y[1] });
  });
  // For tests
  container.dataset.zoomed = "true";
  // Paused, so the data stays as zoomed into
  dioxus.send(["zoomed", []]);
}

function unzoom() {
  if (!zoomed) {
    return;
  }
  zoomed = null;
  container.dataset.zoomed = "false";
  plot?.setData(plotData());
}

// Padded, so one point (or all of one value) has room around it: as no
// points do; as zoomed to, if it is
function scale(log, key) {
  return {
    time: false,
    ...(log && { distr: 3, log: 10 }),
    range: (u, min, max) => {
      if (zoomed) {
        return zoomed[key];
      }
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
      // Zoomed by `onSelect`, so it is told
      cursor: { drag: { x: true, y: true, setScale: false } },
      hooks: {
        draw: [drawLine],
        setCursor: [showStatistics],
        setSelect: [onSelect],
      },
      scales: { x: scale(logs[0], "x"), y: scale(logs[1], "y") },
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
  // Unzoomed by a double click, before uPlot takes it
  plot.root.addEventListener("dblclick", unzoom, true);
  // Over the points, in the plot's own area
  plot.over.appendChild(statisticsBox);
  statisticsBox.hidden = true;
}

function draw() {
  if (!plot || JSON.stringify([labels, logs, colors]) !== made) {
    create();
  } else {
    plot.setData(plotData());
  }
  // For tests: as uPlot has them, not worked out again
  container.dataset.points = String(plot.data[1][0].length);
  container.dataset.draws = String(Number(container.dataset.draws ?? 0) + 1);
  container.dataset.line = line ? line.join(" ") : "";
  showStatistics(plot);
}

// One redraw per frame, and at most each `DRAW_EVERY_MS`: fewer as the page is
// busy
const draws = pacedDraws(draw, DRAW_EVERY_MS);
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
    reset,
    dropped,
    points,
    pointLabels,
    pointLogs,
    pointColors,
    pointLine,
    pointStatistics,
    paused,
  ] = message;
  // Played again, or another scale: as the data has it
  if (!paused || JSON.stringify(pointLogs) !== JSON.stringify(logs)) {
    zoomed = null;
    container.dataset.zoomed = "false";
  }
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
  // Those that left its time range, or past as many as Rust keeps
  if (dropped > 0) {
    xs.splice(0, dropped);
    ys.splice(0, dropped);
  }
  schedule();
}

draws.cancel();
resize.disconnect();
stopWatchingTheme();
plot?.destroy();
