use chrono::{Duration, Local, NaiveDate};
use colored::Colorize;

use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::error::{Result, WitError};
use crate::git::repo::WitRepo;
use crate::query::parse_time_token;
use crate::weather::api::WeatherClient;
use crate::weather::snapshot::LocationMeta;

pub fn run(location: &str, since: &str) -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config = WitConfig::load(&wit_path.join("wit.toml"))
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    let slug = slug::slugify(location);
    let loc = config
        .locations
        .iter()
        .find(|l| l.slug == slug)
        .ok_or_else(|| WitError::LocationNotFound(location.to_string()))?;

    let meta_path = wit_path.join("locations").join(&slug).join("meta.toml");
    let meta_str = std::fs::read_to_string(&meta_path)
        .map_err(|e| anyhow::anyhow!("failed to read meta: {}", e))?;
    let meta: LocationMeta = toml::from_str(&meta_str)
        .map_err(|e| anyhow::anyhow!("failed to parse meta: {}", e))?;

    // Parse the since date
    let start_date = parse_since(since)?;
    let end_date = Local::now().date_naive() - Duration::days(1); // yesterday (today is "current")

    if start_date >= end_date {
        return Err(WitError::Other(anyhow::anyhow!(
            "nothing to backfill — start date {} is not before yesterday",
            start_date
        )));
    }

    let total_days = (end_date - start_date).num_days();
    println!(
        "  Backfilling {} days for {}...",
        total_days.to_string().cyan(),
        loc.name.bold()
    );

    let imperial = config.settings.units == "imperial";
    let client = WeatherClient::new(imperial);

    // Open-Meteo historical API handles up to ~a year per request.
    // Chunk into 90-day windows to be safe.
    let repo = WitRepo::open(&wit_path)?;
    let loc_dir = wit_path.join("locations").join(&slug);
    let mut committed = 0;

    let mut chunk_start = start_date;
    while chunk_start <= end_date {
        let chunk_end = (chunk_start + Duration::days(89)).min(end_date);

        let snapshots = client
            .fetch_historical_range(meta.latitude, meta.longitude, chunk_start, chunk_end)
            .map_err(|e| WitError::Api(e.to_string()))?;

        for (date, snap) in &snapshots {
            let snap_toml = snap
                .to_toml()
                .map_err(|e| anyhow::anyhow!("serialize error: {}", e))?;
            std::fs::write(loc_dir.join("current.toml"), &snap_toml)
                .map_err(|e| anyhow::anyhow!("write error: {}", e))?;

            // Commit with backdated timestamp (noon on that day)
            let epoch = date
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
                .timestamp();

            let summary = snap.summary_line(&config.settings.units);
            let message = format!("snap: {} {}", loc.name, summary);
            repo.commit_all_at(&message, epoch)
                .map_err(|e| anyhow::anyhow!("commit error: {}", e))?;

            committed += 1;
        }

        print!(
            "\r  {} {}/{} days",
            "...".dimmed(),
            committed,
            total_days
        );

        chunk_start = chunk_end + Duration::days(1);
    }

    println!(
        "\r  {} Backfilled {} snapshots for {}",
        "✓".green(),
        committed.to_string().cyan(),
        loc.name.bold()
    );

    Ok(())
}

fn parse_since(s: &str) -> Result<NaiveDate> {
    // Try as a date first: 2025-01-01
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date);
    }
    // Try as a time spec: 30d, 3m, 1y
    let spec = parse_time_token(s)?;
    Ok(spec.resolve())
}
