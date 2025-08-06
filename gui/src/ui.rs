use crate::app::{App, CellDisplay};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
};

pub fn draw(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Stats bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Help line
        ])
        .split(f.size());

    render_title(f, main_chunks[0]);
    render_stats(f, main_chunks[1], app);

    if app.show_details {
        render_detailed_view(f, main_chunks[2], app);
    } else {
        render_plant_overview(f, main_chunks[2], app);
    }

    render_help(f, main_chunks[3]);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("╔═══ FLUX CHLOR-ALKALI CONTROL ROOM ═══╗")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center);
    f.render_widget(title, area);
}

fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let status_indicator = if app.cells.is_empty() {
        "⚠ WAITING FOR DATA"
    } else if app.messages_per_second > 0 {
        "● ONLINE"
    } else {
        "◐ IDLE"
    };

    let stats_text = format!(
        "│ {} │ Cells: {} │ Rate: {}/s │ Total: {} │ Errors: {} │",
        status_indicator,
        app.cells.len(),
        app.messages_per_second,
        app.total_messages,
        app.parse_errors
    );

    let color = if app.cells.is_empty() {
        Color::Yellow
    } else if app.messages_per_second > 0 {
        Color::Green
    } else {
        Color::Gray
    };

    let stats = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(color));
    f.render_widget(stats, area);
}

// ... rest of the UI functions remain the same ...
fn render_plant_overview(f: &mut Frame, area: Rect, app: &App) {
    let units_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    for unit in 1..=3 {
        render_unit_diagram(f, units_layout[unit - 1], app, unit as u8);
    }
}

fn render_unit_diagram(f: &mut Frame, area: Rect, app: &App, unit_id: u8) {
    let selected = app.selected_unit == unit_id;
    let border_color = if selected { Color::Yellow } else { Color::Gray };

    let block = Block::default()
        .title(format!("═══ ELECTROLYZER UNIT {} ═══", unit_id))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let stacks_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(inner);

    for (i, stack) in ['A', 'B', 'C'].iter().enumerate() {
        render_stack(f, stacks_layout[i], app, unit_id, *stack);
    }
}

fn render_stack(f: &mut Frame, area: Rect, app: &App, unit_id: u8, stack_id: char) {
    let mut lines = vec![];

    let is_selected = app.selected_unit == unit_id && app.selected_stack == stack_id;
    let header_style = if is_selected {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Green)
    };

    lines.push(Line::from(vec![Span::styled(
        format!("STACK {}", stack_id),
        header_style,
    )]));

    lines.push(Line::from("┌────────┐"));

    for row in 0..5 {
        let mut cell_line = String::from("│");
        for col in 0..4 {
            let cell_num = row * 4 + col + 1;
            let cell_key = format!("U{}_S{}_C{:02}", unit_id, stack_id, cell_num);

            let cell_char = if let Some(cell) = app.cells.get(&cell_key) {
                get_cell_indicator(cell)
            } else {
                '○'
            };

            let is_selected_cell = is_selected && app.selected_cell == cell_num;
            if is_selected_cell {
                cell_line.push_str(&format!("[{}]", cell_char));
            } else {
                cell_line.push_str(&format!(" {} ", cell_char));
            }
        }
        cell_line.push('│');
        lines.push(Line::from(cell_line));
    }

    lines.push(Line::from("└────────┘"));

    if let Some(avg_voltage) = calculate_stack_average(app, unit_id, stack_id, "voltage") {
        lines.push(Line::from(format!("V: {:.2}V", avg_voltage)));
    }
    if let Some(avg_temp) = calculate_stack_average(app, unit_id, stack_id, "temperature") {
        lines.push(Line::from(format!("T: {:.1}°C", avg_temp)));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn get_cell_indicator(cell: &CellDisplay) -> char {
    if cell.voltage > 3.5 || cell.temperature > 95.0 {
        '⊗'
    } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
        '◐'
    } else if cell.voltage > 3.0 {
        '●'
    } else {
        '○'
    }
}

