"""Sensor simulation with noise and failures."""

from __future__ import annotations
from typing import Dict, Any
import numpy as np

from flux.utils.log import get_logger

log = get_logger(__name__)


class Sensor:
    """Simulates a real sensor with noise, drift and failures."""

    def __init__(
        self,
        sensor_id: str,
        sensor_type: str,
        noise_stddev: float = 0.01,
        drift_rate: float = 0.001,
        failure_prob: float = 1e-4,
    ):
        self.sensor_id = sensor_id
        self.sensor_type = sensor_type
        self.noise_stddev = noise_stddev
        self.drift_rate = drift_rate
        self.failure_prob = failure_prob
        self.hours_operated = 0.0
        self.is_failed = False
        log.debug("Sensor %s (%s) created", sensor_id, sensor_type)

    # ------------------------------------------------------------------ read
    def read(self, true_value: float, dt: float = 1.0) -> Dict[str, Any]:
        """Return a measurement dict with noise / drift / failure applied."""
        self.hours_operated += dt / 3600

        # random failure
        if (not self.is_failed) and np.random.random() < self.failure_prob:
            self.is_failed = True
            log.warning(
                "sensor %s failed after %.2f h", self.sensor_id, self.hours_operated
            )

        if self.is_failed:
            measured_value = None
        else:
            noise = np.random.normal(0, self.noise_stddev * true_value)
            drift = self.drift_rate * self.hours_operated * true_value / 100
            measured_value = true_value + noise + drift

        return {
            "sensor_id": self.sensor_id,
            "type": self.sensor_type,
            "value": measured_value,
            "status": "FAILED" if self.is_failed else "OK",
            "quality": 0 if self.is_failed else 100,
        }
