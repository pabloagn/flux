#!/usr/bin/env python
"""Entry point for plant simulation."""

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))

from flux.simulation.plant import main

if __name__ == "__main__":
    main()
