use crate::config::{DefaultsConfig, FluxConfig};
use crate::data::{CellMetric, QuestDBClient};
use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::event::KeyCode;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Clone)]
pub struct CellData {
    pub voltage: f64,
    pub current: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub efficiency: f64,
    pub power_kw: f64,
    pub specific_energy: f64,
    pub production_rate: f64,
    // --- ADDED FIELDS ---
    pub membrane_resistance: f64,
    pub sensor_quality: f64,
    // ---
    pub voltage_history: VecDeque<f64>,
    pub temp_history: VecDeque<f64>,
    pub efficiency_history: VecDeque<f64>,
    pub last_update: DateTime<Local>,
}

impl CellData {
    pub fn new(defaults: &DefaultsConfig) -> Self {
        Self {
            voltage: defaults.cell_state.voltage_v,
            current: defaults.cell_state.current_a,
            temperature: defaults.cell_state.temperature_c,
            pressure: defaults.cell_state.pressure_mbar,
            efficiency: defaults.cell_state.efficiency_percent,
            power_kw: (defaults.cell_state.voltage_v * defaults.cell_state.current_a) / 1000.0,
            specific_energy: 0.0,
            production_rate: 0.0,
            membrane_resistance: 0.0,
            sensor_quality: 1.0, // Assume 100% quality initially
            voltage_history: VecDeque::with_capacity(300),
            temp_history: VecDeque::with_capacity(300),
            efficiency_history: VecDeque::with_capacity(300),
            last_update: Local::now(),
        }
    }

