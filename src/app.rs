use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tokio::sync::mpsc;

use crate::net::geocode::Place;
use crate::net::model::{Location, WeatherData};
use crate::ui::canvas::{icon_for, Canvas};
use crate::ui::detail;
use crate::ui::particles::ParticleSystem;
use crate::ui::search::{self, truncate};
use crate::ui::theme::{self, rgb};

/// The status bar occupies the bottom row; the canvas gets everything above it.
const STATUS_BAR_HEIGHT: u16 = 1;

/// How long a one-off message (a failed config write, say) sits in the status bar.
const NOTICE_LIFETIME: Duration = Duration::from_secs(6);

/// Work the UI asks the background tasks to do.
#[derive(Debug, Clone)]
pub enum Request {
    /// Re-fetch the weather now.
    Refresh,
    /// Look a place name up with the geocoder.
    Search(String),
    /// Use this location from now on, and remember it.
    Use(Location),
}

/// News from the background tasks.
#[derive(Debug)]
pub enum Update {
    Weather(Result<WeatherData, String>),
    Search(Result<Vec<Place>, String>),
    /// Something worth a line in the status bar, but not worth failing over.
    Notice(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchStatus {
    /// Waiting for the user — the results list, if any, is theirs to pick from.
    Typing,
    Searching,
    NoMatches,
    Failed(String),
}

/// State of the `l` location picker.
#[derive(Debug, Clone)]
pub struct Search {
    pub query: String,
    /// The query the current results belong to; empty until one is sent.
    pub submitted: String,
    pub results: Vec<Place>,
    pub selected: usize,
    pub status: SearchStatus,
}

impl Search {
    fn new() -> Search {
        Search {
            query: String::new(),
            submitted: String::new(),
            results: Vec::new(),
            selected: 0,
            status: SearchStatus::Typing,
        }
    }

    /// Enter picks a result only when the list matches what is typed; otherwise
    /// it runs the search.
    fn results_are_current(&self) -> bool {
        !self.results.is_empty() && self.submitted == self.query.trim()
    }
}

pub struct App {
    pub weather: Option<WeatherData>,
    pub error: Option<String>,
    pub show_details: bool,
    pub should_quit: bool,
    pub last_updated: Option<Instant>,
    pub search: Option<Search>,
    /// A location just picked, shown until its first reading arrives.
    pub pending_location: Option<Location>,
    notice: Option<(String, Instant)>,
    wave_phase: f64,
    flash: f64,
    particles: ParticleSystem,
    req_tx: mpsc::UnboundedSender<Request>,
}

impl App {
    pub fn new(req_tx: mpsc::UnboundedSender<Request>, width: u16, height: u16) -> Self {
        App {
            weather: None,
            error: None,
            show_details: false,
            should_quit: false,
            last_updated: None,
            search: None,
            pending_location: None,
            notice: None,
            wave_phase: 0.0,
            flash: 0.0,
            particles: ParticleSystem::new(
                crate::net::model::Condition::Clear,
                width,
                height.saturating_sub(STATUS_BAR_HEIGHT),
            ),
            req_tx,
        }
    }

    pub fn on_resize(&mut self, width: u16, height: u16) {
        self.particles
            .resize(width, height.saturating_sub(STATUS_BAR_HEIGHT));
    }

    pub fn apply_update(&mut self, update: Update) {
        match update {
            Update::Weather(Ok(weather)) => {
                self.particles.set_condition(weather.condition);
                self.weather = Some(weather);
                self.error = None;
                self.pending_location = None;
                self.last_updated = Some(Instant::now());
            }
            Update::Weather(Err(e)) => {
                // Losing the name the user just picked, silently, reads as if the
                // pick never took — say why the status bar is reverting.
                if self.pending_location.take().is_some() {
                    self.notice = Some((format!("could not load that location: {e}"), Instant::now()));
                }
                self.error = Some(e);
            }
            Update::Search(result) => {
                // Results that arrive after the picker is closed are dropped.
                if let Some(search) = &mut self.search {
                    match result {
                        Ok(places) if places.is_empty() => {
                            search.results.clear();
                            search.status = SearchStatus::NoMatches;
                        }
                        Ok(places) => {
                            search.results = places;
                            search.selected = 0;
                            search.status = SearchStatus::Typing;
                        }
                        Err(e) => {
                            search.results.clear();
                            search.status = SearchStatus::Failed(e);
                        }
                    }
                }
            }
            Update::Notice(msg) => self.notice = Some((msg, Instant::now())),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        // While the picker is open every other key belongs to it.
        if self.search.is_some() {
            self.handle_search_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('d') | KeyCode::Char('i') => self.show_details = !self.show_details,
            KeyCode::Char('l') => {
                self.show_details = false;
                self.search = Some(Search::new());
            }
            KeyCode::Char('r') => {
                let _ = self.req_tx.send(Request::Refresh);
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        let Some(search) = &mut self.search else {
            return;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => self.search = None,
            KeyCode::Enter => {
                if search.results_are_current() {
                    self.use_selected();
                } else {
                    self.submit_search();
                }
            }
            KeyCode::Up | KeyCode::BackTab => {
                if !search.results.is_empty() {
                    search.selected = (search.selected + search.results.len() - 1)
                        % search.results.len();
                }
            }
            KeyCode::Down | KeyCode::Tab => {
                if !search.results.is_empty() {
                    search.selected = (search.selected + 1) % search.results.len();
                }
            }
            KeyCode::Backspace => {
                search.query.pop();
            }
            KeyCode::Char('u') if ctrl => search.query.clear(),
            KeyCode::Char('w') if ctrl => {
                // Drop the trailing word, plus the whitespace in front of it.
                // Pasted text can hold multi-byte spaces, so cut on a char boundary.
                let trimmed = search.query.trim_end();
                let cut = trimmed
                    .char_indices()
                    .rev()
                    .find(|(_, c)| c.is_whitespace())
                    .map_or(0, |(i, c)| i + c.len_utf8());
                search.query.truncate(cut);
            }
            KeyCode::Char(c) if !ctrl => search.query.push(c),
            _ => {}
        }
    }

    fn submit_search(&mut self) {
        let Some(search) = &mut self.search else {
            return;
        };
        let query = search.query.trim().to_string();
        if query.is_empty() {
            return;
        }
        search.submitted = query.clone();
        search.results.clear();
        search.status = SearchStatus::Searching;
        let _ = self.req_tx.send(Request::Search(query));
    }

    fn use_selected(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        let Some(place) = search.results.get(search.selected).cloned() else {
            return;
        };
        let _ = self.req_tx.send(Request::Use(place.location.clone()));
        self.pending_location = Some(place.location);
        self.search = None;
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
                    Some(e) => format!("could not load weather yet: {e}\n(retrying automatically — press r to retry now, l to pick a location)"),
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
        if let Some(s) = &self.search {
            search::render(frame, s);
        }
    }

    /// The place the status bar names: what was just picked, else what was fetched.
    fn location(&self) -> Option<&Location> {
        self.pending_location
            .as_ref()
            .or_else(|| self.weather.as_ref().map(|w| &w.location))
    }

    /// The middle segment: a transient notice, else the age of the reading.
    fn freshness(&self) -> String {
        if let Some((msg, at)) = &self.notice {
            if at.elapsed() < NOTICE_LIFETIME {
                return msg.clone();
            }
        }
        if self.pending_location.is_some() {
            return "updating…".to_string();
        }
        match self.last_updated {
            Some(t) => {
                let secs = t.elapsed().as_secs();
                if secs < 60 {
                    format!("updated {secs}s ago")
                } else {
                    format!("updated {}m ago", secs / 60)
                }
            }
            None => "not yet updated".to_string(),
        }
    }

    fn status_line(&self, width: u16) -> String {
        let width = width as usize;
        let (long, short) = match self.location() {
            Some(loc) if loc.region.is_empty() => (loc.city.clone(), loc.city.clone()),
            Some(loc) => (format!("{}, {}", loc.city, loc.region), loc.city.clone()),
            None => ("locating...".to_string(), "locating...".to_string()),
        };
        let fresh = self.freshness();

        // Widest first: the first line that fits the terminal wins, so hints are
        // dropped rather than truncated mid-word.
        let ladder = [
            format!("{long}  ·  {fresh}  ·  [d] details  [l] location  [r] refresh  [q] quit"),
            format!("{long} · {fresh} · [d] details [l] location [r] refresh [q] quit"),
            format!("{short} · {fresh} · [d] details [l] location [q] quit"),
            format!("{short} · {fresh} · [d]etails [l]ocation [q]uit"),
            format!("{short} · [d]etails [l]ocation [q]uit"),
            format!("{short} · [d] [l] [q]"),
            short.clone(),
        ];
        ladder
            .into_iter()
            .find(|line| line.chars().count() <= width)
            .unwrap_or_else(|| truncate(&short, width))
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

    fn place(city: &str, region: &str, lat: f64, lon: f64) -> Place {
        Place {
            location: Location {
                city: city.into(),
                region: region.into(),
                latitude: lat,
                longitude: lon,
            },
            label: format!("{city}, {region}"),
        }
    }

    fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
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

    fn draw(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        buffer_to_string(&terminal)
    }

    fn render_at(width: u16, height: u16, condition: Condition, details: bool) -> String {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, width, height);
        app.apply_update(Update::Weather(Ok(sample(condition))));
        app.show_details = details;
        app.tick(0.1);
        draw(&app, width, height)
    }

    /// An app with a reading on screen and the picker open on two results.
    fn app_with_results() -> (App, mpsc::UnboundedReceiver<Request>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 100, 30);
        app.apply_update(Update::Weather(Ok(sample(Condition::Clear))));
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        for c in "berlin".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        rx.try_recv().expect("a search request");
        app.apply_update(Update::Search(Ok(vec![
            place("Mitte", "Berlin", 52.52, 13.40),
            place("Hamburg-Mitte", "Hamburg", 53.55, 9.99),
        ])));
        (app, rx)
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

    /// The picker is an overlay drawn by hand too, in every state it can be in.
    #[test]
    fn renders_the_picker_at_any_terminal_size() {
        let states = [
            SearchStatus::Typing,
            SearchStatus::Searching,
            SearchStatus::NoMatches,
            SearchStatus::Failed("geocoder request failed: timeout".into()),
        ];
        for (w, h) in [(1, 1), (2, 3), (10, 4), (20, 6), (46, 14), (100, 30), (240, 70)] {
            for status in &states {
                for results in [0usize, 1, 8] {
                    let (tx, _rx) = mpsc::unbounded_channel();
                    let mut app = App::new(tx, w, h);
                    app.apply_update(Update::Weather(Ok(sample(Condition::Rain))));
                    app.search = Some(Search {
                        query: "berlin mitte".into(),
                        submitted: "berlin mitte".into(),
                        results: (0..results)
                            .map(|i| place("Mitte", "Berlin", 52.5 + i as f64, 13.4))
                            .collect(),
                        selected: results.saturating_sub(1),
                        status: status.clone(),
                    });
                    draw(&app, w, h);
                }
            }
        }
    }

    #[test]
    fn hero_shows_temperature_and_condition_label() {
        // Clear skies are the only particle-free condition: rain and snow are
        // painted *in front of* the label, which would make this a coin flip.
        let out = render_at(100, 30, Condition::Clear, false);
        assert!(out.contains("☀ CLEAR"), "condition label missing:\n{out}");
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

    /// The icon column used to sit flush against the left border.
    #[test]
    fn detail_panel_keeps_a_margin_inside_its_border() {
        let out = render_at(100, 30, Condition::Rain, true);
        let body: Vec<&str> = out
            .lines()
            .filter(|l| l.contains('│') && l.contains("Temperature"))
            .collect();
        assert_eq!(body.len(), 1, "expected one temperature row:\n{out}");

        let row = body[0];
        let inner = &row[row.find('│').unwrap() + '│'.len_utf8()..];
        let inner = &inner[..inner.rfind('│').unwrap()];
        assert!(
            inner.starts_with("  ") && !inner.starts_with("   "),
            "left margin is not two columns: {inner:?}"
        );
        assert!(inner.ends_with("  "), "no right margin: {inner:?}");
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

    /// Hints are dropped whole, never cut in half, and the location always stays.
    #[test]
    fn status_line_keeps_the_location_hint_until_space_runs_out() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 100, 30);
        app.apply_update(Update::Weather(Ok(sample(Condition::Clear))));

        for width in 1u16..=120 {
            let line = app.status_line(width);
            assert!(
                line.chars().count() <= width as usize,
                "status line overflows at width {width}: {line:?}"
            );
            if width >= 46 {
                assert!(line.contains("[l]"), "no location hint at width {width}: {line:?}");
            }
        }
        assert!(app.status_line(100).contains("[l] location"));
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
        app.apply_update(Update::Weather(Ok(sample(Condition::Rain))));
        app.apply_update(Update::Weather(Err("network down".into())));
        assert!(app.weather.is_some(), "last good weather was discarded");
        assert_eq!(app.error.as_deref(), Some("network down"));
    }

    #[test]
    fn l_opens_the_picker_and_esc_closes_it() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert!(app.search.is_some());
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.search.is_none());
        assert!(!app.should_quit, "esc closed the picker and quit the app");
    }

    /// With the picker open the command keys are just letters.
    #[test]
    fn typing_in_the_picker_does_not_trigger_commands() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        for c in "quedri".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(app.search.as_ref().unwrap().query, "quedri");
        assert!(!app.should_quit);
        assert!(!app.show_details);
        assert!(rx.try_recv().is_err(), "a key press leaked out as a request");
    }

    #[test]
    fn editing_keys_work_on_the_query() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        for c in "berlin mitte".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(app.search.as_ref().unwrap().query, "berlin mitt");
        app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(app.search.as_ref().unwrap().query, "berlin ");
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.search.as_ref().unwrap().query, "");
    }

