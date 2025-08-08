"""Single electrolyzer cell model."""

import numpy as np
from pydantic import BaseModel, Field


class CellState(BaseModel):
    """Current state of an electrolyzer cell."""

    voltage: float = Field(gt=0, description="Cell voltage (V)")
    current: float = Field(ge=0, description="Cell current (A)")
    temperature: float = Field(gt=0, description="Temperature (°C)")
    pressure_diff: float = Field(ge=0, description="Pressure differential (mbar)")


class ElectrolyzerCell:
    """Single chlor-alkali electrolyzer cell."""

    def __init__(self, cell_id: str, area: float = 2.7):
        """
        Initialize cell.

        Args:
            cell_id: Unique cell identifier
            area: Membrane area (m²)
        """
        self.cell_id = cell_id
        self.area = area
        self.state = CellState(
            voltage=3.0, current=5000.0, temperature=85.0, pressure_diff=150.0
        )

    def step(self, dt: float = 1.0) -> dict:
        """
        Simulate one time step.

        Returns:
            Dict with cell state
        """
        # For now, just return current state
        return {
            "cell_id": self.cell_id,
            "timestamp": np.datetime64("now"),
            **self.state.model_dump(),
        }
