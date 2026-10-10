# reng

A game engine written in Rust, intended for macOS, Windows, and web through WebAssembly.

## Current milestone

`reng` opens a resizable native window titled **reng**, initially 960 × 540 logical pixels. Closing the window exits the application. The event loop sleeps while idle; there is no drawing or GPU initialization yet.

Window management uses [winit](https://docs.rs/winit/0.30.13/winit/). Rendering with [wgpu](https://wgpu.rs/) and browser packaging are planned.

`main.rs` starts `WindowHandler` with `WindowSpecs::default()`. The specs hold the title, logical width and height, and resizable flag. `WindowSpecs` lives in `src/window/specs.rs`; `winit` integration and window lifecycle logic live in `src/window/handler.rs`.

## Code layout

One Cargo package contains a reusable engine library and a thin executable:

```text
src/
├── main.rs
├── lib.rs
├── window/
│   ├── mod.rs
│   ├── specs.rs
│   └── handler.rs
└── telemetry/
    ├── mod.rs
    ├── native.rs
    ├── web.rs
    └── tests.rs
```

`lib.rs` exports the `window` and `telemetry` modules. Each subsystem's `mod.rs` keeps implementation modules private and exports its public entry points. `main.rs` explicitly initializes telemetry, keeps the trace guard alive through shutdown, and runs the window. Telemetry selects native or browser output at compile time; window code only emits logs and spans.

Tests always live in dedicated files, with unit tests declared from a module using `mod tests;` under the appropriate `cfg` and integration tests under `tests/`. Test bodies, helpers, and fixtures never share production implementation files.

## Run

Use a stable Rust toolchain supporting the Rust 2024 edition:

```sh
cargo run --locked
```

Drag a window edge or corner to resize it, then use the native close button to exit.

## Logging and traces

Native runs print leveled, structured logs to stdout and overwrite `trace.json` in the current working directory. Running `cargo run --locked` from this repository writes the file here; Git ignores it. Close the window normally to finish and flush the capture, then open the file at [ui.perfetto.dev](https://ui.perfetto.dev/) using **Open trace file**. Copy the file elsewhere before the next run if you want to keep it.

The default level is `INFO`. Set `RUST_LOG` to filter both console output and the trace, for example:

```sh
RUST_LOG=debug cargo run --locked
RUST_LOG=reng=debug,winit=warn cargo run --locked
```

Invalid filter directives or an unwritable trace file fail startup with an error. Logs and spans cover application and event-loop startup/shutdown, window configuration and creation, resize callbacks (physical pixel dimensions), closing, and failures. Span fields are included in the trace, and a global subscriber collects events from all threads. Trace durations measure elapsed time in instrumented scopes, not GPU timings.

The trace writer flushes when the application returns, including returned errors. Forced termination can leave an incomplete file. Run one instance per working directory so concurrent runs do not overwrite the same capture.

On WebAssembly, the same events go to the browser console at `INFO` level using `tracing-web`; native filesystem capture and `RUST_LOG` are unavailable there. Browser packaging remains planned.

## Image assets

Image files are stored with [Git LFS](https://git-lfs.com/). The rules in `.gitattributes` cover raster and vector images, GPU texture formats, and image editor files, including uppercase and mixed-case extensions. Add a tracking rule there when introducing a new image format.

Install Git LFS on each development machine, then run from the repository root:

```sh
git lfs install --local
git lfs pull
```

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

For the planned web target, compilation can be checked with:

```sh
rustup target add wasm32-unknown-unknown
cargo check --locked --target wasm32-unknown-unknown
```

This checks compilation only. Browser startup and canvas integration are not implemented.

## Potential nightly uses

Stable Rust remains the default. These are future options to adopt only when a concrete need justifies them; none is required by the current engine.

| Option | When to consider it | Adoption checks and limits |
| --- | --- | --- |
| [Portable `std::simd`](https://doc.rust-lang.org/nightly/std/simd/index.html) (`portable_simd`) | Profiling shows a CPU bottleneck in animation, physics, or bulk data processing. | Benchmark against the stable implementation on the intended targets before adopting it. SIMD can compile to scalar operations; a performance gain is not guaranteed. |
| [Miri](https://github.com/rust-lang/miri) and [sanitizers](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html) | Optional diagnostic checks for focused tests around unsafe memory handling or concurrency. | Miri interprets tested executions and has limited platform API/FFI support. Sanitizer support and instrumentation requirements vary by target. Neither replaces GPU, native-window, or browser runtime checks. |
| [Declarative macros 2.0](https://doc.rust-lang.org/nightly/unstable-book/language-features/decl-macro.html) (`decl_macro`) | A concrete macro hygiene or organization problem justifies an experiment beyond stable `macro_rules!`. | Keep experiments isolated and compare them with the stable solution before making engine code depend on this unstable language feature. |

Nightly diagnostics can coexist with stable builds: select nightly explicitly for those checks (for example, `cargo +nightly miri test` with Miri installed), while keeping the normal stable checks above. See [Rust's release channels](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html).

If engine code later requires unstable features, pin a dated toolchain such as `nightly-YYYY-MM-DD` and verify macOS and Windows builds and runtime behavior, plus `wasm32-unknown-unknown` compilation and browser execution when web packaging exists. Review compatibility before updating that pin.

## macOS proof of concept

The window was opened, resized with the native zoom/restore control, and closed successfully on macOS. Windows and WebAssembly compilation checks also pass; Windows runtime and browser startup have not been tested.

![Resizable reng window on macOS](docs/screenshots/resizable-window-macos.jpg)

See [AGENTS.md](AGENTS.md) for project conventions and validation commands.

## Agent tooling

This repository enables the official [Ponytail](https://ponytail.dev/) plugin for Codex through [.codex/config.toml](.codex/config.toml). Project guidance uses `full` mode by default.

For a new machine, install the plugin with the Codex CLI:

```sh
codex plugin marketplace add DietrichGebert/ponytail
codex plugin add ponytail@ponytail
```

The hooks require Node.js on `PATH`. Run `codex`, review and trust the plugin's hooks in `/hooks`, and start a new chat. Restart the Codex desktop app after a new installation.

Use `/ponytail lite`, `/ponytail full`, `/ponytail ultra`, or `/ponytail off` to change the mode. See the [official installation instructions](https://github.com/DietrichGebert/ponytail/blob/main/INSTALL.md).

### Shared MCP servers

[.codex/config.toml](.codex/config.toml) also configures two project-scoped MCP servers:

- [Playwright MCP](https://github.com/microsoft/playwright-mcp), version `0.0.83`, for browser inspection, screenshots, and coordinate-based canvas interaction. It opens a visible, isolated Chrome session and saves automatically named output under the ignored `target/playwright-mcp/` directory.
- [rust-analyzer MCP](https://github.com/zeenix/rust-analyzer-mcp), version `0.4.0`, for Rust definitions, references, types, and diagnostics.

Install Node.js 18 or newer and Google Chrome. Ensure `npx` is on `PATH`; Playwright MCP downloads automatically on first use. Install the Rust prerequisites:

```sh
rustup component add rust-analyzer
cargo install rust-analyzer-mcp --version 0.4.0 --locked
```

Ensure `rust-analyzer` and `rust-analyzer-mcp` are on `PATH`. Open this repository as a trusted Codex project, then restart Codex or start a new chat to load the servers. Use `/mcp` to check their connection status; `codex mcp list` from the repository root shows the configured servers. See [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp).
