// The theme is `data-theme` on <html>, or else the system preference; the
// user's choice is kept in storage.

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
