# Toolbar

![The toolbar](../media/toolbar.png)

## Navigation bar

| Item | |
|---|---|
| **Documents** | These documents, in a new tab |
| **Licenses** | The licenses of the software Serval bundles, in a new tab |
| GitHub icon | Serval's repository on GitHub |
| **Layout** | Imports or exports the [layout](#layout) |
| Theme switch | Light or dark theme |
| Plugin button | Imports a [plugin](task.md) |

The desktop app has no navigation bar: **Layout**, the theme switch and the
plugin button are in the [menu](#menu), with **Licenses**. These documents
are only on the web.

## Menu

In a narrow window, and always in the desktop app, the toolbar is in a menu
instead. The menu icon button at the top left opens it. It is faint until the
pointer is on it.

The menu has the port's items, and the tabs at its bottom. In the desktop app,
it also has **Layout**, the theme switch, the plugin button and **Licenses**.
Picking a tab closes the menu.

## Port

From the left:

0. **Plug icon button**: requests access to a serial port (only on the web).
1. **Port selector**: picks a device to access.
2. **Refresh button**: lists the devices allowed again.
   In the desktop app, the list follows the devices plugged in and out.
3. **Baud rate**: type a value, or pick one of `9600`, `19200`, `57600`, and `115200`.
   The menu beside it (e.g. `8N1`) sets the data bits, parity, stop bits, and
   flow control (see [Port settings](../reference/serial-port.md#port-settings)).
   Both are disabled while the port is open.
4. **Open / Close**: opens the port with those settings, or closes it.
5. **Signals**: DTR and RTS, which are also used to restart the board.

### Signals

The button does what is picked in the menu beside it:

![The signals menu](../media/toolbar-signals.png)

| Action | |
|---|---|
| **Restart (Arduino)** | Restarts the board as avrdude does: DTR and RTS off for a moment |
| **Restart (Espressif)** | Restarts the board as esptool does, into its program |
| **Manual** | A toggle for each of DTR and RTS |

## Layout

The maps and graphs, with their settings, are the **layout**.
It can be saved to a JSON file and imported again.

![The layout menu](../media/toolbar-layout.png)

* **Export to JSON**: saves the layout to a file.
* **Import from JSON**: replaces the maps and graphs with those of a file.
