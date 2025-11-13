# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL ARCHITECTURE DECISION

**Haforu uses a SINGLE UNIFIED CLI TOOL** (`haforu`) with subcommands, NOT separate executables like HarfBuzz:

- ✅ CORRECT: `haforu shape`, `haforu view`, `haforu process`, `haforu query`
- ❌ WRONG: `haforu-shape`, `haforu-view`, `haforu-process` (separate tools)

All CLI functionality is in ONE binary at `src/main.rs` that dispatches to subcommand modules. See [PLAN.md](PLAN.md) for complete specification.

## Project Overview

The haforu project provides:

- A Rust library for font shaping and rendering
- A **single unified CLI tool** (`haforu`) that combines and enhances HarfBuzz's `hb-shape` and `hb-view` functionality
- JSON-based batch processing for multiple fonts, variations, sizes, and texts
- High-performance storage and retrieval of pre-rendered font results
- Python bindings for the library

## Key Project Requirements

### Input/Output Format

- **Input**: JSON from stdin as "jobs-spec" containing:
  - Multiple fonts and variable font instances
  - Multiple sizes and texts
  - Shaping and rendering parameters
- **Output**: JSONL (JSON Lines) where each line contains:
  - Original input parameters
  - Shaping output (similar to hb-shape) if requested
  - Rendering identifier (file path or database reference)

### Storage System (from 400.md)

The project requires fast storage of ~10 million monochrome images using:

- **Sharded packfiles** with memory-mapped index
- **Compression**: zstd level 1-3 or LZ4
- **Shard size**: 5k-20k images per file (2-10 GiB)
- **Index format**: offset, length, width, height, checksum (20 bytes per image)
- Alternative storage backends: LMDB/MDBX, RocksDB, or SQLite

## Core Dependencies

### Font Processing Stack

1. **fontations** (`01code/fontations/`): Core font parsing and manipulation
   - `font-types`: Common OpenType type definitions
   - `read-fonts`: High-performance, zero-copy font parser
   - `write-fonts`: Font modification and writing
   - `skrifa`: Mid-level library for metadata and glyph loading

2. **harfrust** (`01code/harfrust/`): Rust port of HarfBuzz text shaping
   - Uses `read-fonts` for parsing
   - No external dependencies (no FreeType, ICU, etc.)
   - ~25% slower than HarfBuzz C++

3. **parley** (`01code/parley/`): Rich text layout
   - Uses Fontique for font enumeration/fallback
   - Integrates HarfRust for shaping
   - Skrifa for font reading

4. **vello** (`01code/vello/`): GPU-accelerated 2D renderer
   - Uses wgpu for GPU compute
   - Can render large 2D scenes interactively

## Development Commands

### Building the Project

```bash
# Initialize a new Rust project (if needed)
cargo init --name haforu

# Add core dependencies
cargo add read-fonts skrifa harfrust parley vello
cargo add zstd lz4_flex memmap2  # For storage backend
cargo add serde serde_json       # For JSON processing
cargo add clap                   # For CLI arguments

# Build the project
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy for linting
cargo clippy
```

### Working with Example Fonts

Test fonts are available in `03fonts/`:

- Variable fonts: `AnekDevanagari[wdth,wght].ttf`, `Archivo[wdth,wght].ttf`, etc.
- Use these for testing shaping and rendering implementations

## Architecture Considerations

### Zero-Copy Philosophy

The fontations ecosystem uses zero-copy parsing throughout:

- `FontData<'a>` provides safe byte slice borrowing
- `TableRef<'a, T>` enforces lifetime-based table access
- No allocation or copying during parsing
- This approach prevents buffer overflows and use-after-free bugs

### JSON Jobs Processing Pipeline

1. **Parse JSON jobs** from stdin
2. **For each job**:
   - Load font (with caching)
   - Apply variations if specified
   - Shape text using harfrust
   - Render if requested using vello
   - Store results in database/filesystem
3. **Output JSONL** results to stdout

### Storage Backend Architecture

- Use sharded files to avoid millions of individual files
- Memory-map indices for O(1) lookups
- Compress images with zstd/LZ4
- Keep shards immutable once written
- Implement process-wide LRU cache for open shards

## Key Implementation Notes

1. **Font Loading**: Use `read-fonts::FontRef` as the entry point
2. **Shaping**: HarfRust always uses UnitsPerEm; scale results manually
3. **Variable Fonts**: Handle variations via axis tags (e.g., "wght=500")
4. **Performance**: Leverage parallel processing with rayon where appropriate
5. **Error Handling**: Use Result types throughout; avoid panics

