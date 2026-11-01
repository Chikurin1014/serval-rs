// Draws the arrows sent from `ArrowsGraph` in space with Zdog, turned by
// dragging; axes, legend and hover made as uPlot's.
//
// From Rust: the container id, then `[arrows, hidden, trails]` on each change
// (each arrow: `[label, color, [x, y, z], trail, rows]`; trail `[[x, y, z]]`
// up to the newest, rows `[[name, text]]` its hover's; hidden the labels
// turned off in the legend; trails whether the trails show), then `null` on
// unmount. To Rust: the labels turned off in the legend.

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Its <script> tag may not have run yet
while (!window.Zdog) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const TAU = Zdog.TAU;
// As seen at first: from a little above, between X and Y
const VIEW = { x: -TAU / 14, y: TAU / 8 };
// Room around the furthest an axis reaches, for its name, in CSS pixels
const MARGIN = 28;
// How near the pointer an arrow's tip is to be hovered, in CSS pixels
const HOVER_DISTANCE = 16;

let arrows = [];
const hiddenLabels = trackHidden();
let showTrails = true;
// The arrow whose tip the pointer is near, if any
let hovered = null;
let pointer = null;
// Being turned by dragging: no hover meanwhile
let dragging = false;
// Where the tips are on the canvas, in CSS pixels, as last drawn
let tips = [];

const themed = themeColors(container);
const cssColor = themed.color;

// The plot: the canvas, the axes' labels over it, the hover; the legend below
const plotArea = document.createElement("div");
plotArea.className = "arrows-3d-area";
const canvas = document.createElement("canvas");
const labels = document.createElement("div");
labels.className = "arrows-3d-labels";
const hover = document.createElement("div");
hover.className = "arrows-hover plot-tooltip";
hover.hidden = true;
plotArea.append(canvas, labels, hover);
// Each arrow's label, turned on or off by clicking it
const legend = legendTable(themed, hiddenLabels, 0.35, () => {
  legend.update(arrows);
  // Its arrow gone or back, the axes with it
  follow();
  render();
});
container.append(plotArea, legend.element);

/** In Zdog's space (y down, z towards the viewer), Z up and right-handed. */
function toScene([x, y, z]) {
  return { x, y: -z, z: -y };
}

function shown([label]) {
  return !hiddenLabels.has(label);
}

/** The points an arrow reaches: its tip, and those of its trail if shown. */
function reached([, , tip, trail]) {
  return showTrails ? trail : [tip];
}

/** How far the axes need to reach: past the furthest point, or 1 with none. */
function needed() {
  const furthest = Math.max(
    0,
    ...arrows
      .filter(shown)
      .flatMap(reached)
      .map((point) => Math.hypot(...point)),
  );
  return furthest * 1.1 || 1;
}

// How far each axis reaches from the origin: eased towards what the data needs
const extent = easedExtent(container, () => render());

/** Takes the axes towards what the data needs. */
function follow() {
  extent.follow(needed());
}
/** A round step for ticks, about two of them each way along an axis. */
function tickStep(length) {
  const rough = length / 2;
  const magnitude = 10 ** Math.floor(Math.log10(rough));
  const step = [1, 2, 5, 10].find((each) => each * magnitude >= rough * 0.75);
  return step * magnitude;
}

function tickText(value, step) {
  const decimals = Math.max(0, -Math.floor(Math.log10(step)));
  return value.toFixed(decimals);
}

const illo = new Zdog.Illustration({
  element: canvas,
  dragRotate: true,
  rotate: VIEW,
  onDragStart() {
    dragging = true;
    showHover();
  },
  onDragMove() {
    render();
  },
  onDragEnd() {
    dragging = false;
    render();
  },
});
let scene = new Zdog.Anchor({ addTo: illo });

// The plot's size in CSS pixels, the legend apart: measured as it is fitted,
// not as it is drawn (each measuring makes the page lay itself out)
let size = [0, 0];
function area() {
  return size;
}

