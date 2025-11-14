#!/usr/bin/env python3
# this_file: examples/python/metrics_demo.py

"""Metrics Mode Demo: Compute density/beam without image payloads.

This example shows how to request metrics-only results from haforu by
setting ``rendering.format`` to ``"metrics"``. Results omit the base64
image blob and instead include a ``metrics`` object with normalized
density/beam values in the JSON payload.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import haforu


def main() -> None:
    """Render a glyph in metrics mode and print the results."""
    font_path = Path("testdata/fonts/Arial-Black.ttf")
    if not font_path.exists():
        print(f"Test font not found at {font_path}", file=sys.stderr)
        sys.exit(1)

    job_spec = {
        "version": "1.0",
        "jobs": [
            {
                "id": "metrics-demo",
                "font": {
                    "path": str(font_path.absolute()),
                    "size": 256,
                    "variations": {},
                },
                "text": {"content": "H"},
                "rendering": {
                    "format": "metrics",
                    "encoding": "json",
                    "width": 96,
                    "height": 96,
                },
            }
        ],
    }

    print("Requesting metrics-only render...")
    result_json = next(iter(haforu.process_jobs(json.dumps(job_spec))))
    payload = json.loads(result_json)

    metrics = payload.get("metrics") or {}
    density = metrics.get("density", 0.0)
    beam = metrics.get("beam", 0.0)

    print(f"Job ID: {payload['id']}")
    print("Status:", payload["status"])
    print("Metrics:")
    print(f"  Density: {density:.4f}")
    print(f"  Beam:    {beam:.4f}")
    print()
    print("Rendering data present?", "rendering" in payload)


if __name__ == "__main__":
    main()
