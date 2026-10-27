//! Rune's own CLI, with the modules plugins have (`serval::*`): `serval-rune doc
//! --output <dir>`, in the empty workspace here (`Rune.toml`), builds their API
//! reference of their docstrings.

use rune::{
    Context, ContextError,
    cli::{ContextOptions, Entry},
    modules::capture_io,
};

fn context(options: ContextOptions<'_>) -> Result<Context, ContextError> {
    // As plugins have it: what they print taken, if it is to be
    let mut context = Context::with_config(options.capture.is_none())?;
    if let Some(io) = options.capture {
        context.install(capture_io::module(io)?)?;
    }
    for module in ui::plugin::rune_modules()? {
        context.install(module)?;
    }
    Ok(context)
}

fn main() {
    Entry::new()
        .about("Rune, with Serval's modules (`serval::*`)")
        .context(&mut context)
        .run();
}
