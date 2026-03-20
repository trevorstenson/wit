use colored::Colorize;

use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::error::{Result, WitError};
use crate::git::repo::WitRepo;
use crate::weather::api::WeatherClient;
use crate::weather::snapshot::{LocationMeta, WeatherSnapshot};

pub fn run() -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config_path = wit_path.join("wit.toml");
    let config = WitConfig::load(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    if config.locations.is_empty() {
        return Err(WitError::NoLocations);
    }

    let imperial = config.settings.units == "imperial";
    let client = WeatherClient::new(imperial);

    let mut summaries: Vec<String> = Vec::new();
    let mut snapshots: Vec<(String, WeatherSnapshot)> = Vec::new();

    for loc in &config.locations {
        let loc_dir = wit_path.join("locations").join(&loc.slug);
        let meta_path = loc_dir.join("meta.toml");

        let meta_str = std::fs::read_to_string(&meta_path)
            .map_err(|e| anyhow::anyhow!("failed to read meta for {}: {}", loc.slug, e))?;
        let meta: LocationMeta = toml::from_str(&meta_str)
            .map_err(|e| anyhow::anyhow!("failed to parse meta for {}: {}", loc.slug, e))?;

        print!("  Fetching {}...", loc.name);
        match client.fetch_current(meta.latitude, meta.longitude) {
            Ok(snapshot) => {
                let snap_toml = snapshot.to_toml()
                    .map_err(|e| anyhow::anyhow!("serialize error: {}", e))?;
                std::fs::write(loc_dir.join("current.toml"), &snap_toml)
                    .map_err(|e| anyhow::anyhow!("write error: {}", e))?;

                let summary = snapshot.summary_line(&config.settings.units);
                println!(" {}", summary.green());
                summaries.push(format!("{} {}", loc.name, summary));
                snapshots.push((loc.name.clone(), snapshot));
            }
            Err(e) => {
                println!(" {}", format!("error: {}", e).red());
            }
        }
    }

    if summaries.is_empty() {
        println!("  {} No snapshots fetched", "!".yellow());
        return Ok(());
    }

    // Commit
    let repo = WitRepo::open(&wit_path)?;
    let message = format!("snap: {}", summaries.join(" | "));
    repo.commit_all(&message)
        .map_err(|e| anyhow::anyhow!("git commit failed: {}", e))?;

    println!(
        "\n  {} Committed {} location{}",
        "✓".green(),
        snapshots.len(),
        if snapshots.len() == 1 { "" } else { "s" }
    );

    Ok(())
}
