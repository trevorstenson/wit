use anyhow::Context;
use chrono::{Local, NaiveDate};

use super::codes::describe_weather_code;
use super::models::*;
use super::snapshot::*;

const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const HISTORICAL_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

pub struct WeatherClient {
    client: reqwest::blocking::Client,
    imperial: bool,
}

impl WeatherClient {
    pub fn new(imperial: bool) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            imperial,
        }
    }

    fn temp_unit(&self) -> &str {
        if self.imperial {
            "fahrenheit"
        } else {
            "celsius"
        }
    }

    fn wind_unit(&self) -> &str {
        if self.imperial {
            "mph"
        } else {
            "kmh"
        }
    }

    pub fn geocode(&self, name: &str) -> anyhow::Result<Vec<GeoResult>> {
        let resp: GeocodingResponse = self
            .client
            .get(GEOCODING_URL)
            .query(&[("name", name), ("count", "5"), ("language", "en")])
            .send()
            .context("failed to reach geocoding API")?
            .json()
            .context("failed to parse geocoding response")?;

        Ok(resp.results.unwrap_or_default())
    }

    pub fn fetch_current(&self, lat: f64, lon: f64) -> anyhow::Result<WeatherSnapshot> {
        let resp: ForecastResponse = self
            .client
            .get(FORECAST_URL)
            .query(&[
                ("latitude", lat.to_string()),
                ("longitude", lon.to_string()),
                (
                    "current",
                    "temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,surface_pressure,cloud_cover,precipitation".to_string(),
                ),
                (
                    "daily",
                    "temperature_2m_max,temperature_2m_min,uv_index_max,precipitation_probability_max,snowfall_sum".to_string(),
                ),
                ("temperature_unit", self.temp_unit().to_string()),
                ("wind_speed_unit", self.wind_unit().to_string()),
                ("forecast_days", "1".to_string()),
            ])
            .send()
            .context("failed to reach forecast API")?
            .json()
            .context("failed to parse forecast response")?;

        let current = resp.current.context("no current weather data")?;
        let daily = resp.daily;

        let code = current.weather_code.unwrap_or(0);
        let (desc, _emoji) = describe_weather_code(code);

        let high = daily
            .as_ref()
            .and_then(|d| d.temperature_2m_max.as_ref())
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0);
        let low = daily
            .as_ref()
            .and_then(|d| d.temperature_2m_min.as_ref())
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0);
        let uv = daily
            .as_ref()
            .and_then(|d| d.uv_index_max.as_ref())
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0);
        let precip_prob = daily
            .as_ref()
            .and_then(|d| d.precipitation_probability_max.as_ref())
            .and_then(|v| v.first().copied())
            .unwrap_or(0);
        let snowfall = daily
            .as_ref()
            .and_then(|d| d.snowfall_sum.as_ref())
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0);

        Ok(WeatherSnapshot {
            snapshot: SnapshotMeta {
                timestamp: Local::now(),
                weather_code: code,
                weather_description: desc.to_string(),
            },
            temperature: Temperature {
                current: current.temperature_2m.unwrap_or(0.0),
                feels_like: current.apparent_temperature.unwrap_or(0.0),
                high,
                low,
            },
            wind: Wind {
                speed: current.wind_speed_10m.unwrap_or(0.0),
                direction: current.wind_direction_10m.unwrap_or(0),
                gusts: current.wind_gusts_10m.unwrap_or(0.0),
            },
            atmosphere: Atmosphere {
                humidity: current.relative_humidity_2m.unwrap_or(0),
                pressure: current.surface_pressure.unwrap_or(0.0),
                cloud_cover: current.cloud_cover.unwrap_or(0),
                uv_index: uv,
            },
            precipitation: Precipitation {
                amount: current.precipitation.unwrap_or(0.0),
                probability: precip_prob,
                snowfall,
            },
        })
    }

    pub fn fetch_historical(
        &self,
        lat: f64,
        lon: f64,
        date: NaiveDate,
    ) -> anyhow::Result<WeatherSnapshot> {
        let date_str = date.format("%Y-%m-%d").to_string();

        let resp: HistoricalResponse = self
            .client
            .get(HISTORICAL_URL)
            .query(&[
                ("latitude", lat.to_string()),
                ("longitude", lon.to_string()),
                ("start_date", date_str.clone()),
                ("end_date", date_str),
                (
                    "daily",
                    "temperature_2m_max,temperature_2m_min,temperature_2m_mean,apparent_temperature_max,weather_code,wind_speed_10m_max,wind_direction_10m_dominant,wind_gusts_10m_max,relative_humidity_2m_mean,surface_pressure_mean,precipitation_sum,snowfall_sum,uv_index_max".to_string(),
                ),
                ("temperature_unit", self.temp_unit().to_string()),
                ("wind_speed_unit", self.wind_unit().to_string()),
            ])
            .send()
            .context("failed to reach historical API")?
            .json()
            .context("failed to parse historical response")?;

        let daily = resp.daily.context("no historical data")?;

        // Helper to extract first value from Option<Vec<Option<T>>>
        fn first_f64(field: &Option<Vec<Option<f64>>>) -> f64 {
            field.as_ref().and_then(|v| v.first().copied()).flatten().unwrap_or(0.0)
        }
        fn first_i32(field: &Option<Vec<Option<i32>>>) -> i32 {
            field.as_ref().and_then(|v| v.first().copied()).flatten().unwrap_or(0)
        }

        let code = daily
            .weather_code
            .as_ref()
            .and_then(|v| v.first().copied())
            .flatten()
            .unwrap_or(0);
        let (desc, _) = describe_weather_code(code);

        let temp_mean = first_f64(&daily.temperature_2m_mean);
        let temp_max = first_f64(&daily.temperature_2m_max);
        let temp_min = first_f64(&daily.temperature_2m_min);
        let feels_like = daily
            .apparent_temperature_max
            .as_ref()
            .and_then(|v| v.first().copied())
            .flatten()
            .unwrap_or(temp_mean);

        Ok(WeatherSnapshot {
            snapshot: SnapshotMeta {
                timestamp: date
                    .and_hms_opt(12, 0, 0)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap(),
                weather_code: code,
                weather_description: desc.to_string(),
            },
            temperature: Temperature {
                current: temp_mean,
                feels_like,
                high: temp_max,
                low: temp_min,
            },
            wind: Wind {
                speed: first_f64(&daily.wind_speed_10m_max),
                direction: first_i32(&daily.wind_direction_10m_dominant),
                gusts: first_f64(&daily.wind_gusts_10m_max),
            },
            atmosphere: Atmosphere {
                humidity: first_i32(&daily.relative_humidity_2m_mean),
                pressure: first_f64(&daily.surface_pressure_mean),
                cloud_cover: 0,
                uv_index: first_f64(&daily.uv_index_max),
            },
            precipitation: Precipitation {
                amount: first_f64(&daily.precipitation_sum),
                probability: 0,
                snowfall: first_f64(&daily.snowfall_sum),
            },
        })
    }

    /// Fetch a date range in one API call, returning one snapshot per day
    pub fn fetch_historical_range(
        &self,
        lat: f64,
        lon: f64,
        start: NaiveDate,
        end: NaiveDate,
    ) -> anyhow::Result<Vec<(NaiveDate, WeatherSnapshot)>> {
        let resp: HistoricalResponse = self
            .client
            .get(HISTORICAL_URL)
            .query(&[
                ("latitude", lat.to_string()),
                ("longitude", lon.to_string()),
                ("start_date", start.format("%Y-%m-%d").to_string()),
                ("end_date", end.format("%Y-%m-%d").to_string()),
                (
                    "daily",
                    "temperature_2m_max,temperature_2m_min,temperature_2m_mean,apparent_temperature_max,weather_code,wind_speed_10m_max,wind_direction_10m_dominant,wind_gusts_10m_max,relative_humidity_2m_mean,surface_pressure_mean,precipitation_sum,snowfall_sum,uv_index_max".to_string(),
                ),
                ("temperature_unit", self.temp_unit().to_string()),
                ("wind_speed_unit", self.wind_unit().to_string()),
            ])
            .send()
            .context("failed to reach historical API")?
            .json()
            .context("failed to parse historical response")?;

        let daily = resp.daily.context("no historical data")?;
        let dates = daily.time.as_ref().context("no time array")?;

        let mut results = Vec::new();

        for (i, date_str) in dates.iter().enumerate() {
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .context("invalid date in response")?;

            fn get_f64(field: &Option<Vec<Option<f64>>>, idx: usize) -> f64 {
                field
                    .as_ref()
                    .and_then(|v| v.get(idx).copied())
                    .flatten()
                    .unwrap_or(0.0)
            }
            fn get_i32(field: &Option<Vec<Option<i32>>>, idx: usize) -> i32 {
                field
                    .as_ref()
                    .and_then(|v| v.get(idx).copied())
                    .flatten()
                    .unwrap_or(0)
            }
            fn get_u8(field: &Option<Vec<Option<u8>>>, idx: usize) -> u8 {
                field
                    .as_ref()
                    .and_then(|v| v.get(idx).copied())
                    .flatten()
                    .unwrap_or(0)
            }

            let code = get_u8(&daily.weather_code, i);
            let (desc, _) = describe_weather_code(code);
            let temp_mean = get_f64(&daily.temperature_2m_mean, i);
            let feels = get_f64(&daily.apparent_temperature_max, i);

            let snap = WeatherSnapshot {
                snapshot: SnapshotMeta {
                    timestamp: date
                        .and_hms_opt(12, 0, 0)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap(),
                    weather_code: code,
                    weather_description: desc.to_string(),
                },
                temperature: Temperature {
                    current: temp_mean,
                    feels_like: if feels != 0.0 { feels } else { temp_mean },
                    high: get_f64(&daily.temperature_2m_max, i),
                    low: get_f64(&daily.temperature_2m_min, i),
                },
                wind: Wind {
                    speed: get_f64(&daily.wind_speed_10m_max, i),
                    direction: get_i32(&daily.wind_direction_10m_dominant, i),
                    gusts: get_f64(&daily.wind_gusts_10m_max, i),
                },
                atmosphere: Atmosphere {
                    humidity: get_i32(&daily.relative_humidity_2m_mean, i),
                    pressure: get_f64(&daily.surface_pressure_mean, i),
                    cloud_cover: 0,
                    uv_index: get_f64(&daily.uv_index_max, i),
                },
                precipitation: Precipitation {
                    amount: get_f64(&daily.precipitation_sum, i),
                    probability: 0,
                    snowfall: get_f64(&daily.snowfall_sum, i),
                },
            };

            results.push((date, snap));
        }

        Ok(results)
    }
}

pub fn wind_direction_str(degrees: i32) -> &'static str {
    match ((degrees as f64 + 22.5) / 45.0) as i32 % 8 {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "?",
    }
}
