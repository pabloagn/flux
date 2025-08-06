"""Single electrolyzer cell model."""

import numpy as np
from pydantic import BaseModel, Field
from datetime import datetime


class CellState(BaseModel):
    """Current state of an electrolyzer cell."""
    
    voltage: float = Field(gt=0, description="Cell voltage (V)")
    current: float = Field(ge=0, description="Cell current (A)")
    temperature: float = Field(gt=0, description="Temperature (°C)")
    pressure_diff: float = Field(ge=0, description="Pressure differential (mbar)")
    current_efficiency: float = Field(ge=0, le=100, description="Current efficiency (%)")
    

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
        self.operating_hours = 0.0
        
        # Initial state
        self.state = CellState(
            voltage=3.0,
            current=5000.0,
            temperature=85.0,
            pressure_diff=150.0,
            current_efficiency=94.0
        )
    
    def calculate_voltage(self) -> float:
        """Calculate cell voltage based on current density and age."""
        j = self.state.current / self.area / 1000  # Current density in kA/m²
        
        # Base voltage + ohmic losses + aging effect
        v_base = 2.19  # Theoretical decomposition voltage
        v_ohmic = 0.2 * j  # Simplified ohmic losses
        v_aging = 0.00003 * np.sqrt(self.operating_hours)  # Membrane degradation
        
        return v_base + v_ohmic + v_aging
    
    def step(self, dt: float = 1.0) -> dict:
        """
        Simulate one time step.
        
        Args:
            dt: Time step in seconds
        
        Returns:
            Dict with cell state
        """
        # Update operating hours
        self.operating_hours += dt / 3600
        
        # Update voltage based on model
        self.state.voltage = self.calculate_voltage()
        
        # Add small random variations (process noise)
        self.state.temperature += np.random.normal(0, 0.1)
        self.state.pressure_diff += np.random.normal(0, 2)
        
        # Current efficiency decreases slightly with age
        self.state.current_efficiency = 96 - 0.001 * self.operating_hours
        
        return {
            "cell_id": self.cell_id,
            "timestamp": datetime.now().isoformat(),
            "operating_hours": self.operating_hours,
            **self.state.model_dump()
        }
