use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tokio::sync::mpsc;

use crate::net::model::WeatherData;
use crate::ui::canvas::{icon_for, Canvas};
use crate::ui::particles::ParticleSystem;
use crate::ui::theme::{self, rgb};
use crate::ui::detail;

/// The status bar occupies the bottom row; the canvas gets everything above it.
const STATUS_BAR_HEIGHT: u16 = 1;

pub struct App {
    pub weather: Option<WeatherData>,
    pub error: Option<String>,
    pub show_details: bool,
    pub should_quit: bool,
    pub last_updated: Option<Instant>,
    wave_phase: f64,
    flash: f64,
    particles: ParticleSystem,
    refresh_tx: mpsc::UnboundedSender<()>,
}

impl App {
    pub fn new(refresh_tx: mpsc::UnboundedSender<()>, width: u16, height: u16) -> Self {
        App {
            weather: None,
            error: None,
            show_details: false,
            should_quit: false,
            last_updated: None,
            wave_phase: 0.0,
            flash: 0.0,
            particles: ParticleSystem::new(
                crate::net::model::Condition::Clear,
                width,
                height.saturating_sub(STATUS_BAR_HEIGHT),
            ),
            refresh_tx,
        }
    }

    pub fn on_resize(&mut self, width: u16, height: u16) {
        self.particles
            .resize(width, height.saturating_sub(STATUS_BAR_HEIGHT));
    }

    pub fn apply_update(&mut self, result: Result<WeatherData, String>) {
        match result {
            Ok(weather) => {
                self.particles.set_condition(weather.condition);
                self.weather = Some(weather);
                self.error = None;
                self.last_updated = Some(Instant::now());
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('d') | KeyCode::Char('i') => self.show_details = !self.show_details,
            KeyCode::Char('r') => {
                let _ = self.refresh_tx.send(());
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, dt: f64) {
        self.wave_phase += dt * 0.6;
        if self.flash > 0.0 {
            self.flash = (self.flash - dt * 2.5).max(0.0);
        } else if let Some(w) = &self.weather {
            if w.condition == crate::net::model::Condition::Thunderstorm {
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.02) {
                    self.flash = 1.0;
                }
            }
        }
        self.particles.tick(dt);
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(STATUS_BAR_HEIGHT)])
            .split(area);

        match &self.weather {
            Some(w) => {
                let hero_text = format!("{}°", w.temperature_c.round() as i64);
                let canvas = Canvas {
                    condition: w.condition,
                    is_day: w.is_day,
                    hero_text,
                    sub_label: w.condition.label(),
                    icon: icon_for(w.condition, w.is_day),
                    wave_phase: self.wave_phase,
                    flash: self.flash,
                    particles: self.particles.particles(),
                };
                frame.render_widget(canvas, chunks[0]);
            }
            None => {
                let msg = match &self.error {
                    Some(e) => format!("could not load weather yet: {e}\n(retrying automatically — press r to retry now)"),
                    None => "fetching local weather...".to_string(),
                };
                let p = Paragraph::new(msg)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(rgb(theme::BG)).fg(rgb(theme::MUTED_TEXT)));
                frame.render_widget(p, chunks[0]);
            }
        }

        let status = self.status_line(area.width);
        let status_p = Paragraph::new(status)
            .alignment(Alignment::Center)
            .style(Style::default().bg(rgb(theme::BG_DIM)).fg(rgb(theme::MUTED_TEXT)));
        frame.render_widget(status_p, chunks[1]);

        if self.show_details {
            if let Some(w) = &self.weather {
                detail::render(frame, w);
            }
        }
    }

    fn status_line(&self, width: u16) -> String {
        let location = self
            .weather
            .as_ref()
            .map(|w| {
                if w.location.region.is_empty() || width < 70 {
                    w.location.city.clone()
                } else {
                    format!("{}, {}", w.location.city, w.location.region)
                }
            })
            .unwrap_or_else(|| "locating...".to_string());

        let updated = self
            .last_updated
            .map(|t| {
                let secs = t.elapsed().as_secs();
                if secs < 60 {
                    format!("updated {secs}s ago")
                } else {
                    format!("updated {}m ago", secs / 60)
                }
            })
            .unwrap_or_else(|| "not yet updated".to_string());

        // Keep the bar informative but never let it overflow a narrow terminal.
        if width < 50 {
            format!("{location} · [d]etails [q]uit")
        } else if width < 70 {
            format!("{location} · {updated} · [d] details [q] quit")
        } else {
            format!("{location}  ·  {updated}  ·  [d] details  [r] refresh  [q] quit")
        }
    }
}

