# Reference

* [Serial Port](serial-port.md): the port's settings, signals and what it receives.
* [Console](console.md): its views, history, send buffer and keyboard shortcuts.
* [Record Files](record-files.md): the formats of the files a record writes.
* [Data](data.md): labels, types, how many values they keep, and log levels.
* [CSV Export](csv-export.md): the format of the data list's CSV files.
* [Patterns](patterns.md): the regular expressions and their aliases.
* [Maps](maps.md): each map, and the placeholders they take.
* [Graphs](graphs.md): each graph, and its settings.
* [Layout File](layout-file.md): the format of the layout's JSON files.
* [Tasks](tasks/index.md): the language of plugins, how they run, and what they can use.

## Desktop app

The desktop app works as the web app does, except:

* It works offline.
* All the serial ports are listed, with no need to allow them, so there is no
  plug icon button. The list follows the devices plugged in and out (see
  [Serial Port](serial-port.md)).
* The [port info](serial-port.md#port-info) also shows the port's path.
* It has no navigation bar: the toolbar is always in the
  [menu](../usage/toolbar.md#menu), which also has **Layout**, the theme
  switch, the plugin button and **Licenses**. These documents are only on the web.
* Records, layouts and CSV files are saved where a save dialog asks, not
  downloaded.
* **Licenses** shows the licenses of the desktop app in the app, so it works
  offline.

### Install

Download it from the [latest release](https://github.com/Chikurin1014/serval-rs/releases/latest).
These files are not signed, so the OS may warn you when you first open them.
On Windows, choose **More info**, then **Run anyway**.

| OS | File | How to open it |
| --- | --- | --- |
| Windows | `Serval-x64.msi` | Open it to install Serval and add it to `PATH`. It needs the WebView2 runtime, which Windows 11 has. |
| Windows | `serval.exe` | Open it. It is one file, with no need to install it. It needs the WebView2 runtime, too. |
| macOS (Apple silicon) | `Serval-arm64.dmg` | Open it, and drag **Serval** to **Applications**. If macOS says that it is damaged, run `xattr -cr /Applications/Serval.app`. |
| Linux (x86-64) | `serval` | Run `chmod +x serval`, then open it. It needs a system as new as Ubuntu 24.04, and the libraries below. |

On Linux, install WebKitGTK 4.1 and libxdo. On Debian and Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-0 libxdo3
```

Your user also needs access to the serial ports. On most systems, run this,
then log out and in again:

```bash
sudo usermod -aG dialout $USER
```
