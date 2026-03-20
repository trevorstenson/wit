use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherSnapshot {
    pub snapshot: SnapshotMeta,
    pub temperature: Temperature,
    pub wind: Wind,
    pub atmosphere: Atmosphere,
    pub precipitation: Precipitation,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotMeta {
    pub timestamp: DateTime<Local>,
    pub weather_code: u8,
    pub weather_description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Temperature {
    pub current: f64,
    pub feels_like: f64,
    pub high: f64,
    pub low: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wind {
    pub speed: f64,
    pub direction: i32,
    pub gusts: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Atmosphere {
    pub humidity: i32,
    pub pressure: f64,
    pub cloud_cover: i32,
    pub uv_index: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Precipitation {
    pub amount: f64,
    pub probability: i32,
    pub snowfall: f64,
}

impl WeatherSnapshot {
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(s: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(s)?)
    }

    pub fn summary_line(&self, units: &str) -> String {
        let unit = if units == "metric" { "C" } else { "F" };
        format!(
            "{:.0}{} {}",
            self.temperature.current, unit, self.snapshot.weather_description
        )
    }
}

/// Location metadata stored in meta.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct LocationMeta {
    pub name: String,
    pub slug: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub country: String,
}
