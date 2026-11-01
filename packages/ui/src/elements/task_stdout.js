// The tasks' standard output: an xterm.js terminal, not typed into, each line
// with its task and time in a column beside it.
// From Rust: the container id, then `["lines", cleared, [[shown, text], ...]]`
// (`shown` beside `text`: its task and time; all shown dropped first if
// `cleared`), then `null` on unmount.

// As the console's
const SCROLLBACK = 10000;

const id = await dioxus.recv();
// Its element may not be there yet, nor the scripts of their <script> tags
let container = null;
while (
  !(container = document.getElementById(id))
  || !window.Terminal
  || !window.FitAddon
) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}
const card = container.closest(".task-stdout") ?? container;

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
    "Menlo, Consolas, \"DejaVu Sans Mono\", \"Liberation Mono\", monospace",
  fontSize: 14,
  theme: theme(),
});
const fit = new FitAddon.FitAddon();
term.loadAddon(fit);
term.open(container);
fit.fit();
// For tests
container.xterm = term;

// Beside each line, its task and time (`helper/script/line_column.js`)
const column = document.getElementById("task-stdout-sources");
const sources = lineColumn(term, column, SCROLLBACK);
// Where the next line starts, from the first ever
let next = 0;
// The widest source shown, in px: the column as wide, not to refit the
// terminal as it scrolls
let widest = 0;
const measure = document.createElement("canvas").getContext("2d");

// After a line is written: it started at `next`, and the next one where the
// cursor is
function written(source) {
  sources.mark(next, source);
  sources.settle();
  next = sources.cursorLine();
  if (column && measure) {
    measure.font = getComputedStyle(column).font;
    const width = Math.ceil(measure.measureText(source).width);
    if (width > widest) {
      widest = width;
      column.style.minWidth = `${widest}px`;
    }
  }
}

const resize = new ResizeObserver(() => fit.fit());
resize.observe(container);
const stopWatchingTheme = watchTheme(() => {
  term.options.theme = theme();
});

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }
  const [, cleared, lines] = message;
  if (cleared) {
    term.reset();
    sources.forget();
    next = 0;
    widest = 0;
    if (column) {
      column.style.minWidth = "";
    }
  }
  for (const [shown, text] of lines) {
    term.write(`${text}\r\n`, () => written(shown));
  }
}

resize.disconnect();
stopWatchingTheme();
term.dispose();
