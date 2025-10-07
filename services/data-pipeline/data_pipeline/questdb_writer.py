from __future__ import annotations

import asyncio
from datetime import datetime
from typing import List, Optional

import structlog
from pydantic import BaseModel, Field
from psycopg_pool import AsyncConnectionPool
from tenacity import retry, stop_after_attempt, wait_exponential

logger = structlog.get_logger(__name__)


class CellMetric(BaseModel):
    ts: datetime
    unit_id: int = Field(ge=1, le=3)
    stack_id: str = Field(pattern=r"^[A-C]$")
    cell_id: int = Field(ge=1, le=20)
    voltage_v: Optional[float] = None
    current_a: Optional[float] = None
    temperature_c: Optional[float] = None
    pressure_mbar: Optional[float] = None
    efficiency_pct: Optional[float] = None
    power_kw: Optional[float] = None
    specific_energy: Optional[float] = None
    membrane_resistance: Optional[float] = None
    sensor_quality: float = Field(default=1.0, ge=0.0, le=1.0)


class QuestDBWriter:
    def __init__(
        self,
        host: str,
        port: int,
        database: str,
        user: str,
        password: str,
        pool_size: int = 10,
    ):
        conninfo = (
            f"host={host} port={port} dbname={database} "
            f"user={user} password={password} "
            "sslmode=disable options='-c statement_timeout=10000'"
        )
        self.pool = AsyncConnectionPool(
            conninfo,
            min_size=2,
            max_size=pool_size,
            timeout=30,
            max_lifetime=600,
            max_idle=300,
            open=False,
        )

    async def open(self):
        await self.pool.open()
        logger.info("QuestDB connection pool opened", pool_size=self.pool.max_size)

    async def close(self):
        await self.pool.close()
        logger.info("QuestDB connection pool closed.")

    @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=1, max=10))
    async def write_batch(self, metrics: List[CellMetric]):
        if not metrics:
            return

        async with self.pool.connection() as conn:
            async with conn.cursor() as cur:
                rows = [
                    (
                        m.ts,
                        m.unit_id,
                        m.stack_id,
                        m.cell_id,
                        m.voltage_v,
                        m.current_a,
                        m.temperature_c,
                        m.pressure_mbar,
                        m.efficiency_pct,
                        m.power_kw,
                        m.specific_energy,
                        m.membrane_resistance,
                        m.sensor_quality,
                    )
                    for m in metrics
                ]

                async with cur.copy(
                    """COPY cell_metrics (
                        ts, unit_id, stack_id, cell_id, voltage_v, current_a,
                        temperature_c, pressure_mbar, efficiency_pct, power_kw,
                        specific_energy, membrane_resistance, sensor_quality
                    ) FROM STDIN"""
                ) as copy:
                    await copy.write_rows(rows)
