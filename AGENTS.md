# AGENTS.md

## Project context

- `reng` is a Rust game engine in its initial scaffold stage.
- Intended platforms: macOS, Windows, and web through WebAssembly (`wasm32-unknown-unknown`).
- Current stack: `winit` for windows, input events, and application lifecycle; `wgpu` for rendering; `tracing` for events and spans.
- The repository has one Cargo package with a reusable library in `src/lib.rs` and a thin executable in `src/main.rs`. Resizable native windows, a resizable browser canvas, tracing, and a GPU background clear are implemented.
- The next rendering milestone is the same colored triangle running in a native window and a browser canvas.

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

Use the Rust 1.99.0 toolchain pinned in `rust-toolchain.toml`. Run commands from the repository root:

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
cargo clippy --lib --locked --target wasm32-unknown-unknown -- -D warnings
npm ci
npm run build:web
npx playwright install chromium
npm run test:web
```

Install `wasm-bindgen-cli` 0.2.129 with `cargo install wasm-bindgen-cli --version 0.2.129 --locked`; it must match the Rust dependency. Node.js 24 and Python 3 are required for the web commands. `npm run serve:web` serves the generated browser build on localhost:18080. Keep browser tests in `web/tests/`, separate from production code. On Linux, install `xvfb xauth libvulkan1 mesa-vulkan-drivers` and run `xvfb-run -a npm run test:web`; Chromium needs a virtual display for reliable WebGPU screenshots.

For dependency changes, run `cargo deny --locked check` and `npm audit`; install `cargo-deny` 0.20.2 if needed. Keep the manifest, lockfiles, tool versions, and CI in sync. Review advisory or license failures rather than broadly suppressing them.

## Rust conventions

- Use idiomatic Rust and default rustfmt formatting. Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) when introducing public APIs.
- Keep changes small and reuse existing code. Keep one Cargo package with a library and thin binary until a concrete requirement justifies separate packages.
- Prefer safe Rust. Any necessary `unsafe` code must document its safety invariants.
- Return or handle recoverable errors with `Result`; avoid `unwrap()` or `expect()` for hardware availability, assets, or other external failures.
- Add dependencies only when they solve a concrete need, and check their compatibility with all intended platforms.
- Keep `Cargo.lock` and `package-lock.json` tracked. Generated build output (`target/`, `web/pkg/`) and browser test output stay ignored.

## Code and test structure

- Put reusable engine code in `src/lib.rs` modules; keep `src/main.rs` responsible for application startup, explicit telemetry initialization and guard lifetime, and running the window.
- Use subsystem directories such as `src/window/` and `src/telemetry/`, with `mod.rs` declaring private implementation modules and re-exporting the minimal public API. Window configuration lives in `specs.rs`, lifecycle in `handler.rs`, and platform-specific telemetry in `native.rs` and `web.rs` selected with `cfg`. GPU operations live in `src/renderer/`; all scene data, including background colors and future triangle vertices/colors, live in `src/scene/`, outside the renderer. The renderer consumes scene data rather than defining it. Browser startup lives in `src/web/mod.rs`; the static browser host lives under `web/`.
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

- CI currently checks the web target on Linux, plus dependency audits. macOS and Windows CI builds are deferred to limit costs; run native checks locally for native changes.

- Add focused behavioral tests for nontrivial logic changes. Documentation-only changes need a diff and link review, not new tests.
- Smoke-test rendering changes on an actual GPU, including resizing and closing the window. For web changes, also run the demo in a browser.
- State which platforms and browsers were tested. A successful macOS build does not establish Windows or browser runtime support.
- Keep README.md and these instructions aligned with implemented behavior; label planned functionality explicitly.

This file follows the project context, commands, conventions, and verification structure described at [agents.md](https://agents.md/).