function fit() {
  size = [
    container.clientWidth,
    Math.max(container.clientHeight - legend.element.offsetHeight, 50),
  ];
  const [width, height] = size;
  plotArea.style.width = `${width}px`;
  plotArea.style.height = `${height}px`;
  illo.setSize(width, height);
  render();
}

/** Makes the scene anew, as the arrows, their lengths and the size have it. */
function build() {
  scene.remove();
  scene = new Zdog.Anchor({ addTo: illo });
  const [width, height] = area();
  const length = extent.value ?? needed();
  // Pixels a unit, so the axes reach within the plot whichever way it turns
  illo.zoom = Math.max(Math.min(width, height) / 2 - MARGIN, 10) / length;
  // A width in CSS pixels, in the scene's units
  const pixels = (count) => count / illo.zoom;

  const muted = cssColor("--dc-text-muted");
  const axisColor = withAlpha(muted, 0.6);
  const ends = [];
  const ticks = [];
  const step = tickStep(length);
  for (
    const [name, unit] of [
      ["X", [1, 0, 0]],
      ["Y", [0, 1, 0]],
      ["Z", [0, 0, 1]],
    ]
  ) {
    const along = (value) => toScene(unit.map((each) => each * value));
    new Zdog.Shape({
      addTo: scene,
      path: [along(-length), along(length)],
      stroke: pixels(1),
      color: axisColor,
    });
    ends.push([
      name,
      new Zdog.Anchor({ addTo: scene, translate: along(length * 1.08) }),
    ]);
    for (
      let value = -Math.floor(length / step) * step;
      value <= length;
      value += step
    ) {
      if (Math.abs(value) < step / 2) {
        continue;
      }
      new Zdog.Shape({
        addTo: scene,
        translate: along(value),
        stroke: pixels(3),
        color: axisColor,
      });
      ticks.push([
        tickText(value, step),
        new Zdog.Anchor({ addTo: scene, translate: along(value) }),
      ]);
    }
  }

  const anchors = [];
  arrows.forEach((arrow, index) => {
    const [, color, tip, trail] = arrow;
    if (!shown(arrow)) {
      return;
    }
    const stroke = cssColor(color);
    if (showTrails && trail.length > 1) {
      // The trail of its tip, fainter
      new Zdog.Shape({
        addTo: scene,
        path: trail.map(toScene),
        closed: false,
        stroke: pixels(1),
        color: withAlpha(stroke, 0.35),
      });
    }
    // The arrow, from the origin to the newest
    const end = toScene(tip);
    const size = Math.hypot(end.x, end.y, end.z);
    anchors.push([index, new Zdog.Anchor({ addTo: scene, translate: end })]);
    const head = Math.min(pixels(10), size / 2);
    if (size < pixels(1)) {
      // Of no length: a dot at the origin
      new Zdog.Shape({ addTo: scene, stroke: pixels(5), color: stroke });
      return;
    }
    const [dx, dy, dz] = [end.x / size, end.y / size, end.z / size];
    const shaft = size - head;
    new Zdog.Shape({
      addTo: scene,
      path: [{}, { x: dx * shaft, y: dy * shaft, z: dz * shaft }],
      stroke: pixels(index === hovered ? 3 : 2),
      color: stroke,
    });
    // Its head, a cone turned from Z to the arrow's way
    new Zdog.Cone({
      addTo: scene,
      translate: { x: dx * shaft, y: dy * shaft, z: dz * shaft },
      rotate: { x: Math.atan2(-dy, dz), y: -Math.asin(dx) },
      diameter: pixels(8),
      length: head,
      stroke: false,
      color: stroke,
    });
  });
  return { ends, ticks, anchors };
}

/** Where an anchor is drawn, in CSS pixels from the canvas's corner. */
function onCanvas(anchor) {
  const [width, height] = area();
  return [
    width / 2 + anchor.renderOrigin.x * illo.zoom,
    height / 2 + anchor.renderOrigin.y * illo.zoom,
  ];
}

