function smoothstep(min, max, value) {
  const t = Math.min(1, Math.max(0, (value - min) / (max - min)));
  return t * t * (3 - 2 * t);
}

export function hash(x, z, seed) {
  let bits =
    Math.imul(x, 374761393) ^
    Math.imul(z, 668265263) ^
    Math.imul(seed, 1274126177);
  bits = Math.imul(bits ^ (bits >>> 13), 1274126177);
  return ((bits ^ (bits >>> 16)) >>> 0) / 4294967296;
}

function valueNoise(x, z, seed) {
  const ix = Math.floor(x);
  const iz = Math.floor(z);
  const dx = x - ix;
  const dz = z - iz;
  const sx = dx * dx * (3 - 2 * dx);
  const sz = dz * dz * (3 - 2 * dz);
  const a = hash(ix, iz, seed);
  const b = hash(ix + 1, iz, seed);
  const c = hash(ix, iz + 1, seed);
  const d = hash(ix + 1, iz + 1, seed);
  return a + (b - a) * sx + (c - a) * sz + (a - b - c + d) * sx * sz;
}

function fractal(x, z, seed, octaves) {
  let sum = 0;
  let weight = 0.5;
  let totalWeight = 0;
  for (let octave = 0; octave < octaves; octave++) {
    sum += weight * valueNoise(x, z, seed + octave * 17);
    totalWeight += weight;
    x *= 2;
    z *= 2;
    weight *= 0.5;
  }
  return sum / totalWeight;
}

function gaussian(value) {
  return Math.exp(-value * value);
}

export function toByte(value) {
  return Math.round(Math.min(1, Math.max(0, value)) * 255);
}

export function generateGroundMaskJs(
  resolution,
  { bounds, basins, basinWidth, basinDepth, waterline },
) {
  const step = bounds.size / resolution;
  const basinCount = basins.length;
  const basinX = new Float64Array(resolution * basinCount);
  const columns = [];
  const rows = [];

  // Basin distances are separable. Aisle weights vary along X and their falloff along Z.
  for (let index = 0; index < resolution; index++) {
    const x = bounds.x0 + (index + 0.5) * step;
    const z = bounds.z0 + (index + 0.5) * step;
    const activeBasins = [];
    for (let basin = 0; basin < basinCount; basin++) {
      const [centerX, centerZ, radiusX, radiusZ] = basins[basin];
      const dx = (x - centerX) / (radiusX * basinWidth);
      const dz = (z - centerZ) / (radiusZ * basinWidth);
      const offset = index * basinCount + basin;
      basinX[offset] = dx * dx;
      const distanceZ = dz * dz;
      if (distanceZ < 9) activeBasins.push({ basin, distanceZ });
    }
    const inAisle = x > -37 && x < 29;
    let gutterWeight = 0;
    let laneWeight = 0;
    if (inAisle) {
      const texture = 0.5 + 0.5 * valueNoise(x / 6, 3.1, 5);
      const drain = fractal(x / 9, 7.7, 19, 2);
      const gate =
        smoothstep(0.14, 0.34, drain) *
        smoothstep(-27, -24, x) *
        (1 - smoothstep(14, 17, x));
      gutterWeight = 0.22 * (0.55 + 0.45 * texture) * gate;
      laneWeight = 0.09 * texture * (1 - smoothstep(0.3, 0.6, drain));
    }
    columns.push({
      x,
      inAisle,
      gutterWeight,
      laneWeight,
      inland: 0.25 * smoothstep(28.6, 29.8, x),
    });
    rows.push({
      z,
      activeBasins,
      gutter: gaussian((z - 4.9) / 0.5) + gaussian((z - 7.1) / 0.5),
      lane: gaussian((z - 15.4) / 0.5) + gaussian((z - 17.2) / 0.5),
    });
  }

  const strengths = basins.map((basin) => basin[4] + basinDepth);
  const pixels = new Uint8Array(resolution * resolution * 4);
  let pixel = 0;
  for (const { z, gutter, lane, activeBasins } of rows) {
    for (let col = 0; col < resolution; col++) {
      const { x, inAisle, gutterWeight, laneWeight, inland } = columns[col];
      let height =
        0.5 +
        (fractal(x / 8.5, z / 8.5, 11, 4) - 0.47) * 0.3 +
        (fractal(x / 2.4, z / 2.4, 23, 3) - 0.5) * 0.14;
      for (const { basin, distanceZ } of activeBasins) {
        const distance = basinX[col * basinCount + basin] + distanceZ;
        if (distance < 9) height -= strengths[basin] * gaussian(distance);
      }
      if (inAisle) height -= gutterWeight * gutter + laneWeight * lane;
      height += inland;

      pixels[pixel++] = toByte(0.5 + (height - waterline) * 2.5);
      pixels[pixel++] = toByte(
        0.5 +
          (fractal(x / 5.5, z / 5.5, 41, 3) - 0.5) * 1.5 +
          (0.5 - height) * 0.9,
      );
      pixels[pixel++] = toByte(
        smoothstep(0.6, 0.78, fractal(x / 1.4, z / 1.4, 77, 4)) *
          smoothstep(0.47, 0.62, fractal(x / 9, z / 9, 91, 2)),
      );
      pixels[pixel++] = toByte(
        fractal(x / 22, z / 0.9, 57, 3) * 0.6 +
          fractal(x / 13, z / 13, 63, 3) * 0.4,
      );
    }
  }
  return pixels;
}