pub const TICK_RATE: Duration = Duration::from_millis(66);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::model::{Condition, Location, WeatherData};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample(condition: Condition) -> WeatherData {
        WeatherData {
            location: Location {
                city: "Asahi".into(),
                region: "Chiba".into(),
                latitude: 35.72,
                longitude: 140.65,
            },
            condition,
            is_day: true,
            temperature_c: 18.4,
            feels_like_c: 17.1,
            humidity_pct: 72.0,
            wind_speed_kmh: 14.0,
            wind_direction_deg: 225.0,
            pressure_hpa: 1013.0,
            uv_index: 4.2,
            precipitation_probability_pct: 65.0,
            sunrise: "05:21".into(),
            sunset: "18:04".into(),
        }
    }

    fn render_at(width: u16, height: u16, condition: Condition, details: bool) -> String {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, width, height);
        app.apply_update(Ok(sample(condition)));
        app.show_details = details;
        app.tick(0.1);

        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let buf = terminal.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf.cell((x, y)).unwrap().symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    const ALL_CONDITIONS: [Condition; 11] = [
        Condition::Clear,
        Condition::PartlyCloudy,
        Condition::Overcast,
        Condition::Fog,
        Condition::Drizzle,
        Condition::Rain,
        Condition::RainShowers,
        Condition::Freezing,
        Condition::Snow,
        Condition::SnowShowers,
        Condition::Thunderstorm,
    ];

    /// The canvas paints cells by hand, so odd geometry is where it would panic.
    #[test]
    fn renders_every_condition_at_any_terminal_size() {
        for (w, h) in [
            (1, 1),
            (2, 3),
            (10, 4),
            (20, 6),
            (46, 14),
            (80, 24),
            (100, 30),
            (240, 70),
        ] {
            for condition in ALL_CONDITIONS {
                for details in [false, true] {
                    render_at(w, h, condition, details);
                }
            }
        }
    }

    #[test]
    fn hero_shows_temperature_and_condition_label() {
        let out = render_at(100, 30, Condition::Snow, false);
        assert!(out.contains("❄ SNOW"), "condition label missing:\n{out}");
        // "18°" is drawn as block glyphs, so assert on the blocks being present.
        assert!(out.contains('█'), "hero glyphs missing:\n{out}");
    }

    #[test]
    fn hero_block_sits_optically_centered() {
        // Only the hero uses full blocks, so its extents are safe to measure.
        for width in [40u16, 60, 80, 100, 240] {
            let out = render_at(width, 30, Condition::Clear, false);
            let (mut first, mut last) = (usize::MAX, 0usize);
            for line in out.lines() {
                for (i, ch) in line.chars().enumerate() {
                    if ch == '\u{2588}' {
                        first = first.min(i);
                        last = last.max(i + 1);
                    }
                }
            }
            assert!(first != usize::MAX, "no hero glyphs at width {width}");
            let (left_gap, right_gap) = (first, width as usize - last);
            // The trailing '°' is discounted, so the block deliberately sits right of
            // dead center — but it must never crowd or run off the right-hand edge.
            assert!(
                left_gap > right_gap,
                "hero not nudged right at width {width}: {left_gap} left vs {right_gap} right\n{out}"
            );
            assert!(
                right_gap >= left_gap / 2,
                "hero nudged too far right at width {width}: {left_gap} left vs {right_gap} right\n{out}"
            );
        }
    }

    #[test]
    fn detail_panel_lists_every_stat() {
        let out = render_at(100, 30, Condition::Rain, true);
        for expected in [
            "Temperature",
            "Feels like",
            "Humidity",
            "Wind",
            "Pressure",
            "UV index",
            "Chance of rain",
            "Sunrise",
            "Sunset",
            "18.4°C",
            "72%",
            "14 km/h SW",
            "05:21",
            "18:04",
            "press d to close",
        ] {
            assert!(out.contains(expected), "detail panel missing {expected:?}:\n{out}");
        }
    }

    #[test]
    fn status_line_never_overflows_a_narrow_terminal() {
        for width in [30u16, 46, 60, 69, 70, 100] {
            let out = render_at(width, 12, Condition::Clear, false);
            let status = out.lines().last().unwrap();
            assert!(
                status.chars().count() <= width as usize,
                "status line overflows at width {width}: {status:?}"
            );
            assert!(status.contains("Asahi"), "location missing at width {width}");
        }
    }

    #[test]
    fn toggling_details_and_quitting_via_keys() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        assert!(!app.show_details);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(app.show_details);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(!app.show_details);

        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_but_plain_c_does_not() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('c')));
        assert!(!app.should_quit);
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn failed_refresh_keeps_the_last_good_reading() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.apply_update(Ok(sample(Condition::Rain)));
        app.apply_update(Err("network down".into()));
        assert!(app.weather.is_some(), "last good weather was discarded");
        assert_eq!(app.error.as_deref(), Some("network down"));
    }

    /// Dev helper: `cargo test -- --ignored --nocapture` to eyeball the artwork.
    #[test]
    #[ignore]
    fn dump_preview() {
        for (name, cond, details) in [
            ("RAIN", Condition::Rain, false),
            ("CLEAR", Condition::Clear, false),
            ("DETAILS", Condition::Snow, true),
        ] {
            println!("\n===== {name} =====\n{}", render_at(100, 30, cond, details));
        }
        println!(
            "\n===== SMALL 46x14 =====\n{}",
            render_at(46, 14, Condition::Thunderstorm, false)
        );
    }
}
