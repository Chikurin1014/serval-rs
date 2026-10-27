# Layout File

The maps and graphs, with their settings, as JSON: **Layout › Export to JSON**
saves it (`serval-layout.json`), **Layout › Import from JSON** takes it in.

## Structure

```json
{
  "version": "0.4.1",
  "maps": [
    {
      "kind": "Match",
      "from": ["Log"],
      "to": "Number",
      "enabled": true,
      "settings": {
        "from_label": "log",
        "pattern": "({word}): ({number})",
        "replacement": "$2",
        "to_label": "$1",
        "weakest_level": "NONE"
      }
    }
  ],
  "graphs": [
    { "kind": "Time series", "settings": { "time_range": 30 } }
  ]
}
```

| Field | |
|---|---|
| `version` | The Serval version that wrote it. Optional |
| `maps` | The maps, in order |
| `graphs` | The graphs, in order |

## Maps

| Field | |
|---|---|
| `kind` | The map's name, as in the add menu |
| `from` | Its input types: `"Bytes"`, `"Log"`, `"Number"`, `"Vector2d"`, `"Vector3d"` |
| `to` | Its output type |
| `enabled` | Whether it is on |
| `settings` | Its settings, below. Optional |

A map is told by `kind`, `from`, and `to` together (e.g. `Add` of two Numbers,
or of two Vector2ds).

| Kind | `settings` |
|---|---|
| Decode | `from_label`, `delimiters` (`"CR"`, `"LF"`, `"CRLF"`), `pattern`, `to_label`, `level`, `message` |
| Encode | `from_label`, `to_label`, `weakest_level` |
| Match | `from_label`, `pattern`, `to_label`, `replacement` (value, x, or level), `y_replacement` (y or message), `z_replacement` (z), `weakest_level` |
| Replace | `from_label`, `pattern`, `replacement`, `to_label`, `weakest_level` |
| Concat | `first_label`, `second_label`, `first_weakest_level`, `second_weakest_level`, `separator`, `level`, `to_label` |
| Add, Subtract, Multiply, Divide, Dot, Cross | `first`, `second` (a label, or a constant), `to_label` |
| Differentiate, Integrate, Magnitude | `from_label`, `to_label` |
| Compose | `x`, `y`, `z` (3D only; each a label, or a number), `to_label` |
| Decompose | `from_label`, `x_label`, `y_label`, `z_label` (3D only) |

Levels are written as `"ERROR"`, `"WARN"`, `"INFO"`, `"DEBUG"`, `"TRACE"`, or
`"NONE"`.

## Graphs

| Field | |
|---|---|
| `kind` | `"Time series"`, `"Box plot"`, `"Scatter"`, `"Arrows (2D)"`, `"Arrows (3D)"`, or `"Log"` |
| `settings` | Its settings, below. Optional |

All graphs share the same settings; each uses those of its kind.

| Setting | Values | Default | Used by |
|---|---|---|---|
| `title` | Text, or `null` | `null` (numbered) | All |
| `hidden` | Labels hidden in the legend | `[]` | All |
| `value_scale` | `"linear"`, `"log"` | `"linear"` | Time series, Box plot, Scatter (Y) |
| `x_scale` | `"linear"`, `"log"` | `"linear"` | Scatter |
| `x_label`, `y_label` | Number labels | `""` | Scatter |
| `regression` | `true`, `false` | `false` | Scatter |
| `draw_style` | `"points"`, `"linear"`, `"stepped"` | `"linear"` | Time series |
| `time_range` | Seconds, 10–300 (rounded to 10), or `"all"` (all the values kept) | `10` | Time series, Box plot, Scatter |
| `trails` | `true`, `false` | `false` | Arrows |
| `weakest_level` | A level | `"NONE"` | Log |

## Defaults

* A map's missing `settings`, or a setting missing from them, takes the value
  of a new map of that kind.
* A graph's takes the default in the table above: not the setting a graph
  added from the menu starts with (e.g. `trails` is on for Arrows added from
  the menu, but `false` when left out here).
* A missing `version` is taken as the running Serval's.

## Errors

Importing replaces all the maps and graphs; on an error, nothing changes and
the error is shown in a toast.

| Error | Cause |
|---|---|
| `Not a layout file: …` | Not JSON, a field missing, or a value of the wrong kind (e.g. an unknown level) |
| `No map <kind> from <types> to <type>` | No map of that `kind`, `from`, and `to` |
| `No graph <kind>` | No graph of that `kind` |
| `<kind>: '<setting>' …` | A setting a map cannot take |

## Initial layout

Serval starts with:

| | |
|---|---|
| Decode | `raw_bytes` → `log`, by `[... level ...] message` |
| Match | `log` → `anonymous data`, by `^{number}` |
| Match | `log` → `$1`, by `({word}): ({number})` |
| Time series | Default settings |

See [Concept / Maps](../concept/maps.md#the-default-layout).
