# API Reference

The full reference is generated from their docstrings:
[Serval's modules](../../api/serval.module.html), and
[Rune's standard library](../../api/std.module.html) (`std`).

Serval's modules:

| Module | |
|---|---|
| [`serval::data`](../../api/serval/data.module.html) | The data: `get` (a label's values, as a stream), `get_with_time`, `push` |
| [`serval::serial`](../../api/serval/serial.module.html) | The port: `send`, `flush`, `received`, `wait_for`, `open`, `close`, `PortSettings` (`get_settings`, `set_settings`), the line ending, DTR and RTS, `restart` |
| [`serval::time`](../../api/serval/time.module.html) | `sleep`, `sleep_until`, `Instant` and `Duration` |
| [`serval::rand`](../../api/serval/rand.module.html) | Pseudo random numbers: `random`, `random_range`, `random_bool`, `Rng` |
| [`serval::math`](../../api/serval/math.module.html) | Methods of `f64` that Rune's own lacks, such as `x.sin()` and `f64::PI()` |
