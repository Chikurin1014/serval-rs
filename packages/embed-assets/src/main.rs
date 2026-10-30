//! `embed-assets <binary> <out>`: writes the binary to `out` with the
//! `assets` folder beside it, so the app is one file.

use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [binary, out] = args.as_slice() else {
        eprintln!("Usage: embed-assets <binary> <out>");
        return ExitCode::FAILURE;
    };
    let binary = Path::new(binary);
    let root = binary.parent().unwrap_or(Path::new("."));
    match embed_assets::embed(binary, root, "assets", Path::new(out)) {
        Ok(count) => {
            println!("Embedded {count} assets in {out}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to embed the assets: {error}");
            ExitCode::FAILURE
        }
    }
}
