"""
flux.utils.log
==============

Rich-powered logging helper for the whole Flux code-base.

Usage
-----
from flux.utils.log import get_logger, console, task_progress

log = get_logger(__name__)
log.info("Hello!")

with task_progress("heavy-duty loop"):
    ...                 # expensive work
"""

from __future__ import annotations
import logging
import os
from contextlib import contextmanager
from datetime import datetime
from typing import Iterator

from rich.console import Console
from rich.logging import RichHandler
from rich.progress import Progress, SpinnerColumn, BarColumn, TimeElapsedColumn

# ────────────────────────── global Console ──────────────────────────
console = Console(theme=None, stderr=False)

# ────────────────────────── log configuration ───────────────────────
_LOG_LEVEL = os.getenv("FLUX_LOG_LEVEL", "INFO").upper()
_LOG_FORMAT = "%(message)s"

_rich_handler = RichHandler(
    console=console,
    show_time=True,
    show_level=True,
    markup=True,
    rich_tracebacks=True,
)

logging.basicConfig(
    level=_LOG_LEVEL,
    format=_LOG_FORMAT,
    datefmt="[%X]",
    handlers=[_rich_handler],
)


def get_logger(name: str | None = None) -> logging.Logger:
    """
    Return a module-level logger already wired to Rich.
    Call once per file:

        log = get_logger(__name__)
    """
    return logging.getLogger(name)


# ───────────────────── context helpers (optional) ───────────────────
@contextmanager
def task_progress(description: str) -> Iterator[None]:
    """
    Context manager that shows a neat Rich progress spinner while the
    wrapped block is running.

        with task_progress("streaming sensors"):
            run_my_loop()
    """
    with Progress(
        SpinnerColumn(),
        "[progress.description]{task.description}",
        BarColumn(bar_width=None),
        TimeElapsedColumn(),
        console=console,
        transient=True,
    ) as progress:
        task_id = progress.add_task(description, total=None)
        try:
            yield
        finally:
            progress.remove_task(task_id)


def banner(msg: str) -> None:
    """
    Pretty banner for CLI scripts.
    """
    console.rule(f"[bold cyan]{msg}[/]  {datetime.utcnow():%Y-%m-%d %H:%M:%S} UTC")
