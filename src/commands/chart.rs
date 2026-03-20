use colored::Colorize;
use rgb::RGB8;
use textplots::{Chart, ColorPlot, Shape};

use crate::commands::init::default_wit_path;
use crate::commands::overlay_chart;
use crate::config::{LocationEntry, WitConfig};
use crate::error::{Result, WitError};
use crate::git::repo::WitRepo;
use crate::query::parse_time_token;
use crate::weather::snapshot::WeatherSnapshot;

const COLORS: &[(RGB8, &str)] = &[
    (RGB8 { r: 80, g: 200, b: 255 }, "cyan"),
    (RGB8 { r: 255, g: 100, b: 100 }, "red"),
    (RGB8 { r: 100, g: 255, b: 100 }, "green"),
    (RGB8 { r: 255, g: 200, b: 50 }, "yellow"),
    (RGB8 { r: 200, g: 100, b: 255 }, "magenta"),
    (RGB8 { r: 255, g: 150, b: 50 }, "bright red"),
];

pub fn run(metric: Option<&str>, locations: &[String], range: &str) -> Result<()> {
    let wit_path = default_wit_path();
    if !wit_path.join(".git").exists() {
        return Err(WitError::NotInitialized);
    }

    let config = WitConfig::load(&wit_path.join("wit.toml"))
        .map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    // Resolve which locations to chart
    let locs: Vec<&LocationEntry> = if locations.is_empty() {
        // All tracked locations
        config.locations.iter().collect()
    } else {
        locations
            .iter()
            .map(|name| {
                let slug = slug::slugify(name);
                config
                    .locations
                    .iter()
                    .find(|l| l.slug == slug)
                    .ok_or_else(|| WitError::LocationNotFound(name.to_string()))
            })
            .collect::<Result<Vec<_>>>()?
    };

    if locs.is_empty() {
        return Err(WitError::NoLocations);
    }

    let metric_name = metric.unwrap_or("temp");
    let max_entries = estimate_entries(range);
    let repo = WitRepo::open(&wit_path)?;

    // Collect point series for each location
    struct Series {
        name: String,
        points: Vec<(f32, f32)>,
        min: f64,
        max: f64,
        avg: f64,
    }

    let mut all_series: Vec<Series> = Vec::new();
    let mut global_min = f64::INFINITY;
    let mut global_max = f64::NEG_INFINITY;
    let mut global_x_max: f32 = 1.0;

    for loc in &locs {
        let file_path = format!("locations/{}/current.toml", loc.slug);
        let history = repo
            .walk_history(Some(&file_path), max_entries)
            .map_err(|e| anyhow::anyhow!("failed to walk history for {}: {}", loc.name, e))?;

        if history.len() < 2 {
            eprintln!(
                "  {} skipping {} — not enough snapshots",
                "!".yellow(),
                loc.name
            );
            continue;
        }

        let mut points: Vec<(f32, f32)> = Vec::new();
        for (i, entry) in history.iter().rev().enumerate() {
            if let Ok(content) = repo.read_file_at_commit(entry.oid, &file_path) {
                if let Ok(snap) = WeatherSnapshot::from_toml(&content) {
                    let value = extract_metric(&snap, metric_name);
                    points.push((i as f32, value as f32));
                }
            }
        }

        if points.is_empty() {
            continue;
        }

        let values: Vec<f64> = points.iter().map(|(_, v)| *v as f64).collect();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = values.iter().sum::<f64>() / values.len() as f64;

        global_min = global_min.min(min);
        global_max = global_max.max(max);
        global_x_max = global_x_max.max((points.len() as f32 - 1.0).max(1.0));

        all_series.push(Series {
            name: loc.name.clone(),
            points,
            min,
            max,
            avg,
        });
    }

    if all_series.is_empty() {
        return Err(WitError::NoSnapshots);
    }

    let units = &config.settings.units;
    let unit_label = metric_unit(metric_name, units);

    // Y-axis bounds with padding
    let y_range = global_max - global_min;
    let padding = if y_range < 1.0 { 10.0 } else { y_range * 0.15 };
    let y_min = (global_min - padding) as f32;
    let y_max = (global_max + padding) as f32;

    let max_points = all_series.iter().map(|s| s.points.len()).max().unwrap_or(30);
    let chart_width = (max_points * 10).max(60).min(120) as u32;

    println!();

    // Print legend for all series
    for (i, series) in all_series.iter().enumerate() {
        let color = COLORS[i % COLORS.len()];
        println!(
            "  {} {} — min {:.1}  avg {:.1}  max {:.1} {}",
            "━━".color(color.1),
            series.name.color(color.1).bold(),
            series.min,
            series.avg,
            series.max,
            unit_label
        );
    }
    println!();

    if all_series.len() == 1 {
        // Single series: use textplots for nicer rendering
        let chart_height = 60;
        let shape = Shape::Lines(&all_series[0].points);
        Chart::new_with_y_range(chart_width, chart_height, 0.0, global_x_max, y_min, y_max)
            .linecolorplot(&shape, COLORS[0].0)
            .display();
    } else {
        // Multiple series: overlay on same chart with distinct colors
        let chart_height: u16 = 20;
        let overlay_width: u16 = (chart_width / 2).max(30).min(80) as u16;
        let overlay_colors: Vec<RGB8> = all_series
            .iter()
            .enumerate()
            .map(|(i, _)| COLORS[i % COLORS.len()].0)
            .collect();
        let overlay_series: Vec<overlay_chart::SeriesData> = all_series
            .iter()
            .map(|s| overlay_chart::SeriesData {
                points: s.points.clone(),
            })
            .collect();

        let rows = overlay_chart::render_chart(
            &overlay_series,
            &overlay_colors,
            y_min,
            y_max,
            global_x_max,
            overlay_width,
            chart_height,
        );
        for row in &rows {
            println!("{}", row);
        }
    }

    println!();
    println!(
        "  {} = {} {}    {} = snapshot index (oldest → newest)",
        "y".dimmed(),
        metric_name,
        unit_label.dimmed(),
        "x".dimmed(),
    );
    println!();

    Ok(())
}

