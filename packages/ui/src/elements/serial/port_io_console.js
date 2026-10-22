// The console: an xterm.js terminal, set up as Tera Term's defaults are.
// From Rust: the container id, then `["view", "text" | "hex"]` and
// `["data", reset, bytes, times]` (where in `bytes` each chunk starts, and when
// it came, as the app shows times), then `null` on unmount.
// To Rust: `["typed", text]`, what is typed or pasted, to send; `["redraw"]`
// for all the bytes again, to draw them anew; `["break"]` to send a break;
// `["maximize"]` to maximize the console, or back.

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

const colors = themeColors(container);
const theme = () => ({
  background: colors.color("--dc-screen"),
  foreground: colors.color("--dc-screen-text"),
  cursor: colors.color("--dc-screen-text"),
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
  // Alt is for a block selection (its drag), not to move the cursor by sending
  // arrow keys
  altClickMovesCursor: false,
});
const fit = new FitAddon.FitAddon();
term.loadAddon(fit);
term.open(container);
// WebGL only on a GPU: drawn in software (no GPU, as in VMs), it is slower
// than xterm.js's own DOM renderer, much so for lines full of characters
function hardwareWebgl() {
  const gl = document.createElement("canvas").getContext("webgl2");
  if (!gl) {
    return false;
  }
  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const renderer = info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : "";
  gl.getExtension("WEBGL_lose_context")?.loseContext();
  return !/swiftshader|llvmpipe|softpipe|software|basic render/i.test(renderer);
}
// For tests
container.dataset.renderer = "dom";
if (hardwareWebgl()) {
  try {
    const webgl = new WebglAddon.WebglAddon();
    webgl.onContextLoss(() => {
      webgl.dispose();
      container.dataset.renderer = "dom";
    });
    term.loadAddon(webgl);
    container.dataset.renderer = "webgl";
  } catch {
    // The DOM renderer then
  }
}
fit.fit();
// For tests
container.xterm = term;

// Beside each line, when its first character came: marked as each chunk is
// written (`helper/script/line_column.js`)
const times = lineColumn(
  term,
  document.getElementById("console-times"),
  SCROLLBACK,
);

// `time`: when the chunk came, as the app shows times (`TimeContext`)
function markLines(time) {
  const buffer = term.buffer.active;
  if (buffer.type !== "normal") {
    return;
  }
  const lost = times.trimmed();
  const cursorLine = buffer.baseY + buffer.cursorY;
  const first = Math.max(0, times.lastMarked() - lost + 1);
  for (let line = first; line <= cursorLine; line++) {
    const row = buffer.getLine(line);
    // Not the blank ones, nor those a long line wraps onto
    if (row && !row.isWrapped && row.translateToString(true) !== "") {
      times.mark(lost + line, time);
    }
  }
  times.settle();
}

const resize = new ResizeObserver(() => {
  fit.fit();
  // Another width may take another count of bytes a hex line
  if (view === "hex" && bytesPerLine() !== hex.perLine) {
    dioxus.send(["redraw"]);
  }
});
resize.observe(container);
const stopWatchingTheme = watchTheme(() => {
  colors.clear();
  term.options.theme = theme();
});

async function clipboardText() {
  try {
    return await navigator.clipboard.readText();
  } catch {
    // Clipboard access refused
    return "";
  }
}

async function paste() {
  const text = await clipboardText();
  if (text) {
    term.paste(text);
  }
}

// How the bytes show: as a terminal does, or in hex
let view = "text";

// In hex, as `hexdump -C` without offsets: as many bytes a line as fit, the
// last line written again as more come; each with when its first byte came
let hex = { perLine: 16, bytes: [] };

function hexWidth(count) {
  // "XX " each, a gap between halves of 16, then "  |" and "|" round them
  return count * 3 - 1 + (count > 8 ? 1 : 0) + 3 + count + 1;
}

function bytesPerLine() {
  return [16, 8, 4].find((count) => hexWidth(count) <= term.cols) ?? 4;
}

function hexLine(bytes, perLine) {
  const cells = [];
  for (let index = 0; index < perLine; index++) {
    const byte = bytes[index];
    cells.push(
      byte === undefined
        ? "  "
        : byte.toString(16).toUpperCase().padStart(2, "0"),
    );
    if (perLine > 8 && index === 7) {
      cells.push("");
    }
  }
  const text = bytes
    .map((byte) =>
      byte >= 0x20 && byte < 0x7f ? String.fromCharCode(byte) : ".",
    )
    .join("");
  return `${cells.join(" ")}  |${text}|`;
}

function writeHex(chunk, time) {
  let out = "";
  for (const byte of chunk) {
    hex.bytes.push(byte);
    if (hex.bytes.length === hex.perLine) {
      out += `\r\x1b[K${hexLine(hex.bytes, hex.perLine)}\r\n`;
      hex.bytes = [];
    }
  }
  if (hex.bytes.length > 0) {
    out += `\r\x1b[K${hexLine(hex.bytes, hex.perLine)}`;
  }
  // A line begun in a chunk before was marked as it was first written: those
  // unmarked yet begin in this one
  term.write(out, () => markLines(time));
}

const send = (data) => dioxus.send(["typed", data]);
term.onData(send);
// As Tera Term: Backspace sends BS and Delete DEL; Alt+V pastes, Alt+R pastes
// with a CR after, Alt+B sends a break and Alt+Enter maximizes the console (or
// back). Alt+key by where it is, as the letter Alt types differs on a Mac.
const altActions = {
  KeyV: paste,
  KeyR: async () => {
    const text = await clipboardText();
    if (text) {
      send(`${text.replace(/\r?\n/g, "\r")}\r`);
    }
  },
  KeyB: () => dioxus.send(["break"]),
  Enter: () => dioxus.send(["maximize"]),
  NumpadEnter: () => dioxus.send(["maximize"]),
};
term.attachCustomKeyEventHandler((event) => {
  if (event.type !== "keydown" || event.ctrlKey || event.metaKey) {
    return true;
  }
  if (!event.altKey && event.key === "Backspace") {
    send("\b");
    return false;
  }
  if (!event.altKey && event.key === "Delete") {
    send("\x7f");
    return false;
  }
  const action = event.altKey && altActions[event.code];
  if (action) {
    // Not the browser's, if it has one; once a press, not as the key repeats
    event.preventDefault();
    if (!event.repeat) {
      action();
    }
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
  if (message[0] === "view") {
    // All the bytes follow, to draw anew
    view = message[1];
    continue;
  }
  const [, reset, bytes, chunkTimes] = message;
  if (reset) {
    term.reset();
    times.forget();
    hex = { perLine: bytesPerLine(), bytes: [] };
  }
  const data = new Uint8Array(bytes);
  // A chunk at a time, to mark the lines each starts with its time
  chunkTimes.forEach(([at, time], index) => {
    const chunk = data.subarray(at, chunkTimes[index + 1]?.[0] ?? data.length);
    if (view === "hex") {
      writeHex(chunk, time);
    } else {
      term.write(chunk, () => markLines(time));
    }
  });
}

resize.disconnect();
stopWatchingTheme();
container.removeEventListener("contextmenu", onContextMenu);
term.dispose();
