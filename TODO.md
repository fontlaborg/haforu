this_file: haforu/TODO.md
---

- [ ] address @./issues/101.md 
- [ ] (P14) Audit and upgrade the Rust CLI for the “efficient powerful” contract (flags, streaming throughput, profiling hooks), plus add regression/benchmark coverage.
- [ ] (P15) Align the Python Fire CLI with the Rust feature set (batch/stream/render/metrics/cache knobs), add validation, and ensure console entry points stay fast even without native wheels.
- [ ] (P16) Canonicalize the repo structure/documentation so Rust workspace + Python package best practices (tooling configs, docs, metadata) stay in sync with FontSimi requirements.
- [ ] (P17) Make the new pipeline reproducible locally and in GitHub Actions, including artifact uploads, smoke tests, and documentation of cache keys + prerequisites.
- [ ] (P18) Wire automatic SemVer sourced from git tags (hatch-vcs + cargo) and hook GitHub Actions to cut releases whenever a `vX.Y.Z` tag is pushed.
