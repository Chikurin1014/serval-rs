# Record Files

Recording needs a browser with the
[File System Access API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API),
such as Chrome or Edge.

## Formats

| Menu item | Format | Extension |
|---|---|---|
| Text file | [Text](#text) | `.log` |
| Text file › with time | Text, with [timestamps](#timestamps) | `.log` |
| Text file › with time and sent | Text, with timestamps and what is sent | `.log` |
| Hex file | [Hex](#hex) | `.tsv` |
| Hex file › with time | Hex, with timestamps | `.tsv` |
| Hex file › with time and sent | Hex, with timestamps and what is sent | `.tsv` |
| Raw bytes | [Raw bytes](#raw-bytes) | `.bin` |
| Continue with existing file | That of the file ([detected](#continuing-a-file)) | |

### Text

```text
[1761450000.123] < temp: 22.5
[1761450000.140] > ping
[1761450000.200] --- Port closed: COM3 ---
```

* A line per line received, ended by CR, LF, or CRLF.
* Escape sequences and control characters are removed, but tabs.
* `[time] `: with time.
* `< ` (received) / `> ` (sent): with sent.
* [Events](#events) are written as `--- event ---`.

### Hex

```text
1761450000.123	RX	74 65 6D 70 0D 0A
1761450000.140	TX	70 69 6E 67 0D
```

* A line per chunk, its bytes in uppercase hex, separated by spaces.
* Fields are separated by tabs: time (with time), `RX` / `TX` (with sent),
  then the bytes.

### Raw bytes

* The bytes received, as they are.
* Nothing sent, no timestamps, no events.

## File names

`serval-YYYYMMDD-HHMMSS.<extension>`, of the local time it starts (the browser
asks where to save it).

## Timestamps

* Unix time in seconds, to the millisecond: `1761450000.123`.
* A text line takes the time its first byte arrived.

## Events

Written in text files only.

| Event | Line |
|---|---|
| The port is opened | `--- Port opened: <port> at <baud rate> bps ---` |
| The port is closed | `--- Port closed: <port> ---` |
| The connection is lost | `--- Connection lost: <port>: <error> ---` |
| A break is sent | `--- Break sent ---` |

## Continuing a file

**Continue with existing file** adds to a file picked, in the format it was
written in:

| Extension | Format | Options |
|---|---|---|
| `.bin` | Raw bytes | |
| `.tsv` | Hex | Read from its last line |
| Others | Text | Read from its last lines (up to 64 KiB) |

A record goes on while the port is closed and opened again, until **STOP**.
