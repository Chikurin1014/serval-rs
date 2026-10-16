// Draws the arrows sent from `ArrowsGraph` with uPlot.
//
// From Rust: the container id, then `[arrows, hidden, trails]` on each change
// (each arrow: `[label, color, [x, y], trail, rows]`; trail `[[x, y]]` up to
// the newest, rows `[[name, text]]` its hover's; hidden the labels turned off
// in the legend; trails whether the trails show), then `null` on unmount. To
// Rust: the labels turned off in the legend.

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Its <script> tag may not have run yet
while (!window.uPlot) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

// How near the pointer an arrow's tip is to be hovered, in CSS pixels
const HOVER_DISTANCE = 16;

let arrows = [];
let hiddenLabels = new Set();
// Sent to Rust and not yet echoed back
let hiddenSent = null;
let showTrails = true;
let plot = null;
// The labels and colors the plot was made with
let made = "";
// The arrow whose tip the pointer is near, if any
let hovered = null;

const themed = themeColors(container);
const cssColor = themed.color;

// The arrow the pointer is near: its label and values
const hover = document.createElement("div");
hover.className = "arrows-hover plot-tooltip";
hover.hidden = true;

function shown([label]) {
  return !hiddenLabels.has(label);
}

/** The points an arrow reaches: its tip, and those of its trail if shown. */
function reached([, , tip, trail]) {
  return showTrails ? trail : [tip];
}

/** uPlot's data: a series of each arrow's points, to the legend. */
function plotData() {
  return [
    null,
    ...arrows.map((arrow) => {
      const points = reached(arrow);
      return [points.map(([x]) => x), points.map(([, y]) => y)];
    }),
  ];
}

// How quickly the range follows what the data needs, widening and narrowing:
// time constants, so it is most of the way there in about twice each
const WIDEN_MS = 75;
const NARROW_MS = 300;
// How much wider than needed the range may be before it narrows, so it does
// not pulse with each new value
const NARROW_PAST = 1.25;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

// The units a pixel shows, across as up, so the arrows point as they are; and
// those it is on its way to
let perPixel = null;
let goal = null;
let easing = 0;
let easedAt = 0;

/** The plot's area in pixels: once laid out; the container's, before. */
function area(u) {
  return [
    u.bbox.width || container.clientWidth,
    u.bbox.height || container.clientHeight,
  ];
}

/**
 * The units a pixel for all the arrows within, padded, the origin in the
 * middle; none before the plot has a size.
 */
function needed(u) {
  const points = arrows.filter(shown).flatMap(reached);
  // As far as the furthest from the origin each way; 1 if all are at it
  const reach = (along) =>
    Math.max(0, ...points.map((point) => Math.abs(along(point))));
  let [x, y] = [reach(([px]) => px), reach(([, py]) => py)];
  [x, y] = x || y ? [x * 1.1, y * 1.1] : [1, 1];
  const [width, height] = area(u);
  return width > 0 && height > 0
    ? Math.max((2 * x) / width, (2 * y) / height)
    : null;
}

/** The ranges of both axes: the origin in the middle, as `perPixel` says. */
function ranges(u) {
  const units = perPixel ?? needed(u);
  const [width, height] = area(u);
  if (units === null) {
    return { x: [-1, 1], y: [-1, 1] };
  }
  const [x, y] = [(units * width) / 2, (units * height) / 2];
  return { x: [-x, x], y: [-y, y] };
}

/**
 * Takes the range towards what the data needs: at once at first, as the plot
 * is resized (`now`), or for less motion; else over a few frames, and only
 * once it is much too wide to narrow.
 */
function follow(u, now = false) {
  const units = needed(u);
  if (units === null) {
    return;
  }
  if (perPixel === null || now || reducedMotion.matches) {
    cancelAnimationFrame(easing);
    easing = 0;
    perPixel = goal = units;
  } else if (
    units > perPixel ||
    units > goal ||
    units * NARROW_PAST < perPixel
  ) {
    goal = units;
    if (!easing) {
      easedAt = performance.now();
      easing = requestAnimationFrame(ease);
    }
  }
  // For tests: whether it is where it is going
  container.dataset.settled = String(perPixel === goal);
}

