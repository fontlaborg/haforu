# this_file: python/tests/test_streaming.py

"""Tests for haforu streaming session Python bindings."""

import json
import pytest


def test_streaming_session_import():
    """Test that StreamingSession class can be imported."""
    try:
        import haforu
        assert hasattr(haforu, "StreamingSession")
    except ImportError:
        pytest.skip("haforu Python bindings not installed")


def test_streaming_session_creation():
    """Test that StreamingSession can be created."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    assert session is not None


def test_streaming_session_custom_cache_size():
    """Test that StreamingSession accepts custom cache size."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession(cache_size=1024)
    assert session is not None


def test_streaming_session_cache_stats_and_resize():
    """StreamingSession exposes cache stats + resize knob."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession(cache_size=128)
    stats = session.cache_stats()
    assert stats["capacity"] == 128
    assert stats["glyph_capacity"] >= 1

    session.set_cache_size(64)
    resized = session.cache_stats()
    assert resized["capacity"] == 64

    session.set_glyph_cache_size(32)
    glyph_stats = session.cache_stats()
    assert glyph_stats["glyph_capacity"] == 32


def test_streaming_session_glyph_cache_reuses_results():
    """Identical glyph jobs should hit the glyph cache regardless of ID."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession(max_glyphs=2)
    job = {
        "id": "cache-a",
        "font": {
            "path": "testdata/fonts/Arial-Black.ttf",
            "size": 256,
            "variations": {},
        },
        "text": {"content": "B"},
        "rendering": {
            "format": "pgm",
            "encoding": "base64",
            "width": 64,
            "height": 64,
        },
    }

    first = json.loads(session.render(json.dumps(job)))
    assert first["id"] == "cache-a"

    job["id"] = "cache-b"
    second = json.loads(session.render(json.dumps(job)))
    assert second["id"] == "cache-b"

    stats = session.cache_stats()
    assert stats["glyph_entries"] == 1
    assert stats.get("glyph_hits", 0) >= 1


def test_streaming_session_can_disable_glyph_cache():
    """Setting glyph cache size to zero should disable caching."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession(max_glyphs=0)
    stats = session.cache_stats()
    assert stats["glyph_capacity"] == 0
    session.set_glyph_cache_size(0)
    disabled = session.cache_stats()
    assert disabled["glyph_capacity"] == 0


def test_streaming_session_close():
    """Test that StreamingSession can be closed."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    session.close()
    # Follow-up renders should raise RuntimeError once closed
    job = json.dumps(
        {
            "id": "after-close",
            "font": {"path": "/nonexistent/font.ttf", "size": 1000, "variations": {}},
            "text": {"content": "a"},
            "rendering": {"format": "pgm", "encoding": "base64", "width": 32, "height": 32},
        }
    )
    with pytest.raises(RuntimeError):
        session.render(job)


def test_streaming_session_context_manager():
    """Test that StreamingSession works as context manager."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    with haforu.StreamingSession() as session:
        assert session is not None
    # Should not raise error on exit


def test_streaming_session_render_method_exists():
    """Test that StreamingSession has render method."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    assert hasattr(session, "render")
    assert hasattr(session, "warm_up")
    assert hasattr(session, "ping")


def test_streaming_session_render_invalid_json():
    """Invalid JSON should return JobResult error payload instead of raising."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    payload = json.loads(session.render("not valid json"))
    assert payload["status"] == "error"
    assert "Invalid JSON" in payload["error"]


def test_streaming_session_warm_up_ping():
    """warm_up should succeed without a font path."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    assert session.warm_up() is True
    assert session.ping() is True


