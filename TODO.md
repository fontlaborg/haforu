---
this_file: TODO.md
---

- [ ] Fix build errors in `src/python/batch.rs` and `src/python/mod.rs`:

```
uv pip install --system --upgrade -e .
Using Python 3.12.8 environment at: /Library/Frameworks/Python.framework/Versions/3.12
Resolved 4 packages in 1.72s
Building haforu @ file:///Users/adam/Developer/vcs/github.fontlaborg/haforu
× Failed to build `haforu @ file:///Users/adam/Developer/vcs/github.fontlaborg/haforu`
├─▶ The build backend returned an error
╰─▶ Call to `maturin.build_editable` failed (exit status: 1)

[stdout]
Running `maturin pep517 build-wheel -i /Users/adam/.cache/uv/builds-v0/.tmpjtJzYu/bin/python --compatibility off --editable`

[stderr]
🍹 Building a mixed python/rust project
🔗 Found pyo3 bindings
🐍 Found CPython 3.12 at /Users/adam/.cache/uv/builds-v0/.tmpjtJzYu/bin/python
📡 Using build options features from pyproject.toml
Compiling pyo3-build-config v0.22.6
Compiling pyo3-ffi v0.22.6
Compiling pyo3-macros-backend v0.22.6
Compiling pyo3 v0.22.6
Compiling pyo3-macros v0.22.6
Compiling numpy v0.22.1
Compiling haforu v2.0.0 (/Users/adam/Developer/vcs/github.fontlaborg/haforu)
error[E0063]: missing field `font` in initializer of `JobResult`
--> src/python/batch.rs:142:44
|
142 |                     serde_json::to_string(&JobResult {
|                                            ^^^^^^^^^ missing `font`

error[E0061]: this function takes 3 arguments but 1 argument was supplied
--> src/python/mod.rs:17:5
|
17 |     streaming::StreamingSession::new(1).is_ok()
|     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^--- two arguments of type `std::option::Option<usize>` and `std::option::Option<usize>` are missing
|
note: associated function defined here
--> src/python/streaming.rs:58:12
|
58 |     pub fn new(
|            ^^^
59 |         cache_size: Option<usize>,
|         -------------------------
60 |         max_fonts: Option<usize>,
|         ------------------------
help: provide the arguments
|
17 |     streaming::StreamingSession::new(/* std::option::Option<usize> */, /* std::option::Option<usize> */, 1).is_ok()
|                                      +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

Some errors have detailed explanations: E0061, E0063.
For more information about an error, try `rustc --explain E0061`.
error: could not compile `haforu` (lib) due to 2 previous errors
💥 maturin failed
Caused by: Failed to build a native library through cargo
Caused by: Cargo build finished with "exit status: 101": `env -u CARGO PYO3_BUILD_EXTENSION_MODULE="1" PYO3_ENVIRONMENT_SIGNATURE="cpython-3.12-64bit"
PYO3_PYTHON="/Users/adam/.cache/uv/builds-v0/.tmpjtJzYu/bin/python" PYTHON_SYS_EXECUTABLE="/Users/adam/.cache/uv/builds-v0/.tmpjtJzYu/bin/python"
"cargo" "rustc" "--profile" "release" "--features" "python" "--message-format" "json-render-diagnostics" "--manifest-path"
"/Users/adam/Developer/vcs/github.fontlaborg/haforu/Cargo.toml" "--lib" "--crate-type" "cdylib" "--" "-C" "link-arg=-undefined" "-C" "link-arg=dynamic_lookup"
"-C" "link-args=-Wl,-install_name,@rpath/haforu._haforu.cpython-312-darwin.so"`
Error: command ['maturin', 'pep517', 'build-wheel', '-i', '/Users/adam/.cache/uv/builds-v0/.tmpjtJzYu/bin/python', '--compatibility', 'off', '--editable']
returned non-zero exit status 1

hint: This usually indicates a problem with the package or the build environment.
```



- [ ] Write nice `./build.sh` script that builds the Rust CLI and Python package, runs the tests, and builds the wheels.
- [ ] Add a `./run.sh` script that runs the package using some test data fonts.
- [ ] Audit and upgrade Rust CLI for "efficient powerful" contract (flags, streaming throughput, profiling hooks), plus add regression/benchmark coverage
- [ ] Align Python Fire CLI with Rust feature set (batch/stream/render/metrics/cache knobs), add validation, ensure console entry points stay fast even without native wheels
- [ ] Canonicalize repo structure/documentation so Rust workspace + Python package best practices (tooling configs, docs, metadata) stay in sync with FontSimi requirements
- [ ] Make build pipeline reproducible locally and in GitHub Actions, including artifact uploads, smoke tests, and documentation of cache keys + prerequisites
- [ ] Wire automatic SemVer sourced from git tags (hatch-vcs + cargo) and hook GitHub Actions to cut releases whenever a `vX.Y.Z` tag is pushed
