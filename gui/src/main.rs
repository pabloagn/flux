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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Add panic hook to restore terminal on crash
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let layout = config::config();
    let mut app = app::App::new((*layout).clone());
    let (tx, mut rx) = mpsc::channel::<String>(10000);

    // Kafka producer spawning
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
                let _ =
                    bridge::kafka::run_with_retry("localhost:9092", &topic_owned, tx_clone.clone())
                        .await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    // Main loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = tokio::time::Instant::now();

    loop {
        // Process messages
        while let Ok(msg) = rx.try_recv() {
            app.process_message(&msg);
        }

        // Draw UI (pass app as mutable for scroll state)
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle input
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                // Global quit
                if key.code == KeyCode::Char('Q') {
                    break;
                }
                // Delegate all other key handling to the app
                app.handle_key_event(key.code);
            }
        }

        // Update app state on tick
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
