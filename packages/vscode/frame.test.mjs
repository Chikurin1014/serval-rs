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

/** A frame's window, started: what it is sent, and what it sends its parent. */
function startedHost() {
  const listeners = [];
  const clicks = [];
  const root = fakeRoot();
  const appended = [];
  const sent = [];
  const parent = { postMessage: (message) => sent.push(message) };
  const host = {
    parent,
    addEventListener: (_, listener) => listeners.push(listener),
    document: {
      documentElement: root,
      head: { append: (element) => appended.push(element) },
      createElement: () => ({}),
      addEventListener: (_, listener) => clicks.push(listener),
    },
  };
  start(host);
  return { host, listeners, clicks, root, appended, parent, sent };
}

test("asks VS Code to open the links it would open in a new tab", () => {
  const { clicks, sent } = startedHost();
  const click = (link) => {
    const event = {
      target: { closest: () => link },
      defaultPrevented: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
    };
    clicks.forEach((listener) => listener(event));
    return event;
  };

  const licenses = {
    href: "http://127.0.0.1:5000/third-party-licenses.html",
    textContent: " Licenses ",
  };
  assert.equal(click(licenses).defaultPrevented, true);
  assert.deepEqual(sent, [{
    servalOpen: { url: licenses.href, title: "Licenses" },
  }]);

  // Not a link of a new tab: left to the page
  assert.equal(click(null).defaultPrevented, false);
  assert.equal(sent.length, 1);
});

test("marks the page, and takes the theme from its parent alone", () => {
  const { host, listeners, root, appended, parent } = startedHost();
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
