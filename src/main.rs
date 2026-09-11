mod app;
mod net;
mod ui;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use app::{App, TICK_RATE};
use net::model::WeatherData;

const REFRESH_INTERVAL: Duration = Duration::from_secs(600);

struct Args {
    location: Option<(f64, f64)>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { location: None };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let (mut lat, mut lon) = (None, None);

    while i < argv.len() {
        match argv[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("tenki {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--lat" => {
                i += 1;
                let v = argv.get(i).ok_or("--lat requires a value")?;
                lat = Some(v.parse::<f64>().map_err(|_| format!("invalid latitude: {v}"))?);
            }
            "--lon" => {
                i += 1;
                let v = argv.get(i).ok_or("--lon requires a value")?;
                lon = Some(v.parse::<f64>().map_err(|_| format!("invalid longitude: {v}"))?);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    match (lat, lon) {
        (Some(la), Some(lo)) => args.location = Some((la, lo)),
        (None, None) => {}
        _ => return Err("--lat and --lon must be given together".to_string()),
    }
    Ok(args)
}

fn print_help() {
    println!(
        "tenki — animated terminal weather\n\n\
         USAGE:\n    tenki [OPTIONS]\n\n\
         OPTIONS:\n\
         \x20   --lat <DEGREES>   latitude override (requires --lon)\n\
         \x20   --lon <DEGREES>   longitude override (requires --lat)\n\
         \x20   -h, --help        print this help\n\
         \x20   -V, --version     print version\n\n\
         KEYS:\n\
         \x20   d / i   toggle the detail panel\n\
         \x20   r       refresh now\n\
         \x20   q / Esc quit\n\n\
         Location is detected from your IP address unless --lat/--lon are given.\n\
         Weather data from Open-Meteo."
    );
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::cursor::Hide,
        crossterm::event::EnableMouseCapture
    )?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::Show,
        LeaveAlternateScreen
    )?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("tenki: {e}\n");
            print_help();
            std::process::exit(2);
        }
    };

    // Any panic must not leave the user's terminal in raw mode / alt screen.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        default_hook(info);
    }));

    if let Err(e) = run(args).await {
        let _ = restore_terminal();
        eprintln!("tenki: {e}");
        std::process::exit(1);
    }
    let _ = restore_terminal();
}

async fn run(args: Args) -> io::Result<()> {
    let (weather_tx, mut weather_rx) = mpsc::unbounded_channel::<Result<WeatherData, String>>();
    let (refresh_tx, mut refresh_rx) = mpsc::unbounded_channel::<()>();

    // Background fetcher: initial load, a periodic refresh, and manual `r` requests.
    let override_location = args.location;
    tokio::spawn(async move {
        let client = net::client();
        loop {
            let result = net::fetch_all(&client, override_location).await;
            if weather_tx.send(result).is_err() {
                return; // UI is gone
            }
            tokio::select! {
                _ = tokio::time::sleep(REFRESH_INTERVAL) => {}
                msg = refresh_rx.recv() => {
                    if msg.is_none() {
                        return;
                    }
                }
            }
        }
    });

    let mut terminal = setup_terminal()?;
    let size = terminal.size()?;
    let mut app = App::new(refresh_tx, size.width, size.height);
    let mut last_tick = Instant::now();
    let mut last_size = (size.width, size.height);

    while !app.should_quit {
        while let Ok(update) = weather_rx.try_recv() {
            app.apply_update(update);
        }

        terminal.draw(|frame| app.draw(frame))?;

        // Polling with the remaining slice of the frame budget doubles as the frame clock.
        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == event::KeyEventKind::Press => app.handle_key(key),
                Event::Resize(w, h) => {
                    last_size = (w, h);
                    app.on_resize(w, h);
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            let current = terminal.size()?;
            if (current.width, current.height) != last_size {
                last_size = (current.width, current.height);
                app.on_resize(current.width, current.height);
            }
            app.tick(last_tick.elapsed().as_secs_f64());
            last_tick = Instant::now();
        }
    }

    Ok(())
}
