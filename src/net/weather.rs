use serde::Deserialize;

use super::model::{Condition, Location, WeatherData};

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    current: CurrentBlock,
    daily: DailyBlock,
}

#[derive(Debug, Deserialize)]
struct CurrentBlock {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    is_day: u8,
    weather_code: u32,
    surface_pressure: f64,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

#[derive(Debug, Deserialize)]
struct DailyBlock {
    sunrise: Vec<String>,
    sunset: Vec<String>,
    uv_index_max: Vec<f64>,
    precipitation_probability_max: Vec<f64>,
}

/// Extract "HH:MM" from an Open-Meteo local ISO8601 timestamp like "2023-09-10T06:32".
fn time_of_day(iso: &str) -> String {
    iso.split('T').nth(1).unwrap_or(iso).to_string()
}

pub async fn fetch(client: &reqwest::Client, location: &Location) -> Result<WeatherData, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,is_day,weather_code,surface_pressure,wind_speed_10m,wind_direction_10m&daily=sunrise,sunset,uv_index_max,precipitation_probability_max&timezone=auto",
        location.latitude, location.longitude
    );

    let resp: ForecastResponse = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("weather request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("weather response parse failed: {e}"))?;

    Ok(WeatherData {
        location: location.clone(),
        condition: Condition::from_wmo_code(resp.current.weather_code),
        is_day: resp.current.is_day == 1,
        temperature_c: resp.current.temperature_2m,
        feels_like_c: resp.current.apparent_temperature,
        humidity_pct: resp.current.relative_humidity_2m,
        wind_speed_kmh: resp.current.wind_speed_10m,
        wind_direction_deg: resp.current.wind_direction_10m,
        pressure_hpa: resp.current.surface_pressure,
        uv_index: resp.daily.uv_index_max.first().copied().unwrap_or(0.0),
        precipitation_probability_pct: resp
            .daily
            .precipitation_probability_max
            .first()
            .copied()
            .unwrap_or(0.0),
        sunrise: resp
            .daily
            .sunrise
            .first()
            .map(|s| time_of_day(s))
            .unwrap_or_default(),
        sunset: resp
            .daily
            .sunset
            .first()
            .map(|s| time_of_day(s))
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_time_of_day() {
        assert_eq!(time_of_day("2023-09-10T06:32"), "06:32");
    }

    #[test]
    fn time_of_day_falls_back_when_no_t() {
        assert_eq!(time_of_day("06:32"), "06:32");
    }
}
