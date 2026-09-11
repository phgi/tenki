#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Clear,
    PartlyCloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    RainShowers,
    Freezing,
    Snow,
    SnowShowers,
    Thunderstorm,
}

impl Condition {
    /// Map an Open-Meteo / WMO weather code to our internal condition.
    /// https://open-meteo.com/en/docs (WMO Weather interpretation codes)
    pub fn from_wmo_code(code: u32) -> Condition {
        match code {
            0 => Condition::Clear,
            1 | 2 => Condition::PartlyCloudy,
            3 => Condition::Overcast,
            45 | 48 => Condition::Fog,
            51 | 53 | 55 => Condition::Drizzle,
            56 | 57 => Condition::Freezing,
            61 | 63 | 65 => Condition::Rain,
            66 | 67 => Condition::Freezing,
            71 | 73 | 75 | 77 => Condition::Snow,
            80 | 81 | 82 => Condition::RainShowers,
            85 | 86 => Condition::SnowShowers,
            95 | 96 | 99 => Condition::Thunderstorm,
            _ => Condition::PartlyCloudy,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Condition::Clear => "CLEAR",
            Condition::PartlyCloudy => "CLOUDY",
            Condition::Overcast => "OVERCAST",
            Condition::Fog => "FOG",
            Condition::Drizzle => "DRIZZLE",
            Condition::Rain => "RAIN",
            Condition::RainShowers => "SHOWERS",
            Condition::Freezing => "FREEZING",
            Condition::Snow => "SNOW",
            Condition::SnowShowers => "SNOW",
            Condition::Thunderstorm => "STORM",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub city: String,
    pub region: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone)]
pub struct WeatherData {
    pub location: Location,
    pub condition: Condition,
    pub is_day: bool,
    pub temperature_c: f64,
    pub feels_like_c: f64,
    pub humidity_pct: f64,
    pub wind_speed_kmh: f64,
    pub wind_direction_deg: f64,
    pub pressure_hpa: f64,
    pub uv_index: f64,
    pub precipitation_probability_pct: f64,
    /// Local "HH:MM" strings as returned by Open-Meteo with timezone=auto.
    pub sunrise: String,
    pub sunset: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_clear_sky() {
        assert_eq!(Condition::from_wmo_code(0), Condition::Clear);
    }

    #[test]
    fn maps_thunderstorm_variants() {
        for code in [95, 96, 99] {
            assert_eq!(Condition::from_wmo_code(code), Condition::Thunderstorm);
        }
    }

    #[test]
    fn maps_snow_variants() {
        for code in [71, 73, 75, 77] {
            assert_eq!(Condition::from_wmo_code(code), Condition::Snow);
        }
    }

    #[test]
    fn maps_rain_showers_distinct_from_rain() {
        assert_eq!(Condition::from_wmo_code(61), Condition::Rain);
        assert_eq!(Condition::from_wmo_code(80), Condition::RainShowers);
    }

    #[test]
    fn unknown_code_falls_back_to_partly_cloudy() {
        assert_eq!(Condition::from_wmo_code(9999), Condition::PartlyCloudy);
    }
}
