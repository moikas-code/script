# Development Setup Guide

This guide will help you set up your development environment for contributing to the Script programming language.

## Table of Contents

- [Prerequisites](#prerequisites)
- [System Requirements](#system-requirements)
- [Environment Setup](#environment-setup)
- [Development Tools](#development-tools)
- [IDE Configuration](#ide-configuration)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

1. **Rust Toolchain** (latest stable)
2. **Git** (version 2.20+)
3. **C/C++ Compiler** (for native dependencies)

### Platform-Specific Requirements

#### Linux (Ubuntu/Debian)
```bash
# Update package list
sudo apt update

# Install essential build tools
sudo apt install -y build-essential curl git

# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install additional dependencies
sudo apt install -y pkg-config libssl-dev
```

#### Linux (CentOS/RHEL/Fedora)
```bash
# For CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y curl git openssl-devel

# For Fedora
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y curl git openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Windows

**Option 1: Native Windows (Recommended)**
1. Install **Visual Studio Build Tools** or **Visual Studio Community**
   - Download from: https://visualstudio.microsoft.com/downloads/
   - Select "C++ build tools" workload
   
2. Install **Git for Windows**
   - Download from: https://git-scm.com/download/win
   
3. Install **Rust**
   - Download from: https://rustup.rs/
   - Run the installer and follow prompts

**Option 2: WSL2 (Windows Subsystem for Linux)**
1. Enable WSL2 and install Ubuntu
2. Follow the Linux (Ubuntu/Debian) instructions above

## System Requirements

### Minimum Requirements
- **CPU**: 2 cores, 2.0 GHz
- **RAM**: 4 GB
- **Storage**: 2 GB free space
- **OS**: 
  - Linux (kernel 3.2+)
  - macOS 10.12+
  - Windows 10 (build 1903+)

### Recommended Requirements
- **CPU**: 4+ cores, 3.0+ GHz
- **RAM**: 8+ GB
- **Storage**: 10+ GB SSD
- **OS**: Latest stable versions

## Environment Setup

### 1. Clone the Repository

```bash
# Clone the main repository
git clone https://github.com/moikapy/script-lang.git
cd script-lang

# Or clone your fork
git clone https://github.com/your-username/script-lang.git
cd script-lang

# Add upstream remote (if using a fork)
git remote add upstream https://github.com/moikapy/script-lang.git
```

### 2. Rust Configuration

```bash
# Ensure you have the latest stable Rust
rustup update stable
rustup default stable

# Install additional components
rustup component add rustfmt clippy

# Install useful cargo tools
cargo install cargo-edit      # For managing dependencies
cargo install cargo-watch    # For automatic rebuilding
cargo install cargo-expand   # For macro expansion
cargo install cargo-audit    # For security auditing
```

### 3. Build the Project

```bash
# Build in debug mode (faster compilation)
cargo build

# Build in release mode (optimized)
cargo build --release

# Build with all features
cargo build --all-features
```

### 4. Verify Installation

```bash
# Run the test suite
cargo test

# Run a simple example
cargo run examples/hello.script

# Start the REPL
cargo run
```

## Development Tools

### Essential Tools

#### 1. Cargo Extensions
```bash
# Install recommended cargo extensions
cargo install cargo-watch     # Auto-rebuild on file changes
cargo install cargo-expand    # Expand macros for debugging
cargo install cargo-audit     # Security vulnerability scanning
cargo install cargo-outdated  # Check for outdated dependencies
cargo install cargo-tree      # Visualize dependency tree
```

#### 2. Development Workflow Tools
```bash
# For continuous testing during development
cargo watch -x test

# For continuous building and running
cargo watch -x "run examples/hello.script"

# For benchmarking
cargo bench

# For security auditing
cargo audit
```

#### 3. Debugging Tools
```bash
# Install LLDB (Linux/macOS)
# Ubuntu/Debian
sudo apt install lldb

# macOS (via Homebrew)
brew install llvm

# Generate debug symbols
cargo build --profile dev
```

### Optional but Recommended Tools

#### 1. Performance Profiling
```bash
# Install perf (Linux only)
sudo apt install linux-perf  # Ubuntu/Debian
sudo yum install perf        # CentOS/RHEL

# For cross-platform profiling
cargo install flamegraph
```

#### 2. Code Coverage
```bash
# Install tarpaulin for coverage reports
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out html
```

#### 3. Documentation Tools
```bash
# Generate and open documentation
cargo doc --open

# Install mdbook for extended documentation
cargo install mdbook
```

## IDE Configuration

### Visual Studio Code (Recommended)

#### Required Extensions
1. **rust-analyzer** - Language server for Rust
2. **CodeLLDB** - Debugger for Rust
3. **crates** - Manage Cargo dependencies
4. **Better TOML** - Better TOML syntax highlighting

#### Recommended Extensions
1. **Error Lens** - Show errors inline
2. **GitLens** - Enhanced Git integration
3. **Bracket Pair Colorizer** - Better bracket matching
4. **Test Explorer UI** - Better test running

#### VS Code Configuration
Create `.vscode/settings.json`:
```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.procMacro.enable": true,
    "editor.formatOnSave": true,
    "editor.rulers": [100],
    "files.trimTrailingWhitespace": true,
    "files.insertFinalNewline": true
}
```

Create `.vscode/launch.json` for debugging:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Script REPL",
            "cargo": {
                "args": ["build"],
                "filter": {
                    "name": "script-lang",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        },
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Script File",
            "cargo": {
                "args": ["build"],
                "filter": {
                    "name": "script-lang",
                    "kind": "bin"
                }
            },
            "args": ["examples/hello.script"],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

### IntelliJ IDEA / CLion

1. Install the **Rust plugin**
2. Import the project as a Cargo project
3. Configure run configurations for different targets

### Vim/Neovim

#### Using vim-plug
```vim
" Add to your .vimrc or init.vim
Plug 'rust-lang/rust.vim'
Plug 'neoclide/coc.nvim', {'branch': 'release'}
Plug 'dense-analysis/ale'

" Configure rust.vim
let g:rustfmt_autosave = 1

" Configure CoC for rust-analyzer
" Add to coc-settings.json:
" {
"   "rust-analyzer.server.path": "rust-analyzer",
"   "rust-analyzer.cargo.loadOutDirsFromCheck": true
" }
```

## Verification

### Test Your Setup

1. **Basic functionality**:
   ```bash
   # Build the project
   cargo build
   
   # Run tests
   cargo test
   
   # Run the REPL
   cargo run
   ```

2. **Test parsing**:
   ```bash
   # Parse a script file
   cargo run examples/hello.script
   
   # Run in token mode
   cargo run examples/hello.script --tokens
   ```

3. **Test benchmarks**:
   ```bash
   # Run performance benchmarks
   cargo bench
   ```

4. **Test code quality**:
   ```bash
   # Check formatting
   cargo fmt --check
   
   # Run linter
   cargo clippy -- -D warnings
   
   # Generate documentation
   cargo doc
   ```

### Expected Output

After running `cargo run`, you should see:
```
Script Language REPL v0.1.0
Type 'exit' to quit, 'help' for commands
> 
```

After running `cargo test`, you should see all tests passing:
```
running 45 tests
test lexer::tests::test_identifier_recognition ... ok
test lexer::tests::test_number_parsing ... ok
test parser::tests::test_expression_parsing ... ok
...

test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Troubleshooting

### Common Issues

#### 1. Rust Installation Issues

**Problem**: `cargo` command not found
```bash
# Solution: Ensure Rust is in your PATH
source $HOME/.cargo/env

# Or add to your shell profile (.bashrc, .zshrc, etc.)
export PATH="$HOME/.cargo/bin:$PATH"
```

**Problem**: Rust version too old
```bash
# Solution: Update Rust
rustup update stable
```

#### 2. Build Issues

**Problem**: Compilation errors with native dependencies
```bash
# Linux: Install development packages
sudo apt install build-essential pkg-config libssl-dev

# macOS: Install Xcode Command Line Tools
xcode-select --install

# Windows: Install Visual Studio Build Tools
```

**Problem**: "could not find `cc`" error
```bash
# Install C compiler
# Linux
sudo apt install gcc

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools with C++ support
```

#### 3. Performance Issues

**Problem**: Slow compilation times
```bash
# Use faster linker (Linux)
sudo apt install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache

# Increase parallel jobs
export CARGO_BUILD_JOBS=4
```

**Problem**: High memory usage during compilation
```bash
# Reduce codegen units
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
```

#### 4. IDE Issues

**Problem**: rust-analyzer not working
```bash
# Update rust-analyzer
rustup component add rust-analyzer

# Check language server logs in VS Code
# View > Command Palette > "Developer: Reload Window"
```

**Problem**: Debugging not working
- Ensure CodeLLDB extension is installed
- Check that debug symbols are enabled (`cargo build`)
- Verify launch configuration is correct

#### 5. Platform-Specific Issues

**Windows-specific**:
- Use PowerShell or Command Prompt, not Git Bash for cargo commands
- Ensure Windows Defender doesn't block compilation
- Use `cargo build` instead of `cargo run` if execution fails

**macOS-specific**:
- Install Xcode Command Line Tools if linking fails
- Use Homebrew for additional dependencies
- Check that system Python doesn't conflict

**Linux-specific**:
- Install `pkg-config` and development headers
- Ensure `libc6-dev` is installed
- Check that the correct `gcc` version is used

### Getting Help

If you encounter issues not covered here:

1. **Check the issue tracker**: https://github.com/moikapy/script-lang/issues
2. **Search existing discussions**: Look for similar problems
3. **Create a new issue**: Include:
   - Operating system and version
   - Rust version (`rustc --version`)
   - Complete error messages
   - Steps to reproduce

### Environment Validation Script

Save this as `scripts/validate_setup.sh` and run it to check your environment:

```bash
#!/bin/bash

echo "=== Script Language Development Environment Validation ==="
echo

# Check Rust installation
echo "Checking Rust installation..."
if command -v rustc &> /dev/null; then
    echo "✓ Rust compiler: $(rustc --version)"
else
    echo "✗ Rust compiler not found"
    exit 1
fi

if command -v cargo &> /dev/null; then
    echo "✓ Cargo: $(cargo --version)"
else
    echo "✗ Cargo not found"
    exit 1
fi

# Check required components
echo "✓ rustfmt: $(rustfmt --version)"
echo "✓ clippy: $(cargo clippy --version 2>/dev/null || echo 'Not installed')"

# Check build tools
echo
echo "Checking build tools..."
if command -v cc &> /dev/null; then
    echo "✓ C compiler found"
else
    echo "✗ C compiler not found - install build tools"
fi

# Test project build
echo
echo "Testing project build..."
if cargo build --quiet; then
    echo "✓ Project builds successfully"
else
    echo "✗ Project build failed"
    exit 1
fi

# Test project tests
echo
echo "Testing project tests..."
if cargo test --quiet; then
    echo "✓ All tests pass"
else
    echo "✗ Some tests failed"
fi

echo
echo "=== Setup validation complete! ==="
```

Run with:
```bash
chmod +x scripts/validate_setup.sh
./scripts/validate_setup.sh
```

Your development environment is now ready! See [CONTRIBUTING.md](CONTRIBUTING.md) for the next steps.