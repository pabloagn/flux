"""
Central config for the Flux POC simulator
"""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, Tuple


# 1. Electrochemical geometry
@dataclass(frozen=True)
class Geometry:
    units: Tuple[int, ...] = (1, 2, 3)  # plant has 3 units
    stacks: Tuple[str, ...] = ("A", "B", "C")  # three stacks /unit
    cells_per_stack: int = 20  # 20 cells /stack
    membrane_area_m2: float = 2.7  # surface area per cell


# 2. Sampling frequencies (Hz)
@dataclass(frozen=True)
class Sampling:
    electrical_hz: int = 1  # voltage & current
    pressure_hz: int = 1
    temperature_hz: float = 0.1  # every 10 s

    @property
    def intervals(self) -> Dict[str, float]:
        """Return sampling interval in seconds per measurement type."""
        return {
            "voltage": 1 / self.electrical_hz,
            "current": 1 / self.electrical_hz,
            "pressure": 1 / self.pressure_hz,
            "temp_anolyte": 1 / self.temperature_hz,
            "temp_catholyte": 1 / self.temperature_hz,
        }


# 3. Sensor noise & drift
@dataclass(frozen=True)
class SensorNoise:
    # σ values (fraction or absolute) taken from Appendix D
    voltage_frac: float = 0.005  # ±0.5 %
    current_frac: float = 0.003  # ±0.3 %
    pressure_abs: float = 2.0  # ±2 mbar
    temperature_abs: float = 0.5  # ±0.5 °C

    def stddev(self) -> Dict[str, float]:
        return {
            "voltage": self.voltage_frac,
            "current": self.current_frac,
            "pressure": self.pressure_abs,
            "temp_anolyte": self.temperature_abs,
            "temp_catholyte": self.temperature_abs,
        }


# 4. Kafka topic map
@dataclass(frozen=True)
class Topics:
    """All topic names (underscore: safe for GlassFlow)."""

    voltage: str = "flux_electrical_realtime"
    current: str = "flux_electrical_realtime"
    pressure: str = "flux_process_pressures"
    temp_anolyte: str = "flux_process_temperatures"
    temp_catholyte: str = "flux_process_temperatures"

    def as_dict(self) -> Dict[str, str]:
        return vars(self)


# 5. Top-level aggregate
@dataclass(frozen=True)
class SimulationConfig:
    geometry: Geometry = field(default_factory=Geometry)
    sampling: Sampling = field(default_factory=Sampling)
    noise: SensorNoise = field(default_factory=SensorNoise)
    topics: Topics = field(default_factory=Topics)
    default_run_s: int = 60  # CLI default duration


# single shared instance
CFG = SimulationConfig()
