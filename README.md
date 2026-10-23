# Serval - SERial VisuALizer

Cross-Platform Serial Visualizer

## Development

Your new workspace contains a member crate for each of the web, desktop and mobile platforms, and a `ui` crate for components that are shared between multiple platforms:

```
serval-rs/
├─ README.md
├─ Cargo.toml
└─ packages/
   ├─ web/
   │  └─ ... # Web specific UI/logic
   ├─ desktop/
   │  └─ ... # Desktop specific UI/logic
   ├─ mobile/
   │  └─ ... # Mobile specific UI/logic
   └─  ui/
      └─ ... # Component shared between multiple platforms
```

### Platform crates

Each platform crate contains the entry point for the platform, and any assets, components and dependencies that are specific to that platform. For example, the desktop crate in the workspace looks something like this:

```
desktop/ # The desktop crate contains all platform specific UI, logic and dependencies for the desktop app
├─ assets/ # Assets used by the desktop app - Any platform specific assets should go in this folder
├─ src/
│  ├─ main.rs # The entrypoint for the desktop app (a placeholder for now: Serval runs in the browser)
├─ Cargo.toml # The desktop crate's Cargo.toml - This should include all desktop specific dependencies
```

When you start developing with the workspace setup each of the platform crates will look almost identical. The UI starts out exactly the same on all platforms. However, as you continue developing your application, this setup makes it easy to let the views for each platform change independently.

### Shared UI crate

The workspace contains a `ui` crate with components that are shared between multiple platforms. You should put any UI elements you want to use in multiple platforms in this crate. You can also put some shared client side logic in this crate, but be careful to not pull in platform specific dependencies. The `ui` crate starts out something like this:

```
ui/
├─ src/
│  ├─ lib.rs # The entrypoint for the ui crate
│  ├─ components
```

#### Serving Your App

Navigate to the platform crate of your choice:

```bash
cd web
```

and serve:

```bash
dx serve
```

#### Documents

The documents are an [mdBook](https://rust-lang.github.io/mdBook/) in `docs/`, served at `/docs/` of the app.
`mdbook build docs` builds them into `packages/web/public/docs` (not kept in git), which `dx serve` and `dx build` serve with the app.
While writing, rebuild them as they change:

```bash
mdbook watch docs
```

or preview them alone, at <http://localhost:3000>:

```bash
mdbook serve docs
```

Their videos and screenshots are recorded of the app as it is built, not kept in git: `nix build .#docs` records them as it builds the documents, and `nix build .#web` (as a release deploys it) puts both at `/docs/` of the app.
The checks build the documents without them, so CI records them only on a release.
To see them as you write, record them into `docs/src/media` of an app you built (here, as the e2e tests build it):

```bash
python docs/media/capture.py target/e2e/dx/web/debug/web/public docs/src/media
```

The plugins' API reference (at `api/` of the documents) is generated of the docstrings of their Rune modules (`packages/ui/src/plugin/serval/`) by `tools/serval-rune`, Rune's own CLI with them, not kept in git either.
`nix build .#docs` builds it with the documents; to see it as you write, build it into `docs/src/api`:

```bash
nix build .#docs-api -o docs/src/api
```

or with Cargo, in the empty Rune workspace of `tools/serval-rune` (a Cargo workspace of its own):

```bash
cd tools/serval-rune && cargo run -- doc --output ../../docs/src/api
```

#### Building for Release

`nix build .#web` (or `nix build`) builds the web app as it is deployed, into `result/`.
To build it with `dx` instead, turn off the debug symbols it adds by default:

```bash
dx build --platform web --release --debug-symbols false
```

`wasm-opt` aborts on them, and `dx` then goes on with the wasm unoptimized (about 7 MB instead of 5), reporting only `wasm-opt failed` among its logs.

## Testing

`nix flake check` runs all of the tests below in the Nix sandbox, against a release build of the web app.
It only sees files tracked by git, so `git add` new files first.

Run these inside `nix develop`, which provides Node.js, pytest, Playwright and its browsers.

Unit tests (Rust and JavaScript):

```bash
cargo test -p ui
```

```bash
node --test 'packages/**/*.test.mjs'
```

End-to-end tests drive the web app in headless Chromium with a mock serial port (`e2e/mock_serial.js`) that sends `temp:…` and `volt:…` lines.
They build the app into `target/e2e`, apart from `dx serve`'s build:

```bash
pytest e2e
```

To test an existing build instead, point `SERVAL_E2E_APP` at its `public/` directory.

## License

Serval is licensed under the [MIT License](LICENSE).

It bundles third-party software (KaTeX, uPlot, xterm.js, Zdog, the Dioxus components, the USB ID Repository and the Rust crates compiled into it), whose licenses are listed in [`third-party-licenses.html`](packages/web/public/third-party-licenses.html), served with the app at `/third-party-licenses.html`.
After changing dependencies, regenerate it in `nix develop` (see [`about.toml`](about.toml)):

```bash
cargo about generate --manifest-path packages/web/Cargo.toml about.hbs -o packages/web/public/third-party-licenses.html
```
