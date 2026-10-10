# reng

A game engine written in Rust, intended for macOS, Windows, and web through WebAssembly.

## Current milestone

`reng` opens a resizable native window titled **reng**, initially 960 × 540 logical pixels. Closing the window exits the application. The event loop sleeps while idle; there is no drawing or GPU initialization yet.

Window management uses [winit](https://docs.rs/winit/0.30.13/winit/). Rendering with [wgpu](https://wgpu.rs/) and browser packaging are planned.

`main.rs` starts `WindowHandler` with `WindowSpecs::default()`. The specs hold the title, logical width and height, and resizable flag. All `winit` integration and window lifecycle logic live in `src/window.rs`.

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

## macOS proof of concept

The window was opened, resized with the native zoom/restore control, and closed successfully on macOS. Windows and WebAssembly compilation checks also pass; Windows runtime and browser startup have not been tested.

![Resizable reng window on macOS](docs/screenshots/resizable-window-macos.jpg)
