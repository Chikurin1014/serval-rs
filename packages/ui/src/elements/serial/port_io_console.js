// The console: an xterm.js terminal, set up as Tera Term's defaults are.
// From Rust: the container id, then `[reset, bytes]`, then `null` on unmount.
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
  const [reset, bytes] = message;
  if (reset) {
    term.reset();
  }
  if (bytes.length > 0) {
    term.write(new Uint8Array(bytes));
  }
}

resize.disconnect();
themeObserver.disconnect();
systemTheme.removeEventListener("change", updateTheme);
container.removeEventListener("contextmenu", onContextMenu);
term.dispose();
