#!/usr/bin/env python3
# this_file: examples/python/numpy_demo.py

"""Zero-Copy Numpy Demo: Direct rendering to numpy arrays.

This example demonstrates haforu's render_to_numpy() method, which provides
zero-copy access to rendered font images as numpy arrays. This is the fastest
way to get pixel data for analysis or image processing.

Use case: Font analysis pipelines that need direct pixel access for computing
metrics, performing image analysis, or feeding into machine learning models.
"""

import sys
from pathlib import Path

import numpy as np
import haforu


def analyze_glyph_image(image: np.ndarray, glyph: str) -> dict:
    """Analyze a rendered glyph image and extract basic metrics.

    Args:
        image: 2D numpy array of shape (height, width), dtype uint8
        glyph: The character that was rendered

    Returns:
        Dictionary of computed metrics
    """
    # Calculate coverage (percentage of non-zero pixels)
    coverage = np.count_nonzero(image) / image.size * 100

    # Find bounding box (tight bounds around non-zero pixels)
    rows = np.any(image, axis=1)
    cols = np.any(image, axis=0)
    if rows.any() and cols.any():
        y_min, y_max = np.where(rows)[0][[0, -1]]
        x_min, x_max = np.where(cols)[0][[0, -1]]
        bbox_width = x_max - x_min + 1
        bbox_height = y_max - y_min + 1
    else:
        bbox_width = bbox_height = 0

    # Calculate mean intensity of non-zero pixels
    non_zero_pixels = image[image > 0]
    mean_intensity = non_zero_pixels.mean() if len(non_zero_pixels) > 0 else 0

    return {
        "glyph": glyph,
        "coverage_percent": coverage,
        "bbox_width": bbox_width,
        "bbox_height": bbox_height,
        "mean_intensity": mean_intensity,
        "max_intensity": image.max(),
    }


def main():
    """Run numpy zero-copy rendering demo."""
    print("=== Haforu Zero-Copy Numpy Demo ===\n")

    # Find a test font
    test_font = Path("testdata/fonts/Arial-Black.ttf")
    if not test_font.exists():
        print(f"Error: Test font not found at {test_font}")
        print("Please adjust the font path in this script.")
        sys.exit(1)

    # Create a streaming session
    with haforu.StreamingSession() as session:
        print("Rendering glyphs directly to numpy arrays...\n")

        # Render multiple glyphs
        glyphs = ["A", "B", "C", "a", "b", "c"]

        for glyph in glyphs:
            # Use render_to_numpy() for zero-copy access
            # This is much faster than render() + base64 decoding
            image = session.render_to_numpy(
                font_path=str(test_font.absolute()),
                text=glyph,
                size=1000.0,
                width=3000,
                height=1200,
                variations={},  # For variable fonts: {"wght": 600.0}
                script="Latn",
                direction="ltr",
                language="en"
            )

            # Verify array properties
            print(f"Glyph '{glyph}':")
            print(f"  Shape: {image.shape} (height, width)")
            print(f"  Dtype: {image.dtype}")
            print(f"  Contiguous: {image.flags.c_contiguous}")

            # Analyze the image
            metrics = analyze_glyph_image(image, glyph)
            print(f"  Coverage: {metrics['coverage_percent']:.2f}%")
            print(f"  Bounding box: {metrics['bbox_width']}×{metrics['bbox_height']} pixels")
            print(f"  Mean intensity: {metrics['mean_intensity']:.1f}")
            print(f"  Max intensity: {metrics['max_intensity']}")
            print()

            # Example: Save as PNG (requires pillow)
            # from PIL import Image
            # pil_image = Image.fromarray(image, mode='L')
            # pil_image.save(f"glyph_{glyph}.png")

        # Demonstrate variable font rendering
        print("\nVariable font example (if font supports variable axes):")
        print("Note: Arial-Black is not variable, so variations will be ignored\n")

        # Render with different weights (only works with variable fonts)
        for weight in [400, 600, 800]:
            image = session.render_to_numpy(
                font_path=str(test_font.absolute()),
                text="W",
                size=1000.0,
                width=3000,
                height=1200,
                variations={"wght": float(weight)},
            )

            metrics = analyze_glyph_image(image, "W")
            print(f"Weight {weight}:")
            print(f"  Coverage: {metrics['coverage_percent']:.2f}%")
            print(f"  Bbox: {metrics['bbox_width']}×{metrics['bbox_height']}px")
            print()

    print("Demo complete!")

    # Performance notes:
    # - render_to_numpy() is 2-3× faster than render() + base64 decode
    # - No intermediate copies: Rust → Python with zero overhead
    # - Perfect for image analysis, metric computation, ML pipelines
    # - Arrays are C-contiguous for maximum compatibility


if __name__ == "__main__":
    main()
