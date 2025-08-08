"""Single electrolyzer cell model with Rich logging."""

from __future__ import annotations
from datetime import datetime
from pydantic import BaseModel, Field
import numpy as np

from flux.utils.log import get_logger

log = get_logger(__name__)


class CellState(BaseModel):
    """Current state of an electrolyzer cell."""

    voltage: float = Field(gt=0, description="Cell voltage (V)")
    current: float = Field(ge=0, description="Cell current (A)")
    temperature: float = Field(gt=0, description="Temperature (°C)")
    pressure_diff: float = Field(ge=0, description="Pressure differential (mbar)")
    current_efficiency: float = Field(
        ge=0, le=100, description="Current efficiency (%)"
    )


class ElectrolyzerCell:
    """Single chlor-alkali electrolyzer cell."""

    def __init__(self, cell_id: str, area: float = 2.7):
        self.cell_id = cell_id
        self.area = area
        self.operating_hours = 0.0

        # initial state
        self.state = CellState(
            voltage=3.0,
            current=5000.0,
            temperature=85.0,
            pressure_diff=150.0,
            current_efficiency=94.0,
        )
        log.debug("Cell %s initialised (area %.2f m²)", self.cell_id, self.area)

    # --------------------------------------------------------------------- calc
    def calculate_voltage(self) -> float:
        """Cell voltage = E0 + ohmic + ageing."""
        j = self.state.current / self.area / 1_000  # kA · m⁻²
        v_base = 2.19
        v_ohmic = 0.2 * j
        v_aging = 0.00003 * np.sqrt(self.operating_hours)
        return v_base + v_ohmic + v_aging

    # -------------------------------------------------------------------- tick
    def step(self, dt: float = 1.0) -> dict:
        """Advance cell state by *dt* seconds and return a dict payload."""
        # accumulate operating time
        self.operating_hours += dt / 3600

        # physics
        self.state.voltage = self.calculate_voltage()
        self.state.temperature += np.random.normal(0, 0.1)
        self.state.pressure_diff += np.random.normal(0, 2)
        self.state.current_efficiency = 96 - 0.001 * self.operating_hours

        return {
            "cell_id": self.cell_id,
            "timestamp": datetime.utcnow().isoformat(),
            "operating_hours": self.operating_hours,
            **self.state.model_dump(),
        }
