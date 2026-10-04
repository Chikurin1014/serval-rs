// Light / dark theme switching for `theme.rs`, which runs this through
// `document::eval` followed by a call to one of the functions below.
//
// The theme is `data-theme` on <html>, which `dx-components-theme.css`
// switches on; without one, the system preference applies. The user's choice
// is kept in storage across sessions.

const THEMES = ["light", "dark"];
const STORAGE_KEY = "theme";

/** `localStorage`, or `null` where the page may not use it. */
function browserStorage() {
    try {
        return localStorage;
    } catch (_) {
        return null;
    }
}

/** The theme chosen in an earlier session, if any. */
function storedTheme(storage) {
    try {
        const theme = storage?.getItem(STORAGE_KEY);
        return THEMES.includes(theme) ? theme : null;
    } catch (_) {
        return null;
    }
}

/** Applies the theme chosen in an earlier session to `root` (<html>). */
function restoreTheme(root, storage) {
    const theme = storedTheme(storage);
    if (theme) {
        root.dataset.theme = theme;
    }
}

/**
 * Switches `root` (<html>) to the other theme than the one showing, which is
 * the system preference until one is chosen, and remembers the choice.
 * Returns the new theme.
 */
function toggleTheme(root, storage, systemPrefersDark) {
    const showing = root.dataset.theme ?? (systemPrefersDark ? "dark" : "light");
    const next = showing === "dark" ? "light" : "dark";
    root.dataset.theme = next;
    try {
        storage?.setItem(STORAGE_KEY, next);
    } catch (_) {
        // Not remembered, e.g. storage is full or blocked
    }
    return next;
}
