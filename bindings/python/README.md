---
this_file: bindings/python/README.md
---

# haforu (Python)

Python bindings for the `haforu` Rust crate via PyO3 + maturin.

Minimal API (v0):
- `haforu.version() -> str`
- `haforu.validate_spec(json: str) -> bool`
- `haforu.process(json: str) -> list[str]` (JSONL lines)

Build locally:
```
pip install maturin
maturin develop -m Cargo.toml --release
python -c "import haforu; print(haforu.version())"
```

