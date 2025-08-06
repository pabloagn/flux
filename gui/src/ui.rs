use crate::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.size());

    // Title
    let title = Paragraph::new("Flux ▸ Chlor-Alkali Digital Twin")
        .style(Style::default().fg(Color::Cyan).bold());
    f.render_widget(title, chunks[0]);

    // Cell grid widget draws 180 coloured blocks
    f.render_widget(app.cell_grid_widget(), chunks[1]);
}
