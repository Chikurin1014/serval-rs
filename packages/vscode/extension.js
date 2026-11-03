// Shows the web app (in `media`) in a panel. A webview's own files have URLs
// known only as it runs, and the app's are absolute from its build, so they are
// served from this machine, in a frame. A webview may not use Web Serial, so the
// `serial-bridge` program (in `bin`) opens the ports: the frame and it talk
// through the webview's page and this, in lines of JSON.

const { randomBytes } = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const vscode = require("vscode");

const { startBridge } = require("./bridge.js");

/** The bridge program of this OS, in `bin`. */
const BRIDGE_PROGRAM = process.platform === "win32"
  ? "serial-bridge.exe"
  : "serial-bridge";

const CONTENT_TYPES = {
  ".css": "text/css",
  ".html": "text/html",
  ".ico": "image/x-icon",
  ".js": "text/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
};

/**
 * Tells the app it is in the extension (its ports are the bridge's), and
 * follows VS Code's theme: first in its page (`frame.js`).
 */
const FRAME_SCRIPT = `<script>${
  fs.readFileSync(path.join(__dirname, "frame.js"), "utf8")
}</script>`;

/** Serves the files of `root`, on a free port of this machine only. */
function serve(root) {
  const server = http.createServer((request, response) => {
    const { pathname } = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(pathname));
    const isFile = file.startsWith(root + path.sep)
      && fs.statSync(file, { throwIfNoEntry: false })?.isFile();
    const served = isFile ? file : path.join(root, "index.html");
    response.setHeader(
      "Content-Type",
      CONTENT_TYPES[path.extname(served)] ?? "application/octet-stream",
    );
    if (path.basename(served) === "index.html") {
      response.end(
        fs.readFileSync(served, "utf8").replace(
          "<head>",
          `<head>${FRAME_SCRIPT}`,
        ),
      );
    } else {
      fs.createReadStream(served).pipe(response);
    }
  });
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

/** A panel's page of a frame of `url`, and `script` (of its origin) if any. */
function framed(url, title, cspSource, script) {
  const { origin } = new URL(url);
  const nonce = randomBytes(16).toString("hex");
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src ${origin}; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'" />
    <style>
      html, body, iframe { display: block; width: 100%; height: 100%; margin: 0; padding: 0; border: 0; }
      /* The frame's page prefers VS Code's light or dark (not the OS's), as
         its \`prefers-color-scheme\` is the frame's \`color-scheme\` */
      iframe { color-scheme: light; }
      body[data-vscode-theme-kind="vscode-dark"] iframe,
      body[data-vscode-theme-kind="vscode-high-contrast"] iframe { color-scheme: dark; }
    </style>
  </head>
  <body>
    <iframe id="app" src="${url}" title="${title}"></iframe>
    ${script ? `<script nonce="${nonce}">${script(origin)}</script>` : ""}
  </body>
</html>`;
}

/** A frame of the app at `url`, its messages passed to and from the extension. */
function page(url, cspSource) {
  return framed(url, "Serval", cspSource, (origin) => `
      const vscode = acquireVsCodeApi();
      const app = document.getElementById("app");
      const origin = ${JSON.stringify(origin)};
      window.addEventListener("message", ({ source, data }) => {
        if (source === app.contentWindow) {
          vscode.postMessage(data);
        } else {
          // Of the extension, as VS Code hands it to this page
          app.contentWindow.postMessage(data, origin);
        }
      });

      // VS Code's theme and fonts, as it sets them on this page: to
      // the frame as it loads, and each time they change
      const sendTheme = () => {
        const style = document.documentElement.style;
        const vars = {};
        for (const name of style) {
          if (name.startsWith("--vscode-")) {
            vars[name] = style.getPropertyValue(name);
          }
        }
        const kind = document.body.dataset.vscodeThemeKind ?? document.body.className;
        app.contentWindow.postMessage({ servalTheme: { kind, vars } }, origin);
      };
      app.addEventListener("load", sendTheme);
      const watched = { attributes: true, attributeFilter: ["style", "class", "data-vscode-theme-kind"] };
      new MutationObserver(sendTheme).observe(document.documentElement, watched);
      new MutationObserver(sendTheme).observe(document.body, watched);
  `);
}

/**
 * Opens a link the app would open in a new tab (`{ url, title }`): a page of
 * its own (as its licenses) in a tab of VS Code, `tabs` of them, revealed if
 * it is open; another in the browser.
 */
function openLink({ url, title }, appUrl, tabs) {
  const link = new URL(url);
  if (link.origin === new URL(appUrl).origin) {
    const open = tabs.get(link.href);
    if (open) {
      open.reveal();
      return;
    }
    const tab = vscode.window.createWebviewPanel(
      "serval.page",
      `Serval: ${title}`,
      vscode.ViewColumn.Active,
      {},
    );
    tabs.set(link.href, tab);
    tab.onDidDispose(() => tabs.delete(link.href));
    tab.webview.html = framed(link.href, title, tab.webview.cspSource);
  } else if (link.protocol === "https:" || link.protocol === "http:") {
    void vscode.env.openExternal(vscode.Uri.parse(link.href));
  }
}

function activate(context) {
  let panel;

  const open = async () => {
    if (panel) {
      panel.reveal();
      return;
    }
    const server = await serve(path.join(context.extensionPath, "media"));
    const { port } = server.address();
    const url = await vscode.env.asExternalUri(
      vscode.Uri.parse(`http://127.0.0.1:${port}/`),
    );
    const shown = vscode.window.createWebviewPanel(
      "serval",
      "Serval",
      vscode.ViewColumn.Active,
      {
        enableScripts: true,
        // The app's data and plots are in the frame, gone if it is reloaded
        retainContextWhenHidden: true,
      },
    );
    panel = shown;
    const bridge = startBridge(
      path.join(context.extensionPath, "bin", BRIDGE_PROGRAM),
      (line) => void shown.webview.postMessage(line),
      (why) =>
        void vscode.window.showErrorMessage(
          `Serval: ${why}. Close the panel and open it again to restart it.`,
        ),
    );
    // Its pages in tabs of their own: closed with it, as is their server
    const tabs = new Map();
    shown.webview.onDidReceiveMessage((message) => {
      if (typeof message === "string") {
        bridge.send(message);
      } else if (message?.servalOpen) {
        openLink(message.servalOpen, url.toString(), tabs);
      }
    });
    shown.onDidDispose(() => {
      panel = undefined;
      for (const tab of tabs.values()) {
        tab.dispose();
      }
      bridge.end();
      server.close();
    });
    shown.webview.html = page(url.toString(), shown.webview.cspSource);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("serval.open", open),
    { dispose: () => panel?.dispose() },
  );
}

module.exports = { activate };
