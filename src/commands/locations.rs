use colored::Colorize;

use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::error::{Result, WitError};
use crate::weather::snapshot::LocationMeta;

pub fn run() -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config = WitConfig::load(&wit_path.join("wit.toml"))
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    if config.locations.is_empty() {
        return Err(WitError::NoLocations);
    }

    println!();
    println!("  {}", "Tracked locations".bold());
    println!();

    for loc in &config.locations {
        let meta_path = wit_path
            .join("locations")
            .join(&loc.slug)
            .join("meta.toml");

        if let Ok(content) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = toml::from_str::<LocationMeta>(&content) {
                println!(
                    "  {} {} ({:.2}, {:.2}) {}",
                    "•".cyan(),
                    meta.name.bold(),
                    meta.latitude,
                    meta.longitude,
                    meta.timezone.dimmed()
                );
            } else {
                println!("  {} {}", "•".cyan(), loc.name);
            }
        } else {
            println!("  {} {}", "•".cyan(), loc.name);
        }
    }

    println!();
    Ok(())
}
