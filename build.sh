#!/bin/bash
set -e

if [ ! -d build ]; then
    meson setup build
fi

# Build Rust library first
cargo build --release --manifest-path src/rust/Cargo.toml --target-dir build/cargo

# Copy the updated Rust library to build directory
cp build/cargo/release/librust_core.a build/librust_core.a

# Build the main project
meson compile -C build
meson install -C build

cd run
./inspircd restart
