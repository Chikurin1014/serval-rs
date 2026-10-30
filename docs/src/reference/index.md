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

* It works offline. It is one file, with nothing else to install.
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
