# Journey View

Program for viewing [GPX](https://en.wikipedia.org/wiki/GPS_Exchange_Format) files and sharing them with friends.  

Built with Rust and [egui](https://github.com/emilk/egui), it can run as a native desktop app or in the browser with WebAssembly.

## Requirements

- Rust toolchain
- For the web build: `trunk` and the `wasm32-unknown-unknown` target

Install the web requirements with:

```
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Build

```
cargo build --release
```

## Web

```
trunk build --release
```
