mod cli;
mod commands;
mod config;
mod display;
mod error;
mod git;
mod query;
mod weather;

use clap::Parser;
use colored::Colorize;

use crate::cli::{Cli, Command};
use crate::commands::init::default_wit_path;
use crate::config::WitConfig;
use crate::display::format::{print_current, print_diff};
use crate::error::WitError;
use crate::git::repo::WitRepo;
use crate::query::{parse_query, Query, TimeSpec};
use crate::weather::api::WeatherClient;
use crate::weather::snapshot::{LocationMeta, WeatherSnapshot};

fn main() {
    if let Err(e) = run() {
        eprintln!("  {} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init { path }) => commands::init::run(path),
        Some(Command::Add { location }) => commands::add::run(&location),
        Some(Command::Snap) | Some(Command::Fetch) => commands::snap::run(),
        Some(Command::Log { location, n }) => {
            commands::log::run(location.as_deref(), n)
        }
        Some(Command::Status { location }) => {
            commands::status::run(location.as_deref())
        }
        Some(Command::Locations) => commands::locations::run(),
        Some(Command::Backfill { location, since }) => {
            commands::backfill::run(&location, &since)
        }
        Some(Command::Diff { args }) => {
            let query = parse_query(&args)?;
            match query {
                Query::Current { .. } => {
                    eprintln!("  {} diff requires a comparison — try:", "error:".red().bold());
                    eprintln!("    {}   now vs 7 days ago", "wit diff tokyo 7d".cyan());
                    eprintln!("    {}   compare two cities", "wit diff tokyo..boston".cyan());
                    std::process::exit(1);
                }
                _ => handle_query(&args),
            }
        }
        Some(Command::Chart {
            metric,
            location,
            range,
        }) => commands::chart::run(metric.as_deref(), &location, &range),
        None if !cli.query.is_empty() => handle_query(&cli.query),
        None => {
            // No args — show status if initialized, otherwise help
            let wit_path = default_wit_path();
            if wit_path.join(".git").exists() {
                commands::status::run(None)
            } else {
                println!("  {} — version-controlled weather journal", "wit".bold());
                println!();
                println!("  Get started:");
                println!("    {} initialize weather repo", "wit init".cyan());
                println!("    {} track a city", "wit add <city>".cyan());
                println!("    {} fetch weather snapshot", "wit snap".cyan());
                println!();
                println!("  Quick queries:");
                println!("    {} current conditions", "wit tokyo".cyan());
                println!("    {} now vs 7 days ago", "wit tokyo 7d".cyan());
                println!(
                    "    {} compare two cities",
                    "wit tokyo..boston".cyan()
                );
                println!();
                println!("  Run {} for all commands", "wit --help".cyan());
                Ok(())
            }
        }
    }
}

fn handle_query(args: &[String]) -> error::Result<()> {
    let query = parse_query(args)?;
    let wit_path = default_wit_path();

    // Load config if available
    let (config, repo) = if wit_path.join(".git").exists() {
        let config = WitConfig::load(&wit_path.join("wit.toml"))
            .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;
        let repo = WitRepo::open(&wit_path)?;
        (Some(config), Some(repo))
    } else {
        (None, None)
    };

    let units = config
        .as_ref()
        .map(|c| c.settings.units.as_str())
        .unwrap_or("imperial");
    let imperial = units == "imperial";
    let client = WeatherClient::new(imperial);

    match query {
        Query::Current { location } => {
            let (name, snap) = fetch_location(&client, &location, config.as_ref(), &wit_path)?;

            // If location is tracked and repo exists, also commit the snapshot
            if let (Some(ref cfg), Some(ref repo)) = (&config, &repo) {
                let slug = slug::slugify(&location);
                if cfg.has_location(&slug) {
                    let loc_dir = wit_path.join("locations").join(&slug);
                    let snap_toml = snap.to_toml()
                        .map_err(|e| anyhow::anyhow!("serialize error: {}", e))?;
                    std::fs::write(loc_dir.join("current.toml"), &snap_toml)
                        .map_err(|e| anyhow::anyhow!("write error: {}", e))?;
                    let summary = snap.summary_line(units);
                    let _ = repo.commit_all(&format!("snap: {} {}", name, summary));
                }
            }

            print_current(&name, &snap, units);
            Ok(())
        }

        Query::TimeDiff { location, past } => {
            let (name, current_snap) =
                fetch_location(&client, &location, config.as_ref(), &wit_path)?;

            let past_snap = resolve_past_snapshot(
                &client,
                &location,
                &past,
                config.as_ref(),
                repo.as_ref(),
                &wit_path,
            )?;

            let past_label = format!("{} (past)", name);
            let now_label = format!("{} (now)", name);
            print_diff(&past_label, &now_label, &past_snap, &current_snap, units);
            Ok(())
        }

        Query::TimeRange {
            location,
            from,
            to,
        } => {
            let name = resolve_location_name(&location, config.as_ref())?;

            let from_snap = resolve_past_snapshot(
                &client,
                &location,
                &from,
                config.as_ref(),
                repo.as_ref(),
                &wit_path,
            )?;
            let to_snap = resolve_past_snapshot(
                &client,
                &location,
                &to,
                config.as_ref(),
                repo.as_ref(),
                &wit_path,
            )?;

            let from_date = from.resolve();
            let to_date = to.resolve();
            let from_label = format!("{} ({})", name, from_date.format("%Y-%m-%d"));
            let to_label = format!("{} ({})", name, to_date.format("%Y-%m-%d"));
            print_diff(&from_label, &to_label, &from_snap, &to_snap, units);
            Ok(())
        }

        Query::LocationDiff { left, right } => {
            let (left_name, left_snap) =
                fetch_location(&client, &left, config.as_ref(), &wit_path)?;
            let (right_name, right_snap) =
                fetch_location(&client, &right, config.as_ref(), &wit_path)?;

            print_diff(&left_name, &right_name, &left_snap, &right_snap, units);
            Ok(())
        }

        Query::LocationDiffAt { left, right, time } => {
            let left_name = resolve_location_name(&left, config.as_ref())?;
            let right_name = resolve_location_name(&right, config.as_ref())?;

            let left_snap = resolve_past_snapshot(
                &client,
                &left,
                &time,
                config.as_ref(),
                repo.as_ref(),
                &wit_path,
            )?;
            let right_snap = resolve_past_snapshot(
                &client,
                &right,
                &time,
                config.as_ref(),
                repo.as_ref(),
                &wit_path,
            )?;

            let date = time.resolve();
            let label_suffix = format!("({})", date.format("%Y-%m-%d"));
            print_diff(
                &format!("{} {}", left_name, label_suffix),
                &format!("{} {}", right_name, label_suffix),
                &left_snap,
                &right_snap,
                units,
            );
            Ok(())
        }
    }
}

/// Fetch current weather for a location — uses tracked data or live geocode
fn fetch_location(
    client: &WeatherClient,
    location: &str,
    config: Option<&WitConfig>,
    wit_path: &std::path::Path,
) -> error::Result<(String, WeatherSnapshot)> {
    let slug = slug::slugify(location);

    // Check if tracked
    if let Some(cfg) = config {
        if cfg.has_location(&slug) {
            let meta_path = wit_path.join("locations").join(&slug).join("meta.toml");
            if let Ok(content) = std::fs::read_to_string(&meta_path) {
                if let Ok(meta) = toml::from_str::<LocationMeta>(&content) {
                    let snap = client
                        .fetch_current(meta.latitude, meta.longitude)
                        .map_err(|e| WitError::Api(e.to_string()))?;
                    return Ok((meta.name, snap));
                }
            }
        }
    }

    // Not tracked — live geocode
    let results = client
        .geocode(location)
        .map_err(|e| WitError::Api(e.to_string()))?;

    let geo = results
        .first()
        .ok_or_else(|| WitError::LocationNotFound(location.to_string()))?;

    let snap = client
        .fetch_current(geo.latitude, geo.longitude)
        .map_err(|e| WitError::Api(e.to_string()))?;

    Ok((geo.display_name(), snap))
}

/// Resolve a past snapshot — try git history first, fall back to historical API
fn resolve_past_snapshot(
    client: &WeatherClient,
    location: &str,
    time: &TimeSpec,
    config: Option<&WitConfig>,
    repo: Option<&WitRepo>,
    wit_path: &std::path::Path,
) -> error::Result<WeatherSnapshot> {
    let slug = slug::slugify(location);
    let target_date = time.resolve();
    let target_dt = target_date
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(chrono::Local)
        .unwrap();

    // Try git history first
    if let (Some(_cfg), Some(repo)) = (config, repo) {
        if let Ok(Some(snap)) = repo.snapshot_at_date(&slug, target_dt) {
            return Ok(snap);
        }
    }

    // Fall back to historical API — need coordinates
    let (lat, lon) = resolve_coords(client, location, config, wit_path)?;

    client
        .fetch_historical(lat, lon, target_date)
        .map_err(|e| WitError::Api(e.to_string()))
}

fn resolve_coords(
    client: &WeatherClient,
    location: &str,
    config: Option<&WitConfig>,
    wit_path: &std::path::Path,
) -> error::Result<(f64, f64)> {
    let slug = slug::slugify(location);

    if let Some(cfg) = config {
        if cfg.has_location(&slug) {
            let meta_path = wit_path.join("locations").join(&slug).join("meta.toml");
            if let Ok(content) = std::fs::read_to_string(&meta_path) {
                if let Ok(meta) = toml::from_str::<LocationMeta>(&content) {
                    return Ok((meta.latitude, meta.longitude));
                }
            }
        }
    }

    let results = client
        .geocode(location)
        .map_err(|e| WitError::Api(e.to_string()))?;
    let geo = results
        .first()
        .ok_or_else(|| WitError::LocationNotFound(location.to_string()))?;
    Ok((geo.latitude, geo.longitude))
}

fn resolve_location_name(
    location: &str,
    config: Option<&WitConfig>,
) -> error::Result<String> {
    if let Some(cfg) = config {
        let slug = slug::slugify(location);
        if let Some(entry) = cfg.locations.iter().find(|l| l.slug == slug) {
            return Ok(entry.name.clone());
        }
    }
    Ok(location.to_string())
}
