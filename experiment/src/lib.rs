#[cfg(any(target_arch = "wasm32", test))]
mod world;

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
mod scene;
