import { hash, toByte } from "./mask.js";

function smooth(fraction) {
  return fraction * fraction * (3 - 2 * fraction);
}

function interpolate(corners, offset, stride, sx, sz) {
  const a = corners[offset];
  const b = corners[offset + 1];
  const c = corners[offset + stride];
  const d = corners[offset + stride + 1];
  return a + (b - a) * sx + (c - a) * sz + (a - b - c + d) * sx * sz;
}

function wrap(index, period) {
  return (index + period) % period;
}

class PeriodicNoise {
  constructor(resolution, scale, seed, octaveCount) {
    this.octaves = [];
    this.totalWeight = 0;
    for (let octave = 0; octave < octaveCount; octave++) {
      const period = scale * 2 ** octave;
      const stride = period + 1;
      const corners = new Float64Array(stride * stride);
      const indices = new Uint16Array(resolution);
      const blends = new Float64Array(resolution);
      const weight = 0.5 ** (octave + 1);

      // Duplicate the first row and column at the seam for contiguous lookups.
      for (let z = 0; z <= period; z++) {
        for (let x = 0; x <= period; x++) {
          corners[z * stride + x] = hash(
            x % period,
            z % period,
            seed + octave * 17,
          );
        }
      }
      for (let index = 0; index < resolution; index++) {
        const coordinate = (index / resolution) * period;
        const lattice = Math.floor(coordinate);
        indices[index] = lattice;
        blends[index] = smooth(coordinate - lattice);
      }
      this.octaves.push({ period, stride, corners, indices, blends, weight });
      this.totalWeight += weight;
    }
  }

  row(z, target) {
    target.fill(0);
    for (const { stride, corners, indices, blends, weight } of this.octaves) {
      const rowOffset = indices[z] * stride;
      const sz = blends[z];
      for (let x = 0; x < target.length; x++) {
        target[x] +=
          weight *
          interpolate(corners, rowOffset + indices[x], stride, blends[x], sz);
      }
    }
    for (let x = 0; x < target.length; x++) target[x] /= this.totalWeight;
  }

  at(x, z) {
    let sum = 0;
    for (const { period, stride, corners, weight } of this.octaves) {
      const ix = Math.floor(x);
      const iz = Math.floor(z);
      const offset = wrap(iz, period) * stride + wrap(ix, period);
      sum +=
        weight *
        interpolate(corners, offset, stride, smooth(x - ix), smooth(z - iz));
      x *= 2;
      z *= 2;
    }
    return sum / this.totalWeight;
  }
}

export function generateGroundDetailJs(resolution) {
  const count = resolution * resolution;
  const det = new Uint8Array(count * 4);
  const nrm = new Uint8Array(count * 4);
  const fineHeights = new Float32Array(count);
  const roughHeights = new Float32Array(count);
  const fields = [
    new PeriodicNoise(resolution, 96, 3, 2),
    new PeriodicNoise(resolution, 40, 9, 2),
    new PeriodicNoise(resolution, 6, 21, 4),
    new PeriodicNoise(resolution, 8, 31, 2),
    new PeriodicNoise(resolution, 8, 37, 2),
    new PeriodicNoise(resolution, 28, 51, 3),
    new PeriodicNoise(resolution, 9, 71, 4),
  ];
  const samples = fields.map(() => new Float64Array(resolution));
  const [fine, coarse, patches, warpX, warpZ, speckles, rough] = samples;
  const ridges = new PeriodicNoise(resolution, 5, 43, 3);
  const neighbors = Array.from({ length: resolution }, (_, i) => ({
    minusOne: wrap(i - 1, resolution),
    plusOne: wrap(i + 1, resolution),
    minusTwo: wrap(i - 2, resolution),
    plusTwo: wrap(i + 2, resolution),
  }));

  for (let row = 0; row < resolution; row++) {
    for (let i = 0; i < fields.length; i++) fields[i].row(row, samples[i]);
    const z = row / resolution;
    for (let col = 0; col < resolution; col++) {
      const index = row * resolution + col;
      const x = col / resolution;
      const height = fine[col] * 0.6 + coarse[col] * 0.4;
      const ridge = ridges.at(
        (x + (warpX[col] - 0.5) * 0.25) * 5,
        (z + (warpZ[col] - 0.5) * 0.25) * 5,
      );
      det[index * 4] = toByte(height);
      det[index * 4 + 1] = toByte(patches[col]);
      det[index * 4 + 2] = toByte(1 - Math.abs(2 * ridge - 1));
      det[index * 4 + 3] = toByte(speckles[col]);
      fineHeights[index] = height;
      roughHeights[index] = rough[col];
    }
  }

  for (let row = 0; row < resolution; row++) {
    const z = neighbors[row];
    const offset = row * resolution;
    for (let col = 0; col < resolution; col++) {
      const x = neighbors[col];
      const pixel = (offset + col) * 4;
      const fineX =
        (fineHeights[offset + x.plusOne] - fineHeights[offset + x.minusOne]) *
        2.2;
      const fineZ =
        (fineHeights[z.plusOne * resolution + col] -
          fineHeights[z.minusOne * resolution + col]) *
        2.2;
      const roughX =
        (roughHeights[offset + x.plusTwo] - roughHeights[offset + x.minusTwo]) *
        9;
      const roughZ =
        (roughHeights[z.plusTwo * resolution + col] -
          roughHeights[z.minusTwo * resolution + col]) *
        9;
      nrm[pixel] = toByte(0.5 - fineX);
      nrm[pixel + 1] = toByte(0.5 - fineZ);
      nrm[pixel + 2] = toByte(0.5 - roughX);
      nrm[pixel + 3] = toByte(0.5 - roughZ);
    }
  }

  return { det, nrm };
}
