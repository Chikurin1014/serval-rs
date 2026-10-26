# Serial Port

## Port settings

| Setting | Value |
|---|---|
| Data bits | 8 |
| Parity | None |
| Stop bits | 1 |
| Flow control | None |

Only the baud rate can be changed.

## Selecting a port

| Control | |
|---|---|
| Plug icon button | Asks the browser for access to a new port (web only) |
| Port selector | Picks one of the ports allowed before |
| Refresh button | Lists the allowed ports again |

The vendor and product of a USB port are looked up in the
[USB ID Repository](http://www.linux-usb.org/usb-ids.html).

## Baud rate

* Any positive integer, in bps.
* Presets: `9600`, `19200`, `57600`, and `115200`.
* Takes effect the next time the port is opened.

## Signals

### DTR and RTS

* Both are turned on as the port opens.
* They are not read back from the port: Serval shows them as it last set them.

### Restart

| Action | Sequence |
|---|---|
| **Restart (Arduino)** | DTR off and RTS off, wait 100 ms, DTR on and RTS on |
| **Restart (Espressif)** | DTR off and RTS on, wait 100 ms, RTS off |

Restart (Arduino) resets the board through the capacitor on its reset pin, as
avrdude does. \
Restart (Espressif) resets an ESP32, ESP8266, or similar board into its program,
as esptool does.

### Break

* Holds the line low for 250 ms.
* Ignored while a break is being sent.
* <kbd>Alt</kbd> + <kbd>B</kbd> in the console (see [Console](console.md#keyboard-shortcuts)).

## Receiving

What the port receives goes to:

* the label `raw_bytes` ([Bytes](data.md#types)), one value per chunk,
* the [console](console.md),
* the [record file](record-files.md) being written, if any.

Chunks are gathered for about one frame (16 ms) before they go to the data,
or at once when 256 have gathered.

## Notifications

Shown as a toast, and kept in the [Port Info](../usage/console.md#port-info)
panel, newest first, up to 100.

| Notification | Kind |
|---|---|
| Port opened | Success |
| Port closed | Info |
| Connection lost | Error |
| Other failures (e.g. opening the port) | Error |
