# AGENTS.md

## Project context

- `reng` is a Rust game engine in its initial scaffold stage.
- Intended platforms: macOS, Windows, and web through WebAssembly (`wasm32-unknown-unknown`).
- Current stack: `winit` for windows, input events, and application lifecycle; `tracing` for events and spans. Rendering with `wgpu` is planned.
- The repository has one Cargo package with a reusable library in `src/lib.rs` and a thin executable in `src/main.rs`. Resizable windows and tracing are implemented; rendering and web packaging are not implemented yet.
- The first rendering milestone is the same colored triangle running in a native window and a browser canvas.

## Ponytail

Use [Ponytail](https://ponytail.dev/) in `full` mode for coding tasks unless the user requests another level or disables it. When the plugin is available, apply its `ponytail` skill.

- Read the affected code and its callers before editing; identify the complete scope of the change.
- Prefer existing code, the standard library, platform features, and installed dependencies before adding code or dependencies.
- Make the smallest change that fully solves the task. Avoid speculative abstractions and features.
- Preserve validation, error handling, security, accessibility, and platform verification. Add a focused check for nontrivial logic.
- End task reports with what was skipped or not checked and any material risk.

Codex enables the official plugin through `.codex/config.toml`. See README.md for installation and hook setup; the principles above also apply when the plugin is unavailable.

## MCP tools

- Shared Codex MCP configuration lives in `.codex/config.toml`; see README.md for prerequisites.
- Use `rust_analyzer` for semantic Rust navigation and diagnostics. Continue to use Cargo, Clippy, and tests for build validation.
- Use `playwright` for browser inspection, screenshots, and coordinate-based canvas input. Automatically named output goes under `target/playwright-mcp/`. It controls an isolated Chrome session; native windows use the available computer-use tools.
- Browser inspection alone does not establish GPU or platform compatibility; follow the rendering verification guidance below.

## Development commands

Use a stable Rust toolchain supporting the Rust 2024 edition. Run commands from the repository root:

```sh
cargo run --locked
cargo build --locked
```

For Rust changes, run:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

For changes affecting web compatibility, also check the WebAssembly target:

```sh
rustup target add wasm32-unknown-unknown
cargo check --locked --target wasm32-unknown-unknown
```

The target check verifies compilation only. Document browser build and serving commands here when web packaging is implemented.

## Rust conventions

- Use idiomatic Rust and default rustfmt formatting. Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) when introducing public APIs.
- Keep changes small and reuse existing code. Keep one Cargo package with a library and thin binary until a concrete requirement justifies separate packages.
- Prefer safe Rust. Any necessary `unsafe` code must document its safety invariants.
- Return or handle recoverable errors with `Result`; avoid `unwrap()` or `expect()` for hardware availability, assets, or other external failures.
- Add dependencies only when they solve a concrete need, and check their compatibility with all intended platforms.
- Keep `Cargo.lock` tracked and generated build output under `target/` ignored.

## Code and test structure

- Put reusable engine code in `src/lib.rs` modules; keep `src/main.rs` responsible for application startup, explicit telemetry initialization and guard lifetime, and running the window.
- Use subsystem directories such as `src/window/` and `src/telemetry/`, with `mod.rs` declaring private implementation modules and re-exporting the minimal public API. Window configuration lives in `specs.rs`, lifecycle in `handler.rs`, and platform-specific telemetry in `native.rs` and `web.rs` selected with `cfg`.
- Window code emits events and spans through `tracing`; it must not initialize the global subscriber or choose output destinations.
- Tests must always live in dedicated files, never alongside production implementation. Put unit tests in files such as `src/telemetry/tests.rs`, declared using `#[cfg(test)] mod tests;` (with additional platform conditions when needed), and integration tests under `tests/`.
- All test bodies, test helpers, fixtures, and test assertions belong in those dedicated test files. Never place an inline test module or test code inside an implementation file.

## Rendering and platform conventions

- Share rendering and game logic across platforms. Keep platform-specific startup and browser integration in small, explicit sections using conditional compilation where needed.
- Keep GPU initialization asynchronous and browser execution nonblocking. Avoid native filesystem or thread assumptions in shared code; embed initial assets or load web assets through URLs.
- Use WGSL shaders and request only the GPU features and limits the renderer needs. Check adapter capabilities before enabling optional features.
- Handle resize and display scaling, skip rendering at zero dimensions, and recover from surface errors where possible. Report fatal GPU failures clearly.
- Target WebGPU in the browser initially. WebGL2 fallback is optional and requires its own capability constraints and verification.

## Verification and documentation

- Add focused behavioral tests for nontrivial logic changes. Documentation-only changes need a diff and link review, not new tests.
- Smoke-test rendering changes on an actual GPU, including resizing and closing the window. For web changes, also run the demo in a browser.
- State which platforms and browsers were tested. A successful macOS build does not establish Windows or browser runtime support.
- Keep README.md and these instructions aligned with implemented behavior; label planned functionality explicitly.

This file follows the project context, commands, conventions, and verification structure described at [agents.md](https://agents.md/).
