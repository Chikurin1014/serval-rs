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

| Field     |                                            |
| --------- | ------------------------------------------ |
| `version` | The Serval version that wrote it. Optional |
| `maps`    | The maps, in order                         |
| `graphs`  | The graphs, in order                       |

## Maps

| Field      |                                                                             |
| ---------- | --------------------------------------------------------------------------- |
| `kind`     | The map's name, as in the add menu                                          |
| `from`     | Its input types: `"Bytes"`, `"Log"`, `"Number"`, `"Vector2d"`, `"Vector3d"` |
| `to`       | Its output type                                                             |
| `enabled`  | Whether it is on                                                            |
| `settings` | Its settings, below. Optional                                               |

A map is told by `kind`, `from`, and `to` together (e.g. `Add` of two Numbers,
or of two Vector2ds).

| Kind                                             | `settings`                                                                                                                                    |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Decode                                           | `from_label`, `delimiters` (`"CR"`, `"LF"`, `"CRLF"`), `pattern`, `to_label`, `level`, `message`                                              |
| Encode                                           | `from_label`, `to_label`, `weakest_level`                                                                                                     |
| Match                                            | `from_label`, `pattern`, `to_label`, `replacement` (value, x, or level), `y_replacement` (y or message), `z_replacement` (z), `weakest_level` |
| Replace                                          | `from_label`, `pattern`, `replacement`, `to_label`, `weakest_level`                                                                           |
| Concat                                           | `first_label`, `second_label`, `first_weakest_level`, `second_weakest_level`, `separator`, `level`, `to_label`                                |
| Add, Subtract, Multiply, Divide, Pow, Dot, Cross | `first`, `second` (a label, a number, or a [constant](maps.md#constants) as `#pi`), `to_label`                                                |
| Differentiate, Integrate, Magnitude              | `from_label`, `to_label`                                                                                                                      |
| Compose                                          | `x`, `y`, `z` (3D only; each a label, or a number), `to_label`                                                                                |
| Decompose                                        | `from_label`, `x_label`, `y_label`, `z_label` (3D only)                                                                                       |

Levels are written as `"ERROR"`, `"WARN"`, `"INFO"`, `"DEBUG"`, `"TRACE"`, or
`"NONE"`.

## Graphs

| Field      |                                                                                          |
| ---------- | ---------------------------------------------------------------------------------------- |
| `kind`     | `"Time series"`, `"Box plot"`, `"Scatter"`, `"Arrows (2D)"`, `"Arrows (3D)"`, or `"Log"` |
| `settings` | Its settings, below. Optional                                                            |

Each graph has the settings of every kind, and those of its own kind. An
export writes those alone; an import ignores those of other kinds.

### Every kind

| Setting  | Values                                         | Default           |
| -------- | ---------------------------------------------- | ----------------- |
| `title`  | Text, or `null`                                | `null` (numbered) |
| `hidden` | Labels hidden in the legend (Scatter has none) | `[]`              |

### Time series

| Setting       | Values                                                            | Default    |
| ------------- | ----------------------------------------------------------------- | ---------- |
| `value_scale` | `"linear"`, `"log"`                                               | `"linear"` |
| `draw_style`  | `"points"`, `"linear"`, `"stepped"`                               | `"linear"` |
| `time_range`  | Seconds, 10–300 (rounded to 10), or `"all"` (all the values kept) | `10`       |

### Box plot

| Setting       | Values              | Default    |
| ------------- | ------------------- | ---------- |
| `value_scale` | `"linear"`, `"log"` | `"linear"` |
| `time_range`  | As a Time series's  | `10`       |

### Scatter

| Setting              | Values                            | Default    |
| -------------------- | --------------------------------- | ---------- |
| `x_label`, `y_label` | Number labels                     | `""`       |
| `x_scale`            | `"linear"`, `"log"`               | `"linear"` |
| `value_scale`        | The Y axis's: `"linear"`, `"log"` | `"linear"` |
| `regression`         | `true`, `false`                   | `false`    |
| `time_range`         | As a Time series's                | `10`       |

### Arrows (2D), Arrows (3D)

| Setting  | Values          | Default |
| -------- | --------------- | ------- |
| `trails` | `true`, `false` | `false` |

### Log

| Setting         | Values  | Default  |
| --------------- | ------- | -------- |
| `weakest_level` | A level | `"NONE"` |

## Defaults

- A map's missing `settings`, or a setting missing from them, takes the value
  of a new map of that kind.
- A graph's takes the default in its table above: not the setting a graph
  added from the menu starts with (e.g. `trails` is on for Arrows added from
  the menu, but `false` when left out here).
- A missing `version` is taken as the running Serval's.

## Older layout files

A layout file of an earlier `version` (0.4.0 or later) is taken as of this
one, by each change of the format since:

| Up to 0.4.1                                                                                        | Taken as              |
| -------------------------------------------------------------------------------------------------- | --------------------- |
| `"Linear"`, `"Log"` (`value_scale`, `x_scale`), `"Points"`, `"Linear"`, `"Stepped"` (`draw_style`) | In lower case         |
| `time_window` of a Time series                                                                     | Its `time_range`      |
| Box plot, Scatter (of all the values then)                                                         | `"time_range": "all"` |

## Errors

Importing replaces all the maps and graphs; on an error, nothing changes and
the error is shown in a toast.

| Error                                    | Cause                                                 |
| ---------------------------------------- | ----------------------------------------------------- |
| `Not a layout file: …`                   | Not JSON, or a field missing or of the wrong type     |
| `No map <kind> from <types> to <type>`   | No map of that `kind`, `from`, and `to`               |
| `No graph <kind>`                        | No graph of that `kind`                               |
| `Of a newer Serval, <version>: …`        | Exported by a later Serval than the one running       |
| `No version '<version>'`                 | A `version` not of the form `major.minor.patch`       |
| `No layout file of version <version>: …` | A `version` before 0.4.0, which had no layout files   |
| `<kind>: '<setting>' …`                  | A setting a map cannot take                           |
| `<kind>: …`                              | A setting a graph cannot take (e.g. an unknown level) |

## Initial layout

Serval starts with:

|             |                                                   |
| ----------- | ------------------------------------------------- |
| Decode      | `raw_bytes` → `log`, by `[... level ...] message` |
| Match       | `log` → `anonymous data`, by `^{number}`          |
| Match       | `log` → `$1`, by `({word}): ({number})`           |
| Time series | Default settings                                  |

See [Concept / Maps](../concept/maps.md#the-default-layout).
