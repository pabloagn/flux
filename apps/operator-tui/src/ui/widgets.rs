// Reusable components for interface

use crate::app::{App, CellData};
use crate::config::FluxConfig;
use ratatui::{prelude::*, symbols, widgets::*};

pub fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let text = format!(
        "FLUX CHLOR-ALKALI v4.1 | Power: {:.1} MW | Efficiency: {:.1}% | Cl₂: {:.1} MT/d",
        app.metrics.total_power_mw, app.metrics.avg_efficiency, app.metrics.total_production_cl2
    );
    f.render_widget(
        Paragraph::new(text)
            .style(Style::new().bg(Color::Rgb(50, 50, 50)).fg(Color::White))
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_tabs(f: &mut Frame, area: Rect, app: &App) {
    let titles = vec![
        "Overview [F1]",
        "Performance [F2]",
        "Predictive [F3]",
        "Economics [F4]",
        "Maintenance [F5]",
        "Alarms [F6]",
    ];
    f.render_widget(
        Tabs::new(titles)
            .select(app.current_view as usize)
            .style(Style::new().fg(Color::Gray))
            .highlight_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .divider(" | "),
        area,
    );
}

pub fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = " [↑↓←→] Navigate | [PgUp/PgDn] Scroll | [Enter] Details | [Tab] View | [Space] Pause | [Q] Quit ";
    let status_text = format!(
        "{} | {} cells | {} msgs/s ",
        if app.cells.is_empty() {
            "⚠ DISCONNECTED"
        } else {
            "● CONNECTED"
        },
        app.cells.len(),
        app.messages_per_second
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(status_text.len() as u16),
        ])
        .split(area);
    f.render_widget(
        Paragraph::new(help_text).style(Style::new().bg(Color::Rgb(50, 50, 50)).fg(Color::White)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(status_text)
            .style(
                Style::new()
                    .bg(if app.cells.is_empty() {
                        Color::Red
                    } else {
                        Color::Green
                    })
                    .fg(Color::Black),
            )
            .alignment(Alignment::Right),
        chunks[1],
    );
}

pub fn render_main_grid(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" Plant Overview ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let unit_width = (app.cfg.plant.geometry.stacks.len() * 3) + 3;
    let units_that_fit = (inner_area.width as usize).saturating_sub(1) / unit_width;
    let total_units = app.cfg.plant.geometry.units.len();
    let selected_unit_idx = app
        .cfg
        .plant
        .geometry
        .units
        .iter()
        .position(|&u| u == app.selected_unit)
        .unwrap_or(0);
    if selected_unit_idx < app.unit_scroll_offset {
        app.unit_scroll_offset = selected_unit_idx;
    }
    if selected_unit_idx >= app.unit_scroll_offset + units_that_fit {
        app.unit_scroll_offset = selected_unit_idx - units_that_fit + 1;
    }
    app.unit_scroll_offset = app
        .unit_scroll_offset
        .min(total_units.saturating_sub(units_that_fit));
    let visible_units = app
        .cfg
        .plant
        .geometry
        .units
        .iter()
        .skip(app.unit_scroll_offset)
        .take(units_that_fit)
        .copied()
        .collect::<Vec<_>>();
    let mut constraints = std::iter::repeat(Constraint::Length(unit_width as u16))
        .take(visible_units.len())
        .collect::<Vec<_>>();
    constraints.push(Constraint::Min(0));
    let unit_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner_area);
    for (i, unit_id) in visible_units.iter().enumerate() {
        render_unit_display(f, unit_chunks[i], app, *unit_id);
    }
}

fn render_unit_display(f: &mut Frame, area: Rect, app: &mut App, unit_id: u8) {
    let is_active_unit = app.selected_unit == unit_id;
    let border_style = if is_active_unit {
        Style::new().fg(Color::Yellow)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(format!(" Unit {} ", unit_id))
        .borders(Borders::ALL)
        .border_style(border_style);
    f.render_widget(block, area);
    let inner = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);
    let stacks_area = content_area[0];
    let scrollbar_area = content_area[1];
    let stack_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            std::iter::repeat(Constraint::Length(3))
                .take(app.cfg.plant.geometry.stacks.len())
                .collect::<Vec<_>>(),
        )
        .split(stacks_area);
    let scroll_offset = app.cell_scroll_offsets.entry(unit_id).or_default();
    let visible_height = stacks_area.height.saturating_sub(1) as u8;
    if is_active_unit {
        if app.selected_cell < *scroll_offset + 1 {
            *scroll_offset = app.selected_cell.saturating_sub(1);
        }
        if app.selected_cell > *scroll_offset + visible_height {
            *scroll_offset = app.selected_cell.saturating_sub(visible_height);
        }
    }
    for (i, &stack_id) in app.cfg.plant.geometry.stacks.iter().enumerate() {
        let mut lines = vec![Line::from(Span::styled(
            stack_id.to_string(),
            Style::new().fg(Color::Cyan).bold(),
        ))
        .alignment(Alignment::Center)];
        for row in 0..visible_height {
            let cell_id = *scroll_offset + 1 + row;
            if cell_id > app.cfg.plant.geometry.cells_per_stack {
                break;
            }
            let key = format!("U{unit_id}_S{stack_id}_C{cell_id:02}");
            let (glyph, color) = match app.cells.get(&key) {
                Some(cd) => get_cell_display(cd, &app.cfg),
                None => ('·', Color::DarkGray),
            };
            let is_cell_selected =
                is_active_unit && app.selected_stack == stack_id && app.selected_cell == cell_id;
            let (text, style) = if is_cell_selected {
                (
                    format!("[{}]", glyph),
                    Style::new().fg(color).add_modifier(Modifier::BOLD),
                )
            } else {
                (format!(" {} ", glyph), Style::new().fg(color))
            };
            lines.push(Line::from(Span::styled(text, style)).alignment(Alignment::Center));
        }
        f.render_widget(Paragraph::new(lines), stack_chunks[i]);
    }
    let total_cells = app.cfg.plant.geometry.cells_per_stack as usize;
    let visible_cells = visible_height as usize;
    let max_scroll = total_cells.saturating_sub(visible_cells);
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(*scroll_offset as usize);
    let scrollbar =
        Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::new().fg(Color::DarkGray));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

