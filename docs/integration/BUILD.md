# Build System and Deployment Guide

This guide covers building, configuring, optimizing, and deploying Script applications and the Script runtime itself.

## Table of Contents

- [Building Script from Source](#building-script-from-source)
- [Build Configuration](#build-configuration)
- [Optimization Levels](#optimization-levels)
- [Cross-Platform Building](#cross-platform-building)
- [Deployment Strategies](#deployment-strategies)
- [Container Deployment](#container-deployment)
- [CI/CD Integration](#cicd-integration)
- [Performance Tuning](#performance-tuning)
- [Distribution](#distribution)
- [Troubleshooting](#troubleshooting)

## Building Script from Source

### Prerequisites

Ensure you have the required dependencies:

```bash
# Rust toolchain (minimum version 1.70)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Platform-specific dependencies
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# macOS
xcode-select --install
brew install pkg-config openssl

# Windows (using chocolatey)
choco install visualstudio2022buildtools
choco install pkg-config openssl
```

### Basic Build

```bash
# Clone the repository
git clone https://github.com/moikapy/script.git
cd script

# Debug build (development)
cargo build

# Release build (production)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Build Profiles

The project includes several build profiles in `Cargo.toml`:

```toml
[profile.dev]
opt-level = 0
debug = true
split-debuginfo = 'unpacked'
strip = false
debug-assertions = true
overflow-checks = true
lto = false
panic = 'unwind'
incremental = true
codegen-units = 256
rpath = false

[profile.release]
opt-level = 3
debug = false
strip = true
debug-assertions = false
overflow-checks = false
lto = true
panic = 'abort'
incremental = false
codegen-units = 1
rpath = false

[profile.bench]
inherits = "release"
debug = true
strip = false

[profile.dev-opt]
inherits = "dev"
opt-level = 2
debug-assertions = false

[profile.release-debug]
inherits = "release"
debug = true
strip = false
```

### Feature Flags

Script supports various feature flags for customization:

```bash
# Build with all features
cargo build --release --all-features

# Build with specific features
cargo build --release --features "jit,profiling,async"

# Build without default features
cargo build --release --no-default-features --features "core"
```

Available features:

| Feature | Description | Default |
|---------|-------------|---------|
| `jit` | JIT compilation with Cranelift | Yes |
| `profiling` | Runtime profiling support | Yes |
| `async` | Async/await support | Yes |
| `ffi` | Foreign function interface | Yes |
| `networking` | Network I/O support | Yes |
| `filesystem` | File system access | Yes |
| `regex` | Regular expression support | Yes |
| `json` | JSON parsing/serialization | Yes |
| `compression` | Compression algorithms | No |
| `cryptography` | Cryptographic functions | No |
| `gui` | GUI toolkit bindings | No |
| `game` | Game development features | No |
| `web` | Web server capabilities | No |

## Build Configuration

### Environment Variables

Configure the build process with environment variables:

```bash
# Optimization settings
export SCRIPT_OPTIMIZE_FOR="speed"  # or "size"
export SCRIPT_TARGET_CPU="native"   # or specific CPU
export SCRIPT_ENABLE_LTO="true"     # Link-time optimization

# Memory settings
export SCRIPT_DEFAULT_HEAP_SIZE="64MB"
export SCRIPT_MAX_HEAP_SIZE="1GB"
export SCRIPT_STACK_SIZE="8KB"

# Debug settings
export SCRIPT_ENABLE_ASSERTIONS="false"
export SCRIPT_ENABLE_PROFILING="true"
export SCRIPT_LOG_LEVEL="info"

# Build with custom settings
cargo build --release
```

### Custom build.rs Configuration

For advanced build customization, modify the `build.rs` file:

```rust
// build.rs
use std::env;

fn main() {
    // Platform-specific optimizations
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    
    match target_os.as_str() {
        "linux" => {
            println!("cargo:rustc-link-lib=dl");
            if target_arch == "x86_64" {
                println!("cargo:rustc-cfg=target_has_avx2");
            }
        }
        "macos" => {
            println!("cargo:rustc-link-lib=framework=Security");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=ws2_32");
            println!("cargo:rustc-link-lib=userenv");
        }
        _ => {}
    }
    
    // CPU-specific optimizations
    if let Ok(cpu) = env::var("SCRIPT_TARGET_CPU") {
        println!("cargo:rustc-env=TARGET_CPU={}", cpu);
    }
    
    // Memory configuration
    if let Ok(heap_size) = env::var("SCRIPT_DEFAULT_HEAP_SIZE") {
        println!("cargo:rustc-env=DEFAULT_HEAP_SIZE={}", heap_size);
    }
}
```

## Optimization Levels

### Compiler Optimizations

Configure Rust compiler optimizations:

```toml
# Cargo.toml optimization profiles
[profile.fast]
inherits = "release"
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
```

### Runtime Optimizations

Configure Script runtime optimizations:

```rust
use script::{RuntimeConfig, OptimizationLevel};

let config = RuntimeConfig {
    optimization_level: OptimizationLevel::Aggressive,
    enable_jit: true,
    jit_threshold: 100,  // Functions called 100+ times get JIT compiled
    inline_threshold: 50,
    unroll_loops: true,
    optimize_tail_calls: true,
    dead_code_elimination: true,
    constant_folding: true,
    ..Default::default()
};
```

### Platform-Specific Optimizations

```bash
# x86_64 with AVX2
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2" cargo build --release

# ARM64 optimizations
RUSTFLAGS="-C target-cpu=apple-m1" cargo build --release --target aarch64-apple-darwin

# Size optimization for embedded
RUSTFLAGS="-C opt-level=z -C strip=symbols" cargo build --release
```

## Cross-Platform Building

### Target Installation

```bash
# Install cross-compilation targets
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu
rustup target add wasm32-unknown-unknown
```

### Cross-Compilation

```bash
# Linux to Windows
sudo apt install gcc-mingw-w64
cargo build --release --target x86_64-pc-windows-gnu

# Linux to macOS (requires macOS SDK)
cargo build --release --target x86_64-apple-darwin

# Static Linux binary
cargo build --release --target x86_64-unknown-linux-musl

# WebAssembly
cargo build --release --target wasm32-unknown-unknown
```

### Docker-Based Cross-Compilation

```dockerfile
# Dockerfile.cross
FROM rust:1.70

# Install cross-compilation tools
RUN apt-get update && apt-get install -y \
    gcc-mingw-w64 \
    gcc-aarch64-linux-gnu \
    musl-tools

# Install Rust targets
RUN rustup target add x86_64-pc-windows-gnu
RUN rustup target add aarch64-unknown-linux-gnu
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .

# Build for multiple targets
RUN cargo build --release --target x86_64-unknown-linux-gnu
RUN cargo build --release --target x86_64-pc-windows-gnu
RUN cargo build --release --target aarch64-unknown-linux-gnu
RUN cargo build --release --target x86_64-unknown-linux-musl
```

Build script:
```bash
#!/bin/bash
# build-cross.sh

docker build -f Dockerfile.cross -t script-cross .
docker run --rm -v $(pwd)/target:/app/target script-cross
```

## Deployment Strategies

### Standalone Binary Deployment

```bash
# Create deployment package
mkdir -p deploy/script
cp target/release/script deploy/script/
cp -r examples deploy/script/
cp -r docs deploy/script/
cp README.md LICENSE deploy/script/

# Create installation script
cat > deploy/script/install.sh << 'EOF'
#!/bin/bash
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "${INSTALL_DIR}"
cp script "${INSTALL_DIR}/"
echo "Script installed to ${INSTALL_DIR}/script"
echo "Add ${INSTALL_DIR} to your PATH if not already present"
EOF

chmod +x deploy/script/install.sh

# Create archive
tar -czf script-$(uname -s)-$(uname -m).tar.gz -C deploy script
```

### Library Deployment

For embedding in other applications:

```toml
# Cargo.toml for library distribution
[lib]
name = "script"
crate-type = ["cdylib", "rlib"]

[package.metadata.capi]
min_version = "0.9.0"

[package.metadata.capi.header]
name = "script"
subdirectory = "script"
generation = true
```

Build C-compatible library:

```bash
# Install cargo-c for C-compatible builds
cargo install cargo-c

# Build C library
cargo cinstall --release --prefix=/usr/local

# This creates:
# - libscript.so (shared library)
# - libscript.a (static library)
# - script.h (header file)
# - script.pc (pkg-config file)
```

### Package Manager Distribution

#### Cargo (Rust)

```toml
# Cargo.toml for crates.io publication
[package]
name = "script"
version = "0.1.0"
edition = "2021"
description = "A simple yet powerful programming language"
repository = "https://github.com/moikapy/script"
homepage = "https://script.org"
documentation = "https://docs.rs/script"
license = "MIT"
keywords = ["programming-language", "interpreter", "compiler"]
categories = ["development-tools", "parser-implementations"]
readme = "README.md"
```

Publish to crates.io:
```bash
cargo login
cargo publish --dry-run  # Test the package
cargo publish
```

#### Homebrew (macOS/Linux)

```ruby
# script.rb
class ScriptLang < Formula
  desc "Simple yet powerful programming language"
  homepage "https://script.org"
  url "https://github.com/moikapy/script/archive/v0.1.0.tar.gz"
  sha256 "..."
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    (testpath/"hello.script").write("print(\"Hello, World!\")")
    assert_match "Hello, World!", shell_output("#{bin}/script hello.script")
  end
end
```

#### APT Repository (Debian/Ubuntu)

```bash
# Create .deb package
cargo install cargo-deb

# Configure in Cargo.toml
[package.metadata.deb]
maintainer = "Warren Gates <warren@example.com>"
copyright = "2024, Warren Gates"
license-file = ["LICENSE", "4"]
extended-description = """\
Script is a simple yet powerful programming language designed for\
web applications and games."""
depends = "$auto"
section = "utility"
priority = "optional"
assets = [
    ["target/release/script", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/script/", "644"],
    ["examples/*", "usr/share/doc/script/examples/", "644"],
]

# Build package
cargo deb --release
```

## Container Deployment

### Docker Images

#### Production Image

```dockerfile
# Dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/script /usr/local/bin/

# Create non-root user
RUN useradd -r -s /bin/false script

USER script
WORKDIR /app

EXPOSE 8080

CMD ["script"]
```

#### Multi-stage Build with Alpine

```dockerfile
# Dockerfile.alpine
FROM rust:1.70-alpine as builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

RUN apk --no-cache add ca-certificates

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/script /usr/local/bin/

USER 1000:1000

CMD ["script"]
```

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: script
  labels:
    app: script
spec:
  replicas: 3
  selector:
    matchLabels:
      app: script
  template:
    metadata:
      labels:
        app: script
    spec:
      containers:
      - name: script
        image: script:0.1.0
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "64Mi"
            cpu: "250m"
          limits:
            memory: "128Mi"
            cpu: "500m"
        env:
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5

---
apiVersion: v1
kind: Service
metadata:
  name: script-service
spec:
  selector:
    app: script
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  script:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./scripts:/app/scripts:ro
      - ./config:/app/config:ro
    environment:
      - RUST_LOG=info
      - SCRIPT_CONFIG_PATH=/app/config
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - script
    restart: unless-stopped
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run tests
      run: cargo test --all-features
    
    - name: Run clippy
      run: cargo clippy --all-features -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check

  build:
    name: Build
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    
    - name: Build
      run: cargo build --release
    
    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: script-${{ matrix.os }}
        path: target/release/script*

  release:
    name: Release
    runs-on: ubuntu-latest
    needs: [test, build]
    if: startsWith(github.ref, 'refs/tags/')
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Download artifacts
      uses: actions/download-artifact@v3
    
    - name: Create release
      uses: softprops/action-gh-release@v1
      with:
        files: |
          script-ubuntu-latest/script
          script-windows-latest/script.exe
          script-macos-latest/script
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Docker Build and Push

```yaml
# .github/workflows/docker.yml
name: Docker

on:
  push:
    branches: [ main ]
    tags: [ 'v*' ]

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
    - name: Checkout
      uses: actions/checkout@v3
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    
    - name: Login to Docker Hub
      uses: docker/login-action@v2
      with:
        username: ${{ secrets.DOCKER_USERNAME }}
        password: ${{ secrets.DOCKER_PASSWORD }}
    
    - name: Extract metadata
      id: meta
      uses: docker/metadata-action@v4
      with:
        images: scriptlang/script
        tags: |
          type=ref,event=branch
          type=ref,event=pr
          type=semver,pattern={{version}}
          type=semver,pattern={{major}}.{{minor}}
    
    - name: Build and push
      uses: docker/build-push-action@v4
      with:
        context: .
        platforms: linux/amd64,linux/arm64
        push: true
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}
        cache-from: type=gha
        cache-to: type=gha,mode=max
```

## Performance Tuning

### Build-Time Optimizations

```bash
# Profile-guided optimization (PGO)
export RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data"
cargo build --release
# Run representative workload
./target/release/script benchmark.script
export RUSTFLAGS="-Cprofile-use=/tmp/pgo-data -Cllvm-args=-pgo-warn-missing-function"
cargo build --release

# Link-time optimization
export RUSTFLAGS="-Clto=fat -Cembed-bitcode=yes"
cargo build --release

# CPU-specific optimizations
export RUSTFLAGS="-Ctarget-cpu=native"
cargo build --release
```

### Runtime Configuration

```rust
use script::{RuntimeConfig, MemoryConfig, JitConfig};

let config = RuntimeConfig {
    memory: MemoryConfig {
        initial_heap_size: 64 * 1024 * 1024,  // 64MB
        max_heap_size: Some(1024 * 1024 * 1024),  // 1GB
        gc_threshold: 16 * 1024 * 1024,  // 16MB
        enable_gc: true,
        ..Default::default()
    },
    
    jit: JitConfig {
        enabled: true,
        threshold: 1000,  // Compile after 1000 calls
        optimization_level: 3,
        inline_threshold: 100,
        ..Default::default()
    },
    
    ..Default::default()
};
```

### Benchmarking

```bash
# Run built-in benchmarks
cargo bench

# Profile with perf (Linux)
perf record --call-graph dwarf ./target/release/script benchmark.script
perf report

# Memory profiling with valgrind
valgrind --tool=massif ./target/release/script benchmark.script

# CPU profiling with flamegraph
cargo install flamegraph
cargo flamegraph -- benchmark.script
```

## Distribution

### Release Checklist

1. **Version Bump**
   ```bash
   # Update version in Cargo.toml
   sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml
   
   # Update CHANGELOG.md
   echo "## [0.2.0] - $(date +%Y-%m-%d)" >> CHANGELOG.md
   ```

2. **Build and Test**
   ```bash
   cargo test --all-features
   cargo clippy --all-features
   cargo fmt --check
   cargo audit
   ```

3. **Create Release Artifacts**
   ```bash
   ./scripts/build-release.sh
   ```

4. **Tag and Push**
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

### Automated Release Script

```bash
#!/bin/bash
# scripts/build-release.sh

set -e

VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
TARGETS=("x86_64-unknown-linux-gnu" "x86_64-pc-windows-gnu" "x86_64-apple-darwin")

echo "Building Script v${VERSION}..."

# Clean previous builds
cargo clean

# Build for each target
for target in "${TARGETS[@]}"; do
    echo "Building for ${target}..."
    cargo build --release --target "${target}"
    
    # Create archive
    case "${target}" in
        *windows*)
            zip -j "script-v${VERSION}-${target}.zip" \
                "target/${target}/release/script.exe" \
                README.md LICENSE
            ;;
        *)
            tar -czf "script-v${VERSION}-${target}.tar.gz" \
                -C "target/${target}/release" script \
                -C ../../.. README.md LICENSE
            ;;
    esac
done

echo "Release artifacts created:"
ls -la script-v${VERSION}-*
```

## Troubleshooting

### Common Build Issues

1. **Missing Dependencies**
   ```bash
   # Error: could not find system library 'ssl'
   # Solution:
   sudo apt install libssl-dev pkg-config  # Ubuntu/Debian
   brew install openssl pkg-config          # macOS
   ```

2. **Linker Errors**
   ```bash
   # Error: linking with `cc` failed
   # Solution: Install build tools
   sudo apt install build-essential        # Ubuntu/Debian
   xcode-select --install                  # macOS
   ```

3. **Out of Memory During Build**
   ```bash
   # Reduce parallel build jobs
   cargo build --jobs 1
   
   # Or set CARGO_BUILD_JOBS environment variable
   export CARGO_BUILD_JOBS=2
   ```

4. **Cross-Compilation Issues**
   ```bash
   # Install target and linker
   rustup target add x86_64-pc-windows-gnu
   sudo apt install gcc-mingw-w64
   
   # Configure linker in .cargo/config.toml
   mkdir -p .cargo
   cat > .cargo/config.toml << EOF
   [target.x86_64-pc-windows-gnu]
   linker = "x86_64-w64-mingw32-gcc"
   EOF
   ```

### Performance Issues

1. **Slow Build Times**
   ```bash
   # Use sccache for caching
   cargo install sccache
   export RUSTC_WRAPPER=sccache
   
   # Increase parallel jobs
   export CARGO_BUILD_JOBS=8
   
   # Use faster linker
   sudo apt install lld
   export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
   ```

2. **Large Binary Size**
   ```bash
   # Enable size optimizations
   export RUSTFLAGS="-C opt-level=z -C strip=symbols"
   cargo build --release
   
   # Use UPX compression
   upx --best target/release/script
   ```

3. **Runtime Performance**
   ```bash
   # Profile the application
   cargo install flamegraph
   sudo cargo flamegraph -- slow-script.script
   
   # Enable runtime optimizations
   export SCRIPT_JIT_THRESHOLD=100
   export SCRIPT_GC_THRESHOLD=16MB
   ```

This comprehensive build and deployment guide provides everything needed to build, optimize, and distribute Script applications and the Script runtime itself across different platforms and deployment scenarios.