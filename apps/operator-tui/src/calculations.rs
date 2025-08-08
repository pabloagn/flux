use crate::app::CellData;

pub fn calculate_membrane_health(cell: &CellData) -> f64 {
    // Based on voltage increase over nominal
    let voltage_factor = 100.0 - (cell.voltage - 3.0).max(0.0) * 50.0;
    let temp_factor = 100.0 - (cell.temperature - 85.0).abs() * 2.0;
    let efficiency_factor = cell.efficiency;

    (voltage_factor * 0.5 + temp_factor * 0.2 + efficiency_factor * 0.3).clamp(0.0, 100.0)
}

pub fn calculate_specific_energy(voltage: f64, current: f64, production_rate: f64) -> f64 {
    if production_rate > 0.0 {
        (voltage * current) / (production_rate * 1000.0)
    } else {
        0.0
    }
}

pub fn predict_failure_probability(cell: &CellData) -> f64 {
    let voltage_risk = ((cell.voltage - 3.0) / 0.5 * 100.0).clamp(0.0, 100.0);
    let temp_risk = ((cell.temperature - 85.0) / 10.0 * 100.0).clamp(0.0, 100.0);
    let efficiency_risk = ((94.0 - cell.efficiency) / 10.0 * 100.0).clamp(0.0, 100.0);

    (voltage_risk * 0.4 + temp_risk * 0.3 + efficiency_risk * 0.3).clamp(0.0, 100.0)
}
