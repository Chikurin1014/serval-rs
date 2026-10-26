# Data

## Labels

* Any non-empty text.
* Each holds values of one [type](#types), each with the time it arrived (ms
  since the Unix epoch).
* Pushing a value of another type to a label replaces it.

| Label | |
|---|---|
| `raw_bytes` | What the port receives. Hidden in the data list at first |
| `log` | The lines of the default Decode map |
| `anonymous data` | The default Match map's numbers with no name |

## Types

| Type | Value | As text (data list, CSV) |
|---|---|---|
| **Bytes** | Bytes | Uppercase hex: `6F 6B 0D 0A` |
| **Log** | A message, with a [level](#log-levels) | `[WARN] battery low`; the message alone for `NONE` |
| **Number** | A 64-bit float | `22.5` |
| **Vector2d** | Two numbers | `(1.5, -2)` |
| **Vector3d** | Three numbers | `(1.5, -2, 0.25)` |

## Retention

* Each label keeps its newest 10,000 values, dropping the oldest.
* **Clear all** forgets all the labels; the delete button on a row, that label.

## Number format

| Where | Format |
|---|---|
| Data list, graphs, map cards | 5 significant digits: `4.8950`, `1.2346e5` |
| CSV export | Exact: `4.895` |

## Log levels

From the one that matters most:

| Level | Read from (any case) |
|---|---|
| `ERROR` | `error` |
| `WARN` | `warn`, `warning` |
| `INFO` | `info` |
| `DEBUG` | `debug` |
| `TRACE` | `trace` |
| `NONE` | Empty, or `none` |

A "weakest level" setting keeps the logs of that level and those that matter
more; `NONE` keeps all.

## Data list

* Shows the labels the [label filters](patterns.md#label-filters) let through.
* A row opens to show the 4 values before its newest.
