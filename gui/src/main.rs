use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io, panic};
use tokio::{sync::mpsc, time::Duration};

mod app;
mod bridge;
mod config;
mod ui;

// Plant configuration load
// use crate::config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // NOTE: Set environment to suppress rdkafka logs BEFORE any Kafka initialization
    // std::env::set_var("RDKAFKA_LOG_LEVEL", "0");
    // std::env::set_var("RUST_LOG", "error");

    // NOTE: Setup terminal FIRST before any output
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // --- Add panic hook ---
    // This ensures that if the app panics, the terminal is restored to a usable state.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore the terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);

        // Call the original panic hook to print the error
        original_hook(panic_info);
    }));

    // Load layout
    let layout = config::config();

    let mut app = app::App::new((*layout).clone());

    let (tx, mut rx) = mpsc::channel::<String>(10000);

    // NOTE:
    // These topic names MUST match what your Python simulation is sending to
    // Eventually topics are ingested from config of course
    let topics = [
        &layout.topics.voltage,
        &layout.topics.pressure,
        &layout.topics.temp_anolyte,
    ];

    for topic in topics {
        let tx_clone = tx.clone();
        let topic_owned = topic.to_string();
        tokio::spawn(async move {
            loop {
                // Try to connect, but don't crash on failure
                let _ =
                    bridge::kafka::run_with_retry("localhost:9092", &topic_owned, tx_clone.clone())
                        .await;
                // Wait before retrying to avoid rapid reconnection attempts
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    // Main loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = tokio::time::Instant::now();

    loop {
        // Process ALL pending messages quickly
        let mut batch_count = 0;
        while let Ok(msg) = rx.try_recv() {
            app.process_message(&msg);
            batch_count += 1;
            if batch_count > 100 {
                break; // Process max 100 per frame to avoid blocking
            }
        }

        // Draw UI
        terminal.draw(|f| ui::draw(f, &app))?;

        // Handle input - THIS MUST WORK!
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,

                    // Cycle views
                    KeyCode::Tab => app.next_view(),
                    KeyCode::BackTab => app.previous_view(),

                    // Cycle membranes
                    KeyCode::Up => app.navigate_up(),
                    KeyCode::Down => app.navigate_down(),
                    KeyCode::Left => app.navigate_left(),
                    KeyCode::Right => app.navigate_right(),
                    // KeyCode::Char('1') => app.selected_unit = 1,
                    // KeyCode::Char('2') => app.selected_unit = 2,
                    // KeyCode::Char('3') => app.selected_unit = 3,
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if let Some(id) = c.to_digit(10).map(|d| d as u8) {
                            if layout.geometry.units.contains(&id) {
                                app.selected_unit = id;
                            }
                        }
                    }

                    // Pause interface
                    KeyCode::Char(' ') => app.paused = !app.paused,

                    // Change views
                    KeyCode::F(1) => app.current_view = app::ViewMode::Overview,
                    KeyCode::F(2) => app.current_view = app::ViewMode::Performance,
                    KeyCode::F(3) => app.current_view = app::ViewMode::Predictive,
                    KeyCode::F(4) => app.current_view = app::ViewMode::Economics,
                    KeyCode::F(5) => app.current_view = app::ViewMode::Maintenance,
                    KeyCode::F(6) => app.current_view = app::ViewMode::Alarms,
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = tokio::time::Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
