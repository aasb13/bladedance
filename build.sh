#!/bin/bash
set -e

if [ ! -d build ]; then
    meson setup build
fi

# Build the static lib
cargo build --release --manifest-path src/rust/Cargo.toml --target-dir build/cargo
cp build/cargo/release/librust_core.a build/librust_core.a

# Build Rust modules — one cargo build for all
for rs in src/modules/*.rs; do
    name=$(basename "$rs" .rs)
    mkdir -p build/cargo-modules/"$name"/src
    cp "$rs" build/cargo-modules/"$name"/src/lib.rs

    cat > build/cargo-modules/"$name"/Cargo.toml <<EOF
[package]
name = "$name"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"

[dependencies]
mongodb = "3.1"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
async-trait = "0.1"
rust_core = { path = "../../../src/rust" }
tracing = "0.1"
EOF

    cargo build --release \
        --manifest-path build/cargo-modules/"$name"/Cargo.toml \
        --target-dir build/cargo-modules-target
    
    # Copy the built module to the run modules directory so it's ready to load
    cp build/cargo-modules-target/release/lib"$name".so run/modules/"$name".so
done

# Build and install
meson compile -C build
cp build/cargo/release/librust_core.a build/librust_core.a
meson install -C build