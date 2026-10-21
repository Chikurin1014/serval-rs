// The tasks' standard output: an xterm.js terminal, not typed into, each line
// with its task and time in a column beside it.
// From Rust: the container id, then `["lines", cleared, [[source, time, text], ...]]`
// (all shown dropped first if `cleared`), then `null` on unmount.

// As the console's
const SCROLLBACK = 10000;

const id = await dioxus.recv();
// Its element may not be there yet, nor the scripts of their <script> tags
let container = null;
while (
  !(container = document.getElementById(id)) ||
  !window.Terminal ||
  !window.FitAddon
) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}
const card = container.closest(".task-stdout") ?? container;
const sources = document.getElementById("task-stdout-sources");

// In the card's colors, its cursor not seen
function theme() {
  const style = getComputedStyle(card);
  return {
    background: style.backgroundColor,
    foreground: style.color,
    cursor: style.backgroundColor,
    cursorAccent: style.backgroundColor,
  };
}

const term = new Terminal({
  scrollback: SCROLLBACK,
  disableStdin: true,
  cursorInactiveStyle: "none",
  fontFamily:
    'Menlo, Consolas, "DejaVu Sans Mono", "Liberation Mono", monospace',
  fontSize: 14,
  theme: theme(),
});
const fit = new FitAddon.FitAddon();
term.loadAddon(fit);
term.open(container);
fit.fit();
// For tests
container.xterm = term;

// Beside each line, its task and time: by its count of lines from the first
// (those trimmed off the scrollback, then its own in the buffer), as the
// console marks its times
let marks = [];
// The marker where the next line starts, and its count from the first
let anchor = null;
// Where the next line starts, from the first
let next = 0;
// The widest source shown, in px: the column as wide, not to refit the
// terminal as it scrolls
let widest = 0;
const measure = document.createElement("canvas").getContext("2d");

function trimmed() {
  return anchor ? anchor.count - anchor.marker.line : 0;
}

const two = (n) => String(n).padStart(2, "0");
function localTime(ms) {
  const date = new Date(ms);
  return (
    `${two(date.getHours())}:${two(date.getMinutes())}:${two(date.getSeconds())}` +
    `.${String(date.getMilliseconds()).padStart(3, "0")}`
  );
}

// After `line` is written: it started at `next`, and the next one where the
// cursor is
function written(source) {
  marks.push({ line: next, source });
  const buffer = term.buffer.active;
  const cursorLine = buffer.baseY + buffer.cursorY;
  next = trimmed() + cursorLine;
  const marker = term.registerMarker(0);
  if (marker) {
    anchor?.marker.dispose();
    anchor = { marker, count: next };
  }
  if (marks.length > 2 * SCROLLBACK) {
    const lost = trimmed();
    marks = marks.slice(marks.findIndex((mark) => mark.line >= lost));
  }
  if (sources && measure) {
    measure.font = getComputedStyle(sources).font;
    const width = Math.ceil(measure.measureText(source).width);
    if (width > widest) {
      widest = width;
      sources.style.minWidth = `${widest}px`;
    }
  }
  showSources();
}

let showing = false;
function showSources() {
  if (!showing) {
    showing = true;
    requestAnimationFrame(drawSources);
  }
}

function drawSources() {
  showing = false;
  if (!sources) {
    return;
  }
  const buffer = term.buffer.active;
  const screen = term.element?.querySelector(".xterm-screen");
  const rowHeight = screen ? screen.clientHeight / term.rows : 0;
  while (sources.children.length < term.rows) {
    sources.appendChild(document.createElement("div"));
  }
  while (sources.children.length > term.rows) {
    sources.lastChild.remove();
  }
  // From the first mark in view: they are in line order
  const top = trimmed() + buffer.viewportY;
  let index = 0;
  let high = marks.length;
  while (index < high) {
    const middle = (index + high) >> 1;
    if (marks[middle].line < top) {
      index = middle + 1;
    } else {
      high = middle;
    }
  }
  for (let row = 0; row < term.rows; row++) {
    const line = top + row;
    const cell = sources.children[row];
    cell.style.height = cell.style.lineHeight = `${rowHeight}px`;
    while (index < marks.length && marks[index].line < line) {
      index++;
    }
    const mark = marks[index];
    cell.textContent = mark?.line === line ? mark.source : "";
  }
}

term.onScroll(showSources);
term.onRender(showSources);
term.onResize(showSources);

const resize = new ResizeObserver(() => fit.fit());
resize.observe(container);
const updateTheme = () => {
  term.options.theme = theme();
};
const themeObserver = new MutationObserver(updateTheme);
themeObserver.observe(document.documentElement, {
  attributes: true,
  attributeFilter: ["data-theme"],
});
const systemTheme = matchMedia("(prefers-color-scheme: dark)");
systemTheme.addEventListener("change", updateTheme);

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }
  const [, cleared, lines] = message;
  if (cleared) {
    term.reset();
    anchor?.marker.dispose();
    anchor = null;
    marks = [];
    next = 0;
    widest = 0;
    if (sources) {
      sources.style.minWidth = "";
    }
    showSources();
  }
  for (const [source, time, text] of lines) {
    const shown = `${source} (${localTime(time)})`;
    term.write(`${text}\r\n`, () => written(shown));
  }
}

resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", updateTheme);
term.dispose();