fn calculate_stack_average(app: &App, unit: u8, stack: char, metric: &str) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0;

    for cell_num in 1..=20 {
        let key = format!("U{}_S{}_C{:02}", unit, stack, cell_num);
        if let Some(cell) = app.cells.get(&key) {
            sum += match metric {
                "voltage" => cell.voltage,
                "temperature" => cell.temperature,
                "current" => cell.current,
                "pressure" => cell.pressure,
                _ => 0.0,
            };
            count += 1;
        }
    }

    if count > 0 {
        Some(sum / count as f64)
    } else {
        None
    }
}

fn render_detailed_view(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_cell_matrix(f, chunks[0], app);
    render_cell_details(f, chunks[1], app);
}

fn render_cell_matrix(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("Cell Matrix View")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let selected_key = app.get_cell_key();

    let mut lines = vec![];
    lines.push(Line::from("Unit 1    Unit 2    Unit 3"));

    for cell_num in 1..=20 {
        let mut spans = vec![];
        for unit in 1..=3 {
            for stack in ['A', 'B', 'C'] {
                let key = format!("U{}_S{}_C{:02}", unit, stack, cell_num);
                let is_selected = key == selected_key;

                if let Some(cell) = app.cells.get(&key) {
                    let color = voltage_to_color(cell.voltage);
                    let symbol = if is_selected { "▣" } else { "■" };
                    spans.push(Span::styled(symbol, Style::default().fg(color)));
                } else {
                    spans.push(Span::raw("□"));
                }
            }
            spans.push(Span::raw("  "));
        }
        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn render_cell_details(f: &mut Frame, area: Rect, app: &App) {
    let key = app.get_cell_key();

    let block = Block::default()
        .title(format!("Cell {}", key))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(cell) = app.cells.get(&key) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(inner);

        let voltage_pct = ((cell.voltage - 2.5) / 1.5 * 100.0).clamp(0.0, 100.0) as u16;
        let voltage_gauge = Gauge::default()
            .label(format!("Voltage: {:.3} V", cell.voltage))
            .percent(voltage_pct)
            .gauge_style(voltage_to_style(cell.voltage));
        f.render_widget(voltage_gauge, chunks[0]);

        let current_text = Paragraph::new(format!("Current: {:.1} A", cell.current))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(current_text, chunks[1]);

        let temp_pct = ((cell.temperature - 70.0) / 30.0 * 100.0).clamp(0.0, 100.0) as u16;
        let temp_gauge = Gauge::default()
            .label(format!("Temperature: {:.1} °C", cell.temperature))
            .percent(temp_pct)
            .gauge_style(temp_to_style(cell.temperature));
        f.render_widget(temp_gauge, chunks[2]);

        let pressure_text = Paragraph::new(format!("ΔP: {:.1} mbar", cell.pressure))
            .style(Style::default().fg(Color::Blue));
        f.render_widget(pressure_text, chunks[3]);

        let sparkline_data: Vec<u64> = app
            .message_history
            .iter()
            .rev()
            .take(50)
            .filter(|m| {
                m.unit == app.selected_unit
                    && m.stack == app.selected_stack
                    && m.cell == app.selected_cell
            })
            .filter_map(|m| m.voltage_V)
            .map(|v| ((v - 2.5) * 100.0) as u64)
            .collect();

        if !sparkline_data.is_empty() {
            let sparkline = Sparkline::default()
                .block(Block::default().title("Voltage History"))
                .data(&sparkline_data)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, chunks[4]);
        }
    }
}

fn voltage_to_color(voltage: f64) -> Color {
    if voltage > 3.5 {
        Color::Red
    } else if voltage > 3.3 {
        Color::LightRed
    } else if voltage > 3.1 {
        Color::Yellow
    } else if voltage > 2.9 {
        Color::Green
    } else {
        Color::Blue
    }
}

fn voltage_to_style(voltage: f64) -> Style {
    Style::default().fg(voltage_to_color(voltage))
}

fn temp_to_style(temp: f64) -> Style {
    let color = if temp > 95.0 {
        Color::Red
    } else if temp > 90.0 {
        Color::Yellow
    } else if temp > 80.0 {
        Color::Green
    } else {
        Color::Blue
    };
    Style::default().fg(color)
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = "[Q]uit | [Tab] Toggle View | [↑↓←→] Navigate | [1-3] Select Unit";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, area);
}
