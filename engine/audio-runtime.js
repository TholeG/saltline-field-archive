let audioModule;

export function loadAudioModule() {
  return (audioModule ??= (async () => {
    const response = await fetch(new URL("./audio.wasm", import.meta.url));
    if (!response.ok) return null;
    if (
      WebAssembly.compileStreaming &&
      response.headers.get("content-type") === "application/wasm"
    ) {
      return WebAssembly.compileStreaming(response);
    }
    return WebAssembly.compile(await response.arrayBuffer());
  })().catch(() => null));
}

export function createSoundRenderer(module) {
  const wasm = new WebAssembly.Instance(module).exports;
  return {
    hasSound(sound, variant, sampleRate) {
      return (
        Number.isSafeInteger(variant) &&
        variant >= 0 &&
        Number.isInteger(sampleRate) &&
        sampleRate >= 8000 &&
        sampleRate <= 192000 &&
        !!wasm.audio_has_sound?.(sound)
      );
    },
    render(sound, variant, sampleRate, peak, flags) {
      const address = wasm.audio_synthesize(
        sound,
        variant,
        sampleRate,
        peak,
        flags,
      );
      if (!address) return null;
      const frames = wasm.audio_pcm_frames();
      const channels = wasm.audio_pcm_channels();
      const buffer = wasm.memory.buffer;
      const ch = Array.from({ length: channels }, (_, index) =>
        new Float32Array(buffer, address + index * frames * 4, frames).slice(),
      );
      return { sr: sampleRate, ch, loop: !!(flags & 1) };
    },
  };
}

function options(spec, sound, variant, rate) {
  const flags =
    Number(!!spec.loop) |
    (Number(!!spec.ir) << 1) |
    (Number(!!spec.fixed) << 2);
  return [sound, variant, rate, spec.peak, flags];
}

export function createAudioSynthesis(loadModule) {
  let ready;
  let module;
  let renderer;
  let worker;
  let unavailable =
    typeof window === "undefined" || typeof Worker !== "function";
  let sequence = 0;
  const jobs = new Map();

  function stopWorker() {
    unavailable = true;
    worker?.terminate();
    worker = undefined;
    for (const job of jobs.values()) job(null);
    jobs.clear();
  }

  function send(args, receive) {
    if (unavailable) return false;
    try {
      if (!worker) {
        worker = new Worker(new URL("./audio-worker.js", import.meta.url), {
          type: "module",
        });
        worker.addEventListener("error", stopWorker);
        worker.addEventListener("messageerror", stopWorker);
        worker.addEventListener("message", ({ data: { id, samples } }) => {
          jobs.get(id)?.(samples);
          jobs.delete(id);
        });
        worker.postMessage({ module });
      }
      const id = ++sequence;
      jobs.set(id, receive);
      worker.postMessage({ id, args });
      return true;
    } catch {
      stopWorker();
      return false;
    }
  }

  return {
    prepare() {
      return (ready ??= Promise.resolve()
        .then(loadModule)
        .then((candidate) => {
          if (!candidate) return;
          renderer = createSoundRenderer(candidate);
          module = candidate;
          performance.mark("saltline:audio-synthesis:wasm");
        })
        .catch(() => {}));
    },
    hasSound(sound, variant, rate) {
      return renderer?.hasSound(sound, variant, rate) ?? false;
    },
    synthesize(spec, sound, variant, rate) {
      if (!renderer?.hasSound(sound, variant, rate)) return null;
      return renderer.render(...options(spec, sound, variant, rate));
    },
    steps(spec, sound, variant, rate) {
      const args = options(spec, sound, variant, rate);
      let waiting = false;
      let requested = false;
      let result;
      return {
        get pending() {
          return waiting;
        },
        step() {
          if (result) return result;
          if (waiting) return null;
          if (!requested) {
            requested = true;
            waiting = send(args, (samples) => {
              waiting = false;
              result = samples;
            });
            if (waiting) return null;
          }
          return (result = renderer.render(...args));
        },
      };
    },
  };
}
