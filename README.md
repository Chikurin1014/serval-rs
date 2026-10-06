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