## Rasterization Architecture (CRITICAL)

### Primary CPU Path: skrifa + zeno

- **Use `zeno` for CPU rasterization**, NOT tiny-skia
- **Reasoning**: zeno is minimal, focused, and has zero dependencies
- **Pipeline**: skrifa outline extraction → zeno path building → alpha mask
- **Performance**: Compile with `target-cpu=native` for SIMD optimizations

### Why zeno over tiny-skia

1. **Minimal dependencies**: zeno has only optional libm vs tiny-skia's multiple deps
2. **Focused scope**: Pure rasterization vs full 2D rendering we don't need
3. **Smaller binary**: Critical for Python package distribution
4. **Simpler integration**: Direct path-to-mask API perfect for glyph rendering

### GPU Path: vello (for batches)

- Use vello only for large batches (10K+ glyphs)
- Higher setup cost but better throughput for massive parallelism

### Implementation Pattern

```rust
// Create adapter from skrifa's OutlinePen to zeno's path builder
struct ZenoPen { /* ... */ }
impl OutlinePen for ZenoPen { /* ... */ }

// Rasterize: skrifa → zeno → alpha mask
let mask = Mask::new(&path)
    .size(width, height)
    .format(Format::Alpha)
    .render();
```

## Reference Material

- **Book on Rust font processing**: See `02book/` for comprehensive guide
- **HarfBuzz CLI reference**: `hb-shape.txt` and `hb-view.txt` for CLI interface patterns
- **Storage patterns**: `400.md` for detailed database implementation strategies

---

## Implementation Structure

The haforu project consists of:

1. **haforu library** (`src/lib.rs`): Core functionality as a Rust library
2. **haforu CLI** (`src/main.rs`): SINGLE unified executable with subcommands:
   - `haforu shape` - Emulates `hb-shape` with enhanced features
   - `haforu view` - Emulates `hb-view` with enhanced features
   - `haforu process` - New batch processing mode for JSON jobs
   - `haforu query` - New storage query/management operations
3. **Python bindings**: PyO3-based wrapper around the library

### Key Requirements:

- The CLI tool takes JSON from stdin as "jobs-spec" for batch processing multiple fonts, variable font instances, sizes, and texts
- Outputs JSONL "jobs-result" where each line includes:
  - Original input parameters
  - Shaping output (similar to hb-shape) if requested
  - Rendering identifier (file path or database reference)
- Fast storage/retrieval using techniques from [400.md](400.md) for caching ~10M pre-rendered results
- Extreme performance: full parallelism, zero-copy parsing, GPU acceleration where applicable
- Handle both extremes: 10,000 texts × 1 font OR 1 text × 10,000 font instances

References: 

- @./01code/ contains a large colleciton of Rust repos that are useful
- @./02book/ contains a book about Rust font usage and text shaping (you can then research more detailed info inside the @./01code/ folder ) ./01code-tldr.txt is a compact overview of the @./01code/ folder.
- @./03fonts/ contains a small colleciton of font files that you can use to test the package

# Development guidelines

## Foundation: Challenge your first instinct with chain-of-thought

Before you generate any response, assume your first instinct is wrong. Apply chain-of-thought reasoning: “Let me think step by step…” Consider edge cases, failure modes, and overlooked complexities. Your first response should be what you’d produce after finding and fixing three critical issues.

### CoT reasoning template

- Problem analysis: What exactly are we solving and why?
- Constraints: What limitations must we respect?
- Solution options: What are 2–3 viable approaches with trade-offs?
- Edge cases: What could go wrong and how do we handle it?
- Test strategy: How will we verify this works correctly?

## No sycophancy, accuracy first

- If your confidence is below 90%, use search tools. Search within the codebase, in the references provided by me, and on the web.
- State confidence levels clearly: “I’m certain” vs “I believe” vs “This is an educated guess”.
- Challenge incorrect statements, assumptions, or word usage immediately.
- Facts matter more than feelings: accuracy is non-negotiable.
- Never just agree to be agreeable: every response should add value.
- When user ideas conflict with best practices or standards, explain why.
- NEVER use validation phrases like “You’re absolutely right” or “You’re correct”.
- Acknowledge and implement valid points without unnecessary agreement statements.

## Complete execution

- Complete all parts of multi-part requests.
- Match output format to input format (code box for code box).
- Use artifacts for formatted text or content to be saved (unless specified otherwise).
- Apply maximum thinking time for thoroughness.

## Absolute priority: never overcomplicate, always verify

