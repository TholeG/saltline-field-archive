import { createAudioSynthesis, loadAudioModule } from "./audio-runtime.js";
import { createPhysicsWorld } from "./physics-runtime.js";

const isFloat32 = (buffer) =>
  Object.prototype.toString.call(buffer) === "[object Float32Array]";

export function createGameKernels(module, loadAudio = loadAudioModule) {
  const wasm = new WebAssembly.Instance(module).exports;
  let physicsInstanceAvailable = true;
  let waveSnapshot;
  let surfaceSnapshot;

  function uploadWaves(waves) {
    if (waveSnapshot === waves) return;
    const address = wasm.water_parameters(waves.length);
    const parameters = new Float64Array(
      wasm.memory.buffer,
      address,
      waves.length * 5,
    );
    let offset = 0;
    for (const [x, z, , amplitude, waveNumber, frequency] of waves) {
      parameters[offset++] = x;
      parameters[offset++] = z;
      parameters[offset++] = amplitude;
      parameters[offset++] = waveNumber;
      parameters[offset++] = frequency;
    }
    wasm.water_read_parameters();
    waveSnapshot = waves;
    surfaceSnapshot = undefined;
  }

  return {
    water: {
      height(waves, seaLevel, x, z, time) {
        uploadWaves(waves);
        return wasm.water_height(x, z, time, seaLevel);
      },
      updateSurface(waves, seaLevel, vertices, time) {
        uploadWaves(waves);
        if (surfaceSnapshot !== vertices) {
          const address = wasm.water_vertices(vertices.length / 3);
          new Float32Array(wasm.memory.buffer, address, vertices.length).set(
            vertices,
          );
          wasm.water_prepare_surface();
          surfaceSnapshot = vertices;
        }
        const address = wasm.water_update_surface(time, seaLevel);
        vertices.set(
          new Float32Array(wasm.memory.buffer, address, vertices.length),
        );
      },
    },
    createPhysics(config, adapters) {
      const instance = physicsInstanceAvailable
        ? wasm
        : new WebAssembly.Instance(module).exports;
      physicsInstanceAvailable = false;
      return createPhysicsWorld(instance, config, adapters);
    },
    audio: {
      ...createAudioSynthesis(loadAudio),
      filter(channel, b0, b1, b2, a1, a2) {
        if (!isFloat32(channel) || channel.length < 1024) return false;
        const address = wasm.audio_buffer(channel.length);
        const data = new Float32Array(
          wasm.memory.buffer,
          address,
          channel.length,
        );
        data.set(channel);
        wasm.audio_filter(b0, b1, b2, a1, a2);
        channel.set(data);
        return true;
      },
      finalize(spec, channels, sampleRate) {
        let length = 0;
        const buffers = new Set();
        for (const channel of channels) {
          if (!isFloat32(channel) || buffers.has(channel.buffer)) return null;
          buffers.add(channel.buffer);
          length += channel.length;
        }
        if (length < 1024) return null;
        const address = wasm.audio_buffer(length);
        const data = new Float32Array(wasm.memory.buffer, address, length);
        let start = 0;
        let peak = 0;
        for (const channel of channels) {
          const half = channel.length >> 1;
          const attack = spec.loop
            ? 0
            : Math.max(
                0,
                Math.min(half, Math.round((spec.ir ? 0 : 0.0003) * sampleRate)),
              );
          const release = spec.loop
            ? 0
            : Math.max(
                0,
                Math.min(
                  half,
                  Math.round((spec.ir ? 0.05 : 0.012) * sampleRate),
                ),
              );
          data.set(channel, start);
          peak = Math.max(
            peak,
            wasm.audio_peak_fade(start, channel.length, attack, release),
          );
          start += channel.length;
        }
        const gain = spec.fixed
          ? Math.min(1, spec.peak / (peak || 1))
          : peak > 0
            ? spec.peak / peak
            : 1;
        wasm.audio_scale(gain);
        start = 0;
        for (const channel of channels) {
          channel.set(data.subarray(start, start + channel.length));
          start += channel.length;
        }
        return { sr: sampleRate, ch: channels, loop: !!spec.loop };
      },
    },
  };
}
