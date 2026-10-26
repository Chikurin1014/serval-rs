# Toolbar

![The toolbar](../media/toolbar.png)

## Navigation bar

| Item | |
|---|---|
| **Documents** | These documents, in a new tab |
| **Licenses** | The licenses of the software Serval bundles, in a new tab |
| **Layout** | Imports or exports the [layout](#layout) |
| Theme switch | Light or dark theme |
| Plugin button | Imports a [plugin](task.md) |

## Port

From the left:

0. **Plug icon button**: requests access to a serial port (only on the web).
1. **Port selector**: picks a device to access.
2. **Baud rate**: type a value, or pick one of `9600`, `19200`, `57600`, and `115200`.
3. **Open / Close**: opens the port at that baud rate, or closes it.
4. **Signals**: DTR and RTS, which are also used to restart the board.

### Signals

The button does what is picked in the menu beside it:

![The signals menu](../media/toolbar-signals.png)

| Action | |
|---|---|
| **Restart (Arduino)** | Restarts the board as avrdude does: DTR off for a moment |
| **Restart (Espressif)** | Restarts the board as esptool does, into its program |
| **Manual** | A toggle for each of DTR and RTS |

## Layout

The maps and graphs, with their settings, are the **layout**.
It can be saved to a JSON file and imported again.

![The layout menu](../media/toolbar-layout.png)

* **Export to JSON**: saves the layout to a file.
* **Import from JSON**: replaces the maps and graphs with those of a file.
