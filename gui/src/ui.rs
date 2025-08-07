use crate::app::{App, CellData, ViewMode};
use ratatui::{prelude::*, symbols, widgets::*};

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

    render_header(f, main_layout[0], app);
    render_tabs(f, main_layout[1], app);

    match app.current_view {
        ViewMode::Overview => render_overview(f, main_layout[2], app),
        ViewMode::Performance => render_performance(f, main_layout[2], app),
        ViewMode::Economics => render_economics(f, main_layout[2], app),
        ViewMode::CellDetail => render_cell_detail(f, main_layout[2], app),
        _ => render_placeholder(f, main_layout[2], &format!("{:?}", app.current_view)),
    }

    render_footer(f, main_layout[3], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
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

fn render_tabs(f: &mut Frame, area: Rect, app: &App) {
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

fn render_overview(f: &mut Frame, area: Rect, app: &mut App) {
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
        .constraints([Constraint::Min(0), Constraint::Length(3)]) // Space for main grid + scrollbar
        .split(chunks[0]);

    render_main_grid(f, overview_chunks[0], app);
    render_horizontal_scrollbar(f, overview_chunks[1], app);
    render_kpi_cards(f, side_chunks[0], app);
    render_selection_details(f, side_chunks[1], app);
}

fn render_main_grid(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" Plant Overview ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner_area = area.inner(&Margin {
        vertical: 1,
        horizontal: 1,
    });

    let unit_width = (app.cfg.geometry.stacks.len() * 3) + 3; // 3 chars per stack + 1 for scrollbar + 2 for padding
    let units_that_fit = (inner_area.width as usize) / unit_width;
    let total_units = app.cfg.geometry.units.len();

    let selected_unit_idx = app
        .cfg
        .geometry
        .units
        .iter()
        .position(|&u| u == app.selected_unit)
        .unwrap_or(0);

    // Auto-scroll logic for horizontal view
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
        .geometry
        .units
        .iter()
        .skip(app.unit_scroll_offset)
        .take(units_that_fit)
        .copied()
        .collect::<Vec<_>>();
    let constraints = std::iter::repeat(Constraint::Length(unit_width as u16))
        .take(visible_units.len())
        .collect::<Vec<_>>();

    let unit_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner_area);

    for (i, unit_id) in visible_units.iter().enumerate() {
        // Pass app as mutable because each unit display now updates its own scroll state
        render_unit_display(f, unit_chunks[i], app, *unit_id);
    }
}

fn render_unit_display(f: &mut Frame, area: Rect, app: &mut App, unit_id: u8) {
    let is_active_unit = app.selected_unit == unit_id;
    let border_style = if is_active_unit { Style::new().fg(Color::Yellow) } else { Style::new().fg(Color::DarkGray) };

    let block = Block::default().title(format!(" Unit {} ", unit_id)).borders(Borders::ALL).border_style(border_style);
    f.render_widget(block, area);
    let inner = area.inner(&Margin { vertical: 1, horizontal: 1 });

    let content_area = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)]) // Area for stacks, 1 char for scrollbar
        .split(inner);
    
    let stacks_area = content_area[0];
    let scrollbar_area = content_area[1];

    let stack_chunks = Layout::default().direction(Direction::Horizontal)
        .constraints(std::iter::repeat(Constraint::Length(3)).take(app.cfg.geometry.stacks.len()).collect::<Vec<_>>())
        .split(stacks_area);

    let scroll_offset = app.cell_scroll_offsets.entry(unit_id).or_default();
    let visible_height = stacks_area.height.saturating_sub(1) as u8; // -1 for header

    // Auto-scroll the active unit to keep the selection in view
    if is_active_unit {
        if app.selected_cell < *scroll_offset + 1 { *scroll_offset = app.selected_cell.saturating_sub(1); }
        if app.selected_cell > *scroll_offset + visible_height { *scroll_offset = app.selected_cell.saturating_sub(visible_height); }
    }

    for (i, &stack_id) in app.cfg.geometry.stacks.iter().enumerate() {
        let mut lines = vec![Line::from(Span::styled(stack_id.to_string(), Style::new().fg(Color::Cyan).bold())).alignment(Alignment::Center)];
        
        for row in 0..visible_height {
            let cell_id = *scroll_offset + 1 + row;
            if cell_id > app.cfg.geometry.cells_per_stack { break; }

            let key = format!("U{unit_id}_S{stack_id}_C{cell_id:02}");
            let (glyph, color) = match app.cells.get(&key) {
                Some(cd) => get_cell_display(cd),
                None => ('·', Color::DarkGray),
            };
            
            let is_cell_selected = is_active_unit && app.selected_stack == stack_id && app.selected_cell == cell_id;
            let (text, style) = if is_cell_selected {
                (format!("[{}]", glyph), Style::new().fg(color).add_modifier(Modifier::BOLD))
            } else {
                (format!(" {} ", glyph), Style::new().fg(color))
            };
            lines.push(Line::from(Span::styled(text, style)).alignment(Alignment::Center));
        }
        f.render_widget(Paragraph::new(lines), stack_chunks[i]);
    }
    
    // --- FIX: Render the toned-down scrollbar in its dedicated area ---
    let mut scrollbar_state = ScrollbarState::new(app.cfg.geometry.cells_per_stack as usize).position(*scroll_offset as usize);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::new().fg(Color::DarkGray)), 
        scrollbar_area, 
        &mut scrollbar_state
    );
}

