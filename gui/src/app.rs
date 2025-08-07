use crate::config::PlantLayout;
use chrono::{DateTime, Local};
use crossterm::event::KeyCode;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Deserialize)]
pub struct SensorMessage {
    pub ts: String,
    pub unit: u8,
    pub stack: String,
    pub cell: u8,
    #[serde(default)]
    pub sensor_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub quality: Option<f64>,
    #[serde(default)]
    pub voltage_V: Option<f64>,
    #[serde(default)]
    pub current_A: Option<f64>,
    #[serde(default)]
    pub temperature_C: Option<f64>,
    #[serde(default)]
    pub pressure_mbar: Option<f64>,
}

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
    pub voltage_history: VecDeque<f64>,
    pub temp_history: VecDeque<f64>,
    pub efficiency_history: VecDeque<f64>,
    pub last_update: DateTime<Local>,
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            voltage: 3.0,
            current: 5000.0,
            temperature: 85.0,
            pressure: 150.0,
            efficiency: 94.0,
            power_kw: 15.0,
            specific_energy: 2450.0,
            production_rate: 6.62,
            voltage_history: VecDeque::with_capacity(300),
            temp_history: VecDeque::with_capacity(300),
            efficiency_history: VecDeque::with_capacity(300),
            last_update: Local::now(),
        }
    }
}

impl CellData {
    pub fn update_calculations(&mut self) {
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
    pub fn calculate_from_cells(cells: &HashMap<String, CellData>) -> Self {
        if cells.is_empty() {
            return Self::default();
        }
        let mut metrics = Self::default();
        for cell in cells.values() {
            metrics.total_power_mw += cell.power_kw / 1000.0;
            metrics.avg_efficiency += cell.efficiency;
            metrics.total_production_cl2 += cell.production_rate;
            if cell.voltage > 3.5 || cell.temperature > 95.0 {
                metrics.cells_critical += 1;
            } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
                metrics.cells_warning += 1;
            } else {
                metrics.cells_online += 1;
            }
        }
        let cell_count = cells.len() as f64;
        metrics.avg_efficiency /= cell_count;
        metrics.total_production_cl2 = metrics.total_production_cl2 * 24.0 / 1000.0;
        metrics.total_production_naoh = metrics.total_production_cl2 * (80.0 / 71.0);
        metrics.total_production_h2 = metrics.total_production_cl2 * (2.0 / 71.0);
        if metrics.total_production_cl2 > 0.0 {
            metrics.specific_energy_avg =
                (metrics.total_power_mw * 1000.0 * 24.0) / metrics.total_production_cl2;
        }
        let (electricity_price, chlorine_price, caustic_price, hydrogen_price) =
            (50.0, 250.0, 420.0, 2000.0);
        metrics.hourly_energy_cost = metrics.total_power_mw * electricity_price;
        metrics.hourly_revenue = (metrics.total_production_cl2 / 24.0) * chlorine_price
            + (metrics.total_production_naoh / 24.0) * caustic_price
            + (metrics.total_production_h2 / 24.0) * hydrogen_price;
        if metrics.hourly_revenue > 0.0 {
            metrics.gross_margin = (metrics.hourly_revenue - metrics.hourly_energy_cost)
                / metrics.hourly_revenue
                * 100.0;
        }
        metrics
    }
}

pub struct App {
    pub cfg: PlantLayout,
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
}

impl App {
    pub fn new(cfg: PlantLayout) -> Self {
        Self {
            selected_unit: cfg.geometry.units[0],
            selected_stack: cfg.geometry.stacks[0],
            cfg,
            cells: HashMap::new(),
            metrics: PlantMetrics::default(),
            current_view: ViewMode::Overview,
            selected_cell: 1,
            paused: false,
            unit_scroll_offset: 0,
            cell_scroll_offsets: HashMap::new(), // Initialize the HashMap
            messages_per_second: 0,
            total_messages: 0,
            last_message_count: 0,
            tick_count: 0,
        }
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
                    let max_cells = self.cfg.geometry.cells_per_stack;
                    self.selected_cell = (self.selected_cell + 5).min(max_cells);
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
            self.metrics = PlantMetrics::calculate_from_cells(&self.cells);
        }
        if self.tick_count % 20 == 0 {
            self.messages_per_second = self.total_messages - self.last_message_count;
            self.last_message_count = self.total_messages;
        }
    }

    pub fn process_message(&mut self, json_str: &str) {
        if self.paused {
            return;
        }
        if let Ok(msg) = serde_json::from_str::<SensorMessage>(json_str) {
            self.total_messages += 1;
            let stack_char = msg.stack.chars().next().unwrap_or('?');
            let key = format!("U{}_S{}_C{:02}", msg.unit, stack_char, msg.cell);
            let cell = self.cells.entry(key).or_insert_with(CellData::default);
            if let Some(v) = msg.voltage_V {
                cell.voltage = v;
                cell.voltage_history.push_back(v);
                if cell.voltage_history.len() > 300 {
                    cell.voltage_history.pop_front();
                }
            }
            if let Some(t) = msg.temperature_C {
                cell.temperature = t;
                cell.temp_history.push_back(t);
                if cell.temp_history.len() > 300 {
                    cell.temp_history.pop_front();
                }
            }
            if let Some(c) = msg.current_A {
                cell.current = c;
            }
            if let Some(p) = msg.pressure_mbar {
                cell.pressure = p;
            }
            cell.update_calculations();
            cell.efficiency_history.push_back(cell.efficiency);
            if cell.efficiency_history.len() > 300 {
                cell.efficiency_history.pop_front();
            }
            cell.last_update = Local::now();
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
        if self.selected_cell < self.cfg.geometry.cells_per_stack {
            self.selected_cell += 1;
        }
    }

    pub fn navigate_left(&mut self) {
        let stacks = &self.cfg.geometry.stacks;
        if let Some(idx) = stacks.iter().position(|&s| s == self.selected_stack) {
            if idx > 0 {
                self.selected_stack = stacks[idx - 1];
            } else if let Some(unit_idx) = self
                .cfg
                .geometry
                .units
                .iter()
                .position(|&u| u == self.selected_unit)
            {
                if unit_idx > 0 {
                    self.selected_unit = self.cfg.geometry.units[unit_idx - 1];
                    self.selected_stack = *stacks.last().unwrap();
                }
            }
        }
    }

    pub fn navigate_right(&mut self) {
        let stacks = &self.cfg.geometry.stacks;
        if let Some(idx) = stacks.iter().position(|&s| s == self.selected_stack) {
            if idx + 1 < stacks.len() {
                self.selected_stack = stacks[idx + 1];
            } else if let Some(unit_idx) = self
                .cfg
                .geometry
                .units
                .iter()
                .position(|&u| u == self.selected_unit)
            {
                if unit_idx + 1 < self.cfg.geometry.units.len() {
                    self.selected_unit = self.cfg.geometry.units[unit_idx + 1];
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
