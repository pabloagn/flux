"""Plant simulation"""

from __future__ import annotations
import asyncio
import time
from datetime import datetime, timezone
from typing import Dict, Any

from flux.utils.log import get_logger, banner, task_progress
from flux.config.simulations import CFG
from flux.simulation.cell import ElectrolyzerCell
from flux.simulation.sensors import Sensor
from flux.pipeline.kafka_producer import SensorDataProducer

log = get_logger(__name__)

# --- Constants Pulled From Config ---
UNITS = CFG.geometry.units
STACKS = CFG.geometry.stacks
CELLS_PER_STACK = CFG.geometry.cells_per_stack
# We run the main loop at the highest frequency
HIGHEST_FREQUENCY_HZ = CFG.sampling.electrical_hz
SAMPLING_INTERVAL_S = 1 / HIGHEST_FREQUENCY_HZ
# The single, canonical topic for all cell metrics
CANONICAL_TOPIC = "flux.cell.metrics.v1"


# --- Helper Builders ---
def make_cell(unit: int, stack: str, idx: int) -> ElectrolyzerCell:
    return ElectrolyzerCell(f"U{unit}_S{stack}_C{idx:02d}", area=CFG.geometry.membrane_area_m2)


def make_sensors(cell_id: str) -> Dict[str, Sensor]:
    n = CFG.noise.stddev()
    return {
        "voltage": Sensor(f"{cell_id}_V1", "voltage", noise_stddev=n["voltage"]),
        "current": Sensor(f"{cell_id}_I", "current", noise_stddev=n["current"]),
        "pressure": Sensor(f"{cell_id}_DP", "pressure", noise_stddev=n["pressure"]),
        "temp_anolyte": Sensor(f"{cell_id}_TA", "temperature", noise_stddev=n["temp_anolyte"]),
        "temp_catholyte": Sensor(f"{cell_id}_TC", "temperature", noise_stddev=n["temp_catholyte"]),
    }


# --- Simulator Class ---
class PlantSimulator:
    """Orchestrates cells, sensors and Kafka streaming."""

    def __init__(self, producer: SensorDataProducer, run_seconds: int = 60):
        self.producer = producer
        self.run_seconds = run_seconds
        self.cells: Dict[str, ElectrolyzerCell] = {}
        self.sensors: Dict[str, Dict[str, Sensor]] = {}

        banner("Building plant model")
        for u in UNITS:
            for s in STACKS:
                for c in range(1, CELLS_PER_STACK + 1):
                    cell = make_cell(u, s, c)
                    self.cells[cell.cell_id] = cell
                    # Sensors are now just for internal simulation, not for structuring output
                    self.sensors[cell.cell_id] = make_sensors(cell.cell_id)

        log.info(
            "Plant ready (%d cells, %d sensors). Emitting to topic '%s' at %.1f Hz.",
            len(self.cells),
            sum(len(v) for v in self.sensors.values()),
            CANONICAL_TOPIC,
            HIGHEST_FREQUENCY_HZ,
        )

    def _get_noisy_value(
        self, cell_id: str, measurement_type: str, true_value: float, dt: float
    ) -> tuple[float | None, float]:
        """Helper to get a noisy sensor reading and its quality."""
        sensor = self.sensors[cell_id][measurement_type]
        reading = sensor.read(true_value, dt=dt)
        return reading.get("value"), reading.get("quality", 1.0) / 100.0

    async def run(self):
        """
        Main simulation loop.
        Runs at the highest sampling frequency and emits a complete, unified
        payload for every cell on each tick.
        """
        with task_progress(f"Running plant simulation for {self.run_seconds}s"):
            end_time = time.time() + self.run_seconds
            total_msgs = 0

            while time.time() < end_time:
                loop_start_time = time.time()
                tick_ts = datetime.now(timezone.utc)

                for cell in self.cells.values():
                    # 1. Advance the physical state of the cell
                    state = cell.step(dt=SAMPLING_INTERVAL_S)

                    # 2. Get noisy readings for all sensors to simulate real-world data
                    voltage, v_quality = self._get_noisy_value(
                        cell.cell_id, "voltage", state["voltage"], SAMPLING_INTERVAL_S
                    )
                    current, c_quality = self._get_noisy_value(
                        cell.cell_id, "current", state["current"], SAMPLING_INTERVAL_S
                    )
                    pressure, p_quality = self._get_noisy_value(
                        cell.cell_id, "pressure", state["pressure_diff"], SAMPLING_INTERVAL_S
                    )
                    temp, t_quality = self._get_noisy_value(
                        cell.cell_id, "temp_anolyte", state["temperature"], SAMPLING_INTERVAL_S
                    )

                    # 3. Construct the single, unified payload
                    # This structure MUST match the CellMetric Pydantic model in the data-pipeline
                    payload = {
                        "ts": tick_ts.isoformat(),
                        "unit_id": int(cell.cell_id.split("_")[0][1:]),
                        "stack_id": cell.cell_id.split("_")[1][1:],
                        "cell_id": int(cell.cell_id[-2:]),
                        "voltage_v": voltage,
                        "current_a": current,
                        "temperature_c": temp,
                        "pressure_mbar": pressure,
                        "efficiency_pct": state["current_efficiency"],
                        "power_kw": (voltage * current / 1000) if voltage and current else None,
                        # These would be calculated by a separate analytics service in a real system,
                        # but we can simulate them here.
                        "specific_energy": None,
                        "membrane_resistance": None,
                        "sensor_quality": min(v_quality, c_quality, p_quality, t_quality),
                    }

                    # 4. Send to the canonical topic
                    self.producer.send(CANONICAL_TOPIC, payload, key=cell.cell_id)
                    total_msgs += 1

                # Maintain the loop frequency
                loop_duration = time.time() - loop_start_time
                await asyncio.sleep(max(0, SAMPLING_INTERVAL_S - loop_duration))

        log.info("Simulation loop finished. Total messages sent: %d", total_msgs)


# --- CLI Helper (no changes needed here) ---
def main() -> None:
    import argparse, os

    parser = argparse.ArgumentParser(description="Run full-plant simulator")
    parser.add_argument(
        "-t",
        "--time",
        type=int,
        default=CFG.default_run_s,
        help="Run time in seconds (default: %(default)s)",
    )
    args = parser.parse_args()

    producer = SensorDataProducer(os.getenv("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092"))
    sim = PlantSimulator(producer, run_seconds=args.time)

    try:
        asyncio.run(sim.run())
    finally:
        producer.flush()
        producer.close()
        log.info("Simulation complete and Kafka producer closed.")


if __name__ == "__main__":
    main()