fn render_horizontal_scrollbar(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("Plant Minimap")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray));
    f.render_widget(block, area);
    let inner = area.inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });

    let unit_width = (app.cfg.geometry.stacks.len() * 3) + 2;
    let units_that_fit = (f.size().width as usize).saturating_sub(35) / unit_width;

    let spans: Vec<Span> = app
        .cfg
        .geometry
        .units
        .iter()
        .enumerate()
        .map(|(i, &unit_id)| {
            let is_in_view =
                i >= app.unit_scroll_offset && i < app.unit_scroll_offset + units_that_fit;
            let mut has_alert = false;
            for stack_id in &app.cfg.geometry.stacks {
                for cell_id in 1..=app.cfg.geometry.cells_per_stack {
                    if let Some(cell) = app
                        .cells
                        .get(&format!("U{unit_id}_S{stack_id}_C{cell_id:02}"))
                    {
                        if cell.voltage > 3.3 {
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

fn get_cell_display(cell: &CellData) -> (char, Color) {
    if cell.voltage > 3.5 || cell.temperature > 95.0 {
        ('█', Color::Red)
    } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
        ('▓', Color::Yellow)
    } else if cell.efficiency < 92.0 {
        ('▒', Color::Cyan)
    } else {
        ('█', Color::Green)
    }
}

fn render_kpi_cards(f: &mut Frame, area: Rect, app: &App) {
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

fn render_selection_details(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Details: {} ", app.get_selected_cell_key()))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    f.render_widget(block, area);
    let inner = area.inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });
    let text = if let Some(cell) = app.get_selected_cell_data() {
        let (glyph, color) = get_cell_display(cell);
        vec![
            Line::from(vec![
                Span::from("Status:       "),
                Span::styled(format!("{} HEALTHY", glyph), Style::new().fg(color).bold()),
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
            Line::from("━━━ PREDICTIVE (Simulated) ━━━"),
            Line::from(format!("Health Score: {:>6.1} %", 98.3)),
            Line::from(format!("RUL (days):   {:>6}", 1240)),
            Line::from(""),
            Line::from("Press [Enter] for deep dive...").fg(Color::Cyan),
        ]
    } else {
        vec![Line::from("... no data for selected cell ...").fg(Color::DarkGray)]
    };
    f.render_widget(Paragraph::new(text), inner);
}

fn render_cell_detail(f: &mut Frame, area: Rect, app: &App) {
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

fn render_economics(f: &mut Frame, area: Rect, app: &App) {
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
                (app.metrics.hourly_revenue - app.metrics.hourly_energy_cost) * 24.0
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

// This function now exists just to show gauges.
fn render_performance(f: &mut Frame, area: Rect, app: &App) {
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

fn render_placeholder(f: &mut Frame, area: Rect, title: &str) {
    let text = format!("'{}' View Not Implemented Yet", title);
    let placeholder = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title)),
    );
    f.render_widget(placeholder, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
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
