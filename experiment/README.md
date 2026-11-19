# Tidewatch

Tidewatch is an original Rust/WebAssembly harbor game. Rust generates the harbor and simulates the player, collisions, shore power, the tide gate, the adjustable signal, and the departing ferry. The browser uses `wgpu` for WebGPU and a compact `web-sys` renderer for WebGL 2. Both renderers share the scene and camera code. The completion chime uses Web Audio.

From the repository root, run `bun run start`. The browser bundle in `pkg/` is ready to play and loads all resources from the local server. [Open Tidewatch](http://127.0.0.1:3000/experiment/index.html) · [Force WebGL 2](http://127.0.0.1:3000/experiment/index.html?backend=webgl2)

1. Select **Enter harbor**.
2. Use **W A S D** to walk. Turn with the left and right arrows or drag the scene.
3. Follow the **Power** marker and hold **E** at the breaker. Cross the pier and hold **E** at the winch to lift the gate.
4. At the signal, hold **E** and use **A** or **D** to aim the light into the fairway. Release the steering key once it is centered and hold **E** until the ferry gets underway.

## Build and test

Run the following from `experiment/` with the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` **0.2.127**, which matches the crate version in `Cargo.lock`. The included bundle was built with Rust 1.98.1.

```sh
cargo test --locked
cargo build --locked --release --target wasm32-unknown-unknown
wasm-bindgen "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/tidewatch.wasm" \
  --target web --out-dir pkg --no-typescript
```

To keep the CLI in a temporary tool directory, use `cargo install --locked --version 0.2.127 --root /tmp/tidewatch-tools wasm-bindgen-cli` and run `/tmp/tidewatch-tools/bin/wasm-bindgen` in the last command. The generated browser bundle is 288,085 bytes before transfer compression. Its WASM file accounts for 211,994 bytes.

The Rust tests cover the route over the pier, collisions, and the required order, range, and signal alignment. A desktop Chrome test completed all operations and reached **Channel clear** with WebGPU and automatic WebGL 2 fallback while blocking external requests.
