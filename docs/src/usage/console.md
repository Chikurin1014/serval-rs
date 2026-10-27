# Console Tab

![The Console tab](../media/getting-started-console.png)

The console shows what the port receives, as a terminal does, each line marked with the time it arrived.

What you type in it is sent as it is.

## Views

* **Text**: as a terminal, with its escape sequences (colors, cursor moves).
* **Hex**: the bytes in hex, beside them as characters, as `hexdump -C` shows
  them.

![The console in hex](../media/console-hex.png)

The brush button clears the console.
Only the console is cleared: the data, and a [record](#recording) being written, are kept.

## Sending

* **Typing**: each key is sent as you type it. What is sent is shown in the send buffer below the console.
* **Files**: the button to the left of the send buffer (or a file dropped on the console) sends a file as
  binary data, its progress shown in the send buffer. Click the button again to
  stop.

### Keyboard shortcuts

The console has keyboard shortcuts as in Tera Term, such as
<kbd>Alt</kbd> + <kbd>V</kbd> (<kbd>Option</kbd> + <kbd>V</kbd> on macOS) to
paste. See [Reference / Console](../reference/console.md#keyboard-shortcuts)
for all of them.

## Recording

**REC** writes what the port receives to a file, until **STOP**.
It goes on while the port is closed and opened again.

![The record menu](../media/console-record.png)

| Format | |
|---|---|
| **Text file** | The lines received, without escape sequences or control characters (similar to Tera Term's log) |
| **Hex file** | A line for each chunk received: its bytes in hex |
| **Raw bytes** | The bytes received, as they are |
| **Continue with existing file** | Adds to a file written before, in its format |

The text and hex files can have the time of each line (of each chunk, in hex), and what was sent too
(marked `>` / `TX`, and what was received `<` / `RX`).

## Port Info

The panel beside the console shows information about the port.
