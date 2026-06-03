# Bladedance

Bladedance is a Rust IRC server forked from [InspIRCd](https://www.inspircd.org) v4.10.1.

It is under active, opinionated development and is **not** intended to be a drop-in replacement for InspIRCd.

## Key differences from upstream

- Core components are being progressively rewritten in Rust one by one.
- All server‑to‑server linking (the SpanningTree protocol) has been removed. (Which means things like anope are not going to work, but they do not need to, since the IRC server will have everything inside of itself already)
- The build system is Meson + Cargo; the original Perl configure and CMake files are gone.

## Building

```sh
./build.sh
```

NOTE: Windows is **not supported** until a stable version is released.

## Todo

- [ ] Move the config system to TOML
