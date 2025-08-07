use crate::app::{AlarmSeverity, App, CellData, ViewMode};
use chrono::Local;
use ratatui::layout::Constraint;
use ratatui::{prelude::*, widgets::*};

// --- Helpers ---

/// Build `<n>` equal-width column constraints.
fn columns(n: usize) -> Vec<Constraint> {
    vec![Constraint::Percentage((100 / n.max(1)) as u16); n]
}

/// Iterator over unit IDs (u8) from the config.
fn units(app: &App) -> impl Iterator<Item = u8> + '_ {
    app.cfg.geometry.units.iter().copied()
}

/// Iterator over stack IDs (char) from the config.
fn stacks(app: &App) -> impl Iterator<Item = char> + '_ {
    app.cfg.geometry.stacks.iter().copied()
}

pub fn draw(f: &mut Frame, app: &App) {
    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(1), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(2), // Status bar
        ])
        .split(f.size());

    render_header(f, chunks[0], app);
    render_tabs(f, chunks[1], app);

    // Render content based on current view
    match app.current_view {
        ViewMode::Overview => render_overview(f, chunks[2], app),
        ViewMode::Performance => render_performance(f, chunks[2], app),
        ViewMode::Predictive => render_predictive(f, chunks[2], app),
        ViewMode::Economics => render_economics(f, chunks[2], app),
        ViewMode::Maintenance => render_maintenance(f, chunks[2], app),
        ViewMode::Alarms => render_alarms(f, chunks[2], app),
    }

    render_status_bar(f, chunks[3], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),
            Constraint::Min(0),
            Constraint::Length(30),
        ])
        .split(area);

    // Plant title
    let title = Paragraph::new(
        "█▀▀ █   █ █ ▀▄▀   CHLOR-ALKALI PLANT
█▀▀ █▄▄ █▄█ █ █   CONTROL SYSTEM v4.1",
    )
    .style(Style::default().fg(Color::Cyan).bold());
    f.render_widget(title, header_chunks[0]);

    // Key metrics bar
    let metrics_text = if app.cells.is_empty() {
        "⚠ WAITING FOR DATA...".to_string()
    } else {
        format!(
            "Power: {:.1} MW │ Efficiency: {:.1}% │ Cl₂: {:.1} MT/d",
            app.metrics.total_power_mw,
            app.metrics.avg_efficiency,
            app.metrics.total_production_cl2
        )
    };

    let metrics = Paragraph::new(metrics_text)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    f.render_widget(metrics, header_chunks[1]);

    // Time and status
    let time_text = format!(
        "{}\n{} cells │ {} msgs/s",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        app.cells.len(),
        app.messages_per_second
    );
    let time = Paragraph::new(time_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Right);
    f.render_widget(time, header_chunks[2]);
}

fn render_tabs(f: &mut Frame, area: Rect, app: &App) {
    let tabs = [
        "Overview [F1]",
        "Performance [F2]",
        "Predictive [F3]",
        "Economics [F4]",
        "Maintenance [F5]",
        "Alarms [F6]",
    ];
    let selected = match app.current_view {
        ViewMode::Overview => 0,
        ViewMode::Performance => 1,
        ViewMode::Predictive => 2,
        ViewMode::Economics => 3,
        ViewMode::Maintenance => 4,
        ViewMode::Alarms => 5,
    };

    let tabs_widget = Tabs::new(tabs)
        .select(selected)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Yellow).bold())
        .divider("│");
    f.render_widget(tabs_widget, area);
}

fn render_overview(f: &mut Frame, area: Rect, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left: Electrolyzer units
    render_units_grid(f, main_chunks[0], app);

    // Right: Metrics and alarms
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // KPIs
            Constraint::Min(0),     // Recent alarms
        ])
        .split(main_chunks[1]);

    render_kpi_cards(f, right_chunks[0], app);
    render_alarm_feed(f, right_chunks[1], app);
}

fn render_units_grid(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Electrolyzer Units ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    /* build N-column layout */
    let unit_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(columns(app.cfg.geometry.units.len()))
        .split(inner);

    /* zip the Rects with unit-ids – note the * to pass an owned Rect */
    for (rect, unit_id) in unit_areas.iter().zip(units(app)) {
        render_unit_display(f, *rect, app, unit_id);
    }
}

