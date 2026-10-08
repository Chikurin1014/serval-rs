// The console: an xterm.js terminal.
// From Rust: the container id, then `[reset, bytes]`, then `null` on unmount.
// To Rust: what is typed, to send.

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
  // A lone LF starts a new line too
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

term.onData((data) => dioxus.send(data));

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
term.dispose();
