# reng

A game engine written in Rust, intended for macOS, Windows, and web through WebAssembly.

**Try it in a browser:** [burningbluefox.github.io/reng](https://burningbluefox.github.io/reng/) needs a WebGPU-capable browser, such as Safari 26 or later (including iPadOS) or recent Chrome or Edge. CI redeploys the release build from `main` after its checks pass.

## Current milestone

`reng` opens a resizable native window titled **reng**, initially 960 × 540 logical pixels, or a browser canvas that fills the viewport. [wgpu](https://docs.rs/wgpu/30.0.1/wgpu/) draws a depth-tested die: a cube textured with a dice image whose pips run 1 to 6, each face tinted its own color and lit by one directional light, turning about its vertical axis at 1 radian per second in front of a perspective camera. Above and behind it sit a smooth-shaded light gray sphere and an orange Suzanne, Blender's monkey head, which hold still so the light's falloff across curved surfaces shows. All of this renders over a teal background, with white frame-time and elapsed-time debug labels in the top-right corner. The camera circles a target point; see [Camera controls](#camera-controls). It redraws continuously at the display's presentation rate while visible and pauses while the window is occluded or has zero size. Closing the native window exits the application.

Window management uses [winit](https://docs.rs/winit/0.30.13/winit/). Rendering uses Metal on macOS, DirectX 12 on Windows, and WebGPU in the browser. Browser GPU startup is asynchronous. Resize events configure the surface in physical pixels; zero dimensions skip configuration and drawing. Outdated or lost surfaces are reconfigured or recreated, and temporary acquisition timeouts retry. GPU initialization, validation, or device-loss failures are reported; device loss currently requires restarting the app.

`main.rs` passes `WindowSpecs::default()` and `Scene::default()` to `WindowHandler`. Window specs live in `src/window/specs.rs`, lifecycle logic in `src/window/handler.rs`, and GPU operations in `src/renderer/gpu.rs`. **Edit the background color, `camera`, `textures`, `light`, and `meshes` in `src/scene/data.rs`**. `light` is a `DirectionalLight`: the world-space direction its parallel rays travel, the linear RGB `color` a surface facing them head-on receives, and `ambient` light every surface receives. It shines from the +X, +Y, and +Z side, above and just behind the starting camera, so the cube's top is brightest, then its +Z face, then its +X face. Each `Mesh` lists its triangles as three vertices each, without indices; a vertex contains a model-space XYZ position, a linear RGB color, a UV texture coordinate with (0, 0) at the image's top-left, and a model-space normal pointing out of the front, and front faces wind counter-clockwise when seen from outside. Each mesh also has a [glam](https://docs.rs/glam/0.34.3/glam/) `Mat4` model transform and an optional index into `Scene::textures`, a list of PNG images embedded with `include_bytes!`. The cube samples `src/scene/dice.png`, a 3×2 atlas of the faces 1–3 above 4–6, with opposite faces adding up to seven. `src/scene/clock.rs` measures frame and elapsed time with `web-time`; on each redraw `WindowHandler` stores the formatted labels in `Scene::debug_labels` and calls `Scene::animate` with the elapsed time, which replaces the first mesh's transform with a rotation about +Y, so a transform set on it in `data.rs` has no effect. At startup the renderer decodes each scene PNG to 8-bit RGBA with the [png](https://docs.rs/png/0.18.1/png/) crate and uploads it as an sRGB texture without mipmaps; an unreadable image, such as a Git LFS pointer, or one larger than the GPU's texture limit fails startup with an error. Each frame it transforms every vertex to clip space on the CPU, multiplies each vertex color by the light reaching it along its normal (the ambient light plus Lambert diffuse light; normals are mapped by the inverse transpose of the model matrix so they stay perpendicular under non-uniform scale), appends the debug labels at depth 0, uploads one vertex buffer, and draws each mesh's range, then the labels, with that mesh's texture bound, a `Depth32Float` depth test, and no back-face culling. Untextured meshes and the labels sample a 1×1 white texture. `src/renderer/texture.rs` decodes and uploads textures. `src/renderer/text.rs` draws the labels with a built-in 5×7 bitmap font as white quads. `src/renderer/triangle.wgsl` passes the clip positions through and multiplies the interpolated vertex color by the bilinearly filtered texture color, ignoring texture alpha. Lighting is computed per vertex on the CPU and interpolated across each triangle (Gouraud shading). The cube repeats each face's normal on its corners, so its faces stay flat; the sphere's normals point straight out from its center, and Suzanne's average the faces around each vertex, so both look smooth. There are no shadows, specular highlights, or point lights. `src/scene/suzanne.rs` holds Suzanne's half-head data from Blender's [`bmo_primitive.cc`](https://github.com/blender/blender/blob/main/source/blender/bmesh/operators/bmo_primitive.cc) (GPL-2.0-or-later, used under reng's GPL-3.0-or-later) and unpacks it as Blender does: mirrored across x = 0 into 507 vertices and 500 faces, with each quad split along whichever diagonal keeps both triangles facing the same way. The projection uses the surface aspect ratio with a fixed vertical field of view, so resizing does not stretch the scene. `src/scene/camera.rs` defines the `OrbitCamera`: a target point, yaw, pitch, and distance, with pitch kept short of straight up or down and the distance kept between 1.5 and 50 units. `src/window/controls.rs` turns window input into camera moves, and `WindowHandler` applies held keys each frame using the frame time from `FrameClock`.

## Code layout

One Cargo package contains a reusable engine library and a thin executable:

```text
src/
├── main.rs
├── lib.rs
├── window/
│   ├── mod.rs
│   ├── specs.rs
│   ├── handler.rs
│   ├── controls.rs
│   └── tests.rs
├── scene/
│   ├── mod.rs
│   ├── camera.rs
│   ├── clock.rs
│   ├── data.rs
│   ├── dice.png
│   ├── suzanne.rs
│   └── tests.rs
├── renderer/
│   ├── mod.rs
│   ├── gpu.rs
│   ├── text.rs
│   ├── texture.rs
│   ├── tests.rs
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
    ├── controls.spec.js
    ├── frozen-clock.js
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

### Camera controls

The same controls work in the native window and the browser canvas. Click the canvas first in a browser so it receives keys.

| Input | Camera |
| --- | --- |
| Drag with the left mouse button or one finger | Circles the camera around the target, as if you were turning the scene by hand |
| Scroll wheel, trackpad scroll or pinch, or two-finger pinch | Zooms toward or away from the target |
| Hold W, A, S, or D | Moves the camera and its target forward, left, back, or right across the ground, 2 units per second |
| Space | Returns the camera to its starting view |

Keys follow their physical position, so the same keys work on AZERTY and other layouts. Held keys, drags, and touches are released when the window loses focus, and held keys also when Command is released, because macOS browsers drop key releases while Command is down. Space returns to the camera the scene started with. Browsers deliver a trackpad pinch as a Ctrl scroll, which zooms at the same rate as a native pinch once the canvas has been clicked; before that, the browser does not report Ctrl and a pinch zooms at scroll speed. Native trackpad pinch on Windows arrives as a scroll and zooms at scroll speed.

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

Image files are stored with [Git LFS](https://git-lfs.com/). The rules in `.gitattributes` cover raster and vector images, GPU texture formats, and image editor files, including uppercase and mixed-case extensions. Add a tracking rule there when introducing a new image format. The renderer embeds `src/scene/dice.png` in the build, so pull LFS files before building; a build made from the pointer file reports that the texture is not a readable PNG at startup.

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

The Playwright tests run a local server and check startup, timing spans, viewport resizing, physical resize events, actual scene pixels, zero-size recovery, and visible module-loading, unavailable WebGPU, or asynchronous GPU validation failures. The renderer test freezes the engine clock by replacing `performance.now()` from an init script; it compares the die, sphere, Suzanne, and labels at 960 × 540, then one second later at 640 × 480 and after zero-size recovery, and checks that the cube moves when the frozen clock advances. The controls tests use the same frozen clock (`web/tests/frozen-clock.js`): they drag, scroll, hold W while the clock advances, and drag or pinch with touch points sent through the Chrome DevTools Protocol, then check that the camera moved. A touch pinch and a Ctrl scroll, which is how browsers deliver a trackpad pinch, must reach exactly the same closest view as scrolling all the way in. After the drag, scroll, and touch gestures, Space must restore the starting image exactly; after holding W, when the cube has also spun, Space must visibly move the camera back. They keep test code under `web/tests/` and use Chromium's software WebGPU adapter. macOS runs headless; Linux runs under Xvfb with Vulkan/Mesa (`sudo apt-get install xvfb xauth libvulkan1 mesa-vulkan-drivers`, then `xvfb-run -a npm run test:web`). This [Linux configuration](https://github.com/vercel-labs/vgpu/issues/109) avoids blank WebGPU screenshots in headless mode; this is a rendering regression check, while hardware and browser compatibility still need runtime verification.

For dependencies:

```sh
cargo install cargo-deny --version 0.20.2 --locked
cargo deny --locked check
npm audit
```

`deny.toml` checks Rust advisories, approved dependency licenses, versions, and sources for our native and web targets. It rejects known vulnerabilities, yanked versions, and unmaintained direct dependencies. Transitive duplicates produce warnings; transitive maintenance advisories are left to upstream dependency updates. The existing repository license remains in `LICENSE`.

GitHub Actions currently runs native Rust tests, formatting, WebAssembly Clippy/build checks, and Chromium tests on Linux. The separate `Rust tests` job runs `cargo test --locked` using the pinned toolchain on every PR and push to `main`. Require this check in GitHub branch protection or a ruleset to block merging when Rust tests fail. macOS and Windows CI builds are deferred to keep CI costs low; run native checks locally when changing native code. Dependency checks run on PRs, main, and weekly; Dependabot proposes weekly Cargo, npm, and GitHub Actions updates. These jobs verify compilation, Rust test behavior, and browser behavior; native GUI and GPU checks still require running the app on the target platform. On pushes to `main` and manual runs from `main`, the `Web (Chromium)` job also builds the WebAssembly release bundle with `index.html` into `_site/`, and once both test jobs pass, the `Deploy to GitHub Pages` job publishes it to [burningbluefox.github.io/reng](https://burningbluefox.github.io/reng/). The repository's Pages source must be set to GitHub Actions.

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

The spinning cube was verified on macOS with Metal (Apple M1 Max) at Retina resolution, including very wide and very tall window sizes without stretching and clean window closure. It also rendered in Chrome 157 on macOS using a non-fallback Apple `metal-3` WebGPU adapter, including viewport resizing, with no console warnings or errors. Automated Chromium software WebGPU tests compare the cube at 960 × 540 and 640 × 480 and after zero-size recovery. In the same Chrome, mouse drag, scroll-wheel zoom, holding D, and Space each moved or reset the camera as described in [Camera controls](#camera-controls). Native camera input, trackpad pinch on macOS, and real touch screens have not been tried by hand yet. The release WebAssembly build also rendered the cube and labels in Playwright's WebKit 27.2 (Safari's engine) on macOS with an Apple WebGPU adapter, in an emulated iPad Pro 11 viewport of 1668 × 2388 physical pixels. The checks above predate textures. The textured die was checked as a release build in that WebKit 27.2 and in Playwright's Chromium with its default macOS WebGPU adapter, by the Chromium software WebGPU snapshots, and by native Metal startup, which decoded and uploaded the texture and built the pipeline without validation errors; frames in the native window could not be observed in that session, so native texture drawing, resizing, and closing still need a hand check. Directional lighting was checked the same ways in the browsers and by a Rust test of lit vertices facing toward and away from the light and under non-uniform scale. The sphere and Suzanne were checked in the same WebKit and Chromium release builds and the Chromium software WebGPU snapshots; Rust tests check that they have no degenerate triangles, that the sphere is closed and faces outward, and that Suzanne has Blender's vertex and face counts and bounds and faces outward. Windows, Firefox, and Safari on real iPhone or iPad hardware remain unverified; surface-loss recovery was not forced during these checks.

![Die, sphere, and Suzanne rendered by Chromium software WebGPU](web/tests/snapshots/cube.png)

See [AGENTS.md](AGENTS.md) for project conventions and validation commands.

## Planned additions

Add each dependency when its feature needs it. Some proposed crates already arrive transitively through winit; declare them directly when reng uses their APIs:

| Next feature | Planned additions | When to add them |
| --- | --- | --- |
| Typed vertex and uniform buffers | Potentially `bytemuck` | When structured GPU data needs typed byte casts; the renderer currently packs vertex floats with the standard library. |
| Input bindings | Rebindable keys, gamepads | When a game needs controls beyond the fixed camera keys. |
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

[Symposium](https://symposium.dev/) is an optional tool that agents use to find the exact source of a dependency version through `cargo agents crate-info <crate>`. Install it with `cargo install symposium --version 0.5.1 --locked`. The project uses only `crate-info`, so `cargo agents init` and its hooks are not required.

[Context7](https://github.com/upstash/context7) is an optional documentation tool for current Rust library APIs. Connect it through your agent environment; it is not required to build the project and is not bundled in the shared MCP configuration. No additional MCP servers are needed for the current milestone.
