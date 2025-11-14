---
this_file: INSTALL.md
---

# Haforu Installation Guide

This guide covers installation of Haforu on all supported platforms.

## Quick Install

### Python Package (Recommended)

```bash
pip install haforu
```

This installs both the Python bindings and provides access to the CLI via `python -m haforu`.

### Rust Binary via Cargo

```bash
cargo install haforu
```

## Platform-Specific Instructions

### macOS

#### Universal2 Wheel (Recommended)

Works on both Intel and Apple Silicon Macs:

```bash
pip install haforu
```

The wheel includes native binaries for both architectures.

#### Building from Source

Requirements:
- Rust 1.70+
- Python 3.8+
- Xcode Command Line Tools

```bash
# Clone the repository
git clone https://github.com/fontsimi/haforu.git
cd haforu

# Build everything
./scripts/build.sh all

# Install Python package in development mode
source .venv/bin/activate
maturin develop --release --features python
```

#### Platform-Specific Extras

```bash
pip install haforu[mac]  # macOS-specific optimizations
```

#### Troubleshooting macOS

**Issue**: `dyld: Library not loaded` error
- **Solution**: Ensure Xcode Command Line Tools are installed: `xcode-select --install`

**Issue**: Permission denied when running binary
- **Solution**: `chmod +x /path/to/haforu`

**Issue**: "Cannot verify developer" warning
- **Solution**: Right-click the binary and select "Open", or run: `xattr -d com.apple.quarantine /path/to/haforu`

### Linux

#### Manylinux Wheels (Recommended)

Compatible with most Linux distributions:

```bash
pip install haforu
```

The manylinux wheels work on:
- Ubuntu 18.04+
- Debian 10+
- CentOS 7+
- Fedora 30+
- Other glibc 2.17+ distributions

#### Building from Source

Requirements:
- Rust 1.70+
- Python 3.8+
- GCC or Clang
- pkg-config

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config python3-dev

# Install dependencies (Fedora/RHEL)
sudo dnf install -y gcc gcc-c++ pkg-config python3-devel

# Clone and build
git clone https://github.com/fontsimi/haforu.git
cd haforu
./scripts/build.sh all

# Install
pip install target/wheels/*.whl
```

#### Platform-Specific Extras

```bash
pip install haforu[linux]  # Linux-specific optimizations
```

#### Troubleshooting Linux

**Issue**: `error while loading shared libraries`
- **Solution**: Install missing system libraries:
  ```bash
  sudo apt-get install -y libfontconfig1-dev libfreetype6-dev
  ```

**Issue**: Permission denied accessing fonts
- **Solution**: Ensure font directories are readable:
  ```bash
  chmod -R a+r /usr/share/fonts
  fc-cache -fv
  ```

**Issue**: ImportError with Python bindings
- **Solution**: Verify glibc version: `ldd --version` (need 2.17+)

### Windows

#### Python Wheel (Recommended)

```bash
pip install haforu
```

#### Building from Source

Requirements:
- Rust 1.70+ (via rustup-init.exe)
- Python 3.8+ (from python.org)
- Visual Studio 2019+ with C++ Build Tools

```bash
# In PowerShell or Command Prompt
git clone https://github.com/fontsimi/haforu.git
cd haforu

# Build (using PowerShell)
.\scripts\build.ps1 all  # Or use build.sh in Git Bash

# Install
pip install target/wheels/*.whl
```

#### Platform-Specific Extras

```bash
pip install haforu[windows]  # Windows-specific optimizations
```

#### Troubleshooting Windows

**Issue**: `LINK : fatal error LNK1181: cannot open input file`
- **Solution**: Install Visual Studio Build Tools with C++ components

**Issue**: `rustc.exe not found`
- **Solution**: Add Rust to PATH or restart terminal after installing Rust

**Issue**: Python module not found
- **Solution**: Ensure Python Scripts directory is in PATH:
  ```cmd
  set PATH=%PATH%;%USERPROFILE%\AppData\Local\Programs\Python\Python312\Scripts
  ```

## Verification

After installation, verify everything works:

### Python Package

```python
import haforu
print(haforu.__version__)
print(haforu.is_available())
```

### CLI Tools

```bash
# Rust CLI
haforu version

# Python CLI
python -m haforu version
haforu-py version  # If installed globally
```

### Run Demos

```bash
# Clone repo for test data
git clone https://github.com/fontsimi/haforu.git
cd haforu

# Run all demos
./scripts/run.sh all
```

## Installation Options

### From PyPI (Stable Releases)

```bash
pip install haforu
```

### From GitHub (Latest Development)

```bash
pip install git+https://github.com/fontsimi/haforu.git
```

### From Local Source (Development)

```bash
git clone https://github.com/fontsimi/haforu.git
cd haforu
maturin develop --release --features python
```

### With Optional Dependencies

```bash
# All optional dependencies
pip install haforu[all]

# Development dependencies
pip install haforu[dev]

# Platform-specific
pip install haforu[mac]    # macOS
pip install haforu[linux]  # Linux
pip install haforu[windows]  # Windows
```

## Environment Variables

### HAFORU_BIN

Points to the Rust CLI binary location:

```bash
export HAFORU_BIN=/path/to/haforu
```

This is used by FontSimi and other tools that shell out to the CLI.

### Recommended Setup

Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):

```bash
# For local development
export HAFORU_BIN="$HOME/Developer/haforu/target/release/haforu"

# For installed version
export HAFORU_BIN="$(which haforu)"
```

## Docker Installation

A Dockerfile is provided for containerized usage:

```bash
# Build image
docker build -t haforu:latest .

# Run container
docker run --rm -i haforu:latest batch < jobs.json
```

## Uninstallation

### Python Package

```bash
pip uninstall haforu
```

### Rust Binary

```bash
cargo uninstall haforu
```

### Clean Build Artifacts

```bash
cd haforu
cargo clean
rm -rf target/ .venv/ dist/ build/
```

## Getting Help

- **Documentation**: https://github.com/fontsimi/haforu
- **Issues**: https://github.com/fontsimi/haforu/issues
- **Discussions**: https://github.com/fontsimi/haforu/discussions

## Next Steps

- Read the [README](README.md) for usage examples
- Try the [examples](examples/) directory
- Check [ARCHITECTURE](ARCHITECTURE.md) for technical details
- Review [CHANGELOG](CHANGELOG.md) for recent updates
