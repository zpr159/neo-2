use crate::error::CliResult;

pub fn run() -> CliResult<()> {
    println!("Loaded Models");
    println!("=============");
    println!();
    println!("  Inference backend: {}", "default (cpu)");
    println!("  Status:            no models loaded");
    println!();
    println!("Use 'neo chat' or 'neo reasoning <query>' to interact with the system.");
    Ok(())
}
