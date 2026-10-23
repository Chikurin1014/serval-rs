# Plugins

Plugins are [Rune](https://rune-rs.github.io/) files: each public async
function of one is a task, run as it is turned on in the **Task** tab.

```rust
use serval::{data, time};

pub async fn ticks() {
    let start = time::Instant::now();
    loop {
        data::push("t", start.elapsed().as_millis())?;
        time::sleep(time::Duration::from_millis(100)).await?;
    }
}
```