function ease(now) {
  easing = 0;
  if (!plot) {
    return;
  }
  const time = goal > perPixel ? WIDEN_MS : NARROW_MS;
  perPixel += (goal - perPixel) * (1 - Math.exp(-(now - easedAt) / time));
  easedAt = now;
  if (Math.abs(goal - perPixel) <= goal * 0.002) {
    perPixel = goal;
  } else {
    easing = requestAnimationFrame(ease);
  }
  container.dataset.settled = String(perPixel === goal);
  // Its ranges anew
  plot.setData(plotData());
}

function drawArrows(u) {
  const { left, top, width, height } = u.bbox;
  const ctx = u.ctx;
  const at = (x, y) => [u.valToPos(x, "x", true), u.valToPos(y, "y", true)];
  const line = devicePixelRatio;
  ctx.save();
  ctx.beginPath();
  ctx.rect(left, top, width, height);
  ctx.clip();

  // The axes through the origin
  const [ox, oy] = at(0, 0);
  ctx.strokeStyle = withAlpha(cssColor("--dc-text-muted"), 0.6);
  ctx.lineWidth = line;
  ctx.beginPath();
  ctx.moveTo(left, oy);
  ctx.lineTo(left + width, oy);
  ctx.moveTo(ox, top);
  ctx.lineTo(ox, top + height);
  ctx.stroke();

  arrows.forEach((arrow, index) => {
    const [, color, [x, y], trail] = arrow;
    if (!shown(arrow)) {
      return;
    }
    const stroke = cssColor(color);
    if (showTrails) {
      // The trail of its tip, fainter
      ctx.strokeStyle = withAlpha(stroke, 0.35);
      ctx.lineWidth = line;
      ctx.beginPath();
      trail.forEach(([tx, ty], step) => {
        const [px, py] = at(tx, ty);
        if (step === 0) {
          ctx.moveTo(px, py);
        } else {
          ctx.lineTo(px, py);
        }
      });
      ctx.stroke();
    }

    // The arrow, from the origin to the newest
    const [tipX, tipY] = at(x, y);
    const length = Math.hypot(tipX - ox, tipY - oy);
    ctx.strokeStyle = stroke;
    ctx.fillStyle = stroke;
    ctx.lineWidth = (index === hovered ? 3 : 2) * line;
    if (length < line) {
      // Of no length: a dot at the origin
      ctx.beginPath();
      ctx.arc(ox, oy, 2.5 * line, 0, 2 * Math.PI);
      ctx.fill();
      return;
    }
    const angle = Math.atan2(tipY - oy, tipX - ox);
    const head = Math.min(10 * line, length / 2);
    // The shaft up to the head, so its end stays sharp
    const shaft = length - head * Math.cos(Math.PI / 7);
    ctx.beginPath();
    ctx.moveTo(ox, oy);
    ctx.lineTo(ox + shaft * Math.cos(angle), oy + shaft * Math.sin(angle));
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(tipX, tipY);
    for (const side of [-1, 1]) {
      const wing = angle + Math.PI + (side * Math.PI) / 7;
      ctx.lineTo(tipX + head * Math.cos(wing), tipY + head * Math.sin(wing));
    }
    ctx.closePath();
    ctx.fill();
  });
  ctx.restore();

  // For tests: each arrow shown, its label and tip, and the units a pixel
  // across and up
  container.dataset.trails = String(showTrails);
  container.dataset.arrows = JSON.stringify(
    arrows
      .filter(shown)
      .map(([label, , [x, y]]) => [
        label,
        Math.round(u.valToPos(x, "x")),
        Math.round(u.valToPos(y, "y")),
      ]),
  );
  container.dataset.origin = JSON.stringify([
    Math.round(u.valToPos(0, "x")),
    Math.round(u.valToPos(0, "y")),
  ]);
  const { x, y } = u.scales;
  container.dataset.unitsPerPixel = JSON.stringify([
    (x.max - x.min) / (width / devicePixelRatio),
    (y.max - y.min) / (height / devicePixelRatio),
  ]);
}

