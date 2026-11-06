// The theme is `data-theme` on <html>, or else the system preference; the
// user's choice is kept in storage. Scripts that follow the theme watch it
// here (`watchTheme`), as this tells it.

const THEMES = ["light", "dark"];
const STORAGE_KEY = "theme";

function browserStorage() {
  try {
    return localStorage;
  } catch (_) {
    return null;
  }
}

function storedTheme(storage) {
  try {
    const theme = storage?.getItem(STORAGE_KEY);
    return THEMES.includes(theme) ? theme : null;
  } catch (_) {
    return null;
  }
}

function restoreTheme(root, storage) {
  const theme = storedTheme(storage);
  if (theme) {
    root.dataset.theme = theme;
  }
}

/** Switches `root` to the other theme and remembers it; returns the new one. */
function toggleTheme(root, storage, systemPrefersDark) {
  const showing = root.dataset.theme ?? (systemPrefersDark ? "dark" : "light");
  const next = showing === "dark" ? "light" : "dark";
  root.dataset.theme = next;
  try {
    storage?.setItem(STORAGE_KEY, next);
  } catch (_) {}
  return next;
}

/**
 * The consoles' font, as xterm.js takes it (`--console-font`,
 * `--console-font-size`), else theme.css's, if it has not loaded yet.
 */
function consoleFont(root = document.documentElement) {
  const style = getComputedStyle(root);
  return {
    fontFamily: style.getPropertyValue("--console-font").trim()
      || "Menlo, Consolas, \"DejaVu Sans Mono\", \"Liberation Mono\", monospace",
    fontSize: parseFloat(style.getPropertyValue("--console-font-size"))
      || 14,
  };
}

/** Sets xterm.js's `term` to the consoles' font; tells whether it changed. */
function updateConsoleFont(term) {
  const { fontFamily, fontSize } = consoleFont();
  if (
    fontFamily === term.options.fontFamily
    && fontSize === term.options.fontSize
  ) {
    return false;
  }
  term.options.fontFamily = fontFamily;
  term.options.fontSize = fontSize;
  return true;
}

/**
 * Calls `onChange` as the theme changes, the page's or the system's, or the
 * variables the platform sets on the page (its `style`); gives what stops
 * that.
 */
function watchTheme(onChange) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme", "style"],
  });
  const system = matchMedia("(prefers-color-scheme: dark)");
  system.addEventListener("change", onChange);
  return () => {
    observer.disconnect();
    system.removeEventListener("change", onChange);
  };
}
