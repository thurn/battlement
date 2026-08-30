#!/usr/bin/env python3

"""Forward the frozen pre-cutover policy to the active Ditto CI runner."""

from pathlib import Path
import runpy


runpy.run_path(Path(__file__).with_name("ditto_ci.py"), run_name="__main__")
