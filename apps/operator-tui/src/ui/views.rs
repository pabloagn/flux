use super::widgets;
use crate::app::{App, ViewMode};
use ratatui::{prelude::*, symbols, widgets::*};

pub fn render_overview(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(32)])
        .split(area);
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(chunks[1]);
    let overview_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(chunks[0]);

    // Delegate to smaller widget functions
    widgets::render_main_grid(f, overview_chunks[0], app);
    widgets::render_horizontal_scrollbar(f, overview_chunks[1], app);
    widgets::render_kpi_cards(f, side_chunks[0], app);
    widgets::render_selection_details(f, side_chunks[1], app);
}

pub fn render_performance(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Real-Time Performance Metrics ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);
    let efficiency_pct = app.metrics.avg_efficiency.clamp(0.0, 100.0) as u16;
    let efficiency_gauge = Gauge::default()
        .block(Block::default().title("Current Efficiency"))
        .percent(efficiency_pct)
        .gauge_style(Style::default().fg(match efficiency_pct {
            0..=90 => Color::Red,
            91..=94 => Color::Yellow,
            _ => Color::Green,
        }))
        .label(format!("{:.1}%", app.metrics.avg_efficiency));
    f.render_widget(efficiency_gauge, gauge_chunks[0]);
    let power_pct = ((app.metrics.total_power_mw / 8.0) * 100.0).clamp(0.0, 100.0) as u16;
    let power_gauge = Gauge::default()
        .block(Block::default().title("Power Consumption"))
        .percent(power_pct)
        .gauge_style(Style::default().fg(Color::Cyan))
        .label(format!("{:.1} MW", app.metrics.total_power_mw));
    f.render_widget(power_gauge, gauge_chunks[1]);
}

pub fn render_economics(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Economic Performance ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner_area = area.inner(&Margin {
        vertical: 2,
        horizontal: 2,
    });
    let table = Table::new(
        vec![Row::new(vec![
            Cell::from(format!("$ {:.0}/h", app.metrics.hourly_revenue)),
            Cell::from(format!("$ {:.0}/h", app.metrics.hourly_energy_cost)),
            Cell::from(format!("{:.1}%", app.metrics.gross_margin)),
            Cell::from(format!(
                "$ {:.0}",
                (app.metrics.hourly_revenue - app.metrics.hourly_energy_cost) * 24.0,
            )),
        ])
        .height(2)
        .style(Style::new().fg(Color::White).bold())],
        [
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ],
    )
    .header(
        Row::new(vec!["REVENUE", "COST", "MARGIN", "DAILY PROFIT"])
            .style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::ALL))
    .column_spacing(2);
    f.render_widget(table, inner_area);
}

pub fn render_cell_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Deep Dive: {} ", app.get_selected_cell_key()))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan).bold());
    f.render_widget(block, area);
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    if let Some(cell) = app.get_selected_cell_data() {
        if cell.voltage_history.is_empty() {
            f.render_widget(
                Paragraph::new("Waiting for historical data...").alignment(Alignment::Center),
                inner_area,
            );
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(inner_area);
        let voltage_data: Vec<(f64, f64)> = cell
            .voltage_history
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();
        let voltage_chart = Chart::new(vec![Dataset::default()
            .name("V")
            .marker(symbols::Marker::Braille)
            .style(Style::new().fg(Color::Cyan))
            .data(&voltage_data)])
        .block(
            Block::default()
                .title("Voltage Trend (V)")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (ticks ago)")
                .bounds([0.0, 300.0])
                .labels(vec!["300".into(), "0".into()]),
        )
        .y_axis(
            Axis::default()
                .title("Voltage")
                .bounds([2.5, 3.8])
                .labels(vec!["2.5".into(), "3.0".into(), "3.5".into()]),
        );
        f.render_widget(voltage_chart, chunks[0]);
        let help = Paragraph::new("Press [Esc] or [Enter] to return to Overview")
            .block(Block::default().title("Actions").borders(Borders::ALL))
            .alignment(Alignment::Center);
        f.render_widget(help, chunks[1]);
    } else {
        f.render_widget(
            Paragraph::new("No data for selected cell.").alignment(Alignment::Center),
            inner_area,
        );
    }
}

pub fn render_placeholder(f: &mut Frame, area: Rect, title: &str) {
    let text = format!("'{}' View Not Implemented Yet", title);
    let placeholder = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title)),
    );
    f.render_widget(placeholder, area);
}
