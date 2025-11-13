#!/usr/bin/env python3
# this_file: examples/python/error_handling_demo.py

"""Error Handling Demo: Robust error handling patterns.

This example demonstrates how to handle various error conditions when using
haforu's Python bindings, including missing fonts, invalid JSON, and rendering
failures.

Use case: Production systems that need graceful degradation and helpful error
messages for debugging and monitoring.
"""

import json
import sys
from pathlib import Path

import haforu


def demo_batch_errors():
    """Demonstrate error handling in batch mode."""
    print("=== Batch Mode Error Handling ===\n")

    # Error 1: Invalid JSON
    print("1. Testing invalid JSON...")
    try:
        results = list(haforu.process_jobs("not valid json"))
        print("  ERROR: Should have raised ValueError!")
    except ValueError as e:
        print(f"  ✓ Caught ValueError: {e}")
    print()

    # Error 2: Unsupported version
    print("2. Testing unsupported version...")
    try:
        spec = {"version": "2.0", "jobs": [{"id": "test"}]}
        results = list(haforu.process_jobs(json.dumps(spec)))
        print("  ERROR: Should have raised ValueError!")
    except ValueError as e:
        print(f"  ✓ Caught ValueError: {e}")
    print()

    # Error 3: Empty job list
    print("3. Testing empty job list...")
    try:
        spec = {"version": "1.0", "jobs": []}
        results = list(haforu.process_jobs(json.dumps(spec)))
        print("  ERROR: Should have raised ValueError!")
    except ValueError as e:
        print(f"  ✓ Caught ValueError: {e}")
    print()

    # Error 4: Missing font file (returned in result, not exception)
    print("4. Testing missing font file...")
    spec = {
        "version": "1.0",
        "jobs": [{
            "id": "missing_font_test",
            "font": {
                "path": "/nonexistent/font.ttf",
                "size": 1000,
                "variations": {}
            },
            "text": {"content": "A", "script": "Latn"},
            "rendering": {
                "format": "pgm",
                "encoding": "base64",
                "width": 3000,
                "height": 1200
            }
        }]
    }

    try:
        for result_json in haforu.process_jobs(json.dumps(spec)):
            result = json.loads(result_json)
            if result["status"] == "error":
                print(f"  ✓ Job returned error status")
                print(f"  Error message: {result.get('error', 'Unknown')}")
            else:
                print(f"  ERROR: Expected error status, got: {result['status']}")
    except Exception as e:
        print(f"  ERROR: Unexpected exception: {e}")
    print()


def demo_streaming_errors():
    """Demonstrate error handling in streaming mode."""
    print("=== Streaming Mode Error Handling ===\n")

    # Create a session for testing
    session = haforu.StreamingSession()

    # Error 1: Invalid JSON in render()
    print("1. Testing invalid JSON in render()...")
    try:
        result = session.render("not valid json")
        print("  ERROR: Should have raised ValueError!")
    except ValueError as e:
        print(f"  ✓ Caught ValueError: {e}")
    print()

    # Error 2: Missing font in render()
    print("2. Testing missing font in render()...")
    job = {
        "id": "missing_font",
        "font": {"path": "/nonexistent/font.ttf", "size": 1000, "variations": {}},
        "text": {"content": "A", "script": "Latn"},
        "rendering": {"format": "pgm", "encoding": "base64", "width": 100, "height": 100}
    }
    try:
        result_json = session.render(json.dumps(job))
        result = json.loads(result_json)
        if result["status"] == "error":
            print(f"  ✓ Job returned error status")
            print(f"  Error message: {result.get('error', 'Unknown')}")
    except Exception as e:
        print(f"  ERROR: Unexpected exception: {e}")
    print()

    # Error 3: Invalid parameters in render_to_numpy()
    print("3. Testing invalid parameters in render_to_numpy()...")
    try:
        image = session.render_to_numpy(
            font_path="",  # Empty path
            text="A",
            size=1000.0,
            width=100,
            height=100
        )
        print("  ERROR: Should have raised RuntimeError!")
    except RuntimeError as e:
        print(f"  ✓ Caught RuntimeError: {e}")
    print()

    # Error 4: Missing font in render_to_numpy()
    print("4. Testing missing font in render_to_numpy()...")
    try:
        image = session.render_to_numpy(
            font_path="/nonexistent/font.ttf",
            text="A",
            size=1000.0,
            width=100,
            height=100
        )
        print("  ERROR: Should have raised RuntimeError!")
    except RuntimeError as e:
        print(f"  ✓ Caught RuntimeError: {e}")
    print()

    session.close()


def demo_graceful_degradation():
    """Demonstrate graceful degradation pattern."""
    print("=== Graceful Degradation Pattern ===\n")

    # In production, you might want to try multiple fonts as fallbacks
    fonts_to_try = [
        "/nonexistent/preferred.ttf",
        "/nonexistent/fallback1.ttf",
        "testdata/fonts/Arial-Black.ttf",  # This one should exist
    ]

    session = haforu.StreamingSession()

    print("Attempting to render with fallback fonts...")
    for font_path in fonts_to_try:
        print(f"  Trying: {font_path}")
        job = {
            "id": "fallback_test",
            "font": {"path": font_path, "size": 1000, "variations": {}},
            "text": {"content": "A", "script": "Latn"},
            "rendering": {"format": "pgm", "encoding": "base64", "width": 100, "height": 100}
        }

        try:
            result_json = session.render(json.dumps(job))
            result = json.loads(result_json)

            if result["status"] == "success":
                print(f"  ✓ Success with {font_path}")
                break
            else:
                print(f"  ✗ Failed: {result.get('error', 'Unknown')}")

        except Exception as e:
            print(f"  ✗ Exception: {e}")

    session.close()
    print()


def main():
    """Run all error handling demos."""
    print("=== Haforu Error Handling Demo ===\n")

    try:
        demo_batch_errors()
        demo_streaming_errors()
        demo_graceful_degradation()

        print("=== Demo Complete ===")
        print("\nKey takeaways:")
        print("- ValueError: Invalid JSON, version, or empty job list")
        print("- RuntimeError: Font loading, shaping, or rendering failures")
        print("- Batch mode: Errors returned in result status, not exceptions")
        print("- Streaming mode: render() returns error status, render_to_numpy() raises")
        print("- Always check result['status'] in batch/streaming modes")
        print("- Use try/except for render_to_numpy() and input validation")

    except ImportError as e:
        print(f"ERROR: Failed to import haforu: {e}")
        print("\nMake sure haforu is installed:")
        print("  cd haforu && maturin develop --features python")
        sys.exit(1)


if __name__ == "__main__":
    main()
