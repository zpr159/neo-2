use crate::error::CliResult;

pub fn run() -> CliResult<()> {
    println!("neo {}", crate::config::VERSION);
    println!("  Rust version: {}", option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown"));
    println!("  Platform:     {}-{}", std::env::consts::OS, std::env::consts::ARCH);
    println!("  Build:        {}", if cfg!(debug_assertions) { "debug" } else { "release" });
    Ok(())
}
