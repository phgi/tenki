pub mod geolocation;
pub mod model;
pub mod weather;

use model::{Location, WeatherData};

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("tenki-weather-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client")
}

pub async fn fetch_all(
    client: &reqwest::Client,
    override_location: Option<(f64, f64)>,
) -> Result<WeatherData, String> {
    let location = match override_location {
        Some((lat, lon)) => Location {
            city: "Custom Location".to_string(),
            region: String::new(),
            latitude: lat,
            longitude: lon,
        },
        None => geolocation::locate(client).await?,
    };
    weather::fetch(client, &location).await
}
