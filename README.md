# Bladedance

Bladedance is a Rust IRC server forked from [InspIRCd](https://www.inspircd.org) v4.10.1.

It is under active, opinionated development and is **not** intended to be a drop-in replacement for InspIRCd.

## Key differences from upstream

- Core components are being progressively rewritten in Rust one by one.
- All server‑to‑server linking (the SpanningTree protocol) has been removed. This means external services like Anope are not supported and will not work — the IRC server will contain or already contains everything it needs internally.
- The build system is Meson + Cargo exclusively. The original Perl configure script and CMake files have been completely removed.
- The configuration format has been migrated from InspIRCd's legacy format to **TOML**.

## Building

```sh
./build.sh
```

The build script:
- Sets up Meson build directory if it doesn't exist
- Builds the Rust core library (`src/rust/`)
- Builds all Rust modules in `src/modules/*.rs`
- Compiles the C++ core and modules via Meson/Ninja
- Installs everything to the `run/` directory

NOTE: Windows is **not supported** until a stable version is released.

## Running

```sh
./start.sh
```

This runs the compiled server from `run/bin/inspircd`.

## Configuration

The configuration format is now **TOML-based**. The legacy InspIRCd configuration files (`.conf`) are no longer used.

## Project Status

This is a migration in progress. The end goal is a 100% Rust codebase with Cargo as the sole build system (Meson/Ninja will eventually be removed).
