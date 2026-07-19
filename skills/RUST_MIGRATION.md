---
name: cpp-to-rust-migration
description: Use this skill when the user wants to incrementally rewrite, port, or migrate a C++ project (or part of one) to Rust — including strangler-fig style migrations, setting up cxx/cbindgen/bindgen interop, converting individual modules/files, or planning a phased rewrite strategy. Trigger on phrases like "migrate this C++ to Rust", "gradually rewrite in Rust", "interop between C++ and Rust", "port this module to Rust", or when a C++ codebase is present and the goal is a Rust transition. Not for greenfield Rust projects with no C++ source, or one-off C++ syntax questions unrelated to migration.
---

# Gradual C++ → Rust Migration Guide

## Overview

Rewriting a C++ codebase in Rust all at once is risky: it blocks shipping, throws
away a working system, and produces a giant unreviewable diff. The proven
alternative is the **strangler fig** pattern — build a small Rust "shell" or
sub-module, wire it to the existing C++ via a stable FFI boundary, and migrate
one component at a time while both languages coexist and the whole system keeps
building and passing tests after every step.

This skill covers: choosing what to migrate first, picking the right interop
tool, structuring the repo so C++ and Rust build together, converting idioms
piece by piece, and validating each increment.

## Core principles

1. **Never break the build.** After every migrated unit, the project must
   still compile and its existing test suite must still pass. If a step can't
   be done keeping things green, split it into smaller steps.
2. **Migrate along module boundaries, not arbitrary lines.** Pick units with
   the fewest, cleanest interfaces to the rest of the code (a parser, a codec,
   a data structure, a leaf utility library) before touching central/shared
   headers or anything with pervasive raw-pointer aliasing.
3. **Bottom-up, leaves first.** Convert code with no/few internal dependents
   before code many things depend on. Dependency graph analysis (see below)
   tells you the order.
4. **One stable FFI boundary, not scattered unsafe glue.** Concentrate all
   cross-language calls behind a small, explicit interface (a bridge module or
   a handful of `extern "C"` functions) rather than letting `unsafe` bleed
   throughout the Rust code.
5. **Test at the seam.** Every ported unit needs equivalence tests — ideally
   the *same* existing C++ tests/fixtures run against the new Rust
   implementation — before the old code is deleted.
6. **Delete the old code.** A gradual migration that never removes the C++ it
   replaced isn't done; each increment should end with dead C++ deleted, not
   just new Rust added alongside it.

## Step 0: Assess and plan

Before writing any Rust, build a picture of the codebase:

- **Dependency/coupling analysis.** Find which files/modules are leaves
  (few internal dependents) vs. hubs (many dependents). Tools: `include-what-you-use`,
  clang's `-M`/`-MM` to dump include graphs, or simply `grep -rl '#include "your_header.h"'`
  counts. Migrate leaves first.
- **Identify the API surface** each candidate module exposes to the rest of
  the program — function signatures, exported types, global state, callbacks.
  This becomes the FFI contract.
- **Flag risk areas early**: template-heavy generic code, macro-heavy code,
  pervasive shared mutable state, custom allocators, exceptions crossing
  module boundaries, and anything with manual reference counting — these are
  the hardest to port and interop with, and should be scheduled later or
  handled with extra care.
- **Pick the interop tool** (see next section) based on the shape of the
  boundary.
- **Write down the target end-state repo layout** and the order of modules to
  migrate, and get it agreed before starting — a mid-migration change of plan
  is expensive.

## Step 1: Choose the interop mechanism

| Situation | Tool | Notes |
|---|---|---|
| Rich C++ APIs (classes, `std::string`, `std::vector`, exceptions) both directions | **cxx** (`cxx.rs`) | Safest, most ergonomic; generates matching C++ and Rust bindings from a shared `#[cxx::bridge]` interface; enforces a limited, safe subset of types crossing the boundary |
| Rust exposes a C ABI to be called from C++ | **cbindgen** | Generates a C/C++ header from Rust `extern "C"` functions; simplest option when Rust is "new leaf, C++ is caller" |
| C++ exposes a C ABI to be called from Rust (or wrapping a C library) | **bindgen** | Generates Rust FFI bindings from C/C++ headers; best paired with a thin `extern "C"` shim if the headers are pure C++ (bindgen handles C much better than C++ templates/overloads) |
| Complex bidirectional C++ (templates, inheritance, operator overloading) | **cxx**, with a hand-written shim layer for anything cxx's safe subset can't express | Don't fight cxx's restrictions with excessive escape hatches — that's a sign the module needs a real redesign, not just a wrapper |
| Just need to share build/test infra, not call across a live boundary yet | Keep the two builds separate, migrate build system first (see Step 2) | |

