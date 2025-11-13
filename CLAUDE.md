---
this_file: haforu/CLAUDE.md
---

Claude Guide — haforu (legacy / deprecated)

Status and Scope
- This folder is the legacy, half‑implemented haforu. It remains only as a temporary reference until the canonical `haforu` in `../haforu2/` is complete and integrated.
- Do not add new features here. Only minimal edits strictly necessary to aid migration are acceptable.
- Once `haforu2` is mature (published as `haforu`), remove this folder/symlink from the fontsimi workspace.

Source of Truth
- Follow `../PLAN.md`, `../TODO.md` and `../haforu2/PLAN.md`, `../haforu2/TODO.md`.
- Canonical naming: all artifacts are `haforu` (no “2”). The on‑disk `haforu2/` path is for development only.

What to Keep Here (temporarily)
- Pointers that clarify interfaces and JSON/PGM expectations for the new `haforu` implementation.
- Minimal glue or notes to assist migration; no active rendering logic should evolve here.

Hard Rules
- Don’t diverge API or behavior from the contracts defined in `../haforu2/PLAN.md`.
- No serif/sans classification or style heuristics anywhere in the pipeline.
- Keep `this_file` markers at the top of all files if touched.

Next Actions
- Focus all rendering work in `../haforu2/` and ensure compatibility with fontsimi’s HaforuRenderer.
- When `haforu2` passes fontsimi integration tests, archive or remove this folder.

