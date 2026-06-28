# Journey View

Rust app for viewing GPX files and sharing them with friends.
It can run as a native desktop app or in the browser with WebAssembly.

## Requirements

- Rust toolchain
- For the web build: `trunk` and the `wasm32-unknown-unknown` target

Install the web requirements with:

```
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

