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
 * Calls `onChange` as the theme changes, the page's or the system's; gives
 * what stops that.
 */
function watchTheme(onChange) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
  const system = matchMedia("(prefers-color-scheme: dark)");
  system.addEventListener("change", onChange);
  return () => {
    observer.disconnect();
    system.removeEventListener("change", onChange);
  };
}
