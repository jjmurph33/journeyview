```md
# Journey View

Rust app for viewing GPX files and sharing them with friends.
It can run as a native desktop app or in the browser with WebAssembly.

## Requirements

- Rust toolchain
- For the web build: `trunk` and the `wasm32-unknown-unknown` target

Install the web requirements with:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Run natively

```sh
cargo run
```

## Run in the browser

```sh
./run-trunk.sh
```

The web app serves on port `8001` by default.

## Test data

Sample GPX files are available in `testdata/`. In the native app, use **Load File** or drag and drop a `.gpx` file into the window.

Example path:

```text
testdata/blue_hills.gpx
```

## Project layout

- `src/main.rs` - native and WebAssembly entry points
- `src/app.rs` - egui application UI and plotting logic
- `src/journey.rs` - GPX loading plus journey import/export encoding
- `index.html` - Trunk HTML entry point for the web app
- `run-trunk.sh` - helper script for serving the web build
```