def test_streaming_session_render_single_job():
    """Test that render processes a single job and returns JSONL."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    job = {
        "id": "test1",
        "font": {
            "path": "/nonexistent/font.ttf",
            "size": 1000,
            "variations": {},
        },
        "text": {"content": "a"},
        "rendering": {
            "format": "pgm",
            "encoding": "base64",
            "width": 100,
            "height": 100,
        },
    }
    job_json = json.dumps(job)
    result_json = session.render(job_json)

    # Parse result
    result = json.loads(result_json)
    assert "id" in result
    assert result["id"] == "test1"
    assert "status" in result
    # Will be "error" because font doesn't exist
    assert result["status"] in ["success", "error"]
    assert "timing" in result


def test_streaming_session_multiple_renders():
    """Test that session can handle multiple sequential renders."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()

    # Render same job 10 times with a real font so glyphs are cached
    from pathlib import Path
    font_path = Path(__file__).parent.parent.parent / "testdata" / "fonts" / "Arial-Black.ttf"

    for i in range(10):
        job = {
            "id": f"test{i}",
            "font": {
                "path": str(font_path),
                "size": 1000,
                "variations": {},
            },
            "text": {"content": "a"},
            "rendering": {
                "format": "pgm",
                "encoding": "base64",
                "width": 100,
                "height": 100,
            },
        }
        job_json = json.dumps(job)
        result_json = session.render(job_json)
        result = json.loads(result_json)
        assert result["id"] == f"test{i}"
        assert result["status"] == "success"

    glyph_stats = session.cache_stats()
    # Should have cached the single glyph 'a' after rendering it 10 times
    assert glyph_stats["glyph_entries"] == 1


def test_streaming_session_result_format():
    """Test that streaming session results match expected format."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    with haforu.StreamingSession() as session:
        job = {
            "id": "test1",
            "font": {
                "path": "/nonexistent/font.ttf",
                "size": 1000,
                "variations": {},
            },
            "text": {"content": "a"},
            "rendering": {
                "format": "pgm",
                "encoding": "base64",
                "width": 100,
                "height": 100,
            },
        }
        result_json = session.render(json.dumps(job))
        result = json.loads(result_json)

        # Check required fields
        assert "id" in result
        assert "status" in result
        assert "timing" in result

        # Check timing structure
        timing = result["timing"]
        assert "total_ms" in timing
        # Other timing fields may vary (render_ms, shape_ms, etc.)


def test_streaming_session_error_handling():
    """Test that streaming session handles errors gracefully."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    job = {
        "id": "invalid-render",
        "font": {
            "path": "/nonexistent/font.ttf",
            "size": 1000,
            "variations": {},
        },
        "text": {"content": "a"},
        "rendering": {
            "format": "pgm",
            "encoding": "base64",
            "width": 0,  # invalid width should stay a JSON error
            "height": 64,
        },
    }
    result_json = session.render(json.dumps(job))
    result = json.loads(result_json)
    assert result["status"] == "error"
    assert "Canvas" in result.get("error", "")


def test_streaming_session_metrics_format_returns_metrics_payload():
    """Streaming renders with format=metrics should omit rendering data."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    session = haforu.StreamingSession()
    job = {
        "id": "metrics-stream",
        "font": {
            "path": "testdata/fonts/Arial-Black.ttf",
            "size": 256,
            "variations": {},
        },
        "text": {"content": "M"},
        "rendering": {
            "format": "metrics",
            "encoding": "json",
            "width": 96,
            "height": 96,
        },
    }
    result = json.loads(session.render(json.dumps(job)))
    assert result["status"] == "success"
    assert "metrics" in result
    assert "rendering" not in result
    for key in ("density", "beam"):
        assert 0.0 <= result["metrics"][key] <= 1.0, f"{key} out of range"


def test_haforu_module_is_available_probe():
    """Module-level availability probe should be fast and boolean."""
    try:
        import haforu
    except ImportError:
        pytest.skip("haforu Python bindings not installed")

    available = haforu.is_available()
    assert isinstance(available, bool)

    session = haforu.StreamingSession()

    # Test with missing required field
    job = {
        "id": "test1",
        "font": {"path": "/nonexistent/font.ttf", "size": 1000},
        # Missing text field
        "rendering": {
            "format": "pgm",
            "encoding": "base64",
            "width": 100,
            "height": 100,
        },
    }

    session = haforu.StreamingSession()
    result = json.loads(session.render(json.dumps(job)))
    assert result["status"] == "error"
