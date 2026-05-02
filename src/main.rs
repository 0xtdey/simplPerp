use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    io,
    panic,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

mod app;
mod engine;
mod events;
mod persistence;
mod ui;
mod user;

use app::App;
use events::{Event, EventHandler};
use ui::render;

#[tokio::main]
async fn main() -> Result<()> {
    let app = bootstrap_app().await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn bootstrap_app() -> Result<App> {
    let data_dir = PathBuf::from(".terminal-perps");
    std::fs::create_dir_all(&data_dir)?;

    let state_path = data_dir.join("state.json");
    let app = if state_path.exists() {
        persistence::load(&state_path).unwrap_or_else(|_| App::new(state_path))
    } else {
        App::new(state_path)
    };

    Ok(app)
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let event_handler = EventHandler::new(tx, tick_rate);
    event_handler.spawn();

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| render(f, &mut app))?;

        let _timeout = tick_rate.saturating_sub(last_tick.elapsed());

        if let Some(event) = rx.recv().await {
            match event {
                Event::Tick => {
                    app.on_tick();
                }
                Event::Crossterm(event) => {
                    if let crossterm::event::Event::Key(key) = event {
                        if app.handle_input(key).await? {
                            persistence::save(&app)?;
                            break;
                        }
                    }
                }
                Event::MarketUpdate(_) => {}

            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}
