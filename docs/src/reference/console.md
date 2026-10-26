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
| Send a control character (e.g. `0x03`) | <kbd>Ctrl</kbd> + <kbd>C</kbd> | <kbd>Control</kbd> (⌃) + <kbd>C</kbd> |
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