fn extract_metric(snap: &WeatherSnapshot, metric: &str) -> f64 {
    match metric {
        "temp" | "temperature" => snap.temperature.current,
        "feels" | "feels_like" => snap.temperature.feels_like,
        "high" => snap.temperature.high,
        "low" => snap.temperature.low,
        "humidity" => snap.atmosphere.humidity as f64,
        "pressure" => snap.atmosphere.pressure,
        "wind" | "wind_speed" => snap.wind.speed,
        "gusts" => snap.wind.gusts,
        "uv" | "uv_index" => snap.atmosphere.uv_index,
        "precip" | "precipitation" => snap.precipitation.amount,
        "cloud" | "cloud_cover" => snap.atmosphere.cloud_cover as f64,
        _ => snap.temperature.current,
    }
}

fn metric_unit(metric: &str, units: &str) -> &'static str {
    match metric {
        "temp" | "temperature" | "feels" | "feels_like" | "high" | "low" => {
            if units == "metric" {
                "°C"
            } else {
                "°F"
            }
        }
        "humidity" | "cloud" | "cloud_cover" => "%",
        "pressure" => "hPa",
        "wind" | "wind_speed" | "gusts" => {
            if units == "metric" {
                "km/h"
            } else {
                "mph"
            }
        }
        "uv" | "uv_index" => "",
        "precip" | "precipitation" => "mm",
        _ => "",
    }
}

fn estimate_entries(range: &str) -> usize {
    if let Ok(spec) = parse_time_token(range) {
        let date = spec.resolve();
        let today = chrono::Local::now().date_naive();
        let days = (today - date).num_days().unsigned_abs() as usize;
        days.max(10).min(365)
    } else {
        30
    }
}
