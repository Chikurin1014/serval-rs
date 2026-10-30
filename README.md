# Serval - SERial VisuALizer

Cross-Platform Serial Visualizer

## Development

### Workspace

```
serval-rs/
├─ packages/
│  ├─ ui/       # Components and logic shared by the apps
│  ├─ web/      # Web app
│  ├─ desktop/  # Desktop app
│  ├─ mobile/   # Mobile app
│  └─ embed-assets/  # Puts the desktop app's assets in its binary
├─ docs/        # Documents (mdBook)
├─ e2e/         # End-to-end tests (web app)
└─ tools/serval-rune/  # Generates the plugin API reference
```

Each app crate has its entry point and what only its platform needs.
`ui` has what they share.
The steps below are for the web app.

### Requirements

| Tool                                                                                                      | Version (as tested)                   | Used for                                              |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------- |
| [Rust](https://rustup.rs/)                                                                                | stable, with `wasm32-unknown-unknown` | Everything. `rust-toolchain.toml` installs the target |
| [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started) (`dx`)                                     | 0.7.9                                 | Running and building the apps                         |
| [mdBook](https://rust-lang.github.io/mdBook/), [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid) | 0.5, 0.17                             | Documents                                             |
| [Node.js](https://nodejs.org/)                                                                            | 24                                    | JavaScript unit tests                                 |
| [Python](https://www.python.org/) with pytest, pytest-xdist, Playwright                                   | 3.14                                  | End-to-end tests, screenshots of the documents        |
| [FFmpeg](https://ffmpeg.org/)                                                                             |                                       | Videos of the documents                               |
| [cargo-about](https://github.com/EmbarkStudios/cargo-about)                                               | 0.9                                   | Third-party licenses                                  |

```bash
cargo install dioxus-cli --version 0.7.9 --locked
```

```bash
cargo install mdbook mdbook-mermaid cargo-about --locked
```

```bash
pip install pytest pytest-xdist playwright
```

```bash
python -m playwright install chromium
```

`dx` downloads `wasm-bindgen` and `wasm-opt` by itself when it needs them.

### Run the web app

```bash
dx serve -p web
```

Open it in a Chromium-based browser (Chrome, Edge): Web Serial works only there.

### Documents

The documents are an mdBook in `docs/`.
The web app serves them at `/docs/`.
To include them in the web app, build them into `packages/web/public/docs`:

```bash
mdbook build docs --dest-dir packages/web/public/docs
```

To preview them alone:

```bash
mdbook serve docs
```

These are not in git.
Generate them when you need them:

- Videos and screenshots (`docs/src/media`): record them from a built web app.
  The end-to-end tests build one into `target/e2e`:

  ```bash
  python docs/media/capture.py target/e2e/dx/web/debug/web/public
  ```

- The plugin API reference (`docs/src/api`): generate it from the docstrings of the Rune modules (`packages/ui/src/plugin/serval/`).
  `tools/serval-rune` is a Cargo workspace of its own:

  ```bash
  cd tools/serval-rune && cargo run -- doc --output ../../docs/src/api
  ```

### Third-party licenses

The licenses of the bundled software are in [`third-party-licenses.html`](third-party-licenses.html).
The web app serves it at `/third-party-licenses.html`.
To include it in the web app, copy it into `packages/web/public`:

```bash
cp third-party-licenses.html packages/web/public/
```

The desktop app has its own list, [`third-party-licenses-desktop.html`](third-party-licenses-desktop.html), built into it.

After you change dependencies, generate them again (see [`about.toml`](about.toml)):

```bash
cargo about generate --manifest-path packages/web/Cargo.toml about.hbs -o third-party-licenses.html
```

```bash
cargo about generate --manifest-path packages/desktop/Cargo.toml --target x86_64-pc-windows-msvc --target x86_64-unknown-linux-gnu --target x86_64-apple-darwin --target aarch64-apple-darwin about.hbs -o third-party-licenses-desktop.html
```

### Release build (web)

Build the documents and copy the licenses first, as above. Then:

```bash
dx build --platform web --release --debug-symbols false
```

The web app is in `target/dx/web/release/web/public`.

> [!NOTE]
> Keep `--debug-symbols false`.
> Without it, `wasm-opt` fails, and the wasm stays unoptimized (about 7 MB instead of 5).
> `dx` reports only `wasm-opt failed` in its logs.

### Tests

Unit tests (Rust and JavaScript):

```bash
cargo test -p ui
```

```bash
node --test 'packages/**/*.test.mjs'
```

End-to-end tests run the web app in headless Chromium with a mock serial port (`e2e/mock_serial.js`).
The port sends `temp:…` and `volt:…` lines.
They build the web app (with the documents and the licenses) into `target/e2e`:

```bash
pytest e2e -n auto
```

To test an existing build instead, set `SERVAL_E2E_APP` to its `public/` directory.

## Development with Nix

With [Nix](https://nixos.org/) (flakes enabled), you don't need to install the tools above.
Each section above has these commands:

| Section              | Command                                                                                                              |
| -------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Requirements         | `nix develop` (a shell with all the tools)                                                                           |
| Run the web app      | `nix develop -c dx serve -p web`                                                                                     |
| Documents            | `nix build .#docs` (with the videos and screenshots)                                                                 |
| Plugin API reference | `nix build .#docs-api -o docs/src/api`                                                                               |
| Third-party licenses | `nix develop -c cargo about generate --manifest-path packages/web/Cargo.toml about.hbs -o third-party-licenses.html` |
| Release build (web)  | `nix build` (into `result/share/serval-web`, with the documents and the licenses)                                    |
| Tests                | `nix flake check` (all the tests, in the Nix sandbox)                                                                |

`nix run` serves the release build at http://localhost:8080/.
`nix fmt` formats all the files.

> [!NOTE]
> The checks build the documents without videos and screenshots.
> CI records them only for a release.

## License

Serval is licensed under the [MIT License](LICENSE).

It bundles third-party software.
Their licenses are in [`third-party-licenses.html`](third-party-licenses.html)
(the desktop app's in [`third-party-licenses-desktop.html`](third-party-licenses-desktop.html)).
