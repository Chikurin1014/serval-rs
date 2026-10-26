# Maps

* A map's settings are locked while it is on: turn it off to edit them.
* Turned on again with its settings changed, it reads its input from the start.
* An empty output label (e.g. of a group that matched nothing) gives no value.
* **From** fields take a [pattern](patterns.md), with its aliases
  (`{number}`, `{word}`, `{level}`).

## Placeholders

The output label and the values a map gives (its **To**, **Level** and
**Message** fields) may use what the pattern matched:

| Placeholder | |
|---|---|
| `$0` | The whole match |
| `$1`, `$2`, … | The groups, by their number |
| `${name}` | A named group, `(?<name>…)` |
| `$$` | `$` itself |
| `${level:0}`, `${level:1}`, … | The level of each input log, from the first (`0`) |
| `${level:max}` / `${level:min}` | The level of the input log that matters most / least |

A **Level** field takes a level's name (e.g. `WARN`, or `$1`); left empty,
the log has none (`NONE`).

## Maps by output type

### To Bytes

| Map | From | |
|---|---|---|
| **Encode** | Log | Each message as its UTF-8 bytes |

### To Log

| Map | From | |
|---|---|---|
| **Decode** | Bytes | Cuts the bytes into lines, by the delimiters chosen (CR, LF, CRLF), without escape sequences; each line matching **From** becomes a log of **Level** and **Message** |
| **Match** | Log | Each message matching **From** becomes a log of **Level** and **Message**, to the output label |
| **Replace** | Log | Replaces every match of **From** in each message by **To**, its level kept |
| **Concat** | Log, Log | Joins the newest messages of two labels, with **Separator** between (with escapes such as `\n`), once both have a new one; of **Level**, or that of the one that matters more |

Decode's preset, `[... level ...] message`, takes a level in brackets, with
other words around it, before the message:

| Line | Level | Message |
|---|---|---|
| `[WARN] battery low` | `WARN` | `battery low` |
| `[INFO app] ready` | `INFO` | `ready` |
| `plain text` | `NONE` | `plain text` |

### To Number

| Map | From | |
|---|---|---|
| **Match** | Log | Each message matching **From**: **To** as a number, to the output label |
| **Add**, **Subtract**, **Multiply**, **Divide** | Number, Number | Of two labels, or a label and a number. Of two labels, once both have a new number |
| **Differentiate** / **Integrate** | Number | Over time, in seconds (integrated by the trapezoidal rule) |
| **Magnitude** | Vector2d / Vector3d | The length of each vector |
| **Decompose** | Vector2d / Vector3d | Each component to its own label (left empty for none) |
| **Dot** | Vectors | The dot product |
| **Cross** | Vector2d, Vector2d | The z of the cross product |

Match's presets:

| Preset | Pattern | Output label | To |
|---|---|---|---|
| value | `^{number}` | (yours) | `$0` |
| name: value | `({word}): ({number})` | `$1` | `$2` |
| Arduino | `({word}):({number})` | `$1` | `$2` |
| Teleplot | `>({word}):({number})` | `$1` | `$2` |

### To Vector

| Map | From | |
|---|---|---|
| **Match** | Log | Each message matching **From**: its components (**X**, **Y**, **Z**) as numbers |
| **Compose** | Numbers | A vector of two or three labels (or numbers), once each has a new one |
| **Add**, **Subtract** | Vectors | Of two labels, or a label and a vector, `(x, y)` |
| **Multiply** | Number, Vector | A vector times a number |
| **Cross** | Vector3d, Vector3d | The cross product |
| **Differentiate** / **Integrate** | Vector | Each component over time, in seconds |

Match's presets:

| Preset | Pattern |
|---|---|
| (x, y) | `^\(({number}), ({number})\)$` |
| name: (x, y) | `({word}):\s*\(({number}),\s*({number})\)` |
| (x, y, z) | `^\(({number}), ({number}), ({number})\)$` |
| name: (x, y, z) | `({word}):\s*\(({number}),\s*({number}),\s*({number})\)` |

## Weakest level

The maps that take logs leave out those of a level weaker than their
**weakest level** (`NONE`, at first, takes all).
