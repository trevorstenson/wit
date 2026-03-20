use colored::Colorize;

use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::error::{Result, WitError};
use crate::git::repo::WitRepo;
use crate::weather::api::WeatherClient;
use crate::weather::snapshot::LocationMeta;

pub fn run(location: &str) -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config_path = wit_path.join("wit.toml");
    let mut config = WitConfig::load(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    let imperial = config.settings.units == "imperial";
    let client = WeatherClient::new(imperial);

    // Geocode
    println!("  Searching for {}...", location.cyan());
    let results = client.geocode(location)
        .map_err(|e| WitError::Api(e.to_string()))?;

    if results.is_empty() {
        return Err(WitError::LocationNotFound(location.to_string()));
    }

    // If multiple results, pick the first but show alternatives
    let geo = &results[0];
    if results.len() > 1 {
        println!("  Using: {}", geo.display_name().green());
        println!("  {}", "Other matches:".dimmed());
        for alt in results.iter().skip(1).take(3) {
            println!("    - {}", alt.display_name().dimmed());
        }
    }

    let location_slug = slug::slugify(&geo.name);

    if config.has_location(&location_slug) {
        return Err(WitError::LocationExists(geo.display_name()));
    }

    // Create location directory
    let loc_dir = wit_path.join("locations").join(&location_slug);
    std::fs::create_dir_all(&loc_dir)
        .map_err(|e| anyhow::anyhow!("failed to create location dir: {}", e))?;

    // Write meta.toml
    let meta = LocationMeta {
        name: geo.display_name(),
        slug: location_slug.clone(),
        latitude: geo.latitude,
        longitude: geo.longitude,
        timezone: geo.timezone.clone().unwrap_or_else(|| "UTC".to_string()),
        country: geo.country.clone().unwrap_or_default(),
    };
    let meta_toml = toml::to_string_pretty(&meta)
        .map_err(|e| anyhow::anyhow!("failed to serialize meta: {}", e))?;
    std::fs::write(loc_dir.join("meta.toml"), &meta_toml)
        .map_err(|e| anyhow::anyhow!("failed to write meta.toml: {}", e))?;

    // Fetch initial snapshot
    println!("  Fetching weather...");
    let snapshot = client
        .fetch_current(geo.latitude, geo.longitude)
        .map_err(|e| WitError::Api(e.to_string()))?;

    let snap_toml = snapshot.to_toml()
        .map_err(|e| anyhow::anyhow!("failed to serialize snapshot: {}", e))?;
    std::fs::write(loc_dir.join("current.toml"), &snap_toml)
        .map_err(|e| anyhow::anyhow!("failed to write snapshot: {}", e))?;

    // Update config
    config.add_location(location_slug, geo.display_name());
    config.save(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to save config: {}", e))?;

    // Commit
    let repo = WitRepo::open(&wit_path)?;
    let summary = snapshot.summary_line(&config.settings.units);
    repo.commit_all(&format!("add: {} ({})", geo.display_name(), summary))
        .map_err(|e| anyhow::anyhow!("git commit failed: {}", e))?;

    println!(
        "  {} Tracking {} — {}",
        "✓".green(),
        geo.display_name().bold(),
        summary
    );

    Ok(())
}
