// The console: an xterm.js terminal, set up as Tera Term's defaults are.
// From Rust: the container id, then `[reset, bytes, times]` (where in `bytes`
// each chunk starts, and when it came), then `null` on unmount.
// To Rust: what is typed or pasted, to send.

// Tera Term's default
const SCROLLBACK = 10000;

const id = await dioxus.recv();
const container = document.getElementById(id);
if (!container) {
  return;
}

// Their <script> tags may not have run yet
while (!window.Terminal || !window.FitAddon || !window.WebglAddon) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const probe = document.createElement("span");
probe.style.display = "none";
container.appendChild(probe);
function cssColor(name) {
  probe.style.color = `var(${name})`;
  return getComputedStyle(probe).color;
}
const theme = () => ({
  background: cssColor("--dc-screen"),
  foreground: cssColor("--dc-screen-text"),
  cursor: cssColor("--dc-screen-text"),
});

const term = new Terminal({
  scrollback: SCROLLBACK,
  // A lone LF starts a new line too, as Tera Term's "AUTO" receive
  convertEol: true,
  cursorBlink: true,
  // Named fonts: the WebGL renderer does not know `ui-monospace`
  fontFamily:
    'Menlo, Consolas, "DejaVu Sans Mono", "Liberation Mono", monospace',
  fontSize: 14,
  theme: theme(),
});
const fit = new FitAddon.FitAddon();
term.loadAddon(fit);
term.open(container);
try {
  const webgl = new WebglAddon.WebglAddon();
  webgl.onContextLoss(() => webgl.dispose());
  term.loadAddon(webgl);
} catch {
  // The DOM renderer then
}
fit.fit();
// For tests
container.xterm = term;

// Beside each line, when its first character came: marked as each chunk is
// written, by its count of lines from the first ever (the lines trimmed off the
// scrollback, then its own in the buffer). One marker on the newest line tells
// how many were trimmed: xterm.js tells each marker of each trim, too slow for
// one a line
const times = document.getElementById("console-times");
// `{ line, time }` by `line`, from the first ever
let marks = [];
// The marker on the newest line marked, and its count from the first ever
let anchor = null;

function trimmed() {
  return anchor ? anchor.count - anchor.marker.line : 0;
}

function forgetMarks() {
  anchor?.marker.dispose();
  anchor = null;
  marks = [];
}

function markLines(time) {
  const buffer = term.buffer.active;
  if (buffer.type !== "normal") {
    return;
  }
  const lost = trimmed();
  const cursorLine = buffer.baseY + buffer.cursorY;
  const first = Math.max(0, (marks.at(-1)?.line ?? -1) - lost + 1);
  for (let line = first; line <= cursorLine; line++) {
    const row = buffer.getLine(line);
    // Not the blank ones, nor those a long line wraps onto
    if (row && !row.isWrapped && row.translateToString(true) !== "") {
      marks.push({ line: lost + line, time });
    }
  }
  const marker = term.registerMarker(0);
  if (marker) {
    anchor?.marker.dispose();
    anchor = { marker, count: lost + cursorLine };
  }
  // Those trimmed off, in a while: not one by one
  if (marks.length > 2 * SCROLLBACK) {
    marks = marks.slice(marks.findIndex((mark) => mark.line >= lost));
  }
  showTimes();
}

const two = (n) => String(n).padStart(2, "0");
function localTime(ms) {
  const date = new Date(ms);
  return (
    `${two(date.getHours())}:${two(date.getMinutes())}:${two(date.getSeconds())}` +
    `.${String(date.getMilliseconds()).padStart(3, "0")}`
  );
}

let showing = false;
function showTimes() {
  if (!showing) {
    showing = true;
    requestAnimationFrame(drawTimes);
  }
}

function drawTimes() {
  showing = false;
  if (!times) {
    return;
  }
  const buffer = term.buffer.active;
  const screen = term.element?.querySelector(".xterm-screen");
  const rowHeight = screen ? screen.clientHeight / term.rows : 0;
  while (times.children.length < term.rows) {
    times.appendChild(document.createElement("div"));
  }
  while (times.children.length > term.rows) {
    times.lastChild.remove();
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
    const cell = times.children[row];
    cell.style.height = cell.style.lineHeight = `${rowHeight}px`;
    while (index < marks.length && marks[index].line < line) {
      index++;
    }
    const mark = marks[index];
    cell.textContent =
      buffer.type === "normal" && mark?.line === line
        ? localTime(mark.time)
        : "";
  }
}

term.onScroll(showTimes);
term.onRender(showTimes);
term.onResize(showTimes);

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

async function paste() {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      term.paste(text);
    }
  } catch {
    // Clipboard access refused
  }
}

term.onData((data) => dioxus.send(data));
// As Tera Term: Backspace sends BS and Delete DEL; Alt+V pastes
term.attachCustomKeyEventHandler((event) => {
  if (event.type !== "keydown" || event.ctrlKey || event.metaKey) {
    return true;
  }
  if (!event.altKey && event.key === "Backspace") {
    dioxus.send("\b");
    return false;
  }
  if (!event.altKey && event.key === "Delete") {
    dioxus.send("\x7f");
    return false;
  }
  if (event.altKey && event.key.toLowerCase() === "v") {
    paste();
    return false;
  }
  return true;
});
// As Tera Term: the selection is copied as it is made, and a right click pastes
term.onSelectionChange(() => {
  const selection = term.getSelection();
  if (selection) {
    navigator.clipboard?.writeText(selection).catch(() => {});
  }
});
const onContextMenu = (event) => {
  event.preventDefault();
  paste();
};
container.addEventListener("contextmenu", onContextMenu);

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }
  const [reset, bytes, chunkTimes] = message;
  if (reset) {
    term.reset();
    forgetMarks();
    showTimes();
  }
  const data = new Uint8Array(bytes);
  // A chunk at a time, to mark the lines each starts with its time
  chunkTimes.forEach(([at, time], index) => {
    const end = chunkTimes[index + 1]?.[0] ?? data.length;
    term.write(data.subarray(at, end), () => markLines(time));
  });
}

resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", updateTheme);
container.removeEventListener("contextmenu", onContextMenu);
term.dispose();
