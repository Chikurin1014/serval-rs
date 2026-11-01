# Data Types

Each label holds values of one type, each value with the time it arrived.
A label keeps a limited number of its newest values, dropping the oldest as new ones arrive
(see [Reference / Data](../reference/data.md#retention)).

| Type         | Value                                  | e.g.                                 |
| ------------ | -------------------------------------- | ------------------------------------ |
| **Bytes**    | Bytes, as they arrived                 | What the port receives (`raw_bytes`) |
| **Log**      | A message, with a [level](#log-levels) | `[WARN] battery low`                 |
| **Number**   | A number (64-bit float)                | `22.5`                               |
| **Vector2d** | Two numbers, `(x, y)`                  | A position on a plane                |
| **Vector3d** | Three numbers, `(x, y, z)`             | An acceleration                      |

## Log levels

From the one that matters most:

`ERROR` > `WARN` > `INFO` > `DEBUG` > `TRACE` > `NONE`

`NONE` means a line with no level.
Maps and graphs that take logs can leave out logs weaker than a chosen level.
