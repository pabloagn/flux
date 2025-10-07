use crate::app::App;
use crate::data::QuestDBClient;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io, panic, sync::Arc};
use tokio::time::{Duration, Instant};

mod app;
mod calculations;
mod config;
mod data;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Panic handler
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Initialize configuration
    let config = config::config();

    // Initialize QuestDB client
    let questdb = Arc::new(
        QuestDBClient::new("localhost", 8812, "admin", "quest").await?,
    );

    // Create app with QuestDB client
    let mut app = App::new(config.clone(), questdb.clone()).await?;

    // Timing controls
    let tick_rate = Duration::from_millis(50);
    let refresh_rate = Duration::from_millis(500); // Refresh data every 500ms
    let mut last_tick = Instant::now();
    let mut last_refresh = Instant::now();

    // Main event loop
    loop {
        // Refresh data from QuestDB
        if last_refresh.elapsed() >= refresh_rate && !app.paused {
            if let Err(e) = app.refresh_data().await {
                eprintln!("Failed to refresh data: {}", e);
                // Continue running even if refresh fails
            }
            last_refresh = Instant::now();
        }

        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle events
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('Q') {
                    break;
                }
                app.handle_key_event(key.code);
            }
        }

        // Update app state
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