    pub fn update_calculations(&mut self) {
        // TODO: These local calculations can now be replaced or augmented by data from analytics pipelines
        let current_density = self.current / 2.7;
        self.efficiency =
            (96.0 - 0.3 * (current_density / 1000.0) - 0.002 * (self.temperature - 85.0))
                .clamp(80.0, 100.0);
        self.power_kw = self.voltage * self.current / 1000.0;
        self.production_rate = (self.current / 1000.0) * 1.323 * (self.efficiency / 100.0);
        if self.production_rate > 0.0 {
            self.specific_energy = self.power_kw / self.production_rate;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ViewMode {
    Overview,
    Performance,
    Predictive,
    Economics,
    Maintenance,
    Alarms,
    CellDetail,
}

#[derive(Clone, Default)]
pub struct PlantMetrics {
    pub total_power_mw: f64,
    pub avg_efficiency: f64,
    pub total_production_cl2: f64,
    pub total_production_naoh: f64,
    pub total_production_h2: f64,
    pub specific_energy_avg: f64,
    pub hourly_revenue: f64,
    pub hourly_energy_cost: f64,
    pub gross_margin: f64,
    pub cells_online: usize,
    pub cells_warning: usize,
    pub cells_critical: usize,
}
impl PlantMetrics {
    pub fn calculate_from_cells(cells: &HashMap<String, CellData>, config: &FluxConfig) -> Self {
        if cells.is_empty() {
            return Self::default();
        }
        let mut metrics = Self::default();
        let limits = &config.limits;
        for cell in cells.values() {
            metrics.total_power_mw += cell.power_kw / 1000.0;
            metrics.avg_efficiency += cell.efficiency;
            metrics.total_production_cl2 += cell.production_rate;
            if cell.voltage > limits.critical.voltage_v
                || cell.temperature > limits.critical.temperature_c
            {
                metrics.cells_critical += 1;
            } else if cell.voltage > limits.warning.voltage_v
                || cell.temperature > limits.warning.temperature_c
            {
                metrics.cells_warning += 1;
            } else {
                metrics.cells_online += 1;
            }
        }
        let cell_count = cells.len() as f64;
        if cell_count > 0.0 {
            metrics.avg_efficiency /= cell_count;
        }
        metrics.total_production_cl2 = metrics.total_production_cl2 * 24.0 / 1000.0;
        metrics.total_production_naoh = metrics.total_production_cl2 * (80.0 / 71.0);
        metrics.total_production_h2 = metrics.total_production_cl2 * (2.0 / 71.0);
        if metrics.total_production_cl2 > 0.0 {
            metrics.specific_energy_avg =
                (metrics.total_power_mw * 1000.0 * 24.0) / metrics.total_production_cl2;
        }
        let financials = &config.financials;
        metrics.hourly_energy_cost = metrics.total_power_mw * financials.electricity_price_per_mwh;
        metrics.hourly_revenue = (metrics.total_production_cl2 / 24.0)
            * financials.chlorine_price_per_mt
            + (metrics.total_production_naoh / 24.0) * financials.naoh_price_per_mt
            + (metrics.total_production_h2 / 24.0) * financials.hydrogen_price_per_mt;
        if metrics.hourly_revenue > 0.0 {
            metrics.gross_margin = (metrics.hourly_revenue - metrics.hourly_energy_cost)
                / metrics.hourly_revenue
                * 100.0;
        }
        metrics
    }
}

pub struct App {
    pub cfg: Arc<FluxConfig>,
    pub cells: HashMap<String, CellData>,
    pub metrics: PlantMetrics,
    pub current_view: ViewMode,
    pub selected_unit: u8,
    pub selected_stack: char,
    pub selected_cell: u8,
    pub paused: bool,
    pub unit_scroll_offset: usize,
    pub cell_scroll_offsets: HashMap<u8, u8>,
    pub messages_per_second: usize,
    pub total_messages: usize,
    pub last_message_count: usize,
    pub tick_count: usize,
    pub questdb: Arc<QuestDBClient>,
}

impl App {
    pub async fn new(cfg: Arc<FluxConfig>, questdb: Arc<QuestDBClient>) -> Result<Self> {
        Ok(Self {
            selected_unit: cfg.plant.geometry.units[0],
            selected_stack: cfg.plant.geometry.stacks[0],
            cfg,
            cells: HashMap::new(),
            metrics: PlantMetrics::default(),
            current_view: ViewMode::Overview,
            selected_cell: 1,
            paused: false,
            unit_scroll_offset: 0,
            cell_scroll_offsets: HashMap::new(),
            messages_per_second: 0,
            total_messages: 0,
            last_message_count: 0,
            tick_count: 0,
            questdb,
        })
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        let metrics = self.questdb.get_latest_metrics().await?;
        self.total_messages += metrics.len();

        for metric in metrics {
            let key = format!(
                "U{}_S{}_C{:02}",
                metric.unit_id, metric.stack_id, metric.cell_id
            );
            let cell = self
                .cells
                .entry(key)
                .or_insert_with(|| CellData::new(&self.cfg.defaults));

            if let Some(v) = metric.voltage_v {
                cell.voltage = v;
                cell.voltage_history.push_back(v);
                if cell.voltage_history.len() > 300 {
                    cell.voltage_history.pop_front();
                }
            }
            if let Some(c) = metric.current_a {
                cell.current = c;
            }
            if let Some(t) = metric.temperature_c {
                cell.temperature = t;
                cell.temp_history.push_back(t);
                if cell.temp_history.len() > 300 {
                    cell.temp_history.pop_front();
                }
            }
            if let Some(p) = metric.pressure_mbar {
                cell.pressure = p;
            }
            if let Some(e) = metric.efficiency_pct {
                cell.efficiency = e;
                cell.efficiency_history.push_back(e);
                if cell.efficiency_history.len() > 300 {
                    cell.efficiency_history.pop_front();
                }
            }
            if let Some(pw) = metric.power_kw {
                cell.power_kw = pw;
            }

            // Map the new fields from the database metric to the app's cell state
            cell.specific_energy = metric.specific_energy.unwrap_or(cell.specific_energy);
            cell.membrane_resistance = metric
                .membrane_resistance
                .unwrap_or(cell.membrane_resistance);
            cell.sensor_quality = metric.sensor_quality.unwrap_or(cell.sensor_quality);

            cell.update_calculations(); // Recalculate derived values
            cell.last_update = Local::now();
        }

        self.metrics = PlantMetrics::calculate_from_cells(&self.cells, &self.cfg);
        Ok(())
    }

    // ... (All other methods in App remain unchanged) ...
    pub async fn get_cell_history(&self, _key: &str) -> Result<Vec<CellMetric>> {
        Ok(vec![])
    }
    pub fn handle_key_event(&mut self, key_code: KeyCode) {
        match self.current_view {
            ViewMode::CellDetail => {
                if let KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter = key_code {
                    self.current_view = ViewMode::Overview;
                }
            }
            _ => match key_code {
                KeyCode::Tab => self.next_view(),
                KeyCode::BackTab => self.previous_view(),
                KeyCode::Up => self.navigate_up(),
                KeyCode::Down => self.navigate_down(),
                KeyCode::Left => self.navigate_left(),
                KeyCode::Right => self.navigate_right(),
                KeyCode::PageDown => {
                    self.selected_cell =
                        (self.selected_cell + 5).min(self.cfg.plant.geometry.cells_per_stack);
                }
                KeyCode::PageUp => {
                    self.selected_cell = self.selected_cell.saturating_sub(5).max(1);
                }
                KeyCode::Enter => self.current_view = ViewMode::CellDetail,
                KeyCode::Char(' ') => self.paused = !self.paused,
                KeyCode::F(1) => self.current_view = ViewMode::Overview,
                KeyCode::F(2) => self.current_view = ViewMode::Performance,
                KeyCode::F(3) => self.current_view = ViewMode::Predictive,
                KeyCode::F(4) => self.current_view = ViewMode::Economics,
                KeyCode::F(5) => self.current_view = ViewMode::Maintenance,
                KeyCode::F(6) => self.current_view = ViewMode::Alarms,
                _ => {}
            },
        }
    }
    pub fn on_tick(&mut self) {
        self.tick_count += 1;
        if !self.paused {
            self.metrics = PlantMetrics::calculate_from_cells(&self.cells, &self.cfg);
        }
        if self.tick_count % 20 == 0 {
            self.messages_per_second = self.total_messages - self.last_message_count;
            self.last_message_count = self.total_messages;
        }
    }
    pub fn next_view(&mut self) {
        self.current_view = match self.current_view {
            ViewMode::Overview => ViewMode::Performance,
            ViewMode::Performance => ViewMode::Predictive,
            ViewMode::Predictive => ViewMode::Economics,
            ViewMode::Economics => ViewMode::Maintenance,
            ViewMode::Maintenance => ViewMode::Alarms,
            ViewMode::Alarms => ViewMode::Overview,
            ViewMode::CellDetail => ViewMode::Performance,
        };
    }
    pub fn previous_view(&mut self) {
        self.current_view = match self.current_view {
            ViewMode::Overview => ViewMode::Alarms,
            ViewMode::Performance => ViewMode::Overview,
            ViewMode::Predictive => ViewMode::Performance,
            ViewMode::Economics => ViewMode::Predictive,
            ViewMode::Maintenance => ViewMode::Economics,
            ViewMode::Alarms => ViewMode::Maintenance,
            ViewMode::CellDetail => ViewMode::Alarms,
        };
    }
    pub fn navigate_up(&mut self) {
        if self.selected_cell > 1 {
            self.selected_cell -= 1;
        }
    }
    pub fn navigate_down(&mut self) {
        if self.selected_cell < self.cfg.plant.geometry.cells_per_stack {
            self.selected_cell += 1;
        }
    }
    pub fn navigate_left(&mut self) {
        let stacks = &self.cfg.plant.geometry.stacks;
        if let Some(idx) = stacks.iter().position(|&s| s == self.selected_stack) {
            if idx > 0 {
                self.selected_stack = stacks[idx - 1];
            } else if let Some(unit_idx) = self
                .cfg
                .plant
                .geometry
                .units
                .iter()
                .position(|&u| u == self.selected_unit)
            {
                if unit_idx > 0 {
                    self.selected_unit = self.cfg.plant.geometry.units[unit_idx - 1];
                    self.selected_stack = *stacks.last().unwrap();
                }
            }
        }
    }
    pub fn navigate_right(&mut self) {
        let stacks = &self.cfg.plant.geometry.stacks;
        if let Some(idx) = stacks.iter().position(|&s| s == self.selected_stack) {
            if idx + 1 < stacks.len() {
                self.selected_stack = stacks[idx + 1];
            } else if let Some(unit_idx) = self
                .cfg
                .plant
                .geometry
                .units
                .iter()
                .position(|&u| u == self.selected_unit)
            {
                if unit_idx + 1 < self.cfg.plant.geometry.units.len() {
                    self.selected_unit = self.cfg.plant.geometry.units[unit_idx + 1];
                    self.selected_stack = stacks[0];
                }
            }
        }
    }
    pub fn get_selected_cell_key(&self) -> String {
        format!(
            "U{}_S{}_C{:02}",
            self.selected_unit, self.selected_stack, self.selected_cell
        )
    }
    pub fn get_selected_cell_data(&self) -> Option<&CellData> {
        self.cells.get(&self.get_selected_cell_key())
    }
}
