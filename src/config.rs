//! The remembered location, stored as a small hand-editable TOML file.
//!
//! Only four flat keys are involved, so the file is parsed here rather than
//! pulling in a TOML crate for it.

use std::path::PathBuf;

use crate::net::model::Location;

const HEADER: &str = "# tenki — the location you picked with [l].\n\
                      # Edit the coordinates by hand, or delete this file\n\
                      # (or run `tenki --forget`) to go back to IP detection.\n";

/// `$XDG_CONFIG_HOME/tenki/config.toml`, falling back to `~/.config/tenki/config.toml`.
pub fn path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("tenki").join("config.toml"))
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let inner = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next().unwrap_or('\\')),
            _ => out.push(c),
        }
    }
    out
}

fn quote(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A number, ignoring any trailing `# comment`.
fn number(v: &str) -> Option<f64> {
    v.split('#').next()?.trim().parse().ok()
}

fn parse(text: &str) -> Option<Location> {
    let (mut city, mut region) = (None, String::new());
    let (mut latitude, mut longitude) = (None, None);

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "city" => city = Some(unquote(value)),
            "region" => region = unquote(value),
            "latitude" => latitude = number(value),
            "longitude" => longitude = number(value),
            _ => {}
        }
    }

    // Coordinates are the only part we cannot do without.
    Some(Location {
        city: city.filter(|c| !c.is_empty()).unwrap_or_else(|| "Saved location".to_string()),
        region,
        latitude: latitude?,
        longitude: longitude?,
    })
}

fn render(location: &Location) -> String {
    format!(
        "{HEADER}city = {}\nregion = {}\nlatitude = {}\nlongitude = {}\n",
        quote(&location.city),
        quote(&location.region),
        location.latitude,
        location.longitude
    )
}

/// The saved location, or `None` if there is no readable one.
pub fn load() -> Option<Location> {
    let text = std::fs::read_to_string(path()?).ok()?;
    parse(&text)
}

pub fn save(location: &Location) -> Result<(), String> {
    let path = path().ok_or("no config directory (neither XDG_CONFIG_HOME nor HOME is set)")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, render(location)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Forget the saved location. Succeeds when there was nothing to forget.
pub fn forget() -> Result<Option<PathBuf>, String> {
    let path = path().ok_or("no config directory (neither XDG_CONFIG_HOME nor HOME is set)")?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(Some(path)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Location {
        Location {
            city: "Mitte".into(),
            region: "Berlin".into(),
            latitude: 52.52003,
            longitude: 13.40489,
        }
    }

    #[test]
    fn round_trips_a_location() {
        let parsed = parse(&render(&sample())).unwrap();
        assert_eq!(parsed.city, "Mitte");
        assert_eq!(parsed.region, "Berlin");
        assert_eq!(parsed.latitude, 52.52003);
        assert_eq!(parsed.longitude, 13.40489);
    }

    #[test]
    fn round_trips_names_containing_quotes_and_backslashes() {
        let awkward = Location {
            city: "Mar del \"Plata\"\\".into(),
            ..sample()
        };
        assert_eq!(parse(&render(&awkward)).unwrap().city, "Mar del \"Plata\"\\");
    }

    #[test]
    fn accepts_a_hand_written_file() {
        let parsed = parse(
            "# my place\n[location]\n  latitude = 35.72  # roughly\n  longitude=140.65\n  city = \"Asahi\"\n",
        )
        .unwrap();
        assert_eq!((parsed.latitude, parsed.longitude), (35.72, 140.65));
        assert_eq!(parsed.city, "Asahi");
        assert_eq!(parsed.region, "");
    }

    #[test]
    fn a_file_without_coordinates_is_not_a_location() {
        assert!(parse("city = \"Asahi\"\n").is_none());
        assert!(parse("latitude = 35.72\n").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn an_unnamed_location_still_loads() {
        let parsed = parse("latitude = 1.0\nlongitude = 2.0\n").unwrap();
        assert_eq!(parsed.city, "Saved location");
    }

    /// The only test that touches the (process-wide) environment.
    #[test]
    fn config_path_prefers_xdg_over_home() {
        let (xdg, home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );

        std::env::set_var("HOME", "/home/someone");
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            path().unwrap(),
            PathBuf::from("/home/someone/.config/tenki/config.toml")
        );

        std::env::set_var("XDG_CONFIG_HOME", "/elsewhere");
        assert_eq!(
            path().unwrap(),
            PathBuf::from("/elsewhere/tenki/config.toml")
        );

        std::env::remove_var("HOME");
        std::env::remove_var("XDG_CONFIG_HOME");
        assert!(path().is_none(), "no home means no config file");

        match xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}
