import { Buffer } from "node:buffer";
import { runInNewContext } from "node:vm";
import { generateGroundMaskJs } from "../ground/mask.js";
import { generateGroundDetailJs } from "../ground/detail.js";
import {
  generateGroundMaskWasm,
  generateGroundDetailWasm,
} from "../ground/runtime.js";

const original = await Bun.file(new URL("../game.js", import.meta.url)).text();
const startup = "cL().catch(n=>yT(n&&n.stack||n));";
if (original.split(startup).length !== 2)
  throw new Error("Original game entry has changed.");

const sandbox = {
  window: { addEventListener() {} },
  document: { getElementById: () => null },
  console: { log() {}, warn() {}, error() {} },
  setTimeout,
  clearTimeout,
  performance,
  AbortController,
  URL,
};
const { render, detail, config } = runInNewContext(
  original.replace(
    startup,
    `globalThis.ground = {
    render: Dw,
    detail: Lw,
    config: { bounds: Fi, basins: Pw, basinWidth: Iw, basinDepth: EP, waterline: wP }
  };`,
  ) + "\nground;",
  sandbox,
);
const wasm = await WebAssembly.compile(
  await Bun.file(new URL("../ground/mask.wasm", import.meta.url)).arrayBuffer(),
);

function checkPixels(
  label: string,
  reference: Uint8Array,
  variants: Record<string, Uint8Array>,
) {
  const expected = Buffer.from(reference);
  for (const [name, buffer] of Object.entries(variants)) {
    const actual = Buffer.from(
      buffer.buffer,
      buffer.byteOffset,
      buffer.byteLength,
    );
    if (actual.equals(expected)) continue;
    const offset = actual.findIndex(
      (value, index) => value !== expected[index],
    );
    throw new Error(
      `${label}: ${name} differs from the original (byte ${offset}).`,
    );
  }
  console.log(
    `${label}: ${expected.length} identical RGBA bytes for JavaScript, Rust and the original.`,
  );
}

for (const resolution of [512, 1024]) {
  checkPixels(`Ground mask ${resolution}²`, render(resolution), {
    JavaScript: generateGroundMaskJs(resolution, config),
    WebAssembly: generateGroundMaskWasm(wasm, resolution, config),
  });
}

for (const resolution of [256, 512]) {
  const expected = detail(resolution);
  const js = generateGroundDetailJs(resolution);
  const native = generateGroundDetailWasm(wasm, resolution);
  checkPixels(`Ground detail ${resolution}²`, expected.det, {
    JavaScript: js.det,
    WebAssembly: native.det,
  });
  checkPixels(`Ground normals ${resolution}²`, expected.nrm, {
    JavaScript: js.nrm,
    WebAssembly: native.nrm,
  });
}