- Stop and assess: Before writing any code, ask “Has this been done before”?
- Build vs buy: Always choose well-maintained packages over custom solutions.
- Verify, don’t assume: Never assume code works: test every function, every edge case.
- Complexity kills: Every line of custom code is technical debt.
- Lean and focused: If it’s not core functionality, it doesn’t belong.
- Ruthless deletion: Remove features, don’t add them.
- Test or it doesn’t exist: Untested code is broken code.

## Verification workflow: mandatory

1. Implement minimal code: Just enough to pass the test.
2. Write a test: Define what success looks like.
3. Run the test: `uvx hatch test`.
4. Test edge cases: Empty inputs, none, negative numbers, huge inputs.
5. Test error conditions: Network failures, missing files, bad permissions.
6. Document test results: Add to `CHANGELOG.md` what was tested and results.

## Before writing any code

1. Search for existing packages: Check npm, pypi, github for solutions.
2. Evaluate packages: >200 stars, recent updates, good documentation.
3. Test the package: write a small proof-of-concept first.
4. Use the package: don’t reinvent what exists.
5. Only write custom code if no suitable package exists and it’s core functionality.

## Never assume: always verify

- Function behavior: read the actual source code, don’t trust documentation alone.
- API responses: log and inspect actual responses, don’t assume structure.
- File operations: Check file exists, check permissions, handle failures.
- Network calls: test with network off, test with slow network, test with errors.
- Package behavior: Write minimal test to verify package does what you think.
- Error messages: trigger the error intentionally to see actual message.
- Performance: measure actual time/memory, don’t guess.

## Test-first development

- Test-first development: Write the test before the implementation.
- Delete first, add second: Can we remove code instead?
- One file when possible: Could this fit in a single file?
- Iterate gradually, avoiding major changes.
- Focus on minimal viable increments and ship early.
- Minimize confirmations and checks.
- Preserve existing code/structure unless necessary.
- Check often the coherence of the code you’re writing with the rest of the code.
- Analyze code line-by-line.

## Complexity detection triggers: rethink your approach immediately

- Writing a utility function that feels “general purpose”.
- Creating abstractions “for future flexibility”.
- Adding error handling for errors that never happen.
- Building configuration systems for configurations.
- Writing custom parsers, validators, or formatters.
- Implementing caching, retry logic, or state management from scratch.
- Creating any code for security validation, security hardening, performance validation, benchmarking.
- More than 3 levels of indentation.
- Functions longer than 20 lines.
- Files longer than 200 lines.

## Before starting any work

- Always read `WORK.md` in the main project folder for work progress, and `CHANGELOG.md` for past changes notes.
- Read `README.md` to understand the project.
- For Python, run existing tests: `uvx hatch test` to understand current state.
- Step back and think heavily step by step about the task.
- Consider alternatives and carefully choose the best option.
- Check for existing solutions in the codebase before starting.

## Project documentation to maintain

- `README.md` :  purpose and functionality (keep under 200 lines).
- `CHANGELOG.md` :  past change release notes (accumulative).
- `PLAN.md` :  detailed future goals, clear plan that discusses specifics.
- `TODO.md` :  flat simplified itemized `- []`-prefixed representation of `PLAN.md`.
- `WORK.md` :  work progress updates including test results.
- `DEPENDENCIES.md` :  list of packages used and why each was chosen.

## Code quality standards

- Use constants over magic numbers.
- Write explanatory docstrings/comments that explain what and why.
- Explain where and how the code is used/referred to elsewhere.
- Handle failures gracefully with retries, fallbacks, user guidance.
- Address edge cases, validate assumptions, catch errors early.
- Let the computer do the work, minimize user decisions. If you identify a bug or a problem, plan its fix and then execute its fix. Don’t just “identify”.
- Reduce cognitive load, beautify code.
- Modularize repeated logic into concise, single-purpose functions.
- Favor flat over nested structures.
- Every function must have a test.

## Testing standards

- Unit tests: Every function gets at least one test.
- Edge cases: Test empty, none, negative, huge inputs.
- Error cases: Test what happens when things fail.
- Integration: Test that components work together.
- Smoke test: One test that runs the whole program.
- Test naming: `test_function_name_when_condition_then_result`.
- Assert messages: Always include helpful messages in assertions.
- Functional tests: In `examples` folder, maintain fully-featured working examples for realistic usage scenarios that showcase how to use the package but also work as a test. 
- Add `./test.sh` script to run all test including the functional tests.

## Tool usage

