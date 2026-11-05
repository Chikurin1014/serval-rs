# Serval for VS Code

A prototype: it shows the web app in a panel. A panel's page may not use Web Serial,
so the `serial-bridge` program opens the ports for it.

It does not come as a package yet: the extension needs the program of each OS.

To try it, build the web app and the program, and put them in the extension:

```bash
dx build --platform web --release --debug-symbols false
```

```bash
cp -r target/dx/web/release/web/public packages/vscode/media
```

```bash
cargo build --release -p serial-runtime --features serial-bridge --bin serial-bridge
```

```bash
mkdir -p packages/vscode/bin && cp target/release/serial-bridge packages/vscode/bin/
```

Then run **Serval: Open** from the Command Palette of:

```bash
code --extensionDevelopmentPath=packages/vscode
```

If the program is missing or stops, VS Code shows why.
Close the panel and open it again to restart the program.
