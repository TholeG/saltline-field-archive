import { generateGroundMaskJs } from "./mask.js";
import { generateGroundDetailJs } from "./detail.js";

export function generateGroundMaskWasm(
  module,
  resolution,
  { bounds, basins, basinWidth, basinDepth, waterline },
) {
  const { memory, basin_buffer, render_mask } = new WebAssembly.Instance(module)
    .exports;
  const input = basin_buffer(basins.length);
  const parameters = new DataView(memory.buffer, input, basins.length * 5 * 8);
  let offset = 0;
  for (const basin of basins) {
    for (const value of basin) {
      parameters.setFloat64(offset, value, true);
      offset += 8;
    }
  }

  const output = render_mask(
    resolution,
    bounds.x0,
    bounds.z0,
    bounds.size,
    basinWidth,
    basinDepth,
    waterline,
  );
  // Each raster has its own instance. Three.js can keep the view without a copy.
  return new Uint8Array(memory.buffer, output, resolution * resolution * 4);
}

export function generateGroundDetailWasm(module, resolution) {
  const { memory, render_detail } = new WebAssembly.Instance(module).exports;
  const start = render_detail(resolution);
  const byteLength = resolution * resolution * 4;
  return {
    det: new Uint8Array(memory.buffer, start, byteLength),
    nrm: new Uint8Array(memory.buffer, start + byteLength, byteLength),
  };
}

async function compileGroundTextures() {
  const response = await fetch(new URL("./mask.wasm", import.meta.url));
  if (!response.ok)
    throw new Error(`Ground textures WASM HTTP ${response.status}`);
  if (
    WebAssembly.compileStreaming &&
    response.headers.get("Content-Type") === "application/wasm"
  ) {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

let groundModule;

function loadGroundModule() {
  return (groundModule ??= compileGroundTextures().catch(() => null));
}

export async function prepareGameKernels() {
  if (typeof WebAssembly === "undefined") return null;
  const module = await loadGroundModule();
  if (!module) return null;
  const { createGameKernels } = await import("../engine/runtime.js");
  return createGameKernels(module);
}

function selectTextureBackend(ready, name, generateWasm, generateJs) {
  return async (...args) => {
    const start = performance.now();
    const module = await ready;
    if (module) {
      try {
        const pixels = generateWasm(module, ...args);
        performance.measure(`saltline:ground-${name}:wasm`, { start });
        return pixels;
      } catch {
        // Continue via JavaScript if a browser rejects or cannot run the module.
      }
    }
    const pixels = generateJs(...args);
    performance.measure(`saltline:ground-${name}:js`, { start });
    return pixels;
  };
}

export function prepareGroundTextures(mode) {
  const ready =
    mode !== "js" && typeof WebAssembly !== "undefined"
      ? loadGroundModule()
      : null;

  return {
    mask: selectTextureBackend(
      ready,
      "mask",
      generateGroundMaskWasm,
      generateGroundMaskJs,
    ),
    detail: selectTextureBackend(
      ready,
      "detail",
      generateGroundDetailWasm,
      generateGroundDetailJs,
    ),
  };
}

export function prepareGroundMask(mode) {
  return prepareGroundTextures(mode).mask;
}
