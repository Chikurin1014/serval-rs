# About Rune

[Rune](https://rune-rs.github.io/) is a scripting language for Rust: dynamically
typed, with a syntax close to Rust's, and async functions.
Serval compiles a plugin as it is imported, and runs it in the page, on any
platform.

Some differences from Rust, as plugins meet them:

- No types are written: `fn add(a, b) { a + b }`.
- Objects are written `#{ key: value }`, and tuples and vectors as in Rust.
- `?` works on both `Result` and `Option`; `println!` and `format!` are there.
- Numbers are converted with `as`: `len as f64` (there is no `.to_f64()`).
- `Bytes` are iterated through a vector: `for byte in bytes.into_vec()`.
- Integer patterns of `match` match integers of their own type alone: a byte
  (`u8`) is matched as `byte as i64`.
- A block arm of `match` takes a comma after it: `0 => { … },`.
- There is no `last()` nor negative index: take the last element with
  `pop()`.
- A pattern does not take the name of what it matches: `if let Some(x) = x`
  does not compile; name it otherwise.

> [!NOTE]
> The modules of [`rune-modules`](https://docs.rs/rune-modules) (`time`, `fs`,
> `process`, `http`, …) are not available: many need an OS or Tokio, which a
> web page doesn't have. Use these instead:
>
> | Instead of                           | Use                                                                                      |
> | ------------------------------------ | ---------------------------------------------------------------------------------------- |
> | `time`                               | [`serval::time`](../../api/serval/time.module.html)                                      |
> | `rand`                               | [`serval::rand`](../../api/serval/rand.module.html)                                      |
> | `fs`, `process`, `http`, `signal`, … | None: talk to the device through [`serval::serial`](../../api/serval/serial.module.html) |

See [The Rune Programming Language](https://rune-rs.github.io/book/) for the
language, and the [API Reference](api.md) for its standard library (`std`)
with Serval's modules.
