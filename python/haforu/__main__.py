#!/usr/bin/env python3
# this_file: python/haforu/__main__.py

"""Haforu command-line interface using Fire.

This provides a Python CLI that mirrors the Rust CLI functionality,
allowing users to render fonts via `python -m haforu`.
"""

from __future__ import annotations

import json
import os
import sys
import base64
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

try:
    import fire
except ImportError:
    print("Error: 'fire' package is required for the CLI.", file=sys.stderr)
    print("Install it with: pip install fire", file=sys.stderr)
    sys.exit(1)

try:
    import haforu
except ImportError:
    print("Error: haforu module not found.", file=sys.stderr)
    print("Install it with: pip install haforu", file=sys.stderr)
    sys.exit(1)


class HaforuCLI:
    """Haforu: High-performance batch font renderer.

    A Python CLI interface for the haforu font rendering library.
    """

    def __init__(self, verbose: bool = False):
        """Initialize the CLI.

        Args:
            verbose: Enable verbose output
        """
        self.verbose = verbose

    def batch(
        self,
        input: Optional[str] = None,
        max_fonts: int = 512,
        max_glyphs: int = 2048,
        timeout_ms: int = 0,
        base_dir: Optional[str] = None,
        output: Optional[str] = None,
        format: str = "jsonl",
    ) -> None:
        """Process a batch of rendering jobs.

        Args:
            input: Input JSON file (reads from stdin if not provided)
            max_fonts: Maximum number of fonts to cache
            max_glyphs: Maximum number of glyphs to cache
            timeout_ms: Per-job timeout (0 disables)
            base_dir: Restrict font paths to this directory
            output: Output file (writes to stdout if not provided)
            format: Output format (jsonl, json, or human)
        """
        job_spec = self._load_json(input)

        try:
            iterator = haforu.process_jobs(
                json.dumps(job_spec),
                max_fonts=max_fonts,
                max_glyphs=max_glyphs,
                timeout_ms=timeout_ms if timeout_ms > 0 else None,
                base_dir=base_dir,
            )
            results = list(iterator)
        except Exception as exc:  # pragma: no cover - surfaces native errors
            print(f"Error processing jobs: {exc}", file=sys.stderr)
            sys.exit(1)

        self._output_results(results, output, format)

    def stream(
        self,
        input: Optional[str] = None,
        max_fonts: int = 512,
        max_glyphs: int = 2048,
        output: Optional[str] = None,
        format: str = "jsonl",
    ) -> None:
        """Process jobs in streaming mode (JSONL input).

        Args:
            input: Input JSONL file (reads from stdin if not provided)
            max_fonts: Maximum number of fonts to cache
            max_glyphs: Maximum number of glyphs to cache
            output: Output file (writes to stdout if not provided)
            format: Output format (jsonl, json, or human)
        """
        # Create streaming session
        session = haforu.StreamingSession(max_fonts=max_fonts, max_glyphs=max_glyphs)
        session.warm_up()

        if self.verbose:
            print(f"Streaming session initialized", file=sys.stderr)
            print(f"Cache stats: {session.cache_stats()}", file=sys.stderr)

        # Open input
        if input:
            input_file = open(input, "r")
        else:
            if self.verbose:
                print("Reading from stdin...", file=sys.stderr)
            input_file = sys.stdin

        # Open output
        if output:
            output_file = open(output, "w")
        else:
            output_file = sys.stdout

        # Process lines
        results = []
        try:
            for line in input_file:
                line = line.strip()
                if not line:
                    continue

                try:
                    job = json.loads(line)
                    result = session.render(json.dumps(job))
                    result_obj = json.loads(result)

                    if format == "jsonl":
                        print(result, file=output_file, flush=True)
                    else:
                        results.append(result_obj)

                except json.JSONDecodeError as e:
                    error_result = {
                        "id": "unknown",
                        "status": "error",
                        "error": f"Invalid JSON: {e}",
                    }
                    if format == "jsonl":
                        print(json.dumps(error_result), file=output_file, flush=True)
                    else:
                        results.append(error_result)

        finally:
            session.close()
            if input:
                input_file.close()
            if output:
                output_file.close()

        # Output collected results for non-jsonl formats
        if format != "jsonl":
            self._output_results([json.dumps(r) for r in results], output, format)

    def render(
        self,
        text: str,
        font: str,
        size: int = 72,
        width: int = 800,
        height: int = 200,
        variations: Optional[str] = None,
        format: str = "pgm",
        output: Optional[str] = None,
        script: Optional[str] = None,
        language: Optional[str] = None,
        direction: str = "ltr",
        features: Optional[str] = None,
    ) -> None:
        """Render a single text string (convenience command).

        Args:
            text: Text to render
            font: Path to font file
            size: Font size in points
            width: Canvas width in pixels
            height: Canvas height in pixels
            variations: Font variations (JSON or "wght=700,wdth=80")
            format: Output format (pgm, png, or metrics)
            output: Output file for the image (stdout if not provided)
            script: Script hint (e.g., "Latn")
            language: Language tag (e.g., "en")
            direction: Text direction ("ltr", "rtl", etc.)
            features: OpenType feature string ("liga=0,kern")
        """
        variations_dict = self._parse_variations(variations)
        feature_list = [
            item.strip() for item in (features or "").split(",") if item and item.strip()
        ]

        job = {
            "id": "render",
            "font": {
                "path": font,
                "size": size,
                "variations": variations_dict,
            },
            "text": {
                "content": text,
                "script": script,
                "language": language,
                "direction": direction,
                "features": feature_list,
            },
            "rendering": {
                "format": format,
                "encoding": "json" if format == "metrics" else "base64",
                "width": width,
                "height": height,
            },
        }

        payload = {"version": "1.0", "jobs": [job]}
        iterator = haforu.process_jobs(json.dumps(payload))
        results = list(iterator)
        if not results:
            print("Error: No results returned", file=sys.stderr)
            sys.exit(1)

        result = json.loads(results[0])
        if result.get("status") == "error":
            print(f"Error: {result.get('error', 'Unknown error')}", file=sys.stderr)
            sys.exit(1)

        if format == "metrics":
            metrics = result.get("metrics", {})
            print(f"Density: {metrics.get('density', 0):.4f}")
            print(f"Beam: {metrics.get('beam', 0):.4f}")
            return

        rendering = result.get("rendering", {})
        data = rendering.get("data")
        if not data:
            print("Error: No image data returned", file=sys.stderr)
            sys.exit(1)

        image_bytes = base64.b64decode(data)
        if output:
            with open(output, "wb") as handle:
                handle.write(image_bytes)
            print(f"Image saved to: {output}")
        else:
            sys.stdout.buffer.write(image_bytes)

    def render_single(
        self,
        text: str,
        font: str,
        size: int = 72,
        width: int = 800,
        height: int = 200,
        variations: Optional[str] = None,
        format: str = "pgm",
        output: Optional[str] = None,
        metrics_only: bool = False,
    ) -> None:
        """Deprecated alias for :meth:`render`."""
        if metrics_only and format != "metrics":
            format = "metrics"
        self.render(
            text=text,
            font=font,
            size=size,
            width=width,
            height=height,
            variations=variations,
            format=format,
            output=output,
        )

    def validate(self, input: Optional[str] = None) -> None:
        """Validate a JSON job specification.

        Args:
            input: Input file to validate (reads from stdin if not provided)
        """
        # Read input
        if input:
            with open(input, "r") as f:
                content = f.read()
        else:
            content = sys.stdin.read()

        # Try to parse as JSON
        try:
            job_spec = json.loads(content)
        except json.JSONDecodeError as e:
            print(f"Invalid JSON: {e}", file=sys.stderr)
            sys.exit(1)

        # Validate structure
        errors = []

        if not isinstance(job_spec, dict):
            errors.append("Root must be an object")

        if "jobs" not in job_spec:
            errors.append("Missing 'jobs' field")
        elif not isinstance(job_spec.get("jobs"), list):
            errors.append("'jobs' must be an array")
        else:
            for i, job in enumerate(job_spec["jobs"]):
                job_errors = self._validate_job(job, i)
                errors.extend(job_errors)

        # Report results
        if errors:
            print("Validation failed:", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
            sys.exit(1)
        else:
            print("✓ Valid job specification")
            print(f"  Version: {job_spec.get('version', 'unspecified')}")
            print(f"  Jobs: {len(job_spec.get('jobs', []))}")

    def metrics(
        self,
        input: Optional[str] = None,
        output: Optional[str] = None,
        format: str = "json",
    ) -> None:
        """Compute metrics for rendering jobs without generating images.

        Args:
            input: Input JSON file (reads from stdin if not provided)
            output: Output file (writes to stdout if not provided)
            format: Output format (json, jsonl, or csv)
        """
        # Read input
        if input:
            with open(input, "r") as f:
                job_spec = json.load(f)
        else:
            job_spec = json.load(sys.stdin)

        # Convert all jobs to metrics format
        for job in job_spec.get("jobs", []):
            if "rendering" in job:
                job["rendering"]["format"] = "metrics"

        # Process jobs
        results = haforu.process_jobs(json.dumps(job_spec))

        # Extract metrics
        metrics_data = []
        for result_str in results:
            result = json.loads(result_str)
            if result.get("status") == "success":
                metrics = result.get("metrics", {})
                metrics_data.append(
                    {
                        "id": result.get("id"),
                        "density": metrics.get("density", 0),
                        "beam": metrics.get("beam", 0),
                    }
                )
            else:
                metrics_data.append(
                    {
                        "id": result.get("id"),
                        "error": result.get("error", "Unknown error"),
                    }
                )

        # Output results
        if output:
            output_file = open(output, "w")
        else:
            output_file = sys.stdout

        try:
            if format == "json":
                json.dump(metrics_data, output_file, indent=2)
            elif format == "jsonl":
                for item in metrics_data:
                    print(json.dumps(item), file=output_file)
            elif format == "csv":
                print("id,density,beam,error", file=output_file)
                for item in metrics_data:
                    if "error" in item:
                        print(f"{item['id']},,,,{item['error']}", file=output_file)
                    else:
                        print(
                            f"{item['id']},{item['density']:.4f},{item['beam']:.4f},",
                            file=output_file,
                        )
        finally:
            if output:
                output_file.close()

    def version(self) -> None:
        """Print version information."""
        print(f"haforu {haforu.__version__}")
        print("Python font renderer (Fire CLI)")
        print(f"Available: {haforu.is_available()}")

    def diagnostics(self, format: str = "text") -> None:
        """Print CLI diagnostics similar to the Rust binary."""
        report = {
            "status": "ok",
            "cli_version": haforu.__version__,
            "cpu_count": os.cpu_count() or 1,
            "default_max_fonts": 512,
            "default_max_glyphs": 2048,
        }
        if format == "json":
            print(json.dumps(report, indent=2))
            return
        print(f"haforu {report['cli_version']}")
        print(f"Status       : {report['status']}")
        print(f"CPU threads  : {report['cpu_count']}")
        print(
            "Cache defaults: fonts={default_max_fonts} glyphs={default_max_glyphs}".format(**report)
        )

    def _validate_job(self, job: dict, index: int) -> List[str]:
        """Validate a single job object.

        Args:
            job: Job dictionary to validate
            index: Job index in the array

        Returns:
            List of error messages
        """
        errors = []
        prefix = f"Job [{index}]"

        # Check required fields
        if "id" not in job:
            errors.append(f"{prefix}: Missing 'id' field")

        if "font" not in job:
            errors.append(f"{prefix}: Missing 'font' field")
        elif not isinstance(job["font"], dict):
            errors.append(f"{prefix}: 'font' must be an object")
        else:
            font = job["font"]
            if "path" not in font:
                errors.append(f"{prefix}: Missing 'font.path'")
            if "size" not in font:
                errors.append(f"{prefix}: Missing 'font.size'")
            elif not isinstance(font["size"], (int, float)):
                errors.append(f"{prefix}: 'font.size' must be a number")

        if "text" not in job:
            errors.append(f"{prefix}: Missing 'text' field")
        elif not isinstance(job["text"], dict):
            errors.append(f"{prefix}: 'text' must be an object")
        elif "content" not in job["text"]:
            errors.append(f"{prefix}: Missing 'text.content'")

        if "rendering" in job:
            rendering = job["rendering"]
            if not isinstance(rendering, dict):
                errors.append(f"{prefix}: 'rendering' must be an object")

        return errors

    def _output_results(self, results: List[str], output: Optional[str], format: str) -> None:
        """Output results in the specified format.

        Args:
            results: List of JSON strings
            output: Output file path (None for stdout)
            format: Output format (json, jsonl, or human)
        """
        if output:
            output_file = open(output, "w")
        else:
            output_file = sys.stdout

        try:
            if format == "jsonl":
                for result in results:
                    print(result, file=output_file)

            elif format == "json":
                parsed_results = [json.loads(r) for r in results]
                json.dump(parsed_results, output_file, indent=2)

            elif format == "human":
                for result_str in results:
                    result = json.loads(result_str)
                    status = result.get("status", "unknown")
                    job_id = result.get("id", "unknown")

                    if status == "success":
                        rendering = result.get("rendering", {})
                        metrics = result.get("metrics", {})

                        if metrics:
                            print(
                                f"✓ {job_id}: density={metrics.get('density', 0):.4f}, "
                                f"beam={metrics.get('beam', 0):.4f}",
                                file=output_file,
                            )
                        else:
                            print(
                                f"✓ {job_id}: {rendering.get('width', 0)}x"
                                f"{rendering.get('height', 0)} {rendering.get('format', 'unknown')}",
                                file=output_file,
                            )
                    else:
                        error = result.get("error", "Unknown error")
                        print(f"✗ {job_id}: {error}", file=output_file)

            else:
                print(f"Error: Unknown format: {format}", file=sys.stderr)
                sys.exit(1)

        finally:
            if output:
                output_file.close()

    def _load_json(self, path: Optional[str]) -> Dict[str, Any]:
        """Load JSON from a file or stdin."""
        if path:
            with open(path, "r", encoding="utf-8") as handle:
                return json.load(handle)
        if self.verbose:
            print("Reading from stdin...", file=sys.stderr)
        return json.load(sys.stdin)

    def _parse_variations(self, raw: Optional[str]) -> Dict[str, float]:
        """Parse variation coordinates from CLI input."""
        if not raw:
            return {}
        text = raw.strip()
        if not text:
            return {}
        if text.startswith("{"):
            try:
                data = json.loads(text)
            except json.JSONDecodeError as exc:  # pragma: no cover - user error
                print(f"Invalid variations JSON: {exc}", file=sys.stderr)
                sys.exit(1)
            return {k: float(v) for k, v in data.items()}
        coords: Dict[str, float] = {}
        for token in text.split(","):
            if "=" not in token:
                continue
            axis, value = token.split("=", 1)
            axis = axis.strip()
            if not axis:
                continue
            try:
                coords[axis] = float(value)
            except ValueError:
                print(f"Invalid variation value: {token}", file=sys.stderr)
                sys.exit(1)
        return coords


def main():
    """Main entry point for the CLI."""
    fire.Fire(HaforuCLI)


if __name__ == "__main__":
    main()
