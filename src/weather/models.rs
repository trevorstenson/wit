use serde::Deserialize;

// --- Geocoding API ---

#[derive(Debug, Deserialize)]
pub struct GeocodingResponse {
    pub results: Option<Vec<GeoResult>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeoResult {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub country: Option<String>,
    pub admin1: Option<String>,
    pub timezone: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub country_code: String,
}

impl GeoResult {
    pub fn display_name(&self) -> String {
        let mut parts = vec![self.name.clone()];
        if let Some(ref admin) = self.admin1 {
            parts.push(admin.clone());
        }
        if let Some(ref country) = self.country {
            parts.push(country.clone());
        }
        parts.join(", ")
    }
}

// --- Forecast API ---

#[derive(Debug, Deserialize)]
pub struct ForecastResponse {
    pub current: Option<CurrentWeather>,
    pub daily: Option<DailyWeather>,
}

#[derive(Debug, Deserialize)]
pub struct CurrentWeather {
    pub temperature_2m: Option<f64>,
    pub apparent_temperature: Option<f64>,
    pub relative_humidity_2m: Option<i32>,
    pub weather_code: Option<u8>,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<i32>,
    pub wind_gusts_10m: Option<f64>,
    pub surface_pressure: Option<f64>,
    pub cloud_cover: Option<i32>,
    pub precipitation: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct DailyWeather {
    pub temperature_2m_max: Option<Vec<f64>>,
    pub temperature_2m_min: Option<Vec<f64>>,
    pub uv_index_max: Option<Vec<f64>>,
    pub precipitation_probability_max: Option<Vec<i32>>,
    pub snowfall_sum: Option<Vec<f64>>,
}

// --- Historical API ---

#[derive(Debug, Deserialize)]
pub struct HistoricalResponse {
    pub daily: Option<HistoricalDaily>,
}

#[derive(Debug, Deserialize)]
pub struct HistoricalDaily {
    #[allow(dead_code)]
    pub time: Option<Vec<String>>,
    pub temperature_2m_max: Option<Vec<Option<f64>>>,
    pub temperature_2m_min: Option<Vec<Option<f64>>>,
    pub temperature_2m_mean: Option<Vec<Option<f64>>>,
    pub apparent_temperature_max: Option<Vec<Option<f64>>>,
    pub weather_code: Option<Vec<Option<u8>>>,
    pub wind_speed_10m_max: Option<Vec<Option<f64>>>,
    pub wind_direction_10m_dominant: Option<Vec<Option<i32>>>,
    pub wind_gusts_10m_max: Option<Vec<Option<f64>>>,
    pub relative_humidity_2m_mean: Option<Vec<Option<i32>>>,
    pub surface_pressure_mean: Option<Vec<Option<f64>>>,
    pub precipitation_sum: Option<Vec<Option<f64>>>,
    pub snowfall_sum: Option<Vec<Option<f64>>>,
    pub uv_index_max: Option<Vec<Option<f64>>>,
}