function render() {
  const { ends, ticks, anchors } = build();
  illo.updateRenderGraph();

  // The axes' names at their ends, their ticks' values beside them
  const muted = cssColor("--dc-text-muted");
  labels.replaceChildren();
  const place = (text, [x, y], className) => {
    const label = document.createElement("span");
    label.className = className;
    label.textContent = text;
    label.style.left = `${x}px`;
    label.style.top = `${y}px`;
    label.style.color = muted;
    labels.append(label);
  };
  for (const [name, anchor] of ends) {
    place(name, onCanvas(anchor), "arrows-3d-axis");
  }
  for (const [text, anchor] of ticks) {
    place(text, onCanvas(anchor), "arrows-3d-tick");
  }

  tips = anchors.map(([index, anchor]) => [index, ...onCanvas(anchor)]);
  showHover();

  // For tests: each arrow shown, its label and tip; the origin
  container.dataset.trails = String(showTrails);
  container.dataset.arrows = JSON.stringify(
    tips.map(([index, x, y]) => [
      arrows[index][0],
      Math.round(x),
      Math.round(y),
    ]),
  );
  const [width, height] = area();
  container.dataset.origin = JSON.stringify([
    Math.round(width / 2 + illo.translate.x * illo.zoom),
    Math.round(height / 2 + illo.translate.y * illo.zoom),
  ]);
}

function showHover() {
  let nearest = null;
  if (pointer && !dragging) {
    let distance = HOVER_DISTANCE;
    for (const [index, x, y] of tips) {
      const away = Math.hypot(x - pointer[0], y - pointer[1]);
      if (away <= distance) {
        distance = away;
        nearest = index;
      }
    }
  }
  if (nearest !== hovered) {
    hovered = nearest;
    // Its arrow drawn bolder
    requestAnimationFrame(render);
  }
  hover.hidden = nearest === null;
  if (nearest === null) {
    return;
  }
  const [label, , , , rows] = arrows[nearest];
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
  const [, tipX, tipY] = tips.find(([index]) => index === nearest);
  const [width, height] = area();
  const x = tipX + gap + hover.offsetWidth <= width
    ? tipX + gap
    : tipX - gap - hover.offsetWidth;
  const y = tipY + gap + hover.offsetHeight <= height
    ? tipY + gap
    : tipY - gap - hover.offsetHeight;
  hover.style.left = `${Math.max(0, x)}px`;
  hover.style.top = `${Math.max(0, y)}px`;
}

function onPointerMove(event) {
  const box = canvas.getBoundingClientRect();
  pointer = [event.clientX - box.left, event.clientY - box.top];
  showHover();
}

function onPointerLeave() {
  pointer = null;
  showHover();
}

canvas.addEventListener("pointermove", onPointerMove);
canvas.addEventListener("pointerleave", onPointerLeave);

function draw() {
  if (legend.update(arrows)) {
    // Its height may have changed
    fit();
  }
  follow();
  render();
}

// One redraw per frame, fewer as the page is busy
const draws = pacedDraws(draw);
function schedule() {
  draws.schedule();
}

const resize = new ResizeObserver(fit);
resize.observe(container);

// Colors are taken at each drawing; and the legend's anew
function onTheme() {
  themed.clear();
  legend.invalidate();
  schedule();
}
const stopWatchingTheme = watchTheme(onTheme);

fit();

for await (const message of messages()) {
  const [nextArrows, hidden, trails] = message;
  arrows = nextArrows;
  showTrails = trails;
  hiddenLabels.apply(hidden);
  schedule();
}

draws.cancel();
extent.cancel();
resize.disconnect();
stopWatchingTheme();
canvas.removeEventListener("pointermove", onPointerMove);
canvas.removeEventListener("pointerleave", onPointerLeave);