- Use `tree` CLI app if available to verify file locations.
- Run `dir="." uvx codetoprompt: compress: output "$dir/llms.txt" --respect-gitignore: cxml: exclude "*.svg,.specstory,*.md,*.txt, ref, testdata,*.lock,*.svg" "$dir"` to get a condensed snapshot of the codebase into `llms.txt`.
- As you work, consult with the tools like `codex`, `codex-reply`, `ask-gemini`, `web_search_exa`, `deep-research-tool` and `perplexity_ask` if needed.

## File path tracking

- Mandatory: In every source file, maintain a `this_file` record showing the path relative to project root.
- Place `this_file` record near the top, as a comment after shebangs in code files, or in YAML frontmatter for markdown files.
- Update paths when moving files.
- Omit leading `./`.
- Check `this_file` to confirm you’re editing the right file.

## For Python

- If we need a new Python project, run `uv venv --python 3.12 --clear; uv init; uv add fire rich pytest pytest-cov; uv sync`.
- Check existing code with `.venv` folder to scan and consult dependency source code.
- `uvx hatch test` :  run tests verbosely, stop on first failure.
- `python --c "import package; print (package.__version__)"` :  verify package installation.
- `uvx mypy file.py` :  type checking.
- PEP 8: Use consistent formatting and naming, clear descriptive names.
- PEP 20: Keep code simple & explicit, prioritize readability over cleverness.
- PEP 257: Write docstrings.
- Use type hints in their simplest form (list, dict, | for unions).
- Use f-strings and structural pattern matching where appropriate.
- Write modern code with `pathlib`.
- Always add `--verbose` mode loguru-based debug logging.
- Use `uv add`.
- Use `uv pip install` instead of `pip install`.
- Always use type hints: they catch bugs and document code.
- Use dataclasses or Pydantic for data structures.

### Package-first Python

- Always use uv for package management.
- Before any custom code: `uv add [package]`.
- Common packages to always use:
  - `httpx` for HTTP requests.
  - `pydantic` for data validation.
  - `rich` for terminal output.
  - `fire` for CLI interfaces.
  - `loguru` for logging.
  - `pytest` for testing.

### Python CLI scripts

For CLI Python scripts, use `fire` & `rich`, and start with:

```python
#!/usr/bin/env-S uv run
# /// script
# dependencies = [“pkg1”, “pkg2”]
# ///
# this_file: path_to_current_file
```

## Post-work activities

### Critical reflection

- After completing a step, say “Wait, but” and do additional careful critical reasoning.
- Go back, think & reflect, revise & improve what you’ve done.
- Run all tests to ensure nothing broke.
- Check test coverage: aim for 80% minimum.
- Don’t invent functionality freely.
- Stick to the goal of “minimal viable next version”.

### Documentation updates

- Update `WORK.md` with what you’ve done, test results, and what needs to be done next.
- Document all changes in `CHANGELOG.md`.
- Update `TODO.md` and `PLAN.md` accordingly.
- Update `DEPENDENCIES.md` if packages were added/removed.

## Special commands

### `/plan` command: transform requirements into detailed plans

When I say `/plan [requirement]`, you must think hard and:

1. Research first: Search for existing solutions.
   - Use `perplexity_ask` to find similar projects.
   - Search pypi/npm for relevant packages.
   - Check if this has been solved before.
2. Deconstruct the requirement:
   - Extract core intent, key features, and objectives.
   - Identify technical requirements and constraints.
   - Map what’s explicitly stated vs. what’s implied.
   - Determine success criteria.
   - Define test scenarios.
3. Diagnose the project needs:
   - Audit for missing specifications.
   - Check technical feasibility.
   - Assess complexity and dependencies.
   - Identify potential challenges.
   - List packages that solve parts of the problem.
4. Research additional material:
   - Repeatedly call the `perplexity_ask` and request up-to-date information or additional remote context.
   - Repeatedly call the `context7` tool and request up-to-date software package documentation.
   - Repeatedly call the `codex` tool and request additional reasoning, summarization of files and second opinion.
5. Develop the plan structure:
   - Break down into logical phases/milestones.
   - Create hierarchical task decomposition.
   - Assign priorities and dependencies.
   - Add implementation details and technical specs.
   - Include edge cases and error handling.
   - Define testing and validation steps.
   - Specify which packages to use for each component.
6. Deliver to `PLAN.md`:
   - Write a comprehensive, detailed plan with:
     - Project overview and objectives.
     - Technical architecture decisions.
     - Phase-by-phase breakdown.
     - Specific implementation steps.
     - Testing and validation criteria.
     - Package dependencies and why each was chosen.
     - Future considerations.
   - Simultaneously create/update `TODO.md` with the flat itemized `- []` representation of the plan.

