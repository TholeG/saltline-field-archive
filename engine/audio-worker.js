import { createSoundRenderer } from "./audio-runtime.js";

let renderer;

self.addEventListener("message", ({ data }) => {
  if (data.module) {
    renderer = createSoundRenderer(data.module);
    return;
  }
  const { id, args } = data;
  let samples = null;
  try {
    samples = renderer.render(...args);
  } catch {
    // The main-thread renderer can retry when the worker fails.
  }
  self.postMessage(
    { id, samples },
    samples ? samples.ch.map(({ buffer }) => buffer) : [],
  );
});
