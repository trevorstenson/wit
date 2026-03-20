use colored::Colorize;

use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::error::{Result, WitError};
use crate::git::repo::WitRepo;
use crate::weather::codes::describe_weather_code;
use crate::weather::snapshot::WeatherSnapshot;

pub fn run(location: Option<&str>, count: usize) -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config = WitConfig::load(&wit_path.join("wit.toml"))
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    let repo = WitRepo::open(&wit_path)?;

    // If location specified, filter to that location's file
    let (path_filter, display_name) = if let Some(loc) = location {
        let slug = slug::slugify(loc);
        let entry = config
            .locations
            .iter()
            .find(|l| l.slug == slug)
            .ok_or_else(|| WitError::LocationNotFound(loc.to_string()))?;
        (
            Some(format!("locations/{}/current.toml", slug)),
            entry.name.clone(),
        )
    } else {
        (None, "all locations".to_string())
    };

    let history = repo
        .walk_history(path_filter.as_deref(), count)
        .map_err(|e| anyhow::anyhow!("failed to walk history: {}", e))?;

    if history.is_empty() {
        return Err(WitError::NoSnapshots);
    }

    println!();
    println!("  {} for {}", "log".bold(), display_name.cyan());
    println!();

    for entry in &history {
        let short_oid = format!("{}", entry.oid).chars().take(7).collect::<String>();
        let date = entry.timestamp.format("%Y-%m-%d %H:%M");

        print!("  {} ", short_oid.yellow());
        print!("{} ", date.to_string().dimmed());

        // Try to read and display snapshot data if location-filtered
        if let Some(ref path) = path_filter {
            if let Ok(content) = repo.read_file_at_commit(entry.oid, path) {
                if let Ok(snap) = WeatherSnapshot::from_toml(&content) {
                    let (_, emoji) = describe_weather_code(snap.snapshot.weather_code);
                    let units = &config.settings.units;
                    let unit = if units == "metric" { "C" } else { "F" };
                    print!(
                        "{} {:.0}{} {} ",
                        emoji,
                        snap.temperature.current,
                        unit,
                        snap.snapshot.weather_description
                    );
                }
            }
        }

        // Show commit message (trimmed)
        let msg = entry.message.lines().next().unwrap_or("");
        println!("{}", msg.dimmed());
    }

    println!();
    Ok(())
}
