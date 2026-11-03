import assert from "node:assert/strict";
import { test } from "node:test";

import frame from "./frame.js";

const { apply, start, themeOf } = frame;

/** An element of `dataset` and inline `style`, as the page's root. */
function fakeRoot() {
  const properties = new Map();
  return {
    dataset: {},
    properties,
    style: {
      setProperty: (name, value) => properties.set(name, value),
      removeProperty: (name) => properties.delete(name),
    },
  };
}

test("light kinds are light, the others dark", () => {
  assert.equal(themeOf("vscode-light"), "light");
  assert.equal(themeOf("vscode-high-contrast-light"), "light");
  assert.equal(themeOf("vscode-dark"), "dark");
  assert.equal(themeOf("vscode-high-contrast"), "dark");
  assert.equal(themeOf(undefined), "dark");
});

test("takes VS Code's fonts, and drops those it no longer has", () => {
  const root = fakeRoot();
  apply(root, {
    kind: "vscode-light",
    vars: {
      "--vscode-font-family": " system-ui ",
      "--vscode-font-size": "13px",
      "--vscode-editor-font-family": "\"Fira Code\", monospace",
      "--vscode-editor-font-size": "15px",
    },
  });
  assert.equal(root.dataset.theme, "light");
  assert.equal(root.properties.get("--dc-font"), "system-ui");
  assert.equal(root.properties.get("--dc-font-size"), "13px");
  assert.equal(
    root.properties.get("--dc-console-font"),
    "\"Fira Code\", monospace",
  );
  assert.equal(root.properties.get("--dc-console-font-size"), "15px");

  apply(root, { kind: "vscode-dark", vars: { "--vscode-font-size": "14px" } });
  assert.equal(root.dataset.theme, "dark");
  assert.equal(root.properties.get("--dc-font-size"), "14px");
  assert.equal(root.properties.has("--dc-console-font"), false);
});

test("marks the page, and takes the theme from its parent alone", () => {
  const listeners = [];
  const root = fakeRoot();
  const appended = [];
  const parent = {};
  const host = {
    parent,
    addEventListener: (_, listener) => listeners.push(listener),
    document: {
      documentElement: root,
      head: { append: (element) => appended.push(element) },
      createElement: () => ({}),
    },
  };
  start(host);
  assert.equal(host.servalExtension, true);
  assert.match(appended[0].textContent, /\.theme-switch/);

  const theme = { servalTheme: { kind: "vscode-light", vars: {} } };
  listeners.forEach((listener) => listener({ source: {}, data: theme }));
  assert.equal(root.dataset.theme, undefined);
  listeners.forEach((listener) =>
    listener({ source: parent, data: "{\"id\":1}" })
  );
  assert.equal(root.dataset.theme, undefined);
  listeners.forEach((listener) => listener({ source: parent, data: theme }));
  assert.equal(root.dataset.theme, "light");
});
