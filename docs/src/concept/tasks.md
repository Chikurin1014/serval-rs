# Tasks

If maps are not enough, you can write a script of your own: a **plugin**,
written in [Rune](https://rune-rs.github.io/). \
Its public async functions become **tasks**, each of which runs when you turn it on.

A task can:

* **send** to the port, and **read** what it receives,
* **open and close the port**, set its settings (baud rate and the like),
  DTR and RTS, and restart the board,
* **read** the data of labels as they arrive, and **push** values to labels,
* **wait**, and keep time.

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

A function that takes arguments or is not public is not a task, but tasks can
call it.

See [Usage / Plugin](../usage/task.md) to run them, and
[Reference / Tasks](../reference/tasks/index.md) for what they can use.
