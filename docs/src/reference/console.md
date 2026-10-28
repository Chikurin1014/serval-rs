# Console

## Views

### Text

* A terminal ([xterm.js](https://xtermjs.org/)): escape sequences (colors, cursor
  moves) take effect.
* A lone LF starts a new line, as Tera Term's "AUTO" receive mode.
* Each line is marked with the local time its first byte arrived
  (`HH:MM:SS.mmm`).

### Hex

* Each byte in uppercase hex, with its character beside it, as `hexdump -C`.
* 16, 8, or 4 bytes per line, as the console's width allows.
* Bytes outside printable ASCII (`0x20`–`0x7E`) are shown as `.`.

## History

| | |
|---|---|
| Bytes kept | The newest 1 MiB |
| Lines kept | 10,000 |
| Clear button | Clears the console only: not the data, nor a record being written |

## Send buffer

* Shows what is sent: each send stays for 3 s after the port takes it.
* Up to 1,000 sends.
* In the Hex view, sends are shown in hex.

## Sending files

* The file button, or a file dropped on the console.
* Sent as it is, in 4 KiB chunks.
* Its progress is shown in place of the send buffer.
* Click the button again to stop between chunks.

## Keyboard shortcuts

| Action | Windows / Linux | macOS |
|---|---|---|
| Send CR | <kbd>Enter</kbd> | <kbd>Return</kbd> |
| Send BS (`0x08`) | <kbd>Backspace</kbd> | <kbd>Delete</kbd> (⌫) |
| Send DEL (`0x7F`) | <kbd>Delete</kbd> | <kbd>Fn</kbd> + <kbd>Delete</kbd> (⌦) |
| Send a control character (see [Control characters](#control-characters)) | <kbd>Ctrl</kbd> + <kbd>C</kbd> | <kbd>Control</kbd> (⌃) + <kbd>C</kbd> |
| Paste | <kbd>Alt</kbd> + <kbd>V</kbd> | <kbd>Option</kbd> (⌥) + <kbd>V</kbd> |
| Paste, with a CR after | <kbd>Alt</kbd> + <kbd>R</kbd> | <kbd>Option</kbd> (⌥) + <kbd>R</kbd> |
| Send a break | <kbd>Alt</kbd> + <kbd>B</kbd> | <kbd>Option</kbd> (⌥) + <kbd>B</kbd> |
| Maximize or restore the console | <kbd>Alt</kbd> + <kbd>Enter</kbd> | <kbd>Option</kbd> (⌥) + <kbd>Return</kbd> |
| Select a block | <kbd>Alt</kbd> + drag | <kbd>Option</kbd> (⌥) + drag |
| Copy | Select text (copied as it is selected) | Select text (copied as it is selected) |
| Paste | Right click | Right click (or <kbd>Control</kbd> + click) |

* As in Tera Term.
* The <kbd>Alt</kbd> / <kbd>Option</kbd> shortcuts go by the key's position on
  a US layout, not by the character it types.
* "Paste, with a CR after" turns each line break of the text into a CR.

## Control characters

The keys that send common control characters in the console:

| Code | Name | Windows / Linux | macOS | Often used for |
|---|---|---|---|---|
| `0x00` | NUL | <kbd>Ctrl</kbd> + <kbd>Space</kbd> | <kbd>Control</kbd> + <kbd>Space</kbd> | |
| `0x01` | SOH | <kbd>Ctrl</kbd> + <kbd>A</kbd> | <kbd>Control</kbd> + <kbd>A</kbd> | MicroPython: raw REPL |
| `0x03` | ETX | <kbd>Ctrl</kbd> + <kbd>C</kbd> | <kbd>Control</kbd> + <kbd>C</kbd> | Stopping a program |
| `0x04` | EOT | <kbd>Ctrl</kbd> + <kbd>D</kbd> | <kbd>Control</kbd> + <kbd>D</kbd> | End of input; MicroPython: soft reset |
| `0x07` | BEL | <kbd>Ctrl</kbd> + <kbd>G</kbd> | <kbd>Control</kbd> + <kbd>G</kbd> | |
| `0x08` | BS | <kbd>Backspace</kbd>, <kbd>Ctrl</kbd> + <kbd>H</kbd> | <kbd>Delete</kbd> (⌫), <kbd>Control</kbd> + <kbd>H</kbd> | Deleting a character |
| `0x09` | HT (tab) | <kbd>Tab</kbd>, <kbd>Ctrl</kbd> + <kbd>I</kbd> | <kbd>Tab</kbd>, <kbd>Control</kbd> + <kbd>I</kbd> | Completion in a shell |
| `0x0A` | LF | <kbd>Ctrl</kbd> + <kbd>J</kbd> | <kbd>Control</kbd> + <kbd>J</kbd> | End of a line (e.g. Arduino's `readStringUntil('\n')`) |
| `0x0D` | CR | <kbd>Enter</kbd>, <kbd>Ctrl</kbd> + <kbd>M</kbd> | <kbd>Return</kbd>, <kbd>Control</kbd> + <kbd>M</kbd> | End of a line (e.g. AT commands) |
| `0x11` | XON (DC1) | <kbd>Ctrl</kbd> + <kbd>Q</kbd> | <kbd>Control</kbd> + <kbd>Q</kbd> | Software flow control: resume |
| `0x13` | XOFF (DC3) | <kbd>Ctrl</kbd> + <kbd>S</kbd> | <kbd>Control</kbd> + <kbd>S</kbd> | Software flow control: pause |
| `0x1A` | SUB | <kbd>Ctrl</kbd> + <kbd>Z</kbd> | <kbd>Control</kbd> + <kbd>Z</kbd> | End of a text (e.g. an SMS by AT commands) |
| `0x1B` | ESC | <kbd>Esc</kbd>, <kbd>Ctrl</kbd> + <kbd>[</kbd> | <kbd>Esc</kbd>, <kbd>Control</kbd> + <kbd>[</kbd> | Start of an escape sequence; canceling |
| `0x7F` | DEL | <kbd>Delete</kbd> | <kbd>Fn</kbd> + <kbd>Delete</kbd> (⌦) | Deleting a character |

* <kbd>Ctrl</kbd> + <kbd>A</kbd>–<kbd>Z</kbd> send `0x01`–`0x1A`, in order.
* <kbd>Enter</kbd> sends a CR alone: for CR LF, press <kbd>Enter</kbd>, then
  <kbd>Ctrl</kbd> + <kbd>J</kbd>.
* <kbd>Ctrl</kbd> + <kbd>Space</kbd> may be taken by the OS, to switch the
  input method.
* On Windows and Linux, the browser keeps <kbd>Ctrl</kbd> + <kbd>N</kbd>,
  <kbd>T</kbd>, and <kbd>W</kbd> for itself (a new window, a new tab, closing
  the tab): `0x0E`, `0x14`, and `0x17` cannot be typed.
  A [plugin](tasks/index.md) can send them (`serial::send`).
