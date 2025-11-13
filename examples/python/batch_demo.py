#!/usr/bin/env python3
# this_file: examples/python/batch_demo.py

"""Batch Mode Demo: Process multiple font rendering jobs in parallel.

This example demonstrates how to use haforu's batch processing API to render
multiple glyphs efficiently. The batch mode processes jobs in parallel and
streams results as they complete.

Use case: Initial font analysis where you need to render thousands of glyphs
from hundreds of fonts as quickly as possible.
"""

import json
import sys
from pathlib import Path

import haforu


def main():
    """Run batch processing demo."""
    print("=== Haforu Batch Mode Demo ===\n")

    # Find a test font (assumes you're running from project root)
    # Adjust path as needed for your environment
    test_font = Path("testdata/fonts/Arial-Black.ttf")
    if not test_font.exists():
        print(f"Error: Test font not found at {test_font}")
        print("Please adjust the font path in this script.")
        sys.exit(1)

    # Create a batch job specification
    # In production, you would generate thousands of these jobs
    job_spec = {
        "version": "1.0",
        "jobs": [
            {
                "id": "Arial-Black_A_1000pt",
                "font": {
                    "path": str(test_font.absolute()),
                    "size": 1000,
                    "variations": {},  # For variable fonts, use {"wght": 600.0}
                    "face_index": 0
                },
                "text": {
                    "content": "A",
                    "script": "Latn",
                    "direction": "ltr",
                    "language": "en"
                },
                "rendering": {
                    "format": "pgm",
                    "encoding": "base64",
                    "width": 3000,
                    "height": 1200
                }
            },
            {
                "id": "Arial-Black_B_1000pt",
                "font": {
                    "path": str(test_font.absolute()),
                    "size": 1000,
                    "variations": {},
                    "face_index": 0
                },
                "text": {
                    "content": "B",
                    "script": "Latn",
                    "direction": "ltr",
                    "language": "en"
                },
                "rendering": {
                    "format": "pgm",
                    "encoding": "base64",
                    "width": 3000,
                    "height": 1200
                }
            },
            {
                "id": "Arial-Black_C_1000pt",
                "font": {
                    "path": str(test_font.absolute()),
                    "size": 1000,
                    "variations": {},
                    "face_index": 0
                },
                "text": {
                    "content": "C",
                    "script": "Latn",
                    "direction": "ltr",
                    "language": "en"
                },
                "rendering": {
                    "format": "pgm",
                    "encoding": "base64",
                    "width": 3000,
                    "height": 1200
                }
            }
        ]
    }

    # Convert to JSON string
    spec_json = json.dumps(job_spec)

    print(f"Processing {len(job_spec['jobs'])} jobs in parallel...")
    print()

    # Process jobs using haforu
    # The process_jobs() function returns an iterator that yields results
    # as they complete, allowing you to process results progressively
    results_count = 0
    for result_json in haforu.process_jobs(spec_json):
        # Each result is a JSONL string containing rendering result
        result = json.loads(result_json)

        results_count += 1
        job_id = result["id"]
        status = result["status"]

        print(f"Job {results_count}: {job_id}")
        print(f"  Status: {status}")

        if status == "success":
            # Rendering data is base64-encoded PGM image
            rendering = result["rendering"]
            print(f"  Format: {rendering['format']}")
            print(f"  Size: {rendering['width']}x{rendering['height']}")
            print(f"  Data size: {len(rendering['data'])} bytes (base64)")

            # Timing information
            timing = result["timing"]
            print(f"  Timing: shape={timing['shape_ms']:.2f}ms, "
                  f"render={timing['render_ms']:.2f}ms, "
                  f"total={timing['total_ms']:.2f}ms")

            # To decode the image:
            # import base64
            # pgm_bytes = base64.b64decode(rendering['data'])
            # You can then save to file or process further

        elif status == "error":
            print(f"  Error: {result.get('error', 'Unknown error')}")

        print()

    print(f"Batch processing complete: {results_count} jobs processed")

    # Performance notes:
    # - Jobs are processed in parallel using multiple CPU cores
    # - Font cache is shared across jobs (faster for repeated fonts)
    # - Results stream as they complete (no need to wait for all jobs)
    # - Expected throughput: 100-150 jobs/sec on modern hardware


if __name__ == "__main__":
    main()
