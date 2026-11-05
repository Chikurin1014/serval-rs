# Serval for VS Code

[Serval](https://github.com/Chikurin1014/serval-rs), the serial visualizer, in
a panel of VS Code. It shows what a serial port receives as text, data and
graphs, and sends to it.

Run **Serval: Open** from the Command Palette.

A panel's page may not use Web Serial, so the extension comes with a program,
`serial-bridge`, that opens the ports for it. If the program is missing or
stops, VS Code shows why. Close the panel and open it again to restart it.

## Building

Build the web app and the program, and put them in the extension:

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

To try it, open VS Code with it:

```bash
code --extensionDevelopmentPath=packages/vscode
```

To make a package of it for this OS (here, Linux on x64):

```bash
cp LICENSE packages/vscode/
```

```bash
cd packages/vscode && npx @vscode/vsce package --no-dependencies --target linux-x64
```