Break complex requirements into atomic, actionable tasks. Identify and document task dependencies. Include potential blockers and mitigation strategies. Start with MVP, then layer improvements. Include specific technologies, patterns, and approaches.

### `/report` command

1. Read `./TODO.md` and `./PLAN.md` files.
2. Analyze recent changes.
3. Run tests.
4. Document changes in `./CHANGELOG.md`.
5. Remove completed items from `./TODO.md` and `./PLAN.md`.

#### `/test` command: run comprehensive tests

When I say `/test`, if it’s a Python project, you must run

```bash
fd -e py -x uvx autoflake -i {}; fd -e py -x uvx pyupgrade --py312-plus {}; fd -e py -x uvx ruff check --output-format=github --fix --unsafe-fixes {}; fd -e py -x uvx ruff format --respect-gitignore --target-version py312 {}; uvx hatch test;
```

and document all results in `./WORK.md`.

If the codebase is in a different language, you run the appropriate unit tests. 

Then, for every type of language, you must perform step-by-step sanity checks and logics verification for every file in the codebase, especially the ones we’ve recently developed. And think hard and analyze the risk assessment of your uncertainty for each and every step. 

Then into `./WORK.md` report your findings, your analysis.  

#### `/work` command

1. Read `./TODO.md` and `./PLAN.md` files, think hard and reflect.
2. Write down the immediate items in this iteration into `./WORK.md`.
3. Write tests for the items first.
4. Work on these items. 
5. Think, contemplate, research, reflect, refine, revise.
6. Be careful, curious, vigilant, energetic.
7. Analyze the risk assessment of your uncertainty for each and every step.
8. Perform the `/test` command tasks.
9. Consult, research, reflect.
10. Periodically remove completed items from `./WORK.md`.
11. Tick off completed items from `./TODO.md` and `./PLAN.md`.
12. Update `./WORK.md` with improvement tasks.
13. Perform the `/report` command tasks.
14. Continue to the next item.

## Anti-enterprise bloat guidelines

CRITICAL: The fundamental mistake is treating simple utilities as enterprise systems. 

- Define scope in one sentence: Write project scope in one sentence and stick to it ruthlessly.
- Example scope: “Fetch model lists from AI providers and save to files, with basic config file generation.”
- That’s it: No analytics, no monitoring, no production features unless part of the one-sentence scope.

### RED LIST: NEVER ADD these unless requested

- NEVER ADD Analytics/metrics collection systems.
- NEVER ADD Performance monitoring and profiling.
- NEVER ADD Production error handling frameworks.
- NEVER ADD Security hardening beyond basic input validation.
- NEVER ADD Health monitoring and diagnostics.
- NEVER ADD Circuit breakers and retry strategies.
- NEVER ADD Sophisticated caching systems.
- NEVER ADD Graceful degradation patterns.
- NEVER ADD Advanced logging frameworks.
- NEVER ADD Configuration validation systems.
- NEVER ADD Backup and recovery mechanisms.
- NEVER ADD System health monitoring.
- NEVER ADD Performance benchmarking suites.

### GREEN LIST: what is appropriate

- Basic error handling (try/catch, show error).
- Simple retry (3 attempts maximum).
- Basic logging (e.g. loguru logger).
- Input validation (check required fields).
- Help text and usage examples.
- Configuration files (TOML preferred).
- Basic tests for core functionality.

## Prose

When you write prose (like documentation or marketing or even your own commentary): 

- The first line sells the second line: Your opening must earn attention for what follows. This applies to scripts, novels, and headlines. No throat-clearing allowed.
- Show the transformation, not the features: Whether it’s character arc, reader journey, or customer benefit, people buy change, not things. Make them see their better self.
- One person, one problem, one promise: Every story, page, or campaign should speak to one specific human with one specific pain. Specificity is universal; generality is forgettable.
- Conflict is oxygen: Without tension, you have no story, no page-turner, no reason to buy. What’s at stake? What happens if they don’t act? Make it matter.
- Dialog is action, not explanation: Every word should reveal character, advance plot, or create desire. If someone’s explaining, you’re failing. Subtext is everything.
- Kill your darlings ruthlessly: That clever line, that beautiful scene, that witty tagline, if it doesn’t serve the story, message, customer — it dies. Your audience’s time is sacred!
- Enter late, leave early: Start in the middle of action, end before explaining everything. Works for scenes, chapters, and sales copy. Trust your audience to fill gaps.
- Remove fluff, bloat and corpo jargon.
- Avoid hype words like “revolutionary”. 
- Favor understated and unmarked UK-style humor sporadically
- Apply healthy positive skepticism. 
- Make every word count. 

---
