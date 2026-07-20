# AGENTS.md — Bladedance

This file is meant ONLY for LLM agents.

Bladedance is an IRC server forked from InspIRCd v4.10.1, in the middle of a
full, permanent migration from C++ to Rust. This file tells any AI agent
working in this repo how to build/run it and how to make migration progress
correctly. Read this in full before touching any file.

## 1. Build and run — ALWAYS use the scripts

- Build: `./build.sh`
- Run: `./start.sh`

**Never invoke `meson` or `ninja` directly** (`meson setup`, `meson compile`,
`meson install`, `ninja`, `ninja -C build`, etc. are all off-limits), and never
call `cargo build`/`cargo test` by hand as a substitute either. Always go
through `./build.sh`. Reasons this matters, not just style:

- `build.sh` does several things a bare `meson compile` will not: it builds
  the `src/rust` core static lib with Cargo, copies it to
  `build/librust_core.a` where meson's linker step expects it, dynamically
  scaffolds a throwaway Cargo project per file in `src/modules/*.rs` and
  builds each as a `cdylib`, and copies the resulting `.so` files into
  `run/modules/`. Skipping this means stale or missing Rust artifacts and a
  server that silently fails to load modules.
- It re-copies `librust_core.a` a second time after `meson compile`, which
  matters because meson's own build graph does not know about Cargo's output
  and won't rebuild/copy it for you.
- If `build/` doesn't exist yet it also runs `meson setup build` for you —
  there is no separate "first-time setup" step to remember.

If `./build.sh` fails, fix the underlying problem (missing dependency, broken
Rust code, meson.build error) — do not "unblock yourself" by dropping to
`meson`/`ninja`/`cargo` directly. If you believe the scripts themselves need a
new step (e.g. a newly added Rust module needs building, a new dependency
needs adding to the generated per-module `Cargo.toml` in `build.sh`), **edit
`build.sh` itself** so the fix is permanent and the next agent/run benefits
too. The same applies to `start.sh` if the run invocation needs to change.

## 2. The end goal: 100% Rust, meson and ninja fully removed

This is not a permanent hybrid architecture — it is a migration in progress.
The finished state has:

- Zero `.cpp`/`.h` files under `src/` (or wherever remains at that point).
- No `meson.build`, `meson_options.txt`, or Meson/Ninja dependency anywhere in
  the repo or in `build.sh`.
- A single (or small number of) Cargo workspace(s) as the only build system.
  `build.sh` should eventually be a thin wrapper around `cargo build --release`
  (or be retired in favor of calling Cargo directly, once agents/docs are
  updated to match) and `start.sh` should run the compiled Rust binary
  directly.
- No `_glue.cpp` shim files — these are temporary FFI adapters, not
  permanent architecture. Every one that currently exists should eventually
  be deleted once the C++ code it bridges to is gone.

Treat every session as an opportunity to shrink the C++ surface, not just add
Rust alongside it forever. When in doubt about priority, prefer work that
lets a `_glue.cpp` file or a `meson.build` block be deleted over work that
only adds new standalone Rust.

## 3. How to migrate a unit of code

Follow the strangler-fig approach already in use in this repo (see
`skills/RUST_MIGRATION.md` for the general methodology). Concretely, here:

1. Pick the next target using the dependency order below (leaves first).
2. Look for the existing `*_glue.cpp` file for that subsystem if one exists
   (e.g. `xline_glue.cpp` pairs with `xline.rs`) — it defines the current FFI
   contract you need to either extend or fully replace.
3. Port behavior, not syntax, into a `.rs` file following the idioms already
   established in the existing `src/*.rs` files (they set the house style —
   match it rather than inventing a new pattern per file).
4. Wire it in so `./build.sh` picks it up:
    - Core library code goes in `src/rust/` (part of the `rust_core` crate
      built once and linked into the C++ binary via `librust_core.a`).
    - Loadable IRC modules go in `src/modules/*.rs` — `build.sh` automatically
      scaffolds and builds each one as its own `cdylib` and drops it in
      `run/modules/`. Follow the existing per-module `Cargo.toml` template
      inside `build.sh` (dependencies: `mongodb`, `tokio`, `chrono`,
      `async-trait`, `rust_core`, `tracing` are already available; add new
      deps to the heredoc in `build.sh` if a module needs one, not by hand
      per-build).
5. Update or delete the corresponding `_glue.cpp` and any `meson.build` entry
   that referenced the old `.cpp` file once nothing else depends on it.
6. Run `./build.sh` and confirm a clean build, then `./start.sh` and smoke
   test the affected behavior before moving on.
7. Delete the superseded `.cpp`/`.h` files. Don't leave dead C++ behind "just
   in case" — that is the opposite of progress on this migration.

### Suggested migration order (leaves → hubs)

Rough guide, adjust as coupling is discovered:

1. Remaining standalone `m_*.cpp` modules with few cross-module dependents
   (most IRC feature modules — check `src/modules/`).
2. `core_*` command modules under `src/coremods/` (whowas, whois, stats,
   lusers, etc.) — most are already fairly isolated.
3. Supporting subsystems that still have a paired `*_glue.cpp` but no
   dependents left after step 1–2 clears (check with `grep -rl` on the header
   name across `src/`).
4. Central/hub files last: `base.cpp`, `channels.cpp`, `server.cpp`,
   `modulemanager_glue.cpp`, `socketengine*`, `streamsocket.cpp` — these have
   the most dependents and the least room for error.
5. Only once step 4 is done: collapse the build itself — remove
   `meson.build`/`meson_options.txt`, rewrite `build.sh`/`start.sh` (or retire
   them) to be pure Cargo, delete `include/` C++ headers that are no longer
   referenced.

## 4. Ground rules while both languages coexist

- Never break `./build.sh` or `./start.sh`. If a change can't keep both
  working, split it into smaller increments.
- Keep all FFI surface concentrated in the `*_glue.cpp` files and the
  corresponding `.rs` modules — don't scatter new ad hoc `extern "C"` glue
  elsewhere.
- Don't reintroduce SpanningTree/server-linking — it was deliberately removed
  (see README) and is out of scope to restore.
- New features and bugfixes should be written in Rust, not C++, even if the
  surrounding subsystem hasn't been migrated yet — don't add new `.cpp` files.
- Config format migration to TOML (see README Todo) is a separate, valid unit
  of work and can be picked up independently of the C++→Rust migration.