function showHover(u) {
  const { left, top } = u.cursor;
  let nearest = null;
  if (left != null && left >= 0) {
    let distance = HOVER_DISTANCE;
    arrows.forEach((arrow, index) => {
      const [, , [x, y]] = arrow;
      if (!shown(arrow)) {
        return;
      }
      const away = Math.hypot(
        u.valToPos(x, "x") - left,
        u.valToPos(y, "y") - top,
      );
      if (away <= distance) {
        distance = away;
        nearest = index;
      }
    });
  }
  if (nearest !== hovered) {
    hovered = nearest;
    // Its arrow drawn bolder
    u.redraw(false);
  }
  hover.hidden = nearest === null;
  if (nearest === null) {
    return;
  }
  const [label, , [x, y], , rows] = arrows[nearest];
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

  // Below right of the tip, or on the side with room
  const gap = 12;
  const [tipX, tipY] = [u.valToPos(x, "x"), u.valToPos(y, "y")];
  const { clientWidth: width, clientHeight: height } = u.over;
  const hoverX =
    tipX + gap + hover.offsetWidth <= width
      ? tipX + gap
      : tipX - gap - hover.offsetWidth;
  const hoverY =
    tipY + gap + hover.offsetHeight <= height
      ? tipY + gap
      : tipY - gap - hover.offsetHeight;
  hover.style.left = `${Math.max(0, hoverX)}px`;
  hover.style.top = `${Math.max(0, hoverY)}px`;
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
  dioxus.send(["hidden", hidden]);
  // Its arrow gone or back, the ranges with it
  follow(u);
  u.setData(plotData());
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

/** What the plot is made with: its arrows' labels and colors. */
function makings() {
  return JSON.stringify(arrows.map(([label, color]) => [label, color]));
}

function fit() {
  if (!plot) {
    return;
  }
  const legend = plot.root.querySelector(".u-legend");
  const height = container.clientHeight - (legend ? legend.offsetHeight : 0);
  plot.setSize({ width: container.clientWidth, height: Math.max(height, 50) });
  // Its ranges anew, for the new shape
  follow(plot, true);
  plot.setData(plotData());
}

function create() {
  plot?.destroy();
  plot = null;
  made = makings();
  // None before there is an arrow: uPlot takes a series at least
  if (!arrows.length) {
    return;
  }
  const text = cssColor("--dc-text-muted");
  const grid = withAlpha(cssColor("--dc-border"), 0.35);
  const axis = {
    stroke: text,
    grid: { stroke: grid, width: 1 },
    ticks: { stroke: grid, width: 1 },
  };

  plot = new uPlot(
    {
      mode: 2,
      width: container.clientWidth,
      height: Math.max(container.clientHeight, 50),
      // Each arrow's label, to turn it on or off
      legend: { live: false },
      cursor: { drag: { x: false, y: false } },
      scales: {
        x: { time: false, range: (u) => ranges(u).x },
        y: { range: (u) => ranges(u).y },
      },
      axes: [axis, axis],
      hooks: {
        draw: [drawArrows],
        setCursor: [showHover],
        setSeries: [onSeriesToggle],
      },
      series: [
        {},
        ...arrows.map(([label, color]) => {
          const stroke = cssColor(color);
          return {
            label,
            show: !hiddenLabels.has(label),
            stroke,
            fill: withAlpha(stroke, 0.35),
            facets: [
              { scale: "x", auto: true },
              { scale: "y", auto: true },
            ],
            // Drawn by `drawArrows`
            paths: () => null,
            points: { show: false },
          };
        }),
      ],
    },
    plotData(),
    container,
  );
  plot.over.appendChild(hover);
  hover.hidden = true;
  fit();
  // The legend only gets its height after layout
  requestAnimationFrame(fit);
}

function draw() {
  if (makings() !== made) {
    create();
  } else if (plot) {
    follow(plot);
    plot.setData(plotData());
  }
  if (plot && hovered !== null) {
    showHover(plot);
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
  const [nextArrows, hidden, trails] = message;
  arrows = nextArrows;
  showTrails = trails;
  applyHidden(hidden);
  schedule();
}

draws.cancel();
cancelAnimationFrame(easing);
resize.disconnect();
stopWatchingTheme();
plot?.destroy();
