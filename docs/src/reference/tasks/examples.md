# Examples

Each example is a whole plugin: save it as a `.rn` file and import it.

| Example | Uses |
|---|---|
| [Polling a sensor](#polling-a-sensor) | `serial::send`, `serial::wait_for`, `data::push` |
| [Moving average](#moving-average) | `data::get`, `data::push` |
| [Threshold alert](#threshold-alert) | `data::get`, `data::LogEntry` |
| [Binary frames](#binary-frames) | `serial::received`, `data::push` |
| [Restarting and configuring a board](#restarting-and-configuring-a-board) | `serial::restart`, `serial::wait_for` |
| [Test data without a device](#test-data-without-a-device) | `time`, `rand`, `data::push` |

## Polling a sensor

For a device that answers a command, rather than sending on its own.

```rust
use serval::{data, serial, time};

/// Asks for the temperature each second (`TEMP?`), and pushes the number it
/// answers to `temp`.
pub async fn poll_temp() {
    loop {
        serial::send("TEMP?\r\n")?;
        // All it received until the answer: the answer is its last line
        let reply = String::from_utf8(serial::wait_for("\r\n").await?)?;
        let last_line = reply.trim().split("\n").collect::<Vec>().pop()?;
        data::push("temp", last_line.trim().parse::<f64>()?)?;
        time::sleep(time::Duration::from_secs(1)).await?;
    }
}
```

## Moving average

```rust
use serval::data;
use std::collections::VecDeque;

/// Pushes the mean of the last 10 temperatures to `temp mean`.
pub async fn moving_average() {
    let temps = data::get("temp")?;
    let window = VecDeque::new();
    while let Some(temp) = temps.next().await {
        window.push_back(temp);
        if window.len() > 10 {
            window.pop_front();
        }
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        data::push("temp mean", mean)?;
    }
}
```

## Threshold alert

Pushes a log, which the [Log graph](../graphs.md#log) shows.

```rust
use serval::data;

/// Pushes a warning to `alerts` each time the voltage drops below 3.3 V.
pub async fn low_voltage() {
    let volts = data::get("volt")?;
    let low = false;
    while let Some(volt) = volts.next().await {
        if volt < 3.3 && !low {
            let message = format!("low voltage: {} V", volt);
            data::push("alerts", data::LogEntry::new(Some("WARN"), message))?;
        }
        low = volt < 3.3;
    }
}
```

## Binary frames

For a device that sends binary frames rather than lines.

```rust
use serval::{data, serial};

/// Takes frames of STX (`0x02`), a payload, and ETX (`0x03`) from what the
/// port receives, and pushes the first two bytes of each payload, as a
/// big-endian integer, to `raw value`.
pub async fn frames() {
    let chunks = serial::received()?;
    let frame = None;
    while let Some(chunk) = chunks.next().await {
        for byte in chunk.into_vec() {
            // A byte is a `u8`: as an `i64`, for the integer patterns to match
            match byte as i64 {
                0x02 => frame = Some([]),
                0x03 => {
                    if let Some([high, low, ..]) = frame {
                        data::push("raw value", high * 256 + low)?;
                    }
                    frame = None;
                },
                value => if let Some(payload) = frame {
                    payload.push(value);
                },
            }
        }
    }
}
```

## Restarting and configuring a board

```rust
use serval::{serial, time};

/// Restarts an ESP32, waits for it to start, then sets its sample rate.
pub async fn reboot() {
    serial::restart("espressif").await?;
    serial::wait_for("ready").await?;
    time::sleep(time::Duration::from_millis(100)).await?;
    serial::send("rate 100\r\n")?;
    serial::flush().await?;
    "configured"
}
```

## Test data without a device

For trying maps and graphs.

```rust
use serval::{data, rand, time};

/// Pushes a noisy sine wave to `wave`, and a turning vector to `spin`, 20
/// times a second.
pub async fn test_data() {
    let start = time::Instant::now();
    loop {
        let t = start.elapsed().as_secs_f64();
        data::push("wave", t.sin() + rand::random_range(-0.1, 0.1))?;
        data::push("spin", (t.cos(), t.sin(), 0.5))?;
        time::sleep(time::Duration::from_millis(50)).await?;
    }
}
```
