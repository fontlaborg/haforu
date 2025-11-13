---
this_file: README.md
---

# haforu

A fast Rust toolkit that shapes and renders text like hb-shape/hb-view, but at batch scale with JSON jobs and a sharded on-disk cache.

Why this exists: shaping and rendering a handful of strings is easy; shaping and rendering 10,000 texts across hundreds of variable font instances—repeatedly and reproducibly—is not. Haforu focuses on that high‑volume case while staying simple to operate.

## What It Does

- Accepts a JSON “jobs-spec” from stdin describing multiple fonts (including variable instances), sizes, and texts.
- Shapes text (hb-shape‑style output when requested) and optionally renders.
- Stores rendered results in sharded packfiles for fast lookup, or writes images to files.
- Emits JSONL “jobs-result” on stdout: each line echoes the input params plus shaping output and a render identifier (path or storage key).

CLI, library, and Python bindings target the same core pipeline.

## How It Works

- Font parsing: read-fonts and skrifa provide zero‑copy access to OpenType tables.
- Shaping: harfrust shapes text using UnitsPerEm metrics; we scale positioning as needed.
- CPU rasterization: skrifa outlines → zeno path rasterization (minimal deps, SIMD friendly).
- Parallelism: rayon powers per‑glyph or per‑job concurrency.
- Storage: images are batched into mmapped, compressed shard files with a tiny fixed index for O(1) retrieval.
- I/O: stdin JSON → streamed JSONL results, suitable for pipelines and caches.

## Why These Choices

- Zero‑copy font stack: safety and speed without external C deps.
- zeno over full 2D engines: smaller binary, tighter hot path for glyph masks.
- Sharded packfiles: millions of items without filesystem thrash; mmapped indices keep lookups cheap.
- JSON in/out: simple to compose and test; ideal for batch orchestration.

## Critical Components (unique to this project)

- HarfRust shaping wired for batch scale: shape once, fan out across sizes or instances.
- Zeno‑based CPU rasterizer: direct path‑to‑mask with subpixel positioning.
- Sharded storage format: compressed blobs plus 20‑byte index entries; immutable shards with process‑wide LRU of open mmaps.
- Unified CLI that mirrors hb‑tools semantics while adding JSON batch mode and storage IDs.

## The Codebase Snapshot Tool (llms.sh)

This repository includes a tiny helper that produces a compact, LLM‑friendly snapshot of the codebase.

- What: `llms.sh` generates `llms.txt`—a compressed, pruned view of the repo useful for code review and AI assistance.
- How: it calls a local `llms` wrapper, which runs `uvx codetoprompt` with sensible defaults:
  - Respects `.gitignore`, compresses output, emits a project tree and CXML, and caps content via `--max-tokens`.
  - Excludes heavy or non‑essential assets. The script adds: `*.txt, 01code, 02book, 03fonts, AGENTS.md, CLAUDE.md, GEMINI.md, LLXPRT.md, QWEN.md, WORK.md, issues, test_results.txt, external, *.html, 01code-tldr.txt`.
- Why: keeps the working set focused (code > assets), reduces tokens, and makes shareable snapshots repeatable.

References:
- Script entry: `llms.sh:1`
- Wrapper used by the script (on this machine): `/Users/adam/bin/llms` → runs `uvx codetoprompt --output ./llms.txt --respect-gitignore --compress --cxml --tree-depth 10 --max-tokens 1000000 --exclude "…" .`

Usage:
- Generate/update snapshot: `./llms.sh` → writes `llms.txt` in repo root.
- Customize excludes: edit the comma‑separated list in `llms.sh`.
- Requirements: `uv` installed; `uvx` runs `codetoprompt` on demand (no global install needed).

## Quick Start

- Build: `cargo build --release`
- Test: `cargo test`
- Run on a jobs file: `cat test_job.json | cargo run -- process --render --storage ./dist`

Outputs are JSON lines. Each includes the job echo, optional shaping details, and a render identifier that is either a file path or a `shard_id:local_idx` key into storage.

## Storage TLDR

- File layout per shard: [compressed images][index][footer].
- Index entry is fixed 20 bytes: offset, len, width, height, checksum.
- Footer records magic, version, count, index offset, shard id.
- Retrieval mmaps the shard, reads index, and decodes zstd content.

## Status

Active work-in-progress. See `PLAN.md` and `TODO.md` for detailed milestones, and `CHANGELOG.md` / `WORK.md` for progress and test notes.