pub fn render_horizontal_scrollbar(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("Plant Minimap")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner = area.inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });
    let unit_width = (app.cfg.plant.geometry.stacks.len() * 3) + 3;
    let units_that_fit = (f.size().width as usize)
        .saturating_sub(35)
        .saturating_sub(2)
        / unit_width;
    let spans: Vec<Span> = app
        .cfg
        .plant
        .geometry
        .units
        .iter()
        .enumerate()
        .map(|(i, &unit_id)| {
            let is_in_view =
                i >= app.unit_scroll_offset && i < app.unit_scroll_offset + units_that_fit;
            let mut has_alert = false;
            for stack_id in &app.cfg.plant.geometry.stacks {
                for cell_id in 1..=app.cfg.plant.geometry.cells_per_stack {
                    if let Some(cell) = app
                        .cells
                        .get(&format!("U{unit_id}_S{stack_id}_C{cell_id:02}"))
                    {
                        if cell.voltage > app.cfg.limits.warning.voltage_v {
                            has_alert = true;
                            break;
                        }
                    }
                    if has_alert {
                        break;
                    }
                }
            }
            let color = if has_alert { Color::Red } else { Color::Green };
            let symbol = if is_in_view {
                symbols::block::FULL
            } else {
                symbols::block::ONE_QUARTER
            };
            Span::styled(symbol, Style::new().fg(color))
        })
        .collect();
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

pub fn get_cell_display(cell: &CellData, config: &FluxConfig) -> (char, Color) {
    if cell.sensor_quality < 0.9 {
        ('?', Color::Magenta)
    } else if cell.voltage > config.limits.critical.voltage_v
        || cell.temperature > config.limits.critical.temperature_c
    {
        ('█', Color::Red)
    } else if cell.voltage > config.limits.warning.voltage_v
        || cell.temperature > config.limits.warning.temperature_c
    {
        ('▓', Color::Yellow)
    } else if cell.efficiency < 92.0 {
        ('▒', Color::Cyan)
    } else {
        ('█', Color::Green)
    }
}

pub fn render_kpi_cards(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" KPIs ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = area.inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });
    f.render_widget(block, area);
    let text = vec![
        Line::from(vec![
            Span::from("Production Cl₂: "),
            Span::from(format!("{:>7.1} MT/d", app.metrics.total_production_cl2)).bold(),
        ]),
        Line::from(vec![
            Span::from("Production NaOH:"),
            Span::from(format!("{:>7.1} MT/d", app.metrics.total_production_naoh)).bold(),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::from("Avg Efficiency: "),
            Span::from(format!("{:>6.1}%", app.metrics.avg_efficiency)).bold(),
        ]),
        Line::from(vec![
            Span::from("Specific Energy:"),
            Span::from(format!("{:>5.0} kWh/MT", app.metrics.specific_energy_avg)).bold(),
        ]),
    ];
    f.render_widget(Paragraph::new(text), inner);
}

pub fn render_selection_details(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Details: {} ", app.get_selected_cell_key()))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    let inner = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });
    f.render_widget(block, area);
    let text = if let Some(cell) = app.get_selected_cell_data() {
        let (glyph, color) = get_cell_display(cell, &app.cfg);
        let status_text = if cell.sensor_quality < 0.9 {
            "SENSOR FAULT"
        } else if color == Color::Red {
            "CRITICAL"
        } else if color == Color::Yellow {
            "WARNING"
        } else {
            "HEALTHY"
        };
        vec![
            Line::from(vec![
                Span::from("Status:       "),
                Span::styled(
                    format!("{} {}", glyph, status_text),
                    Style::new().fg(color).bold(),
                ),
            ]),
            Line::from(format!(
                "Last Update:  {}",
                cell.last_update.format("%H:%M:%S")
            )),
            Line::from(""),
            Line::from("━━━ REAL-TIME ━━━"),
            Line::from(format!("Voltage:      {:>6.3} V", cell.voltage)),
            Line::from(format!("Current:      {:>6.0} A", cell.current)),
            Line::from(format!("Temperature:  {:>6.1} °C", cell.temperature)),
            Line::from(format!("Efficiency:   {:>6.1} %", cell.efficiency)),
            Line::from(format!("Power:        {:>6.2} kW", cell.power_kw)),
            Line::from(""),
            Line::from("━━━ DIAGNOSTICS ━━━"),
            Line::from(format!(
                "Sensor Qual:  {:>6.1} %",
                cell.sensor_quality * 100.0
            )),
            Line::from(format!("Spec. Energy: {:>6.0} kWh/t", cell.specific_energy)),
            Line::from(format!("Membrane Ω:   {:>6.3}", cell.membrane_resistance)),
            Line::from(""),
            Line::from("Press [Enter] for deep dive...").fg(Color::Cyan),
        ]
    } else {
        vec![Line::from("... no data for selected cell ...").fg(Color::DarkGray)]
    };
    f.render_widget(Paragraph::new(text), inner);
}
