# reng

A game engine written in Rust, intended for macOS, Windows, and web through WebAssembly.

## Current milestone

`reng` opens a resizable native window titled **reng**, initially 960 × 540 logical pixels, or a browser canvas that fills the viewport. [wgpu](https://docs.rs/wgpu/30.0.1/wgpu/) draws a triangle with smoothly interpolated red, green, and blue vertex colors over a teal background on startup and redraws when resized or exposed. Closing the native window exits the application; the event loop sleeps while idle.

Window management uses [winit](https://docs.rs/winit/0.30.13/winit/). Rendering uses Metal on macOS, DirectX 12 on Windows, and WebGPU in the browser. Browser GPU startup is asynchronous. Resize events configure the surface in physical pixels; zero dimensions skip configuration and drawing. Outdated or lost surfaces are reconfigured or recreated, and temporary acquisition timeouts retry. GPU initialization, validation, or device-loss failures are reported; device loss currently requires restarting the app.

`main.rs` passes `WindowSpecs::default()` and `Scene::default()` to `WindowHandler`. Window specs live in `src/window/specs.rs`, lifecycle logic in `src/window/handler.rs`, and GPU operations in `src/renderer/gpu.rs`. **Edit the background color and triangle vertices in `src/scene/data.rs`**: each vertex contains a clip-space XYZ position followed by a linear RGB color. The renderer uploads these three vertices on each redraw. `src/renderer/triangle.wgsl` passes the positions through and outputs the interpolated colors; this is vertex-color shading without lighting. Positions are relative to the viewport, so resizing can stretch the triangle; camera transforms and aspect correction are not implemented yet.

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
├── scene/
│   ├── mod.rs
│   └── data.rs
├── renderer/
│   ├── mod.rs
│   ├── gpu.rs
│   └── triangle.wgsl
├── telemetry/
│   ├── mod.rs
│   ├── native.rs
│   ├── web.rs
│   └── tests.rs
└── web/
    └── mod.rs
web/
├── index.html
├── serve.py
└── tests/
    ├── window.spec.js
    ├── renderer.spec.js
    └── test_serve.py
```

`lib.rs` exports the `window`, `scene`, and `telemetry` modules. The renderer is internal to the engine. Each subsystem's `mod.rs` keeps implementation modules private and exports its public entry points. `main.rs` explicitly initializes telemetry, keeps the trace guard alive through shutdown, and runs the window. Telemetry selects native or browser output at compile time; window code only emits logs and spans.

Tests always live in dedicated files, with unit tests declared from a module using `mod tests;` under the appropriate `cfg` and integration tests under `tests/`. Test bodies, helpers, and fixtures never share production implementation files.

## Run on desktop (debug)

Install [Rust through rustup](https://rustup.rs/) and Node.js 24 LTS. `rust-toolchain.toml` pins Rust 1.99.0, rustfmt, Clippy, rust-analyzer, and the WebAssembly target; rustup installs them automatically on the first build. Launch the desktop debug build with:

```sh
npm run desktop
```

Drag a window edge or corner to resize it, then use the native close button to exit.

## Run in a browser (debug)

Install Python 3, then install the browser build and test tools:

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci
```

Build, serve, and open the browser demo with:

```sh
npm run web
```

This builds the WebAssembly debug bundle with `npm run build:web`, then starts the local server and opens your default browser once the server is listening. `npm run serve:web` serves an existing build without opening a browser. Both launch commands use Cargo's default debug profile.

The demo runs at [localhost:18080](http://localhost:18080). Your default browser needs WebGPU enabled and a supported GPU; if it cannot be opened automatically, the terminal prints the URL to open manually. WebGPU requires a secure context; localhost is allowed, deployed builds need HTTPS. There is no WebGL fallback. Stop the local server with Ctrl+C. `web/pkg/` contains generated JavaScript and WebAssembly and is ignored by Git; rebuild it after Rust changes. The `wasm-bindgen` CLI and Rust dependency must use the same version, so update both the manifest and CI install command together.

`src/web/mod.rs` is the WebAssembly startup entry point. It installs panic diagnostics, initializes telemetry, and starts the shared window handler using winit's nonblocking browser event loop. CSS controls canvas size; winit reports physical pixel resize events, and the renderer updates the canvas drawing buffer. The loading message remains until GPU initialization completes. Startup and rendering failures appear on the page and in the console.

## Logging and traces

Native runs print leveled, structured logs to stdout and overwrite `trace.json` in the current working directory. Running `npm run desktop` from this repository writes the file here; Git ignores it. Close the window normally to finish and flush the capture, then open the file at [ui.perfetto.dev](https://ui.perfetto.dev/) using **Open trace file**. Copy the file elsewhere before the next run if you want to keep it.

The default level is `INFO`. Set `RUST_LOG` to filter both console output and the trace, for example:

```sh
RUST_LOG=debug npm run desktop
RUST_LOG=reng=debug,winit=warn npm run desktop
```

Invalid filter directives or an unwritable trace file fail startup with an error. Logs and spans cover application and event-loop startup/shutdown, window configuration and creation, resize callbacks (physical pixel dimensions), closing, GPU initialization, frame submission/presentation, and failures. Span fields are included in the trace, and a global subscriber collects events from all threads. Trace durations measure elapsed time in instrumented scopes, not GPU timings.

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
npm run test:launcher
npm run test:web
```

The Playwright tests run a local server and check startup, timing spans, viewport resizing, physical resize events, actual background pixels, zero-size recovery, and visible module-loading, unavailable WebGPU, or asynchronous GPU validation failures. They keep test code under `web/tests/` and use Chromium's software WebGPU adapter. macOS runs headless; Linux runs under Xvfb with Vulkan/Mesa (`sudo apt-get install xvfb xauth libvulkan1 mesa-vulkan-drivers`, then `xvfb-run -a npm run test:web`). This [Linux configuration](https://github.com/vercel-labs/vgpu/issues/109) avoids blank WebGPU screenshots in headless mode; this is a rendering regression check, while hardware and browser compatibility still need runtime verification.

For dependencies:

```sh
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check
npm audit
```

`deny.toml` checks Rust advisories, approved dependency licenses, versions, and sources for our native and web targets. It rejects known vulnerabilities, yanked versions, and unmaintained direct dependencies. Transitive duplicates produce warnings; transitive maintenance advisories are left to upstream dependency updates. The existing repository license remains in `LICENSE`.

GitHub Actions currently runs native Rust tests, formatting, WebAssembly Clippy/build checks, and Chromium tests on Linux. The separate `Rust tests` job runs `cargo test --locked` using the pinned toolchain on every PR and push to `main`. Require this check in GitHub branch protection or a ruleset to block merging when Rust tests fail. macOS and Windows CI builds are deferred to keep CI costs low; run native checks locally when changing native code. Dependency checks run on PRs, main, and weekly; Dependabot proposes weekly Cargo, npm, and GitHub Actions updates. These jobs verify compilation, Rust test behavior, and browser behavior; native GUI and GPU checks still require running the app on the target platform.

## Potential nightly uses

Stable Rust remains the default. These are future options to adopt only when a concrete need justifies them; none is required by the current engine.

| Option | When to consider it | Adoption checks and limits |
| --- | --- | --- |
| [Portable `std::simd`](https://doc.rust-lang.org/nightly/std/simd/index.html) (`portable_simd`) | Profiling shows a CPU bottleneck in animation, physics, or bulk data processing. | Benchmark against the stable implementation on the intended targets before adopting it. SIMD can compile to scalar operations; a performance gain is not guaranteed. |
| [Miri](https://github.com/rust-lang/miri) and [sanitizers](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html) | Optional diagnostic checks for focused tests around unsafe memory handling or concurrency. | Miri interprets tested executions and has limited platform API/FFI support. Sanitizer support and instrumentation requirements vary by target. Neither replaces GPU, native-window, or browser runtime checks. |
| [Declarative macros 2.0](https://doc.rust-lang.org/nightly/unstable-book/language-features/decl-macro.html) (`decl_macro`) | A concrete macro hygiene or organization problem justifies an experiment beyond stable `macro_rules!`. | Keep experiments isolated and compare them with the stable solution before making engine code depend on this unstable language feature. |

Nightly diagnostics can coexist with stable builds: select nightly explicitly for those checks (for example, `cargo +nightly miri test` with Miri installed), while keeping the normal stable checks above. See [Rust's release channels](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html).

If engine code later requires unstable features, pin a dated toolchain such as `nightly-YYYY-MM-DD` and verify macOS and Windows builds and runtime behavior, plus `wasm32-unknown-unknown` compilation and browser execution when web packaging exists. Review compatibility before updating that pin.

## Rendering proof of concept

The shaded triangle was verified on macOS with Metal, including zoom/restore resizing at Retina resolution and clean window closure. It also rendered in Chromium 154 on macOS using a non-fallback Apple `metal-3` WebGPU adapter, including viewport resizing, with no console warnings or errors. Automated Chromium software WebGPU tests compare the triangle at 960 × 540 and 640 × 360 and after zero-size recovery. Windows, Safari, and Firefox triangle rendering remain unverified; surface-loss recovery was not forced during these checks.

![Shaded triangle rendered by Chromium software WebGPU](web/tests/snapshots/triangle.png)

See [AGENTS.md](AGENTS.md) for project conventions and validation commands.

## Planned additions

Add each dependency when its feature needs it. Some proposed crates already arrive transitively through winit; declare them directly when reng uses their APIs:

| Next feature | Planned additions | When to add them |
| --- | --- | --- |
| Typed vertex and uniform buffers | Potentially `bytemuck` | When structured GPU data needs typed byte casts; the current triangle packs its floats with the standard library. |
| Camera and transforms | `glam` | When the first scene needs vector and matrix math. |
| Frame loop and input | A shared frame clock, potentially `web-time` | When continuous updates replace the current idle event loop; add keyboard and pointer handling then. |
| Renderer errors | `thiserror` | When callers need to distinguish recoverable rendering failures. |
| GPU profiling | Potentially `wgpu-profiler` | When GPU work exists; current tracing measures CPU scopes only. |
| Remote telemetry | OpenTelemetry and an exporter | When we need a collector or shared analysis beyond local console and Perfetto captures. |
| Engine systems | Assets, ECS, audio, physics, editor | After the rendering and input milestones, driven by a small playable example. No libraries selected yet. |

Verify the renderer on real macOS and Windows GPUs and in browsers before claiming platform support. WebGPU is the initial browser rendering target; a WebGL2 fallback needs a separate decision and its own tests. Additional Safari and Firefox automated coverage can be added as the browser path grows.

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
