# About Rune

[Rune](https://rune-rs.github.io/) is a scripting language for Rust: dynamically
typed, with a syntax close to Rust's, and async functions.
Serval compiles a plugin as it is imported, and runs it in the page, on any
platform.

Some differences from Rust, as plugins meet them:

* No types are written: `fn add(a, b) { a + b }`.
* Objects are written `#{ key: value }`, and tuples and vectors as in Rust.
* `?` works on both `Result` and `Option`; `println!` and `format!` are there.

See [The Rune Programming Language](https://rune-rs.github.io/book/) for the
language, and the [API Reference](api.md) for its standard library (`std`)
with Serval's modules.
