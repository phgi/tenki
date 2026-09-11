use serde::Deserialize;

use super::model::Location;

/// How many matches to ask the geocoder for. The picker shows all of them.
const MAX_RESULTS: usize = 8;

/// One candidate the user can pick in the location search.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub location: Location,
    /// The full "Mitte, Berlin, Germany" line shown in the picker.
    pub label: String,
}

#[derive(Debug, Deserialize)]
struct GeocodeResponse {
    results: Option<Vec<GeocodeResult>>,
}

/// Open-Meteo reports a bad request as `{"error": true, "reason": "..."}`.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: Option<bool>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeocodeResult {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
}

/// Percent-encode a query for use in a URL path parameter.
fn encode(q: &str) -> String {
    let mut out = String::with_capacity(q.len());
    for b in q.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

impl Place {
    /// A place the user pinned by typing coordinates — no geocoder involved.
    pub fn from_coordinates(latitude: f64, longitude: f64) -> Place {
        let label = format!("{latitude:.4}, {longitude:.4}");
        Place {
            location: Location {
                city: label.clone(),
                region: String::new(),
                latitude,
                longitude,
            },
            label: format!("{label}  (exact coordinates)"),
        }
    }

    /// "52.52,13.41" — the coordinate column of the picker.
    pub fn coordinates(&self) -> String {
        format!("{:.2},{:.2}", self.location.latitude, self.location.longitude)
    }
}

impl From<GeocodeResult> for Place {
    fn from(r: GeocodeResult) -> Place {
        let admin1 = non_empty(r.admin1);
        let country = non_empty(r.country);
        let country_code = non_empty(r.country_code);

        let mut parts = vec![r.name.clone()];
        parts.extend(admin1.clone());
        parts.extend(country.clone());
        // Duplicate segments are common ("Berlin, Berlin, Germany"); drop them.
        parts.dedup();

        Place {
            location: Location {
                city: r.name,
                // The status bar has room for one qualifier only.
                region: admin1.or(country_code).or(country).unwrap_or_default(),
                latitude: r.latitude,
                longitude: r.longitude,
            },
            label: parts.join(", "),
        }
    }
}

/// Coordinates typed straight into the search box ("52.52, 13.41"), which is the
/// most precise thing a user can give us. A bare number is a postcode, not a
/// coordinate, so both halves must be present.
pub fn parse_coordinates(query: &str) -> Option<Place> {
    let parts: Vec<&str> = query
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 2 {
        return None;
    }
    let lat: f64 = parts[0].parse().ok()?;
    let lon: f64 = parts[1].parse().ok()?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    Some(Place::from_coordinates(lat, lon))
}

/// The complaint the geocoder made about the query, if it made one.
fn reported_error(body: &str) -> Option<String> {
    let resp: ErrorResponse = serde_json::from_str(body).ok()?;
    if resp.error != Some(true) {
        return None;
    }
    Some(
        resp.reason
            .unwrap_or_else(|| "the geocoder rejected that search".to_string()),
    )
}

fn parse(body: &str) -> Result<Vec<Place>, String> {
    let resp: GeocodeResponse =
        serde_json::from_str(body).map_err(|e| format!("geocoder response parse failed: {e}"))?;
    Ok(resp
        .results
        .unwrap_or_default()
        .into_iter()
        .map(Place::from)
        .collect())
}

pub async fn search(client: &reqwest::Client, query: &str) -> Result<Vec<Place>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(place) = parse_coordinates(query) {
        return Ok(vec![place]);
    }

    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count={MAX_RESULTS}&language=en&format=json",
        encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("geocoder request failed: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("geocoder response read failed: {e}"))?;

    if let Some(reason) = reported_error(&body) {
        return Err(reason);
    }
    if !status.is_success() {
        return Err(format!("geocoder returned HTTP {}", status.as_u16()));
    }
    parse(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "results": [
            {"id": 1, "name": "Mitte", "latitude": 52.52003, "longitude": 13.40489,
             "country_code": "DE", "admin1": "Berlin", "country": "Germany"},
            {"id": 2, "name": "Hamburg-Mitte", "latitude": 53.55073, "longitude": 9.99302,
             "country_code": "DE", "admin1": "Hamburg", "country": "Germany"}
        ],
        "generationtime_ms": 0.7
    }"#;

    #[test]
    fn parses_matches_into_places() {
        let places = parse(SAMPLE).unwrap();
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].label, "Mitte, Berlin, Germany");
        assert_eq!(places[0].location.city, "Mitte");
        assert_eq!(places[0].location.region, "Berlin");
        assert_eq!(places[0].coordinates(), "52.52,13.40");
    }

    #[test]
    fn a_response_without_results_is_an_empty_list_not_an_error() {
        let places = parse(r#"{"generationtime_ms": 0.2}"#).unwrap();
        assert!(places.is_empty());
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn repeated_name_and_region_collapse_in_the_label() {
        let body = r#"{"results":[{"id":1,"name":"Berlin","latitude":52.5,"longitude":13.4,
                       "admin1":"Berlin","country":"Germany"}]}"#;
        assert_eq!(parse(body).unwrap()[0].label, "Berlin, Germany");
    }

    #[test]
    fn missing_region_falls_back_to_the_country() {
        let body = r#"{"results":[{"id":1,"name":"Monaco","latitude":43.7,"longitude":7.4,
                       "country_code":"MC","country":"Monaco"}]}"#;
        let place = &parse(body).unwrap()[0];
        assert_eq!(place.location.region, "MC");
        // City and country coincide here, so the label says it once.
        assert_eq!(place.label, "Monaco");
    }

    #[test]
    fn coordinates_are_accepted_in_several_shapes() {
        for q in ["52.52,13.41", "52.52, 13.41", "52.52 13.41", "  52.52 , 13.41 "] {
            let place = parse_coordinates(q).unwrap_or_else(|| panic!("rejected {q:?}"));
            assert_eq!(place.coordinates(), "52.52,13.41");
        }
    }

    #[test]
    fn negative_and_integer_coordinates_work() {
        let place = parse_coordinates("-33, 151").unwrap();
        assert_eq!((place.location.latitude, place.location.longitude), (-33.0, 151.0));
    }

    #[test]
    fn a_postcode_or_place_name_is_not_coordinates() {
        for q in ["10115", "berlin", "Kings Cross, London", "52.52", "1 2 3"] {
            assert!(parse_coordinates(q).is_none(), "{q:?} was read as coordinates");
        }
    }

    #[test]
    fn out_of_range_coordinates_are_rejected() {
        assert!(parse_coordinates("95.0, 13.4").is_none());
        assert!(parse_coordinates("52.5, 200.0").is_none());
    }

    #[test]
    fn a_reported_error_becomes_its_reason() {
        assert_eq!(
            reported_error(r#"{"error": true, "reason": "Parameter name is empty"}"#),
            Some("Parameter name is empty".to_string())
        );
        assert_eq!(
            reported_error(r#"{"error": true}"#),
            Some("the geocoder rejected that search".to_string())
        );
    }

    #[test]
    fn a_normal_response_reports_no_error() {
        assert!(reported_error(SAMPLE).is_none());
        assert!(reported_error("<html>502</html>").is_none());
    }

    #[test]
    fn queries_are_percent_encoded() {
        assert_eq!(encode("berlin mitte"), "berlin%20mitte");
        assert_eq!(encode("sao paulo & co"), "sao%20paulo%20%26%20co");
        assert_eq!(encode("K\u{f6}ln"), "K%C3%B6ln");
    }
}
