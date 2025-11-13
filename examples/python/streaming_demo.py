#!/usr/bin/env python3
# this_file: examples/python/streaming_demo.py

"""Streaming Mode Demo: Persistent session for multiple renders.

This example demonstrates how to use haforu's StreamingSession for efficient
repeated rendering. The session maintains a persistent font cache, avoiding
repeated font loading overhead.

Use case: Deep matching optimization where you need to render the same fonts
repeatedly with different parameters (e.g., during SLSQP optimization).
"""

import json
import sys
from pathlib import Path

import haforu


def main():
    """Run streaming session demo."""
    print("=== Haforu Streaming Mode Demo ===\n")

    # Find a test font
    test_font = Path("testdata/fonts/Arial-Black.ttf")
    if not test_font.exists():
        print(f"Error: Test font not found at {test_font}")
        print("Please adjust the font path in this script.")
        sys.exit(1)

    # Create a streaming session with custom cache size
    # The session keeps fonts in memory for fast repeated access
    # Default cache size is 512 fonts
    print("Creating streaming session with cache_size=128...")
    with haforu.StreamingSession(cache_size=128) as session:
        print("Session created successfully\n")

        # Render multiple jobs using the same session
        # Each render reuses the cached font instance
        glyphs = ["A", "B", "C", "D", "E"]

        print(f"Rendering {len(glyphs)} glyphs sequentially...")
        print()

        for glyph in glyphs:
            # Create a job specification for this glyph
            job = {
                "id": f"Arial-Black_{glyph}_1000pt",
                "font": {
                    "path": str(test_font.absolute()),
                    "size": 1000,
                    "variations": {},
                    "face_index": 0
                },
                "text": {
                    "content": glyph,
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

            # Render using the session
            # Note: render() takes a JSON string and returns a JSON string
            result_json = session.render(json.dumps(job))
            result = json.loads(result_json)

            job_id = result["id"]
            status = result["status"]
            timing = result["timing"]

            print(f"Glyph '{glyph}':")
            print(f"  Job ID: {job_id}")
            print(f"  Status: {status}")
            print(f"  Timing: shape={timing['shape_ms']:.2f}ms, "
                  f"render={timing['render_ms']:.2f}ms, "
                  f"total={timing['total_ms']:.2f}ms")

            if status == "success":
                rendering = result["rendering"]
                print(f"  Image: {rendering['width']}x{rendering['height']} pixels")

            print()

        # Demonstrate cache benefits by rendering the same glyph again
        print("Rendering 'A' again to demonstrate cache benefit...")
        job_a_again = {
            "id": "Arial-Black_A_cached",
            "font": {
                "path": str(test_font.absolute()),
                "size": 1000,
                "variations": {},
                "face_index": 0
            },
            "text": {"content": "A", "script": "Latn"},
            "rendering": {
                "format": "pgm",
                "encoding": "base64",
                "width": 3000,
                "height": 1200
            }
        }

        result_json = session.render(json.dumps(job_a_again))
        result = json.loads(result_json)
        timing = result["timing"]

        print(f"Cached render timing: {timing['total_ms']:.2f}ms")
        print("(Should be faster than the first render due to font caching)")
        print()

        # Session automatically closes when exiting the 'with' block
        # You can also manually call session.close() if needed

    print("Session closed. Font cache released.")

    # Performance notes:
    # - First render of a font loads it into cache (~10-20ms overhead)
    # - Subsequent renders reuse cached font (~1-2ms per render)
    # - Perfect for optimization loops that need repeated renders
    # - Thread-safe: can be used from multiple threads concurrently


if __name__ == "__main__":
    main()
