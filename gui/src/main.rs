use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tokio::{sync::mpsc, time::Duration};

mod app;
mod bridge;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // NO LOGGER! It breaks the TUI
    // env_logger would write to stdout and destroy our terminal UI

    // Setup terminal FIRST before any output
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = app::App::new();

    // Setup Kafka consumers for all topics
    let (tx, mut rx) = mpsc::channel::<String>(10000);

    // These topic names MUST match what your Python simulation is sending to
    let topics = [
        "flux_electrical_realtime",
        "flux_process_temperatures",
        "flux_process_pressures",
    ];

    for topic in topics {
        let tx_clone = tx.clone();
        let topic_owned = topic.to_string();
        tokio::spawn(async move {
            // Silently handle errors - no printing!
            let _ = bridge::kafka::run("localhost:9092", &topic_owned, tx_clone).await;
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

        // Handle input
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Tab => app.show_details = !app.show_details,
                    KeyCode::Up => {
                        if app.selected_cell > 1 {
                            app.selected_cell -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.selected_cell < 20 {
                            app.selected_cell += 1;
                        }
                    }
                    KeyCode::Left => match app.selected_stack {
                        'B' => app.selected_stack = 'A',
                        'C' => app.selected_stack = 'B',
                        _ => {}
                    },
                    KeyCode::Right => match app.selected_stack {
                        'A' => app.selected_stack = 'B',
                        'B' => app.selected_stack = 'C',
                        _ => {}
                    },
                    KeyCode::Char('1') => app.selected_unit = 1,
                    KeyCode::Char('2') => app.selected_unit = 2,
                    KeyCode::Char('3') => app.selected_unit = 3,
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