Default recommendation for most C++ projects: **cxx**, because it keeps the
FFI boundary memory-safe and typed on both sides, catches mismatches at
compile time, and scales well as more modules move over.

## Step 2: Set up the build so both languages coexist

- **CMake + Corrosion**: if the project uses CMake, the `corrosion` CMake
  package integrates a Cargo-built Rust static/shared library into the CMake
  build graph cleanly (`corrosion_import_crate(...)`).
- **Cargo build.rs**: if Rust is the "outer" build driving C++, use the `cc`
  crate in `build.rs` to compile the remaining C++ sources/objects, or `cmake`
  crate to invoke an existing CMake subproject.
- **Bazel**: `rules_rust` + existing `cc_library` targets, with an explicit
  `rust_static_library` / `cc_library` dependency edge at the FFI boundary.
- Keep the Rust crate(s) in a `rust/` subdirectory of the existing repo rather
  than a separate repo, so history and CI stay unified during the transition.
- Get CI green with **zero Rust code migrated yet** — just the empty/minimal
  crate wired into the build — before porting anything. This proves the
  scaffolding works in isolation from the actual migration risk.

## Step 3: Migrate one module at a time

For each module, in dependency order (leaves first):

1. Write the Rust implementation from scratch — don't attempt a mechanical
   line-by-line transliteration. C++ idioms (raw pointers, manual RAII,
   inheritance-based polymorphism, exceptions) have different idiomatic Rust
   equivalents (see the idiom table below). A faithful behavioral port, not a
   faithful *syntactic* port, is the goal.
2. Expose it through the FFI boundary chosen in Step 1, matching the existing
   C++ API surface so callers don't need to change yet (or change minimally).
3. Point the existing call sites at the new Rust implementation.
4. Run the *existing* C++ test suite against it unmodified first — this
   proves behavioral equivalence using tests that were already trusted.
   Then add Rust-native unit tests (`#[test]`) for anything the old suite
   didn't cover.
5. Delete the old C++ implementation for that module (keep the header/shim
   only if other unconverted modules still include it).
6. Commit as an isolated, revertable change. One module per PR/commit where
   possible.

## Common C++ → Rust idiom mapping

| C++ | Rust |
|---|---|
| Raw pointer / reference ownership | `Box<T>` (owned), `&T`/`&mut T` (borrowed), `Rc<RefCell<T>>` / `Arc<Mutex<T>>` for shared mutable state |
| `std::unique_ptr<T>` | `Box<T>` |
| `std::shared_ptr<T>` | `Rc<T>` (single-threaded) or `Arc<T>` (multi-threaded) |
| Manual destructors / RAII | `Drop` trait (usually simpler, no need to hand-manage) |
| Inheritance + virtual methods | Traits + trait objects (`dyn Trait`) or enums with `match` for closed sets |
| Templates | Generics + trait bounds (monomorphized like templates, but with real interface checking) |
| Exceptions | `Result<T, E>` propagated with `?`; reserve `panic!` for truly unrecoverable bugs |
| `nullptr` checks | `Option<T>` — encode absence in the type instead of a sentinel value |
| Macros for codegen | Rust generics/traits first; `macro_rules!` only if genuinely needed |
| Global mutable state / singletons | `OnceLock`/`LazyLock`, or better, pass state explicitly — this is a good opportunity to remove hidden global state during the port |
| Manual thread synchronization | Prefer the type system (`Send`/`Sync`, `Mutex<T>` owning its data) over discipline-based correctness |

## Validating each step

- Re-run the full existing test suite (both C++ and any new Rust tests) after
  every module migration — not just unit tests for the new module.
- Consider differential/fuzz testing for anything with subtle behavior:
  run old C++ and new Rust implementations side by side on the same random
  inputs and diff outputs, especially for parsers, codecs, and numeric code.
- Use `cargo miri` for the new Rust code if it does anything `unsafe` at the
  FFI boundary, to catch UB early.
- Track migration progress with a simple checklist/table of modules
  (leaf → hub order) and their status: not started / in progress / ported /
  old code deleted. Keep this visible to reviewers.

## Pitfalls to avoid

- **Scattering `unsafe` everywhere** instead of concentrating it at the FFI
  boundary — defeats the safety benefit of the migration.
- **Porting syntax instead of redesigning idiom** — a Rust file full of
  `Rc<RefCell<...>>` mimicking C++ shared-mutable-pointer patterns everywhere
  is usually a sign the ownership model wasn't rethought.
- **Migrating hubs before leaves** — creates a long-lived, painful mixed state
  where a core module has two implementations and everything depends on the
  FFI shim.
- **Skipping the "delete old code" step** — leads to permanent dual
  maintenance and bit rot in the abandoned C++.
- **No agreed target layout** — teams that migrate ad hoc often end up with
  circular C++↔Rust↔C++ call chains that are harder to reason about than the
  original code.