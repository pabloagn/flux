use chrono::{DateTime, Local};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Deserialize)]
pub struct SensorMessage {
    pub ts: String,
    pub unit: u8,
    pub stack: char,
    pub cell: u8,
    #[serde(default)]
    pub sensor_id: Option<String>,
    #[serde(default, rename = "voltage_V")]
    pub voltage_v: Option<f64>,
    #[serde(default, rename = "current_A")]
    pub current_a: Option<f64>,
    #[serde(default, rename = "temperature_C")]
    pub temperature_c: Option<f64>,
    #[serde(default, rename = "pressure_mbar")]
    pub pressure_mbar: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub quality: Option<f64>,
}

#[derive(Clone)]
pub struct CellData {
    // REAL DATA from Kafka
    pub voltage: f64,
    pub current: f64,
    pub temperature: f64,
    pub pressure: f64,

    // CALCULATED from real data
    pub efficiency: f64,      // Calculated from voltage/current/temp
    pub power_kw: f64,        // V * I / 1000
    pub specific_energy: f64, // kWh/kg Cl2
    pub production_rate: f64, // kg/h Cl2

    // History for trends (REAL)
    pub voltage_history: VecDeque<f64>,
    pub temp_history: VecDeque<f64>,
    pub efficiency_history: VecDeque<f64>,

    // TODO: HARDCODED predictions (until we have digital twins)
    pub predicted_lifetime_days: i32,
    pub degradation_rate: f64,

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
            predicted_lifetime_days: 1825, // Hardcoded
            degradation_rate: 0.02,        // Hardcoded
            last_update: Local::now(),
        }
    }
}

