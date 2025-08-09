pub mod views;
pub mod widgets;

use crate::app::{App, ViewMode};
use ratatui::prelude::*;

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.size());

    widgets::render_header(f, main_layout[0], app);
    widgets::render_tabs(f, main_layout[1], app);

    match app.current_view {
        ViewMode::Overview => views::render_overview(f, main_layout[2], app),
        ViewMode::Performance => views::render_performance(f, main_layout[2], app),
        ViewMode::Economics => views::render_economics(f, main_layout[2], app),
        ViewMode::CellDetail => views::render_cell_detail(f, main_layout[2], app),
        _ => views::render_placeholder(f, main_layout[2], &format!("{:?}", app.current_view)),
    }

    widgets::render_footer(f, main_layout[3], app);
}
