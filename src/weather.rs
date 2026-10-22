use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Forecast {
    pub current: Current,
    pub hourly: Hourly,
    pub daily: Daily,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Current {
    pub time: String,
    pub temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub weather_code: u32,
    pub wind_speed_10m: f64,
    pub is_day: u32,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Hourly {
    pub time: Vec<String>,
    pub temperature_2m: Vec<Option<f64>>,
    pub weather_code: Vec<Option<u32>>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Daily {
    pub time: Vec<String>,
    pub weather_code: Vec<Option<u32>>,
    pub temperature_2m_max: Vec<Option<f64>>,
    pub temperature_2m_min: Vec<Option<f64>>,
    pub precipitation_probability_max: Vec<Option<f64>>,
}
impl Forecast {
    pub fn validate(&self) -> Result<()> {
        let h = self.hourly.time.len();
        let d = self.daily.time.len();
        if h < 24
            || d < 5
            || h > 240
            || d > 16
            || self.hourly.temperature_2m.len() != h
            || self.hourly.weather_code.len() != h
            || self.daily.weather_code.len() != d
            || self.daily.temperature_2m_max.len() != d
            || self.daily.temperature_2m_min.len() != d
            || self.daily.precipitation_probability_max.len() != d
        {
            bail!("Unvollständige Wetterdaten");
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
pub struct CachedForecast {
    pub data: Forecast,
    pub fetched_at: String,
    pub stale: bool,
}
struct CacheEntry {
    value: CachedForecast,
    fetched: Instant,
    attempted: Instant,
}
#[derive(Default)]
pub struct Weather(Mutex<HashMap<String, CacheEntry>>);
impl Weather {
    pub async fn fetch(
        &self,
        client: &reqwest::Client,
        lat: f64,
        lon: f64,
    ) -> Result<CachedForecast> {
        let key = format!("{lat:.4},{lon:.4}");
        // Single flight: Open-Meteo requests are short, bounded and deduplicated.
        let mut cache = self.0.lock().await;
        if let Some(e) = cache.get(&key) {
            if e.fetched.elapsed() < Duration::from_secs(900) {
                return Ok(e.value.clone());
            }
            if e.attempted.elapsed() < Duration::from_secs(60) {
                let mut v = e.value.clone();
                v.stale = true;
                return Ok(v);
            }
        }
        let result = async {
            let response = client.get("https://api.open-meteo.com/v1/forecast").query(&[
                ("latitude", lat.to_string()), ("longitude", lon.to_string()),
                ("timezone", "Europe/Zurich".into()), ("forecast_days", "5".into()),
                ("current", "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,is_day".into()),
                ("hourly", "temperature_2m,weather_code".into()),
                ("daily", "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max".into()),
            ]).send().await?.error_for_status()?;
            let data: Forecast = crate::bounded_json(response).await?;
            data.validate()?;
            Ok::<_, anyhow::Error>(CachedForecast { data, fetched_at: chrono::Utc::now().to_rfc3339(), stale: false })
        }.await;
        match result {
            Ok(value) => {
                if cache.len() >= 100 {
                    cache.clear();
                }
                cache.insert(
                    key,
                    CacheEntry {
                        value: value.clone(),
                        fetched: Instant::now(),
                        attempted: Instant::now(),
                    },
                );
                Ok(value)
            }
            Err(e) => {
                if let Some(old) = cache.get_mut(&key) {
                    old.attempted = Instant::now();
                    let mut v = old.value.clone();
                    v.stale = true;
                    return Ok(v);
                }
                Err(e)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_example_forecast_and_reject_truncation() {
        let mut f: Forecast =
            serde_json::from_str(include_str!("../tests/fixtures/weather.json")).unwrap();
        assert_eq!(f.current.temperature_2m, 18.5);
        assert!(f.validate().is_ok());
        f.daily.temperature_2m_max.pop();
        assert!(f.validate().is_err());
        assert!(serde_json::from_str::<Forecast>(r#"{"error":true}"#).is_err());
    }
}
