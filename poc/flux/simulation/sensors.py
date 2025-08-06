"""Sensor simulation with noise and failures."""

import numpy as np
from typing import Dict, Any


class Sensor:
    """Simulates a real sensor with noise, drift, and failures."""

    def __init__(
        self,
        sensor_id: str,
        sensor_type: str,
        noise_stddev: float = 0.01,
        drift_rate: float = 0.001,
        failure_prob: float = 0.0001,
    ):
        """
        Initialize sensor.

        Args:
            sensor_id: Unique sensor identifier
            sensor_type: Type of sensor (voltage, temperature, etc.)
            noise_stddev: Standard deviation of measurement noise (% of value)
            drift_rate: Drift per hour (% of value)
            failure_prob: Probability of failure per reading
        """
        self.sensor_id = sensor_id
        self.sensor_type = sensor_type
        self.noise_stddev = noise_stddev
        self.drift_rate = drift_rate
        self.failure_prob = failure_prob
        self.hours_operated = 0.0
        self.is_failed = False

    def read(self, true_value: float, dt: float = 1.0) -> Dict[str, Any]:
        """
        Read sensor value with noise and drift.

        Args:
            true_value: True process value
            dt: Time step in seconds

        Returns:
            Sensor reading dict
        """
        self.hours_operated += dt / 3600

        # Check for random failure
        if np.random.random() < self.failure_prob:
            self.is_failed = True

        if self.is_failed:
            # Failed sensor returns null
            measured_value = None
        else:
            # Add noise and drift
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
