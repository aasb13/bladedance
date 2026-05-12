#!/bin/bash
set -e

if [ ! -d build ]; then
    meson setup build
fi

# Build Rust library first
cd src/rust
cargo build --release
cd ../..

# Copy the updated Rust library to build directory
cp src/rust/target/release/librust_core.a build/librust_core.a

# Build the main project
meson compile -C build
meson install -C build

cd run
./inspircd restart
