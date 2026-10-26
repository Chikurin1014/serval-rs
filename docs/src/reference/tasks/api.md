# API Reference

The full reference is generated of their docstrings:
[Serval's modules](../../api/serval.module.html), and
[Rune's standard library](../../api/std.module.html) (`std`).

Serval's modules:

| Module | |
|---|---|
| `serval::data` | The data: `get` (a label's values, as a stream), `get_with_time`, `push` |
| `serval::serial` | The port: `send`, `flush`, `received`, `wait_for`, `open`, `close`, the baud rate, DTR and RTS, `restart` |
| `serval::time` | `sleep`, `sleep_until`, `Instant` and `Duration` |
| `serval::rand` | Pseudo random numbers: `random`, `random_range`, `random_bool`, `Rng` |

Floats have the methods of Rust's `f64` that Rune's own lack, such as
`x.sin()` and `f64::PI()`.
