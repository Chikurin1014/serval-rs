# Serial Port

## Port settings

| Setting      | Values                      | Default |
| ------------ | --------------------------- | ------- |
| Baud rate    | See [Baud rate](#baud-rate) |         |
| Data bits    | 7, 8                        | 8       |
| Parity       | none, even, odd             | none    |
| Stop bits    | 1, 2                        | 1       |
| Flow control | none, hardware (RTS/CTS)    | none    |

- All but the baud rate are set in the menu beside the baud rate field.
  Its button shows them as data bits, parity, and stop bits: `8N1`, `7E2`.
- Each port keeps its own settings, taken as it opens.
- They can be changed only while a port is selected and closed: else the
  fields are disabled, and a plugin's `serial::set_settings` fails.

## Selecting a port

| Control          |                                                      |
| ---------------- | ---------------------------------------------------- |
| Plug icon button | Asks the browser for access to a new port (web only) |
| Port selector    | Picks one of the ports allowed before                |
| Refresh button   | Lists the allowed ports again                        |

In the desktop app, all the ports are listed, and the list follows the
devices plugged in and out. Each is named by its product and its path (e.g.
`Uno R3 (CDC ACM) · ttyACM0`), or by its path alone.

The vendor and product of a USB port are looked up in the
[USB ID Repository](http://www.linux-usb.org/usb-ids.html).

## Baud rate

- Any positive integer, in bps.
- Presets: `9600`, `19200`, `57600`, and `115200`.
- Set as the other [port settings](#port-settings).

## Signals

### DTR and RTS

- Both are turned on as the port opens.
- They are not read back from the port: Serval shows them as it last set them.

### Restart

| Action                  | Sequence                                            |
| ----------------------- | --------------------------------------------------- |
| **Restart (Arduino)**   | DTR off and RTS off, wait 100 ms, DTR on and RTS on |
| **Restart (Espressif)** | DTR off and RTS on, wait 100 ms, RTS off            |

Restart (Arduino) resets the board through the capacitor on its reset pin, as
avrdude does.\
Restart (Espressif) resets an ESP32, ESP8266, or similar board into its program,
as esptool does.

### Break

- Holds the line low for 250 ms.
- Ignored while a break is being sent.
- <kbd>Alt</kbd> + <kbd>B</kbd> in the console (see [Console](console.md#keyboard-shortcuts)).

## Receiving

What the port receives goes to:

- the label `raw_bytes` ([Bytes](data.md#types)), one value per chunk,
- the [console](console.md),
- the [record file](record-files.md) being written, if any.

Chunks are gathered for about one frame (16 ms) before they go to the data,
or at once when 256 have gathered.

## Port Info

The panel beside the console:

| Item                                       |                                                                                     |
| ------------------------------------------ | ----------------------------------------------------------------------------------- |
| Vendor, Product                            | Of a USB port, if known                                                             |
| Path                                       | Where the port is on the system (e.g. `/dev/ttyACM0` or `COM3`): in the desktop app |
| Baud rate                                  | In bps                                                                              |
| Data bits, Parity, Stop bits, Flow control | As in [Port settings](#port-settings)                                               |
| Received, Sent                             | The bytes received and sent since the port was opened                               |
| Notifications                              | Below them, newest first (see [Notifications](#notifications))                      |

## Notifications

Shown as a toast, and kept in the [Port Info](#port-info) panel, newest
first, up to 100.

| Notification                           | Kind    |
| -------------------------------------- | ------- |
| Port opened                            | Success |
| Port closed                            | Info    |
| Connection lost                        | Error   |
| Other failures (e.g. opening the port) | Error   |
