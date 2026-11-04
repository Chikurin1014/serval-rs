// Runs in the app's frame, first in its <head> (as `extension.js` serves it):
// tells the app its host has a bridge (`servalHost`), and follows VS Code's
// theme (light or dark: the colors are the app's own) and fonts, as the panel's
// page passes them on (`{ servalTheme }`). The links it would open in a new
// tab, it asks VS Code to (`{ servalOpen }`).

const servalFrame = (() => {
  /** The app's fonts, each of VS Code's it takes. */
  const PROPERTIES = {
    "--dc-font": "--vscode-font-family",
    "--dc-font-size": "--vscode-font-size",
    "--dc-mono-font": "--vscode-editor-font-family",
    "--dc-console-font": "--vscode-editor-font-family",
    "--dc-console-font-size": "--vscode-editor-font-size",
  };

  /** The app's theme, of VS Code's kind (e.g. `vscode-high-contrast`). */
  function themeOf(kind) {
    return /light/.test(kind ?? "") ? "light" : "dark";
  }

  /** Sets `root`'s theme and variables of VS Code's (`kind`, `vars`). */
  function apply(root, { kind, vars }) {
    root.dataset.theme = themeOf(kind);
    for (const [own, theirs] of Object.entries(PROPERTIES)) {
      const value = vars?.[theirs]?.trim();
      if (value) {
        root.style.setProperty(own, value);
      } else {
        root.style.removeProperty(own);
      }
    }
  }

  function start(host) {
    host.servalHost = true;
    host.addEventListener("message", ({ source, data }) => {
      if (source === host.parent && data?.servalTheme) {
        apply(host.document.documentElement, data.servalTheme);
      }
    });
    // A panel's frame opens no new tab: VS Code opens what it would
    host.document.addEventListener("click", (event) => {
      const link = event.target?.closest?.("a[target=\"_blank\"]");
      if (link) {
        event.preventDefault();
        const servalOpen = { url: link.href, title: link.textContent.trim() };
        host.parent.postMessage({ servalOpen }, "*");
      }
    }, true);
    // The theme is VS Code's: not switched here
    const style = host.document.createElement("style");
    style.textContent = ".theme-switch { display: none !important; }";
    host.document.head.append(style);
  }

  return { PROPERTIES, themeOf, apply, start };
})();

if (typeof module === "undefined") {
  servalFrame.start(window);
} else {
  module.exports = servalFrame;
}
