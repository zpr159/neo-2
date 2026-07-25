use std::time::Instant;

use crate::config::{AppConfig, VERSION};

pub(crate) const BANNER: &str = r#"
    _   __     __  _____
   / | / /__  / /_/ ___/____  ________
  /  |/ / _ \/ __/\__ \/ __ \/ ___/ _ \
 / /|  /  __/ /_ ___/ / /_/ / /__/  __/
/_/ |_/\___/\__//____/ .___/\___/\___/
                     /_/
"#;

pub(crate) fn print_banner(config: &AppConfig, start_time: Instant) {
    let uptime = start_time.elapsed();
    let uptime_str = format_uptime(uptime);

    println!("{BANNER}");
    println!("  Version:      {VERSION}");
    println!("  Environment:  {}", config.core.environment);
    println!("  Debug:        {}", config.core.debug);
    println!("  Data dir:     {}", config.core.data_dir);
    println!("  Uptime:       {uptime_str}");
    println!();
}

pub(crate) fn print_startup_banner(config: &AppConfig) {
    println!("{BANNER}");
    println!("  Neo AGI OS v{VERSION}");
    println!("  Starting in {} mode...", config.core.environment);
    println!();
}

pub(crate) fn print_module_status(modules: &[(&str, bool)]) {
    println!("  Loaded modules:");
    for (name, loaded) in modules {
        let status = if *loaded { "  OK" } else { "FAIL" };
        let color = if *loaded { "\x1b[32m" } else { "\x1b[31m" };
        let reset = "\x1b[0m";
        println!("    {color}{status}{reset} {name}");
    }
    println!();
}

fn format_uptime(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_uptime_seconds() {
        assert_eq!(format_uptime(std::time::Duration::from_secs(30)), "30s");
    }

    #[test]
    fn format_uptime_minutes() {
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(125)),
            "2m 5s"
        );
    }

    #[test]
    fn format_uptime_hours() {
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(3661)),
            "1h 1m"
        );
    }

    #[test]
    fn format_uptime_days() {
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(90000)),
            "1d 1h"
        );
    }
}
