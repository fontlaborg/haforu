---
this_file: haforu/TODO.md
---

- [x] Add StreamingSession cache knobs + `warm_up()` helper, ensure `close()` frees descriptors immediately, and expose a fast `is_available()` probe.
- [x] Implement stdin→stdout JSONL batch processing with flush-per-line behavior, runtime `--jobs` flag, and the bundled `scripts/batch_smoke.sh` + `jobs_smoke.json` sanity check.
- [x] Publish prebuilt universal2/manylinux wheels via maturin and document the exact `uv pip install haforu` / `cargo install haforu` commands plus `HAFORU_BIN` wiring expected by fontsimi.
