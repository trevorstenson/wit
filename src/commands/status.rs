use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::display::format::print_status_table;
use crate::error::{Result, WitError};
use crate::weather::snapshot::WeatherSnapshot;

pub fn run(location: Option<&str>) -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config = WitConfig::load(&wit_path.join("wit.toml"))
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    if config.locations.is_empty() {
        return Err(WitError::NoLocations);
    }

    let mut entries: Vec<(String, WeatherSnapshot)> = Vec::new();

    for loc in &config.locations {
        if let Some(filter) = location {
            let filter_slug = slug::slugify(filter);
            if loc.slug != filter_slug {
                continue;
            }
        }

        let snap_path = wit_path
            .join("locations")
            .join(&loc.slug)
            .join("current.toml");

        if snap_path.exists() {
            let content = std::fs::read_to_string(&snap_path)
                .map_err(|e| anyhow::anyhow!("failed to read snapshot: {}", e))?;
            if let Ok(snap) = WeatherSnapshot::from_toml(&content) {
                entries.push((loc.name.clone(), snap));
            }
        }
    }

    if entries.is_empty() {
        return Err(WitError::NoSnapshots);
    }

    print_status_table(&entries, &config.settings.units);

    Ok(())
}
