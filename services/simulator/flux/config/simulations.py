"""
Flux POC – global simulation config.
"""

from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Tuple
import json
import os


@dataclass(frozen=True)
class Geometry:
    units: Tuple[int, ...]
    stacks: Tuple[str, ...]
    cells_per_stack: int
    membrane_area_m2: float


@dataclass(frozen=True)
class Sampling:
    electrical_hz: int
    pressure_hz: int
    temperature_hz: float

    @property
    def intervals(self) -> Dict[str, float]:
        return {
            "voltage": 1 / self.electrical_hz,
            "current": 1 / self.electrical_hz,
            "pressure": 1 / self.pressure_hz,
            "temp_anolyte": 1 / self.temperature_hz,
            "temp_catholyte": 1 / self.temperature_hz,
        }


@dataclass(frozen=True)
class SensorNoise:
    voltage_frac: float
    current_frac: float
    pressure_abs: float
    temperature_abs: float

    def stddev(self) -> Dict[str, float]:
        return {
            "voltage": self.voltage_frac,
            "current": self.current_frac,
            "pressure": self.pressure_abs,
            "temp_anolyte": self.temperature_abs,
            "temp_catholyte": self.temperature_abs,
        }


@dataclass(frozen=True)
class Topics:
    voltage: str
    current: str
    pressure: str
    temp_anolyte: str
    temp_catholyte: str

    def as_dict(self) -> Dict[str, str]:
        return vars(self)


@dataclass(frozen=True)
class SimulationConfig:
    geometry: Geometry
    sampling: Sampling
    noise: SensorNoise
    topics: Topics
    default_run_s: int


# --- Load Json ---
def _layout_path() -> Path:
    """Return the absolute path to the plant-layout JSON or raise FileNotFoundError."""
    # 1. explicit env-var
    env = os.getenv("FLUX_PLANT_LAYOUT")
    if env is not None and env.strip():
        p = Path(env).expanduser()
    else:
        # 2. repo-relative default
        p = (
            Path(__file__).resolve().parents[3]
            / "config"
            / "plant"
            / "plant.json"
        )

    if not p.is_file():
        raise FileNotFoundError(f"Plant layout file not found: {p}")
    return p


def _load_cfg() -> SimulationConfig:
    with _layout_path().open() as fp:
        raw = json.load(fp)

    try:
        geo_raw = raw["geometry"]
        topics_raw = raw["topics"]
        sample_raw = raw["sampling"]
        noise_raw = raw["noise"]
        defaults = raw.get("default_run_s", 60)  # sensible fallback for CLI

    except KeyError as miss:
        raise KeyError(f"Missing key in plant layout JSON: {miss}") from None

    geometry = Geometry(
        units=tuple(geo_raw["units"]),
        stacks=tuple(geo_raw["stacks"]),
        cells_per_stack=geo_raw["cells_per_stack"],
        membrane_area_m2=geo_raw["membrane_area_m2"],
    )

    sampling = Sampling(**sample_raw)
    noise = SensorNoise(**noise_raw)
    topics = Topics(**topics_raw)

    return SimulationConfig(
        geometry=geometry,
        sampling=sampling,
        noise=noise,
        topics=topics,
        default_run_s=defaults,
    )


# --- Public Singleton ---
CFG: SimulationConfig = _load_cfg()
