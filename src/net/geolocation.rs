use serde::Deserialize;

use super::model::Location;

#[derive(Debug, Deserialize)]
struct IpWhoIsResponse {
    success: Option<bool>,
    city: Option<String>,
    region: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct IpApiComResponse {
    status: Option<String>,
    city: Option<String>,
    #[serde(rename = "regionName")]
    region_name: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
}

pub async fn locate(client: &reqwest::Client) -> Result<Location, String> {
    match locate_via_ipwho(client).await {
        Ok(loc) => Ok(loc),
        Err(primary_err) => locate_via_ip_api(client)
            .await
            .map_err(|fallback_err| format!("{primary_err}; fallback also failed: {fallback_err}")),
    }
}

async fn locate_via_ipwho(client: &reqwest::Client) -> Result<Location, String> {
    let resp: IpWhoIsResponse = client
        .get("https://ipwho.is/")
        .send()
        .await
        .map_err(|e| format!("ipwho.is request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("ipwho.is response parse failed: {e}"))?;

    if resp.success == Some(false) {
        return Err("ipwho.is reported failure".to_string());
    }

    Ok(Location {
        city: resp.city.unwrap_or_else(|| "Unknown".to_string()),
        region: resp.region.unwrap_or_default(),
        latitude: resp.latitude.ok_or("ipwho.is missing latitude")?,
        longitude: resp.longitude.ok_or("ipwho.is missing longitude")?,
    })
}

async fn locate_via_ip_api(client: &reqwest::Client) -> Result<Location, String> {
    let resp: IpApiComResponse = client
        .get("http://ip-api.com/json")
        .send()
        .await
        .map_err(|e| format!("ip-api.com request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("ip-api.com response parse failed: {e}"))?;

    if resp.status.as_deref() != Some("success") {
        return Err("ip-api.com reported failure".to_string());
    }

    Ok(Location {
        city: resp.city.unwrap_or_else(|| "Unknown".to_string()),
        region: resp.region_name.unwrap_or_default(),
        latitude: resp.lat.ok_or("ip-api.com missing lat")?,
        longitude: resp.lon.ok_or("ip-api.com missing lon")?,
    })
}