fn render_unit_display(f: &mut Frame, area: Rect, app: &App, unit_id: u8) {
    /* selection & colours */
    let sel = app.selected_unit == unit_id;
    let bd_colour = if sel { Color::Yellow } else { Color::Gray };

    /* aggregate metrics for this unit */
    let mut pw = 0.0;
    let mut eff = 0.0;
    let mut cnt = 0u32;
    let mut warn = 0u32;
    let mut crit = 0u32;

    for stk in stacks(app) {
        for c in 1..=app.cfg.geometry.cells_per_stack {
            let key = format!("U{unit_id}_S{stk}_C{c:02}");
            if let Some(cell) = app.cells.get(&key) {
                pw += cell.power_kw;
                eff += cell.efficiency;
                cnt += 1;
                if cell.voltage > 3.5 || cell.temperature > 95.0 {
                    crit += 1;
                } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
                    warn += 1;
                }
            }
        }
    }
    if cnt > 0 {
        eff /= cnt as f64;
    }

    let (sym, col) = match (crit, warn, cnt) {
        (c, _, _) if c > 0 => ("⚠", Color::Red),
        (_, w, _) if w > 0 => ("◐", Color::Yellow),
        (_, _, c) if c > 0 => ("●", Color::Green),
        _ => ("○", Color::Gray),
    };

    /* frame for the whole unit */
    let block = Block::default()
        .title(format!(" {sym} Unit {unit_id} – {:.1} MW ", pw / 1_000.0))
        .title_style(Style::default().fg(col))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(bd_colour));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // --- CORRECTED LAYOUT LOGIC ---

    // Define a layout to separate the header row from the cell grid area.
    let unit_content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Row for stack headers (A, B, C...)
            Constraint::Min(0),    // Area for the cell grids
        ])
        .split(inner);

    let header_area = unit_content_chunks[0];
    let grid_area = unit_content_chunks[1];

    let num_stacks = app.cfg.geometry.stacks.len();

    // Create columns for both the header and the grid area.
    // The constraints must be the same to ensure alignment.
    let col_constraints = columns(num_stacks);
    let header_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints.clone())
        .split(header_area);

    let grid_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints)
        .split(grid_area);

    // Iterate through the stacks, rendering the header and the cell grid
    // for each one in its designated column.
    for (i, stack_id) in stacks(app).enumerate() {
        // Render the stack header (e.g., 'A') centered in its column.
        let header_text = Paragraph::new(stack_id.to_string())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(header_text, header_columns[i]);

        // Render the stack's cell matrix in the column below the header.
        render_stack_matrix(f, grid_columns[i], app, unit_id, stack_id, i == 0);
    }
}

fn render_stack_matrix(
    f: &mut Frame,
    area: Rect,
    app: &App,
    unit: u8,
    stack: char,
    first_column: bool, // true only for the very first stack in the row
) {
    /* selected? */
    let selected_stack = app.selected_unit == unit && app.selected_stack == stack;

    /* build a single column of cells for this stack */
    let num_cells = app.cfg.geometry.cells_per_stack;
    let mut lines = Vec::with_capacity(num_cells as usize);

    for cell_num in 1..=num_cells {
        let key = format!("U{unit}_S{stack}_C{cell_num:02}");
        let (glyph, colour) = match app.cells.get(&key) {
            Some(cd) => get_cell_display(cd),
            None => ('·', Color::DarkGray),
        };

        let txt = if selected_stack && app.selected_cell == cell_num {
            format!("[{glyph}]")
        } else {
            " · ".replace('·', &glyph.to_string()) // keep 3-char width
        };

        // Each line represents one cell in the stack's column.
        // It is a single span, centered within the line.
        let line =
            Line::from(Span::styled(txt, Style::default().fg(colour))).alignment(Alignment::Center);
        lines.push(line);
    }

    /* optional left margin so the very first column aligns nicely */
    let paragraph = Paragraph::new(lines).block(if first_column {
        Block::default().padding(Padding::left(1))
    } else {
        Block::default()
    });

    f.render_widget(paragraph, area);
}

