# Tasks

A plugin is a [Rune](rune.md) file (`.rn`).
Each top-level `pub async fn` of it that takes no arguments is a task.

```rust
use serval::{data, serial, time};

/// Pings the device each second, and keeps how long it takes to answer.
pub async fn ping() {
    loop {
        let sent = time::Instant::now();
        serial::send("ping\n")?;
        serial::wait_for("pong").await?;
        data::push("ping ms", sent.elapsed().as_millis())?;
        time::sleep(time::Duration::from_secs(1)).await?;
    }
}
```

- A task runs until its function returns, an error stops it (e.g. one of `?`),
  or it is turned off.
- What it returns is shown in its card; what it prints (`println!`) in
  **STDOUT**.
- A loop with no `.await` would freeze the page: a task that runs too long
  without one is stopped (see [Runtime](runtime.md#limits)).

What a task can use: [About Rune](rune.md) for the language and its standard
library, [Examples](examples.md) of whole plugins, and the [API Reference](api.md)
for Serval's modules (`serval::*`).
