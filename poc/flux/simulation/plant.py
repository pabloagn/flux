"""Plant simulation"""

from __future__ import annotations
import asyncio, time
from datetime import datetime
from collections import defaultdict
from typing import Dict, List, Tuple

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
SAMPLE: Dict[str, float] = CFG.sampling.intervals
NOISE_STD: Dict[str, float] = CFG.noise.stddev()
TOPIC = CFG.topics.as_dict()


# --- Helper Builders ---
def make_cell(unit: int, stack: str, idx: int) -> ElectrolyzerCell:
    return ElectrolyzerCell(f"U{unit}_S{stack}_C{idx:02d}")


def make_sensors(cell_id: str) -> Dict[str, Sensor]:
    n = NOISE_STD
    return {
        "voltage": Sensor(f"{cell_id}_V1", "voltage", noise_stddev=n["voltage"]),
        "current": Sensor(f"{cell_id}_I", "current", noise_stddev=n["current"]),
        "pressure": Sensor(f"{cell_id}_DP", "pressure", noise_stddev=n["pressure"]),
        "temp_anolyte": Sensor(
            f"{cell_id}_TA", "temperature", noise_stddev=n["temp_anolyte"]
        ),
        "temp_catholyte": Sensor(
            f"{cell_id}_TC", "temperature", noise_stddev=n["temp_catholyte"]
        ),
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
                    self.sensors[cell.cell_id] = make_sensors(cell.cell_id)

        log.info(
            "Plant ready (%d cells, %d sensors)",
            len(self.cells),
            sum(len(v) for v in self.sensors.values()),
        )

        # bucket sensors by sampling interval
        self._sensor_buckets: Dict[int, List[Tuple[ElectrolyzerCell, str, Sensor]]] = (
            defaultdict(list)
        )
        for cell in self.cells.values():
            for mtype, sensor in self.sensors[cell.cell_id].items():
                dt = int(SAMPLE[mtype])
                self._sensor_buckets[dt].append((cell, mtype, sensor))

    # ---------------------------------------------------------------- loop
    async def _loop(self, dt: int, bucket: List[Tuple[ElectrolyzerCell, str, Sensor]]):
        """Stream all sensors in *bucket* every *dt* seconds."""
        end_wall = time.time() + self.run_seconds
        total_msgs = 0
        while time.time() < end_wall:
            tick_ts = datetime.utcnow().isoformat()
            for cell, mtype, sensor in bucket:
                # physics + measurement
                state = cell.step(dt=dt)
                true_val = {
                    "voltage": state["voltage"],
                    "current": state["current"],
                    "pressure": state["pressure_diff"],
                    "temp_anolyte": state["temperature"],
                    "temp_catholyte": state["temperature"],
                }[mtype]
                reading = sensor.read(true_val, dt=dt)

                payload = {
                    "ts": tick_ts,
                    "unit": int(cell.cell_id[1]),
                    "stack": cell.cell_id[4],
                    "cell": int(cell.cell_id[-2:]),
                    "sensor_id": reading["sensor_id"],
                    "status": reading["status"],
                    "quality": reading["quality"],
                }
                if mtype in ("voltage", "current"):
                    payload["voltage_V" if mtype == "voltage" else "current_A"] = (
                        reading["value"]
                    )
                elif "temp" in mtype:
                    payload["temperature_C"] = reading["value"]
                else:
                    payload["pressure_mbar"] = reading["value"]

                self.producer.send(TOPIC[mtype], payload, key=cell.cell_id)
                total_msgs += 1

            await asyncio.sleep(dt)

        log.info("Loop %ds finished (%,d msgs)", dt, total_msgs)

    # ---------------------------------------------------------------- API
    async def run(self):
        with task_progress("running plant simulation"):
            tasks = [
                asyncio.create_task(self._loop(dt, bucket))
                for dt, bucket in self._sensor_buckets.items()
            ]
            await asyncio.gather(*tasks)


# --- CLI Helper ---
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

    producer = SensorDataProducer(
        os.getenv("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092")
    )
    sim = PlantSimulator(producer, run_seconds=args.time)

    try:
        asyncio.run(sim.run())
    finally:
        producer.flush()
        producer.close()
        log.info("Simulation complete and Kafka producer closed.")


if __name__ == "__main__":
    main()