fn get_cell_display(cell: &CellData) -> (char, Color) {
    if cell.voltage > 3.5 || cell.temperature > 95.0 {
        ('█', Color::Red) // Critical
    } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
        ('▓', Color::Yellow) // Warning
    } else if cell.efficiency < 90.0 {
        ('▒', Color::Blue) // Low efficiency
    } else {
        ('█', Color::Green) // Normal
    }
}

fn render_kpi_cards(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Key Performance Indicators ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let kpi_text = format!(
        "━━━ PRODUCTION ━━━
Cl₂:  {:.1} MT/day
NaOH: {:.1} MT/day
H₂:   {:.1} MT/day

━━━ EFFICIENCY ━━━
Current: {:.1}%
Energy:  {:.0} kWh/MT

━━━ ECONOMICS ━━━
Margin: {:.1}%",
        app.metrics.total_production_cl2,
        app.metrics.total_production_naoh,
        app.metrics.total_production_h2,
        app.metrics.avg_efficiency,
        app.metrics.specific_energy_avg,
        app.metrics.gross_margin
    );

    let paragraph = Paragraph::new(kpi_text).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

fn render_alarm_feed(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Recent Alarms ({}) ", app.alarms.len()))
        .borders(Borders::ALL)
        .border_style(
            Style::default().fg(
                if app
                    .alarms
                    .iter()
                    .any(|a| matches!(a.severity, AlarmSeverity::Critical))
                {
                    Color::Red
                } else if !app.alarms.is_empty() {
                    Color::Yellow
                } else {
                    Color::Gray
                },
            ),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let alarms: Vec<ListItem> = app
        .alarms
        .iter()
        .rev()
        .take(10)
        .map(|alarm| {
            let (symbol, color) = match alarm.severity {
                AlarmSeverity::Critical => ("▲", Color::Red),
                AlarmSeverity::Warning => ("▼", Color::Yellow),
            };

            let content = format!(
                "{} {} {} {}: {:.1} > {:.1}",
                alarm.timestamp.format("%H:%M:%S"),
                symbol,
                alarm.location,
                alarm.parameter,
                alarm.value,
                alarm.threshold
            );

            ListItem::new(content).style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(alarms);
    f.render_widget(list, inner);
}

fn render_performance(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(15), // Gauges
            Constraint::Min(0),     // Charts
        ])
        .split(area);

    render_performance_gauges(f, chunks[0], app);
    render_performance_charts(f, chunks[1], app);
}

fn render_performance_gauges(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Real-Time Performance Metrics ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    // Efficiency gauge
    let efficiency_pct = app.metrics.avg_efficiency.clamp(0.0, 100.0) as u16;
    let efficiency_gauge = Gauge::default()
        .block(
            Block::default()
                .title("Current Efficiency")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(match efficiency_pct {
            0..=85 => Color::Red,
            86..=90 => Color::Yellow,
            _ => Color::Green,
        }))
        .percent(efficiency_pct)
        .label(format!("{:.1}%", app.metrics.avg_efficiency));
    f.render_widget(efficiency_gauge, gauge_chunks[0]);

    // Power gauge (0-400 MW scale)
    let power_pct = ((app.metrics.total_power_mw / 400.0) * 100.0).clamp(0.0, 100.0) as u16;
    let power_gauge = Gauge::default()
        .block(
            Block::default()
                .title("Power Consumption")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(power_pct)
        .label(format!("{:.1} MW", app.metrics.total_power_mw));
    f.render_widget(power_gauge, gauge_chunks[1]);

    // Production rate gauge (0-4000 MT/day scale)
    let production_pct =
        ((app.metrics.total_production_cl2 / 4000.0) * 100.0).clamp(0.0, 100.0) as u16;
    let production_gauge = Gauge::default()
        .block(
            Block::default()
                .title("Cl₂ Production")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Blue))
        .percent(production_pct)
        .label(format!("{:.0} MT/d", app.metrics.total_production_cl2));
    f.render_widget(production_gauge, gauge_chunks[2]);

    // Cell health (based on warnings/critical)
    let total_cells = (app.cfg.geometry.units.len()
        * app.cfg.geometry.stacks.len()
        * app.cfg.geometry.cells_per_stack as usize) as f64;
    let health_pct = if total_cells > 0.0 {
        ((app.metrics.cells_online as f64 / total_cells) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        100
    };
    let health_gauge = Gauge::default()
        .block(Block::default().title("Cell Health").borders(Borders::ALL))
        .gauge_style(Style::default().fg(match health_pct {
            0..=80 => Color::Red,
            81..=95 => Color::Yellow,
            _ => Color::Green,
        }))
        .percent(health_pct)
        .label(format!(
            "{}/{} OK",
            app.metrics.cells_online, total_cells as usize
        ));
    f.render_widget(health_gauge, gauge_chunks[3]);
}

fn render_performance_charts(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Efficiency trend chart
    render_efficiency_trend(f, chunks[0], app);

    // Voltage distribution
    render_voltage_distribution(f, chunks[1], app);
}

fn render_efficiency_trend(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Efficiency Trend (Last 5 min) ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block.clone(), area);

    // Collect efficiency data from cells
    let mut efficiency_data: Vec<u64> = Vec::new();
    let selected_key = app.get_selected_cell_key();

    if let Some(cell) = app.cells.get(&selected_key) {
        efficiency_data = cell
            .efficiency_history
            .iter()
            .map(|e| (e - 80.0).max(0.0) as u64) // Scale from 80-100% to 0-20
            .collect();
    }

    if !efficiency_data.is_empty() {
        let sparkline = Sparkline::default()
            .data(&efficiency_data)
            .style(Style::default().fg(Color::Green))
            .max(20);
        f.render_widget(sparkline, inner);
    } else {
        let no_data = Paragraph::new("No efficiency data available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(no_data, inner);
    }
}

fn render_voltage_distribution(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Voltage Distribution ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Create histogram of voltages
    let mut voltage_buckets = vec![0u64; 10]; // 2.5V to 3.5V in 0.1V buckets

    for cell in app.cells.values() {
        let bucket_idx = ((cell.voltage - 2.5) * 10.0).clamp(0.0, 9.0) as usize;
        voltage_buckets[bucket_idx] += 1;
    }

    let max_count = *voltage_buckets.iter().max().unwrap_or(&1).max(&1);

    let bars: Vec<(&str, u64)> = vec![
        ("2.5", voltage_buckets[0]),
        ("2.6", voltage_buckets[1]),
        ("2.7", voltage_buckets[2]),
        ("2.8", voltage_buckets[3]),
        ("2.9", voltage_buckets[4]),
        ("3.0", voltage_buckets[5]),
        ("3.1", voltage_buckets[6]),
        ("3.2", voltage_buckets[7]),
        ("3.3", voltage_buckets[8]),
        ("3.4+", voltage_buckets[9]),
    ];

    let barchart = BarChart::default()
        .data(&bars)
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White))
        .label_style(Style::default().fg(Color::Gray))
        .max(max_count);

    f.render_widget(barchart, inner);
}

fn render_predictive(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_predictions(f, chunks[0], app);
    render_recommendations(f, chunks[1], app);
}

fn render_predictions(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Predictive Analytics (ML Simulated) ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Simulated predictions based on current trends
    let efficiency_trend = if app.metrics.avg_efficiency > 94.0 {
        ("↑", Color::Green, "+0.02")
    } else if app.metrics.avg_efficiency > 92.0 {
        ("→", Color::Yellow, "-0.01")
    } else {
        ("↓", Color::Red, "-0.05")
    };

    let failure_risk = if app.metrics.cells_critical > 5 {
        ("HIGH", Color::Red, 85.0)
    } else if app.metrics.cells_warning > 10 {
        ("MEDIUM", Color::Yellow, 45.0)
    } else {
        ("LOW", Color::Green, 12.0)
    };

    let text = format!(
        "━━━ 24H FORECAST ━━━
Efficiency: {:.1}% {} {}%/day
Production: {:.0} MT Cl₂
Energy Cost: ${:.0}K

━━━ MEMBRANE HEALTH ━━━
Avg Lifetime Remaining: {} days
Degradation Rate: {:.3} mV/day
Next Replacement: Unit 2 Stack B

━━━ FAILURE RISK ━━━
Overall Risk: {} ({:.0}%)
Critical Cells: {}
Time to Action: {} hours

━━━ OPTIMIZATION ━━━
Optimal Current: {:.0} A
Potential Savings: ${:.0}/hour
Efficiency Gain: +{:.1}%",
        app.metrics.efficiency_24h_forecast,
        efficiency_trend.0,
        efficiency_trend.2,
        app.metrics.total_production_cl2 * 1.02, // Slight increase
        app.metrics.hourly_energy_cost * 24.0 / 1000.0,
        1825 - (app.tick_count / 1200) as i32, // Simulated countdown
        0.15,                                  // Hardcoded degradation
        failure_risk.0,
        failure_risk.2,
        app.metrics.cells_critical,
        if app.metrics.cells_critical > 0 {
            4
        } else {
            72
        },
        app.metrics.recommended_current,
        app.metrics.hourly_revenue * 0.03, // 3% potential improvement
        1.5
    );

    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

fn render_recommendations(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" AI Recommendations ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Generate recommendations based on current state
    let mut recommendations = vec![];

    if app.metrics.cells_critical > 0 {
        recommendations.push((
            "🔴 CRITICAL",
            "Inspect high-voltage cells immediately",
            Color::Red,
        ));
    }

    if app.metrics.avg_efficiency < 92.0 {
        recommendations.push((
            "⚠ EFFICIENCY",
            "Consider reducing current density",
            Color::Yellow,
        ));
    }

    if app.metrics.cells_warning > 10 {
        recommendations.push((
            "⚠ MAINTENANCE",
            "Schedule preventive maintenance",
            Color::Yellow,
        ));
    }

    if app.metrics.gross_margin < 30.0 {
        recommendations.push((
            "📊 ECONOMICS",
            "Optimize for off-peak operation",
            Color::Blue,
        ));
    }

    recommendations.push(("✓ OPTIMAL", "Increase production by 2%", Color::Green));
    recommendations.push((
        "💡 SUGGESTION",
        "Enable predictive load balancing",
        Color::Cyan,
    ));

    let items: Vec<ListItem> = recommendations
        .iter()
        .map(|(label, text, color)| {
            ListItem::new(format!("{}\n  {}\n", label, text)).style(Style::default().fg(*color))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

fn render_economics(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Summary cards
            Constraint::Min(0),     // Detailed breakdown
        ])
        .split(area);

    render_economic_summary(f, chunks[0], app);
    render_economic_details(f, chunks[1], app);
}

fn render_economic_summary(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Economic Performance ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let revenue_color = if app.metrics.gross_margin > 35.0 {
        Color::Green
    } else if app.metrics.gross_margin > 25.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let summary = format!(
        "┌─────────────┬──────────────┬──────────────┬──────────────┐
│   REVENUE   │     COST     │    MARGIN    │    DAILY     │
├─────────────┼──────────────┼──────────────┼──────────────┤
│  ${:>8.0}/h │  ${:>8.0}/h │    {:>5.1}%    │  ${:>9.0} │
└─────────────┴──────────────┴──────────────┴──────────────┘",
        app.metrics.hourly_revenue,
        app.metrics.hourly_energy_cost,
        app.metrics.gross_margin,
        (app.metrics.hourly_revenue - app.metrics.hourly_energy_cost) * 24.0
    );

    let paragraph = Paragraph::new(summary).style(Style::default().fg(revenue_color));
    f.render_widget(paragraph, inner);
}

fn render_economic_details(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Revenue breakdown
    let revenue_block = Block::default()
        .title(" Revenue Streams ")
        .borders(Borders::ALL);
    let revenue_inner = revenue_block.inner(chunks[0]);
    f.render_widget(revenue_block, chunks[0]);

    let revenue_text = format!(
        "Product      Price    Volume    Revenue
────────── ──────── ──────── ──────────
Chlorine    ${:>6.0}  {:>6.1}   ${:>8.0}
Caustic     ${:>6.0}  {:>6.1}   ${:>8.0}
Hydrogen    ${:>6.0}  {:>6.1}   ${:>8.0}
────────── ──────── ──────── ──────────
                    TOTAL:   ${:>8.0}",
        app.metrics.chlorine_price,
        app.metrics.total_production_cl2 / 24.0,
        app.metrics.total_production_cl2 / 24.0 * app.metrics.chlorine_price,
        app.metrics.caustic_price,
        app.metrics.total_production_naoh / 24.0,
        app.metrics.total_production_naoh / 24.0 * app.metrics.caustic_price,
        app.metrics.hydrogen_price,
        app.metrics.total_production_h2 / 24.0,
        app.metrics.total_production_h2 / 24.0 * app.metrics.hydrogen_price,
        app.metrics.hourly_revenue
    );

    let revenue_para = Paragraph::new(revenue_text).style(Style::default().fg(Color::Green));
    f.render_widget(revenue_para, revenue_inner);

    // Cost breakdown
    let cost_block = Block::default()
        .title(" Operating Costs ")
        .borders(Borders::ALL);
    let cost_inner = cost_block.inner(chunks[1]);
    f.render_widget(cost_block, chunks[1]);

    let total_cost = app.metrics.hourly_energy_cost * 2.38;
    let electricity_pct = if total_cost > 0.0 {
        (app.metrics.hourly_energy_cost / total_cost) * 100.0
    } else {
        0.0
    };

    let cost_text = format!(
        "Category        $/hour      % of Total
────────────── ──────── ──────────────
Electricity     ${:>7.0}        {:>5.1}%
Brine/Salt      ${:>7.0}        {:>5.1}%
Water           ${:>7.0}        {:>5.1}%
Maintenance     ${:>7.0}        {:>5.1}%
Labor           ${:>7.0}        {:>5.1}%
────────────── ──────── ──────────────
TOTAL:          ${:>7.0}       100.0%",
        app.metrics.hourly_energy_cost,
        electricity_pct,
        app.metrics.hourly_energy_cost * 0.3, // Hardcoded ratios
        12.6,
        app.metrics.hourly_energy_cost * 0.08,
        3.4,
        app.metrics.hourly_energy_cost * 0.25,
        10.5,
        app.metrics.hourly_energy_cost * 0.75,
        31.5,
        total_cost
    );

    let cost_para = Paragraph::new(cost_text).style(Style::default().fg(Color::Yellow));
    f.render_widget(cost_para, cost_inner);
}

fn render_maintenance(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_cell_health_matrix(f, chunks[0], app);
    render_maintenance_schedule(f, chunks[1], app);
}

fn render_cell_health_matrix(f: &mut Frame, area: Rect, app: &App) {
    /* ── outer frame ─────────────────────────────────────────────────────── */
    let block = Block::default()
        .title(" Cell-Health Matrix (voltage deviation) ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    /* ── dynamic header rows ─────────────────────────────────────────────── */
    // stacks header  (e.g.     A B C D E F   A B C D E F   …)
    let mut hdr_stacks = String::from("     "); // left margin for row numbers
    for _ in units(app) {
        for s in stacks(app) {
            hdr_stacks.push_str(&format!("{s} "));
        }
        hdr_stacks.push_str("  "); // unit separator
    }
    lines.push(Line::from(hdr_stacks));

    // units header   (e.g.    Unit 1            Unit 2            …)
    let hdr_units = units(app)
        .map(|u| {
            let w = app.cfg.geometry.stacks.len() * 2; // glyph+space per stack
            format!(" {:^width$} ", format!("Unit {u}"), width = w)
        })
        .collect::<Vec<_>>()
        .join("  ");
    lines.push(Line::from(Span::raw(hdr_units)));

    /* ── one row per cell index ──────────────────────────────────────────── */
    let n_cells = app.cfg.geometry.cells_per_stack;
    for cell_num in 1..=n_cells {
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(format!("{:2} ", cell_num))); // row label

        for unit in units(app) {
            for stack in stacks(app) {
                let key = format!("U{unit}_S{stack}_C{cell_num:02}");
                let (glyph, colour) = match app.cells.get(&key) {
                    Some(cell) => {
                        let dev = (cell.voltage - 3.0).abs();
                        if dev > 0.5 {
                            ('●', Color::Red)
                        } else if dev > 0.3 {
                            ('●', Color::Yellow)
                        } else if dev > 0.1 {
                            ('●', Color::Blue)
                        } else {
                            ('●', Color::Green)
                        }
                    }
                    None => ('·', Color::DarkGray),
                };
                spans.push(Span::styled(glyph.to_string(), Style::default().fg(colour)));
                spans.push(Span::raw(" ")); // spacing
            }
            spans.push(Span::raw(" ")); // unit separator
        }
        lines.push(Line::from(spans));
    }

    /* ── legend row ──────────────────────────────────────────────────────── */
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("● ≤0.1 V  ", Style::default().fg(Color::Green)),
        Span::styled("● 0.1-0.3 V  ", Style::default().fg(Color::Blue)),
        Span::styled("● 0.3-0.5 V  ", Style::default().fg(Color::Yellow)),
        Span::styled("● >0.5 V", Style::default().fg(Color::Red)),
    ]));

    /* ── render ──────────────────────────────────────────────────────────── */
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_maintenance_schedule(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Maintenance Schedule ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let schedule_text = format!(
        "━━━ UPCOMING ━━━
□ Tomorrow 08:00
 Unit 2 Stack B inspection
 
□ In 3 days
 Brine quality analysis
 
□ In 7 days
 Current distribution check
 
□ In 14 days
 Membrane voltage testing

━━━ OVERDUE ━━━
▲ Unit 1 Stack A Cell 15
 High voltage for 48h
 
▲ Unit 3 pressure sensors
 Calibration needed

━━━ PREDICTIVE ━━━
◊ ~30 days: U2-B membrane
◊ ~45 days: Rectifier service
◊ ~60 days: Full shutdown"
    );

    let paragraph = Paragraph::new(schedule_text).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

fn render_alarms(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Alarm Console - {} Active ", app.alarms.len()))
        .borders(Borders::ALL)
        .border_style(
            Style::default().fg(
                if app
                    .alarms
                    .iter()
                    .any(|a| matches!(a.severity, AlarmSeverity::Critical))
                {
                    Color::Red
                } else if !app.alarms.is_empty() {
                    Color::Yellow
                } else {
                    Color::Green
                },
            ),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.alarms.is_empty() {
        let no_alarms = Paragraph::new("✓ No active alarms - System operating normally")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        f.render_widget(no_alarms, inner);
    } else {
        let header = "Time      Severity Location    Parameter    Value   Limit";
        let mut lines = vec![Line::from(header)];
        lines.push(Line::from("─".repeat(60)));

        for alarm in app.alarms.iter().rev() {
            let (symbol, color) = match alarm.severity {
                AlarmSeverity::Critical => ("CRIT", Color::Red),
                AlarmSeverity::Warning => ("WARN", Color::Yellow),
            };

            let line = format!(
                "{} {} {} {:11} {:6.1} > {:.1}",
                alarm.timestamp.format("%H:%M:%S"),
                symbol,
                alarm.location,
                alarm.parameter,
                alarm.value,
                alarm.threshold
            );

            lines.push(Line::from(Span::styled(line, Style::default().fg(color))));
        }

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(area);

    // Controls help
    let help_text = if app.paused {
        " ⏸ PAUSED | [Space] Resume | [Tab] View | [↑↓←→] Navigate | [Ctrl+R] Reset | [Q] Quit "
    } else {
        " ▶ LIVE | [Space] Pause | [Tab] Next View | [Shift+Tab] Prev View | [↑↓←→] Navigate | [Ctrl+R] Reset | [Q] Quit "
    };

    let help = Paragraph::new(help_text).style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(help, chunks[0]);

    // Connection status
    let status_text = format!(
        " {} {} cells | {} msg/s ",
        if app.cells.is_empty() {
            "⚠ DISCONNECTED"
        } else {
            "● CONNECTED"
        },
        app.cells.len(),
        app.messages_per_second
    );

    let status = Paragraph::new(status_text)
        .style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(if app.cells.is_empty() {
                    Color::Yellow
                } else {
                    Color::Green
                }),
        )
        .alignment(Alignment::Right);
    f.render_widget(status, chunks[1]);
}

// Loading screen
pub fn draw_loading(f: &mut Frame) {
    let area = f.size();

    let text = vec![
        "",
        "",
        "╔═══════════════════════════════════════╗",
        "║                                       ║",
        "║         FLUX CONTROL SYSTEM           ║",
        "║                                       ║",
        "║         Initializing...               ║",
        "║                                       ║",
        "║     Connecting to Kafka brokers       ║",
        "║     Loading plant configuration       ║",
        "║     Starting real-time monitoring     ║",
        "║                                       ║",
        "╚═══════════════════════════════════════╝",
    ];

    let paragraph = Paragraph::new(text.join("\n"))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
