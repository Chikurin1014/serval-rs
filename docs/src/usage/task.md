# Plugin (Task Tab)

A plugin is a [Rune](https://rune-rs.github.io/) file of your own: each of its public async functions becomes a **task**.

See [Concept / Tasks](../concept/tasks.md).

## Importing a plugin

Click the plugin button at the right end of the navigation bar and pick a `.rn` file.

Its tasks appear in the **Task** tab, which appears once there is at least one task.
After changing the file, import it again to replace its tasks.

If it does not compile, a dialog shows its errors.

## Running tasks

![The Task tab](../media/task.png)

The plugin in this screenshot:

```rust
use serval::{data, time};

/// Pushes a sine wave to `wave`, 20 times a second.
pub async fn wave() {
    let start = time::Instant::now();
    loop {
        let t = start.elapsed().as_secs_f64();
        data::push("wave", t.sin())?;
        time::sleep(time::Duration::from_millis(50)).await?;
    }
}

/// Prints each temperature the device sends.
pub async fn watch_temp() {
    let temps = data::get("temp")?;
    while let Some(temp) = temps.next().await {
        println!("temp: {temp}");
    }
}
```

* Turn a task on with its switch to run it, and off to stop it.
  **It turns off by itself when it ends.**
* Its card shows the last line it printed, and what it returned or the error
  it ended with. Click its title to see its function.
* **STDOUT**, beside the tasks, shows everything the tasks print.
