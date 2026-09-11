pub mod geocode;
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

/// Fetch the weather for `location`, falling back to IP geolocation when the
/// user has not told us where they are.
pub async fn fetch_all(
    client: &reqwest::Client,
    location: Option<Location>,
) -> Result<WeatherData, String> {
    let location = match location {
        Some(loc) => loc,
        None => geolocation::locate(client).await?,
    };
    weather::fetch(client, &location).await
}
