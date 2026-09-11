use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Row, Table};
use ratatui::Frame;

use crate::net::model::WeatherData;
use crate::ui::theme::{self, rgb};

const PANEL_WIDTH: u16 = 56;
const PANEL_HEIGHT: u16 = 16;

/// A rect of at most `width` x `height`, centred in `r` and clamped to fit it.
pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    Rect {
        x: r.x + (r.width - w) / 2,
        y: r.y + (r.height - h) / 2,
        width: w,
        height: h,
    }
}

fn compass(deg: f64) -> &'static str {
    const DIRS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let idx = (((deg.rem_euclid(360.0)) / 45.0).round() as usize) % 8;
    DIRS[idx]
}

fn row(icon: &'static str, label: &'static str, value: String) -> Row<'static> {
    Row::new(vec![
        Cell::from(icon).style(Style::default().fg(rgb(theme::PEACH))),
        Cell::from(label).style(
            Style::default()
                .fg(rgb(theme::MUTED_TEXT))
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(value).style(Style::default().fg(rgb(theme::PALE))),
    ])
}

fn spacer() -> Row<'static> {
    Row::new(vec![Cell::from(""), Cell::from(""), Cell::from("")])
}

pub fn render(frame: &mut Frame, weather: &WeatherData) {
    let popup = centered_rect(PANEL_WIDTH, PANEL_HEIGHT, frame.area());
    frame.render_widget(Clear, popup);

    let title = if weather.location.region.is_empty() {
        format!(" {} ", weather.location.city)
    } else {
        format!(" {}, {} ", weather.location.city, weather.location.region)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb(theme::LAVENDER)))
        .style(Style::default().bg(rgb(theme::BG_DIM)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(rgb(theme::PINK))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .title_bottom(
            Line::from(Span::styled(
                " press d to close ",
                Style::default().fg(rgb(theme::MUTED_TEXT)),
            ))
            .centered(),
        );

    let rows = vec![
        spacer(),
        row(
            "🌡",
            "Temperature",
            format!("{:.1}°C", weather.temperature_c),
        ),
        row("🤔", "Feels like", format!("{:.1}°C", weather.feels_like_c)),
        row("💧", "Humidity", format!("{:.0}%", weather.humidity_pct)),
        row(
            "🌬",
            "Wind",
            format!(
                "{:.0} km/h {}",
                weather.wind_speed_kmh,
                compass(weather.wind_direction_deg)
            ),
        ),
        row("⏲", "Pressure", format!("{:.0} hPa", weather.pressure_hpa)),
        row("😎", "UV index", format!("{:.1}", weather.uv_index)),
        row(
            "🌂",
            "Chance of rain",
            format!("{:.0}%", weather.precipitation_probability_pct),
        ),
        spacer(),
        row("🌅", "Sunrise", weather.sunrise.clone()),
        row("🌇", "Sunset", weather.sunset.clone()),
    ];

    // Fixed-width icon/label columns keep the values aligned even though the
    // icons are a mix of single- and double-width glyphs.
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(16),
            Constraint::Min(10),
        ],
    )
    .column_spacing(2)
    .block(block);

    frame.render_widget(table, popup);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_north() {
        assert_eq!(compass(0.0), "N");
        assert_eq!(compass(360.0), "N");
    }

    #[test]
    fn compass_quadrants() {
        assert_eq!(compass(90.0), "E");
        assert_eq!(compass(180.0), "S");
        assert_eq!(compass(270.0), "W");
    }

    #[test]
    fn panel_is_centred_in_a_large_area() {
        let area = Rect::new(0, 0, 100, 30);
        let r = centered_rect(PANEL_WIDTH, PANEL_HEIGHT, area);
        assert_eq!(r.width, PANEL_WIDTH);
        assert_eq!(r.height, PANEL_HEIGHT);
        assert_eq!(r.x + r.width / 2, 50);
    }

    #[test]
    fn panel_clamps_to_a_small_area() {
        let area = Rect::new(0, 0, 30, 10);
        let r = centered_rect(PANEL_WIDTH, PANEL_HEIGHT, area);
        assert_eq!((r.width, r.height), (30, 10));
        assert_eq!((r.x, r.y), (0, 0));
    }
}
