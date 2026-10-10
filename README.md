# reng

A game engine written in Rust, intended for macOS, Windows, and web through WebAssembly.

## Current milestone

`reng` opens a resizable native window titled **reng**, initially 960 × 540 logical pixels. Closing the window exits the application. The event loop sleeps while idle; there is no drawing or GPU initialization yet.

The browser build opens a blank canvas that fills and resizes with the viewport. Window management on both platforms uses [winit](https://docs.rs/winit/0.30.13/winit/). Rendering with [wgpu](https://wgpu.rs/) is the next milestone.

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
├── telemetry/
│   ├── mod.rs
│   ├── native.rs
│   ├── web.rs
│   └── tests.rs
└── web/
    └── mod.rs
web/
├── index.html
└── tests/
    └── window.spec.js
```

`lib.rs` exports the `window` and `telemetry` modules. Each subsystem's `mod.rs` keeps implementation modules private and exports its public entry points. `main.rs` explicitly initializes telemetry, keeps the trace guard alive through shutdown, and runs the window. Telemetry selects native or browser output at compile time; window code only emits logs and spans.

Tests always live in dedicated files, with unit tests declared from a module using `mod tests;` under the appropriate `cfg` and integration tests under `tests/`. Test bodies, helpers, and fixtures never share production implementation files.

## Run

Install [Rust through rustup](https://rustup.rs/). `rust-toolchain.toml` pins Rust 1.99.0, rustfmt, Clippy, rust-analyzer, and the WebAssembly target; rustup installs them automatically when a Cargo command runs here:

```sh
cargo run --locked
```

Drag a window edge or corner to resize it, then use the native close button to exit.

## Run in a browser

Install Node.js 24 LTS and Python 3, then install the build and test tools:

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci
npm run build:web
npm run serve:web
```

Open [localhost:18080](http://localhost:18080). Stop the local server with Ctrl+C. `web/pkg/` contains generated JavaScript and WebAssembly and is ignored by Git; rebuild it after Rust changes. The `wasm-bindgen` CLI and Rust dependency must use the same version, so update both the manifest and CI install command together.

`src/web/mod.rs` is the WebAssembly startup entry point. It installs panic diagnostics, initializes telemetry, and starts the shared window handler using winit's nonblocking browser event loop. CSS controls canvas size; winit reports physical pixel resize events. Startup failures appear on the page and in the console.

## Logging and traces

Native runs print leveled, structured logs to stdout and overwrite `trace.json` in the current working directory. Running `cargo run --locked` from this repository writes the file here; Git ignores it. Close the window normally to finish and flush the capture, then open the file at [ui.perfetto.dev](https://ui.perfetto.dev/) using **Open trace file**. Copy the file elsewhere before the next run if you want to keep it.

The default level is `INFO`. Set `RUST_LOG` to filter both console output and the trace, for example:

```sh
RUST_LOG=debug cargo run --locked
RUST_LOG=reng=debug,winit=warn cargo run --locked
```

Invalid filter directives or an unwritable trace file fail startup with an error. Logs and spans cover application and event-loop startup/shutdown, window configuration and creation, resize callbacks (physical pixel dimensions), closing, and failures. Span fields are included in the trace, and a global subscriber collects events from all threads. Trace durations measure elapsed time in instrumented scopes, not GPU timings.

The trace writer flushes when the application returns, including returned errors. Forced termination can leave an incomplete file. Run one instance per working directory so concurrent runs do not overwrite the same capture.

On WebAssembly, the same events go to the browser console at `INFO` level using `tracing-web`. Spans also create User Timing marks and measures visible in browser DevTools performance recordings. `console_error_panic_hook` reports Rust panics to the console. The browser build does not write `trace.json` or read `RUST_LOG`.

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

For browser changes:

```sh
cargo clippy --lib --locked --target wasm32-unknown-unknown -- -D warnings
npm run build:web
npx playwright install chromium
npm run test:web
```

The Playwright tests run a local server, check startup, timing spans, viewport resizing, physical resize events, and visible module-loading failures. They run headless Chromium and keep test code under `web/tests/`.

For dependencies:

```sh
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check
npm audit
```

`deny.toml` checks Rust advisories, approved dependency licenses, versions, and sources for our native and web targets. It rejects known vulnerabilities, yanked versions, and unmaintained direct dependencies. Transitive duplicates produce warnings; transitive maintenance advisories are left to upstream dependency updates. The existing repository license remains in `LICENSE`.

GitHub Actions currently runs formatting, WebAssembly Clippy/build checks, and Chromium tests on Linux. macOS and Windows CI builds are deferred to keep CI costs low; run native checks locally when changing native code. Dependency checks run on PRs, main, and weekly; Dependabot proposes weekly Cargo, npm, and GitHub Actions updates. These jobs verify compilation and browser behavior; native GUI and GPU checks still require running the app on the target platform.

## macOS proof of concept

The window was opened, resized with the native zoom/restore control, and closed successfully on macOS. The browser canvas is covered by startup and resize tests in Chromium. Windows GUI execution and rendering on any platform remain unverified.

![Resizable reng window on macOS](docs/screenshots/resizable-window-macos.jpg)

See [AGENTS.md](AGENTS.md) for project conventions and validation commands.

## Planned additions

Add each dependency when its feature needs it. Some proposed crates already arrive transitively through winit; declare them directly when reng uses their APIs:

| Next feature | Planned additions | When to add them |
| --- | --- | --- |
| First renderer | `wgpu`, WGSL shaders | Clear the window and canvas, then draw the same colored triangle on macOS, Windows, and web; handle resize, display scaling, zero size, and surface errors. |
| GPU startup | `pollster` on native; `wasm-bindgen-futures` on web | When asynchronous adapter and device initialization exists. Keep the browser event loop nonblocking. |
| Vertex and uniform buffers | `bytemuck` | When passing typed Rust data to GPU buffers. |
| Camera and transforms | `glam` | When the first scene needs vector and matrix math. |
| Frame loop and input | A shared frame clock, potentially `web-time` | When continuous updates replace the current idle event loop; add keyboard and pointer handling then. |
| Renderer errors | `thiserror` | When callers need to distinguish recoverable rendering failures. |
| GPU profiling | Potentially `wgpu-profiler` | When GPU work exists; current tracing measures CPU scopes only. |
| Remote telemetry | OpenTelemetry and an exporter | When we need a collector or shared analysis beyond local console and Perfetto captures. |
| Engine systems | Assets, ECS, audio, physics, editor | After the rendering and input milestones, driven by a small playable example. No libraries selected yet. |

Verify the renderer on real macOS and Windows GPUs and in browsers before claiming platform support. WebGPU is the initial browser rendering target; a WebGL2 fallback needs a separate decision and its own tests. Safari and Firefox runtime coverage can be added as the browser path grows.

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

Ensure `rust-analyzer` and `rust-analyzer-mcp` are on `PATH`. Open this repository as a trusted Codex project, then restart Codex or start a new chat to load the servers. Use `/mcp` to check their connection status. If project servers are absent, check project trust and project configuration loading before changing the shared configuration. See [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp).

[Context7](https://github.com/upstash/context7) is an optional documentation tool for current Rust library APIs. Connect it through your agent environment; it is not required to build the project and is not bundled in the shared MCP configuration. No additional MCP servers are needed for the current milestone.