impl CellData {
    // Calculate real metrics from sensor data
    pub fn update_calculations(&mut self) {
        // Current efficiency based on Butler-Volmer (from requirements.md)
        // CE = 96 - 0.3×(J/1000) - 0.002×(T-85)
        let current_density = self.current / 2.7; // A/m² (2.7 m² membrane area)
        self.efficiency =
            96.0 - 0.3 * (current_density / 1000.0) - 0.002 * (self.temperature - 85.0);
        self.efficiency = self.efficiency.clamp(80.0, 100.0);

        // Power in kW
        self.power_kw = self.voltage * self.current / 1000.0;

        // Production rate (kg/h Cl₂) - from requirements: 1.323 kg/h per kA
        self.production_rate = (self.current / 1000.0) * 1.323 * (self.efficiency / 100.0);

        // Specific energy consumption (kWh/kg Cl₂)
        if self.production_rate > 0.0 {
            self.specific_energy = self.power_kw / self.production_rate;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Overview,
    Performance,
    Predictive,
    Economics,
    Maintenance,
    Alarms,
}

#[derive(Clone)]
pub struct PlantMetrics {
    // CALCULATED from real cell data
    pub total_power_mw: f64,
    pub total_current_ka: f64,
    pub avg_voltage: f64,
    pub avg_efficiency: f64,
    pub total_production_cl2: f64,  // MT/day
    pub total_production_naoh: f64, // MT/day
    pub total_production_h2: f64,   // MT/day
    pub specific_energy_avg: f64,   // kWh/MT

    // HARDCODED economic data (no real market feed yet)
    pub electricity_price: f64, // $/MWh
    pub chlorine_price: f64,    // $/MT
    pub caustic_price: f64,     // $/MT
    pub hydrogen_price: f64,    // $/MT

    // CALCULATED economics using real + hardcoded
    pub hourly_revenue: f64,
    pub hourly_energy_cost: f64,
    pub gross_margin: f64,

    // HARDCODED predictions (placeholder for ML)
    pub efficiency_24h_forecast: f64,
    pub failure_risk_score: f64,
    pub recommended_current: f64,

    // Statistics from real data
    pub cells_online: usize,
    pub cells_warning: usize,
    pub cells_critical: usize,
    pub data_rate_hz: f64,
}

impl PlantMetrics {
    pub fn calculate_from_cells(cells: &HashMap<String, CellData>) -> Self {
        let mut metrics = Self {
            // Start with zeros
            total_power_mw: 0.0,
            total_current_ka: 0.0,
            avg_voltage: 0.0,
            avg_efficiency: 0.0,
            total_production_cl2: 0.0,
            total_production_naoh: 0.0,
            total_production_h2: 0.0,
            specific_energy_avg: 0.0,

            // Hardcoded market prices
            electricity_price: 50.0, // $/MWh
            chlorine_price: 250.0,   // $/MT
            caustic_price: 420.0,    // $/MT
            hydrogen_price: 2000.0,  // $/MT

            hourly_revenue: 0.0,
            hourly_energy_cost: 0.0,
            gross_margin: 0.0,

            // Hardcoded predictions
            efficiency_24h_forecast: 93.8,
            failure_risk_score: 12.5,
            recommended_current: 5100.0,

            cells_online: 0,
            cells_warning: 0,
            cells_critical: 0,
            data_rate_hz: 0.0,
        };

        if cells.is_empty() {
            return metrics;
        }

        // Calculate from REAL data
        let mut total_voltage = 0.0;
        let mut total_efficiency = 0.0;

        for cell in cells.values() {
            metrics.total_power_mw += cell.power_kw / 1000.0;
            metrics.total_current_ka += cell.current / 1000.0;
            total_voltage += cell.voltage;
            total_efficiency += cell.efficiency;

            // Production in kg/h
            metrics.total_production_cl2 += cell.production_rate;

            // Cell status based on REAL thresholds
            if cell.voltage > 3.5 || cell.temperature > 95.0 {
                metrics.cells_critical += 1;
            } else if cell.voltage > 3.3 || cell.temperature > 90.0 {
                metrics.cells_warning += 1;
            } else {
                metrics.cells_online += 1;
            }
        }

        let cell_count = cells.len() as f64;
        metrics.avg_voltage = total_voltage / cell_count;
        metrics.avg_efficiency = total_efficiency / cell_count;

        // Convert kg/h to MT/day
        metrics.total_production_cl2 = metrics.total_production_cl2 * 24.0 / 1000.0;

        // Stoichiometry: 1 Cl₂ : 2 NaOH : 1 H₂ (molar)
        // Mass ratio: 71 g Cl₂ : 80 g NaOH : 2 g H₂
        metrics.total_production_naoh = metrics.total_production_cl2 * (80.0 / 71.0);
        metrics.total_production_h2 = metrics.total_production_cl2 * (2.0 / 71.0);

        // Average specific energy
        if metrics.total_production_cl2 > 0.0 {
            metrics.specific_energy_avg =
                (metrics.total_power_mw * 24.0) / metrics.total_production_cl2;
        }

        // Economic calculations (mix of real and hardcoded)
        metrics.hourly_energy_cost = metrics.total_power_mw * metrics.electricity_price;
        metrics.hourly_revenue = (metrics.total_production_cl2 / 24.0) * metrics.chlorine_price
            + (metrics.total_production_naoh / 24.0) * metrics.caustic_price
            + (metrics.total_production_h2 / 24.0) * metrics.hydrogen_price;

        if metrics.hourly_revenue > 0.0 {
            metrics.gross_margin = ((metrics.hourly_revenue - metrics.hourly_energy_cost)
                / metrics.hourly_revenue)
                * 100.0;
        }

        metrics
    }
}

#[derive(Clone)]
pub struct Alarm {
    pub timestamp: DateTime<Local>,
    pub severity: AlarmSeverity,
    pub location: String,
    pub value: f64,
    pub threshold: f64,
    pub parameter: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AlarmSeverity {
    Warning,
    Critical,
}

pub struct App {
    pub cells: HashMap<String, CellData>,
    pub metrics: PlantMetrics,
    pub alarms: VecDeque<Alarm>,
    pub message_history: VecDeque<SensorMessage>,

    // UI state
    pub current_view: ViewMode,
    pub selected_unit: u8,
    pub selected_stack: char,
    pub selected_cell: u8,
    pub show_details: bool,
    pub paused: bool,

    // Stats
    pub messages_per_second: usize,
    pub total_messages: usize,
    pub last_message_count: usize,
    pub tick_count: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            metrics: PlantMetrics::calculate_from_cells(&HashMap::new()),
            alarms: VecDeque::with_capacity(100),
            message_history: VecDeque::with_capacity(1000),
            current_view: ViewMode::Overview,
            selected_unit: 1,
            selected_stack: 'A',
            selected_cell: 1,
            show_details: false,
            paused: false,
            messages_per_second: 0,
            total_messages: 0,
            last_message_count: 0,
            tick_count: 0,
        }
    }

    pub fn process_message(&mut self, json_str: &str) {
        if self.paused {
            return;
        }

        self.total_messages += 1;

        if let Ok(msg) = serde_json::from_str::<SensorMessage>(json_str) {
            let cell_key = format!("U{}_S{}_C{:02}", msg.unit, msg.stack, msg.cell);

            let cell = self
                .cells
                .entry(cell_key.clone())
                .or_insert_with(CellData::default);

            // Update REAL data
            if let Some(v) = msg.voltage_v {
                cell.voltage = v;
                cell.voltage_history.push_back(v);
                if cell.voltage_history.len() > 300 {
                    cell.voltage_history.pop_front();
                }

                // Check for alarms
                if v > 3.5 {
                    self.add_alarm(AlarmSeverity::Critical, &cell_key, "Voltage", v, 3.5);
                } else if v > 3.3 {
                    self.add_alarm(AlarmSeverity::Warning, &cell_key, "Voltage", v, 3.3);
                }
            }

            if let Some(i) = msg.current_a {
                cell.current = i;
            }

            if let Some(t) = msg.temperature_c {
                cell.temperature = t;
                cell.temp_history.push_back(t);
                if cell.temp_history.len() > 300 {
                    cell.temp_history.pop_front();
                }

                if t > 95.0 {
                    self.add_alarm(AlarmSeverity::Critical, &cell_key, "Temperature", t, 95.0);
                } else if t > 90.0 {
                    self.add_alarm(AlarmSeverity::Warning, &cell_key, "Temperature", t, 90.0);
                }
            }

            if let Some(p) = msg.pressure_mbar {
                cell.pressure = p;

                if p > 300.0 {
                    self.add_alarm(AlarmSeverity::Warning, &cell_key, "Pressure", p, 300.0);
                }
            }

            // Calculate derived metrics from real data
            cell.update_calculations();
            cell.efficiency_history.push_back(cell.efficiency);
            if cell.efficiency_history.len() > 300 {
                cell.efficiency_history.pop_front();
            }

            cell.last_update = Local::now();

            // Store message
            self.message_history.push_back(msg);
            if self.message_history.len() > 1000 {
                self.message_history.pop_front();
            }
        }
    }

    fn add_alarm(
        &mut self,
        severity: AlarmSeverity,
        location: &str,
        parameter: &str,
        value: f64,
        threshold: f64,
    ) {
        // Avoid duplicate alarms
        if let Some(last) = self.alarms.back() {
            if last.location == location && last.parameter == parameter {
                return;
            }
        }

        self.alarms.push_back(Alarm {
            timestamp: Local::now(),
            severity,
            location: location.to_string(),
            parameter: parameter.to_string(),
            value,
            threshold,
        });

        if self.alarms.len() > 100 {
            self.alarms.pop_front();
        }
    }

    pub fn calculate_advanced_metrics(&mut self) {
        self.metrics = PlantMetrics::calculate_from_cells(&self.cells);
        self.metrics.data_rate_hz = self.messages_per_second as f64;
    }

    pub fn on_tick(&mut self) {
        self.tick_count += 1;
        if self.tick_count % 20 == 0 {
            let current = self.total_messages;
            self.messages_per_second = (current - self.last_message_count) * 20;
            self.last_message_count = current;
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
        };
    }

    // Navigation methods
    pub fn navigate_up(&mut self) {
        if self.selected_cell > 1 {
            self.selected_cell -= 1;
        }
    }

    pub fn navigate_down(&mut self) {
        if self.selected_cell < 20 {
            self.selected_cell += 1;
        }
    }

    pub fn navigate_left(&mut self) {
        match self.selected_stack {
            'B' => self.selected_stack = 'A',
            'C' => self.selected_stack = 'B',
            _ => {
                if self.selected_unit > 1 {
                    self.selected_unit -= 1;
                    self.selected_stack = 'C';
                }
            }
        }
    }

    pub fn navigate_right(&mut self) {
        match self.selected_stack {
            'A' => self.selected_stack = 'B',
            'B' => self.selected_stack = 'C',
            _ => {
                if self.selected_unit < 3 {
                    self.selected_unit += 1;
                    self.selected_stack = 'A';
                }
            }
        }
    }

    pub fn select_current(&mut self) {
        self.show_details = true;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn reset_alarms(&mut self) {
        self.alarms.clear();
    }

    pub fn zoom_in(&mut self) {
        // Future: implement zoom
    }

    pub fn zoom_out(&mut self) {
        // Future: implement zoom
    }

    pub fn get_selected_cell_key(&self) -> String {
        format!(
            "U{}_S{}_C{:02}",
            self.selected_unit, self.selected_stack, self.selected_cell
        )
    }
}
