# Desktop app

Serval as a desktop app: serial ports through the OS, and no network needed.

## Run

```bash
dx serve --platform desktop
```

## Build one file

The app reads its assets from the `assets` folder beside it, unless they are
in its binary. To hand it out as one file, build it, then put the assets in:

```bash
dx build --platform desktop --release --profile desktop-release
cargo run -p embed-assets -- target/dx/desktop/release/<os>/app/desktop serval
```

`desktop-release` is the release profile with LTO, and with no symbols.

On Windows, build for the `msvc` target. With `gnu`, the app needs
`WebView2Loader.dll` beside it.
