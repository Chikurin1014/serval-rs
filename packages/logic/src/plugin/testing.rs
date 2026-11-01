//! For the tests of plugins: run off the page, with nothing to wait on.

use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

use super::{compile_plugin, run_task};

/// Runs `future`, which must not wait on anything outside it.
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// Compiles `source` and runs its task `name`: what it returns, formatted.
pub fn run(source: &str, name: &str) -> Result<Option<String>, String> {
    let plugin = compile_plugin("test.rn", source)?;
    block_on(run_task(&plugin, name, |_| {}))
}

/// Runs `body` as a task of its own: what it returns, formatted.
pub fn run_body(body: &str) -> Result<Option<String>, String> {
    run(&format!("pub async fn body() {{ {body} }}\n"), "body")
}
