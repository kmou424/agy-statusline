set shell := ["bash", "-euo", "pipefail", "-c"]

# Show available recipes
default:
    @just --list

# Fast test run via Blaze
test *args:
    blaze test {{args}}

# Standard test run via pinned Cargo toolchain
test-cargo *args:
    cargo test {{args}}

# Fast type and borrow check via Blaze
check *args:
    blaze check {{args}}

# Build debug binary via pinned Cargo toolchain
build *args:
    cargo build {{args}}

# Fast incremental build via Blaze
build-fast *args:
    blaze build {{args}}

# Build optimized release binary via pinned Cargo toolchain
build-release *args:
    cargo build --release {{args}}

# Run binary via Blaze
run *args:
    blaze run -- {{args}}

# Format Rust sources
fmt:
    cargo fmt --all

# Check Rust formatting without modifying files
fmt-check:
    cargo fmt --all --check

# Run Clippy with warnings denied
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Fast local verification loop (fmt, clippy, Blaze tests)
verify: fmt-check clippy test

# Full verification against the pinned Rust toolchain
ci: fmt-check clippy test-cargo

# Inspect Blaze daemon status
blaze-status:
    blaze status

# Stop Blaze background daemon
blaze-stop:
    blaze stop

# Remove Cargo build artifacts
clean:
    cargo clean