    #[test]
    fn erasing_a_word_survives_multi_byte_whitespace() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        // A non-breaking space, as pasted from a web page.
        for c in "berlin\u{a0}mitte".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(c)));
        }
        app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(app.search.as_ref().unwrap().query, "berlin\u{a0}");
    }

    #[test]
    fn a_location_that_will_not_load_says_so_instead_of_silently_reverting() {
        let (mut app, _rx) = app_with_results();
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(app.pending_location.is_some());
        app.apply_update(Update::Weather(Err("weather request failed".into())));
        assert!(app.pending_location.is_none());
        let status = app.status_line(100);
        assert!(status.contains("could not load that location"), "no explanation: {status:?}");
        assert!(status.contains("Asahi"), "lost the last good location: {status:?}");
    }

    #[test]
    fn enter_searches_then_picks() {
        let (mut app, mut rx) = app_with_results();
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.status, SearchStatus::Typing);
        assert_eq!(search.results.len(), 2);

        app.handle_key(KeyEvent::from(KeyCode::Down));
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        match rx.try_recv().expect("a use-location request") {
            Request::Use(loc) => {
                assert_eq!(loc.city, "Hamburg-Mitte");
                assert_eq!(loc.latitude, 53.55);
            }
            other => panic!("unexpected request: {other:?}"),
        }
        assert!(app.search.is_none(), "picker stayed open after picking");
        assert_eq!(app.pending_location.as_ref().unwrap().city, "Hamburg-Mitte");
    }

    /// Editing after a search makes Enter search again rather than pick a stale row.
    #[test]
    fn editing_the_query_re_arms_the_search() {
        let (mut app, mut rx) = app_with_results();
        app.handle_key(KeyEvent::from(KeyCode::Char('x')));
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        match rx.try_recv().expect("a second search request") {
            Request::Search(q) => assert_eq!(q, "berlinx"),
            other => panic!("unexpected request: {other:?}"),
        }
        assert_eq!(app.search.as_ref().unwrap().status, SearchStatus::Searching);
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        let (mut app, _rx) = app_with_results();
        assert_eq!(app.search.as_ref().unwrap().selected, 0);
        app.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(app.search.as_ref().unwrap().selected, 1);
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(app.search.as_ref().unwrap().selected, 0);
    }

    #[test]
    fn an_empty_query_is_not_searched() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        app.handle_key(KeyEvent::from(KeyCode::Char(' ')));
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(rx.try_recv().is_err(), "blank query hit the geocoder");
        assert!(app.search.is_some());
    }

    #[test]
    fn a_search_with_no_matches_says_so() {
        let (mut app, _rx) = app_with_results();
        app.apply_update(Update::Search(Ok(Vec::new())));
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.status, SearchStatus::NoMatches);
        assert!(search.results.is_empty());

        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(app.search.is_some(), "enter closed the picker with nothing picked");
    }

    #[test]
    fn a_failed_search_leaves_the_picker_usable() {
        let (mut app, _rx) = app_with_results();
        app.apply_update(Update::Search(Err("geocoder request failed".into())));
        assert_eq!(
            app.search.as_ref().unwrap().status,
            SearchStatus::Failed("geocoder request failed".into())
        );
        let out = draw(&app, 100, 30);
        assert!(out.contains("geocoder request failed"), "error not shown:\n{out}");
    }

    /// Results for a picker the user already closed must not reopen it.
    #[test]
    fn late_results_are_dropped_once_the_picker_is_closed() {
        let (mut app, _rx) = app_with_results();
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.apply_update(Update::Search(Ok(vec![place("Mitte", "Berlin", 52.5, 13.4)])));
        assert!(app.search.is_none());
    }

    #[test]
    fn the_picked_place_shows_in_the_status_bar_before_its_weather_arrives() {
        let (mut app, _rx) = app_with_results();
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        let status = draw(&app, 100, 30);
        let status = status.lines().last().unwrap();
        assert!(status.contains("Mitte"), "picked place missing: {status:?}");
        assert!(status.contains("updating"), "no pending marker: {status:?}");

        let mut arrived = sample(Condition::Clear);
        arrived.location = place("Mitte", "Berlin", 52.52, 13.40).location;
        app.apply_update(Update::Weather(Ok(arrived)));
        assert!(app.pending_location.is_none());
        assert!(draw(&app, 100, 30).contains("Mitte, Berlin"));
    }

    #[test]
    fn a_notice_replaces_the_freshness_segment_for_a_while() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(tx, 100, 30);
        app.apply_update(Update::Weather(Ok(sample(Condition::Clear))));
        app.apply_update(Update::Notice("could not save location: read-only".into()));
        assert!(app.status_line(100).contains("could not save location"));
    }

    #[test]
    fn the_picker_draws_the_query_and_its_results() {
        let (app, _rx) = app_with_results();
        let out = draw(&app, 100, 30);
        assert!(out.contains("where are you?"), "no title:\n{out}");
        assert!(out.contains("berlin"), "query missing:\n{out}");
        assert!(out.contains("Mitte, Berlin"), "first result missing:\n{out}");
        assert!(out.contains("Hamburg-Mitte, Hamburg"), "second result missing:\n{out}");
        assert!(out.contains("52.52,13.40"), "coordinates missing:\n{out}");
        assert!(out.contains("⏎ use"), "no key hints:\n{out}");
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
        let (app, _rx) = app_with_results();
        println!("\n===== SEARCH =====\n{}", draw(&app, 100, 30));
    }
}
