import init, { start } from "./pkg/tidewatch.js";

const status = document.querySelector("#load-status");
const backend = document.querySelector("#gpu-backend");
const enter = document.querySelector("#enter-harbor");

async function launch() {
  await init();
  const forceWebGl = new URLSearchParams(location.search).get("backend") === "webgl2" || !navigator.gpu;
  const canvas = document.querySelector("#harbor");
  try {
    return await start(canvas, forceWebGl);
  } catch (error) {
    if (forceWebGl) throw error;
    // A canvas with a WebGPU context cannot be reused for WebGL.
    const replacement = canvas.cloneNode();
    canvas.replaceWith(replacement);
    return await start(replacement, true);
  }
}

try {
  backend.textContent = await launch();
  status.hidden = true;
  enter.disabled = false;
} catch (error) {
  console.error("Tidewatch could not start", error);
  backend.textContent = "3D unavailable";
  enter.hidden = true;
  status.textContent = "Open in a hardware-accelerated browser with WebGPU or WebGL 2.";
}
