---
this_file: DEPENDENCIES.md
---

# Dependency Rationale

## Rust Crate

- **clap** — battle-tested CLI arg parsing so we can expose batch/stream/render/diagnostics switches without writing our own parser.
- **rayon** — lock-free parallel iterator runtime powering batch rendering throughput and Python iterator fan-out.
- **harfbuzz_rs** + **skrifa/read-fonts** — production-grade shaping + variable font handling; no bespoke font math.
- **zeno** + **image** — raster + image encoding so we emit deterministic base64 payloads without custom encoders.
- **memmap2** — zero-copy font loading to keep RSS flat while cycling through 1000s of fonts.
- **env_logger/log** — structured logging (JSON/text) for CLI, smoke scripts, and integration debugging.

## Python Package

- **PyO3/maturin** — exposes the Rust engine to Python with shared glyph cache + streaming session.
- **fire** — lightweight CLI surface matching the Rust commands; keeps parity without manual argparse plumbing.
- **numpy** — optional zero-copy path for `StreamingSession.render_to_numpy`.

Each dependency keeps Haforu lean: we offload parsing/shaping/rasterization to maintained libraries instead of re-implementing fragile infrastructure.
