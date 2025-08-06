use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;
use tokio::{sync::mpsc, task, time};

mod app;
mod bridge;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Kafka → UI channel
    let (tx, mut rx) = mpsc::channel::<String>(128);
    task::spawn(async move {
        let _ = bridge::kafka::run("localhost:9092", "flux.electrical.realtime", tx).await;
    });

    // Terminal init
    let mut stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = app::App::new();

    loop {
        // non-blocking message pull
        if let Ok(line) = rx.try_recv() {
            app.last_msg = line;
        }

        term.draw(|f| app.ui(f))?;

        if event::poll(time::Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                if matches!(k.code, KeyCode::Char('q' | 'Q')) {
                    break;
                }
            }
        }
    }

    // restore terminal
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}
