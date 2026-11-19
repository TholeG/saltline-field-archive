import { createHarbor } from "./harbor.js";
import { createMeshBatcher } from "./mesh-batch.js";
import { createMeshEditor } from "./mesh-edit.js";

export function createObjectGeometry(module) {
  const wasm = new WebAssembly.Instance(module).exports;

  function copyMesh(address, vertexCount, indexAddress, indexCount) {
    // The mesh owns its arrays after the scratch buffer is reused or WASM memory grows.
    const { buffer } = wasm.memory;
    const attributes = new Float32Array(
      buffer,
      address,
      vertexCount * 8,
    ).slice();
    const positionLength = vertexCount * 3;
    return {
      positions: attributes.subarray(0, positionLength),
      normals: attributes.subarray(positionLength, positionLength * 2),
      uvs: attributes.subarray(positionLength * 2),
      indices:
        indexCount === undefined
          ? null
          : new Uint16Array(buffer, indexAddress, indexCount).slice(),
    };
  }

  function uploadPositions(positions) {
    const offset = wasm.soft_mesh_positions(positions.length / 3);
    new Float32Array(wasm.memory.buffer, offset, positions.length).set(
      positions,
    );
  }

  return {
    harbor: createHarbor(wasm),
    assembleMeshes: createMeshBatcher(wasm),
    ...createMeshEditor(wasm),
    paintContainerSide(
      positions,
      {
        seed,
        wear,
        sideDepth,
        roof,
        upperLimit,
        side,
        baseTint,
        streakTint,
        dents,
        streaks,
      },
    ) {
      if (!(positions instanceof Float32Array)) return;
      const sideDents = dents.filter((dent) => dent.side === side);
      const sideStreaks = streaks.filter((streak) => streak.side === side);
      const length = 6 + sideDents.length * 4 + sideStreaks.length * 3;
      const address = wasm.panel_paint_inputs(positions.length / 3, length);
      const buffer = wasm.memory.buffer;
      new Float32Array(buffer, address, positions.length).set(positions);
      const input = new Float64Array(
        buffer,
        wasm.panel_paint_details(),
        length,
      );
      input.set(baseTint);
      input.set(streakTint, 3);
      let offset = 6;
      for (const dent of sideDents) {
        input.set([dent.x, dent.y, dent.r, dent.depth], offset);
        offset += 4;
      }
      for (const streak of sideStreaks) {
        input.set([streak.x, streak.w, streak.k], offset);
        offset += 3;
      }
      const output = wasm.paint_container_panel(
        seed,
        wear,
        sideDepth,
        roof,
        upperLimit,
        sideDents.length,
      );
      return new Float32Array(
        wasm.memory.buffer,
        output,
        positions.length,
      ).slice();
    },
    sampleHull(stations, parameters) {
      const length =
        parameters.length +
        2 * stations.reduce((sum, station) => sum + station[1].length, 0);
      const address = wasm.vessel_input(length);
      const input = new Float64Array(wasm.memory.buffer, address, length);
      input.set(parameters);
      let offset = parameters.length;
      for (const [station, levels] of stations) {
        for (const height of levels) {
          input[offset++] = station;
          input[offset++] = height;
        }
      }
      const output = wasm.hull_samples();
      return new Float64Array(
        wasm.memory.buffer,
        output,
        6 * (length - parameters.length),
      ).slice();
    },
    hullPanel(
      parameters,
      start,
      end,
      bottom,
      top,
      side,
      offset,
      columns,
      rows,
    ) {
      const vertices = (columns + 1) * (rows + 1);
      if (
        vertices > 65535 ||
        ![columns, rows].every((count) => Number.isInteger(count) && count > 0)
      )
        return;
      const address = wasm.vessel_input(parameters.length);
      new Float64Array(wasm.memory.buffer, address, parameters.length).set(
        parameters,
      );
      const output = wasm.hull_panel(
        start,
        end,
        bottom,
        top,
        side,
        offset,
        columns,
        rows,
      );
      return copyMesh(
        output,
        vertices,
        wasm.hull_indices(),
        columns * rows * 6,
      );
    },
    dinghyHull(stations, beam, upper, keel) {
      const length = stations.length;
      if (
        length < 2 ||
        ![beam, upper, keel].every((section) => section.length === length)
      )
        return;
      const address = wasm.vessel_input(length * 4);
      const input = new Float64Array(wasm.memory.buffer, address, length * 4);
      for (let i = 0; i < length; i++) {
        input.set([stations[i], beam[i], upper[i], keel[i]], i * 4);
      }
      const output = wasm.dinghy_hull();
      const mesh = copyMesh(output, 6 * ((length - 1) * 4 + 2));
      mesh.uvs = null;
      return mesh;
    },
    extrudeOutline(outline, depth, soft, triangulate) {
      const count = outline.length;
      const vertexCount = count * (triangulate ? 6 : 4);
      if (
        count < 2 ||
        vertexCount > 65535 ||
        !Number.isFinite(depth) ||
        !Number.isFinite(soft)
      )
        return;

      const address = wasm.extrusion_points(count * 3);
      const points = new Float64Array(wasm.memory.buffer, address, count * 3);
      for (let i = 0; i < count; i++) {
        const [x, y] = outline[i];
        if (!Number.isFinite(x) || !Number.isFinite(y)) return;
        const [nextX, nextY] = outline[(i + 1) % count];
        points[i * 3] = x;
        points[i * 3 + 1] = y;
        points[i * 3 + 2] = Math.hypot(nextX - x, nextY - y) || 1;
      }
      const faces = triangulate?.().flat() ?? [];
      if (triangulate) {
        const address = wasm.extrusion_caps(faces.length);
        new Uint16Array(wasm.memory.buffer, address, faces.length).set(faces);
      }
      const output = wasm.extrude_outline(
        depth,
        Math.cos(soft),
        Boolean(triangulate),
      );
      return copyMesh(
        output,
        vertexCount,
        wasm.extrusion_indices(),
        count * 6 + faces.length * 2,
      );
    },
    sweepRibbon(path, offset, doubleSided) {
      const count = path.length;
      const vertexCount = count * (doubleSided ? 4 : 2);
      const [dx, dy, dz] = offset;
      if (
        count < 1 ||
        vertexCount > 65535 ||
        ![dx, dy, dz].every(Number.isFinite)
      )
        return;
      const address = wasm.extrusion_points(count * 4);
      const points = new Float64Array(wasm.memory.buffer, address, count * 4);
      let previous = path[0];
      for (let i = 0; i < count; i++) {
        const [x, y, z] = path[i];
        if (![x, y, z].every(Number.isFinite)) return;
        points.set(
          [
            x,
            y,
            z,
            Math.hypot(x - previous[0], y - previous[1], z - previous[2]),
          ],
          i * 4,
        );
        previous = path[i];
      }
      const output = wasm.sweep_ribbon(
        dx,
        dy,
        dz,
        Math.hypot(dx, dy, dz) * 2,
        Boolean(doubleSided),
      );
      return copyMesh(
        output,
        vertexCount,
        wasm.extrusion_indices(),
        (count - 1) * (doubleSided ? 12 : 6),
      );
    },
    drapeMesh(
      supports,
      columns,
      rows,
      skirtRows,
      bounds,
      limits,
      straps,
      uvScale,
      seed,
      random,
    ) {
      const topCount = (columns + 1) * (rows + 1);
      const boundaryCount = 2 * (columns + rows);
      const vertexCount = topCount + boundaryCount * skirtRows;
      if (
        vertexCount > 65535 ||
        ![columns, rows, skirtRows].every(
          (n) => Number.isInteger(n) && n > 0,
        ) ||
        ![...bounds, ...limits, uvScale, seed].every(Number.isFinite) ||
        limits.some((n) => n <= 0)
      )
        return;
      const supportLength = supports.length * 5;
      const inputLength = supportLength + boundaryCount + 1;
      const address = wasm.drape_input(inputLength);
      const input = new Float64Array(wasm.memory.buffer, address, inputLength);
      for (let i = 0; i < supports.length; i++) input.set(supports[i], i * 5);
      for (let i = supportLength; i < inputLength; i++) input[i] = random();
      const output = wasm.drape_mesh(
        columns,
        rows,
        skirtRows,
        supports.length,
        ...bounds,
        ...limits,
        ...straps,
        uvScale,
        seed,
      );
      const mesh = copyMesh(
        output,
        vertexCount,
        wasm.drape_indices(),
        6 * (columns * rows + boundaryCount * skirtRows),
      );

      // The tie-downs use the same perimeter order as the mesh generator.
      const topVertex = (column, row) => row * (columns + 1) + column;
      const boundary = [];
      const edgeIndices = new Map();
      function addEdge(column, row) {
        edgeIndices.set(`${column},${row}`, boundary.length);
        boundary.push(topVertex(column, row));
      }
      for (let x = 0; x < columns; x++) addEdge(x, 0);
      for (let z = 0; z < rows; z++) addEdge(columns, z);
      for (let x = columns; x > 0; x--) addEdge(x, rows);
      for (let z = rows; z > 0; z--) addEdge(0, z);
      const skirtVertex = (edge, layer) =>
        layer === 0 ? boundary[edge] : topCount + edge * skirtRows + layer - 1;
      return { mesh, topVertex, skirtVertex, edgeIndices };
    },
    boxGrid(width, height, depth, segments, radius, snapEdges = false) {
      const rounded = radius !== undefined;
      if (
        ![width, height, depth].every(
          (size) => Number.isFinite(size) && size > 0,
        ) ||
        !segments.every((count) => Number.isInteger(count) && count > 0) ||
        (rounded && !Number.isFinite(radius))
      )
        return;
      const [x, y, z] = segments;
      const vertexCount =
        2 * ((x + 1) * (y + 1) + (x + 1) * (z + 1) + (y + 1) * (z + 1));
      if (vertexCount > 65535) return;
      const faces = [y * z, y * z, x * z, x * z, x * y, x * y];
      let start = 0;
      const groups = faces.map((quads, material) => {
        const count = quads * 6;
        const group = [start, count, material];
        start += count;
        return group;
      });
      const address = wasm.box_grid_mesh(
        width,
        height,
        depth,
        x,
        y,
        z,
        snapEdges ? 2 : Number(rounded),
        radius ?? 0,
      );
      const mesh = copyMesh(
        address,
        vertexCount,
        wasm.box_grid_indices(),
        start,
      );
      mesh.groups = groups;
      return mesh;
    },
    deformSandbag(positions, width, height, depth, seed, lean, sag) {
      if (
        ![width, height, depth].every(
          (size) => Number.isFinite(size) && size > 0,
        ) ||
        ![seed, lean, sag].every(Number.isFinite)
      )
        return false;
      uploadPositions(positions);
      const output = wasm.deform_sandbag(width, height, depth, seed, lean, sag);
      positions.set(
        new Float32Array(wasm.memory.buffer, output, positions.length),
      );
      return true;
    },
    deformCarton(positions, width, height, depth, crush) {
      if (
        !Number.isFinite(crush) ||
        ![width, height, depth].every(
          (size) => Number.isFinite(size) && size > 0,
        )
      )
        return false;
      uploadPositions(positions);
      const output = wasm.deform_carton(width, height, depth, crush);
      positions.set(
        new Float32Array(wasm.memory.buffer, output, positions.length),
      );
      return true;
    },
    smoothNormals(positions, indices) {
      const vertexCount = positions.length / 3;
      if ((indices?.length ?? vertexCount) % 3 !== 0) return;
      uploadPositions(positions);
      if (indices) {
        const address = wasm.soft_mesh_indices(indices.length);
        new Uint32Array(wasm.memory.buffer, address, indices.length).set(
          indices,
        );
      }
      const output = wasm.smooth_mesh_normals(Boolean(indices));
      if (!output) return;
      return new Float32Array(
        wasm.memory.buffer,
        output,
        positions.length,
      ).slice();
    },
    corrugatedProfile(start, end, period, depth, flat, slope, origin) {
      if (
        ![start, end, period, depth, flat, slope, origin].every(
          Number.isFinite,
        ) ||
        period <= 0 ||
        Math.abs((start - origin) / period) > 1000000 ||
        Math.abs((end - origin) / period) > 1000000 ||
        (end - start) / period > 20000
      )
        return;
      const count = wasm.corrugated_profile(
        start,
        end,
        period,
        depth,
        flat,
        slope,
        origin,
      );
      const points = new Float64Array(
        wasm.memory.buffer,
        wasm.corrugated_points(),
        count * 2,
      );
      return Array.from({ length: count }, (_, i) => [
        points[i * 2],
        points[i * 2 + 1],
      ]);
    },
    sweepSurface(profile, levels, project, closed, center, inward, caps) {
      const columns = profile.length;
      const rows = levels.length;
      if (columns < 2 || rows < 2) return;
      const edgeCount = closed ? columns : columns - 1;
      const capped = Boolean(closed && caps);
      const vertexCount =
        (rows - 1) * edgeCount * 6 + (capped ? columns * 6 : 0);
      const capLength = capped ? columns * 2 + 8 : 0;
      const size = rows + edgeCount + 1 + columns * rows * 3 + capLength;
      if (vertexCount > 1000000) return;
      const address = wasm.surface_input(size);
      const data = new Float64Array(wasm.memory.buffer, address, size);
      // Pack row coordinates, cumulative UV distances, projected vertices and optional cap data.
      data.set(levels);
      let offset = rows;
      let distance = 0;
      data[offset++] = distance;
      for (let i = 0; i < edgeCount; i++) {
        const [x, y] = profile[i];
        const [nextX, nextY] = profile[(i + 1) % columns];
        distance += Math.hypot(nextX - x, nextY - y);
        data[offset++] = distance;
      }
      // The game projects each grid point with a deterministic coordinate mapping.
      for (let row = 0; row < rows; row++) {
        for (const [x, y] of profile) {
          const point = project(x, y, levels[row], row);
          data.set(point, offset);
          offset += 3;
        }
      }
      if (capped) {
        let x = 0,
          y = 0;
        for (const [nextX, nextY] of profile) {
          data[offset++] = nextX;
          data[offset++] = nextY;
          x += nextX;
          y += nextY;
        }
        x /= columns;
        y /= columns;
        data[offset++] = x;
        data[offset++] = y;
        data.set(project(x, y, levels[0], 0), offset);
        data.set(project(x, y, levels[rows - 1], rows - 1), offset + 3);
      }
      const [x, y, z] = center || [0, 0, 0];
      const output = wasm.sweep_surface(
        columns,
        rows,
        Boolean(closed),
        capped,
        Boolean(inward),
        Boolean(center),
        x,
        y,
        z,
      );
      return copyMesh(output, vertexCount);
    },
    sweepTube(
      curve,
      rows,
      radius,
      segments,
      closed,
      frames,
      length,
      uvScale = NaN,
    ) {
      const vertexCount = (rows + 1) * (segments + 1);
      if (
        !Number.isInteger(rows) ||
        !Number.isInteger(segments) ||
        rows < 1 ||
        segments < 1 ||
        vertexCount > 65535 ||
        !Number.isFinite(radius) ||
        !Number.isFinite(length) ||
        typeof closed !== "boolean"
      )
        return;
      const address = wasm.tube_frames(rows + 1);
      const data = new Float64Array(
        wasm.memory.buffer,
        address,
        (rows + 1) * 9,
      );
      const target = frames.normals[0].clone();
      let offset = 0;
      for (let row = 0; row <= (closed ? rows - 1 : rows); row++) {
        curve.getPointAt(row / rows, target).toArray(data, offset);
        frames.normals[row].toArray(data, offset + 3);
        frames.binormals[row].toArray(data, offset + 6);
        offset += 9;
      }
      const output = wasm.sweep_tube(
        rows,
        segments,
        radius,
        length,
        closed,
        uvScale,
      );
      return copyMesh(
        output,
        vertexCount,
        wasm.tube_indices(),
        rows * segments * 6,
      );
    },
    beveledBox(width, height, depth, bevel) {
      if (![width, height, depth, bevel].every(Number.isFinite)) return;
      const address = wasm.beveled_box(width, height, depth, bevel);
      return copyMesh(address, 24, wasm.beveled_box_indices(), 132);
    },
    cylinder(top, bottom, height, radialSegments, open) {
      if (
        ![top, bottom, height, radialSegments].every(Number.isFinite) ||
        top < 0 ||
        bottom < 0 ||
        height <= 0 ||
        typeof open !== "boolean" ||
        !Number.isFinite((bottom - top) / height)
      )
        return;

      const segments = Math.floor(radialSegments);
      const topCap = !open && top > 0;
      const bottomCap = !open && bottom > 0;
      const capCount = Number(topCap) + Number(bottomCap);
      const vertexCount = 2 * (segments + 1) + capCount * (2 * segments + 1);
      if (segments < 1 || vertexCount > 65535) return;

      const sideCount = segments * (Number(top > 0) + Number(bottom > 0)) * 3;
      const capSize = segments * 3;
      const address = wasm.cylinder_mesh(top, bottom, height, segments, open);
      const mesh = copyMesh(
        address,
        vertexCount,
        wasm.cylinder_indices(),
        sideCount + capCount * capSize,
      );
      mesh.groups = [[0, sideCount, 0]];
      if (topCap) mesh.groups.push([sideCount, capSize, 1]);
      if (bottomCap)
        mesh.groups.push([sideCount + Number(topCap) * capSize, capSize, 2]);
      return mesh;
    },
    revolveProfile(profile, segments, halfTurn, tireRadius) {
      const vertexCount = profile.length * (segments + 1);
      if (
        !Number.isInteger(segments) ||
        segments < 1 ||
        profile.length < 2 ||
        vertexCount > 65535
      )
        return;

      const pointAddress = wasm.profile_points(profile.length);
      const points = new Float64Array(
        wasm.memory.buffer,
        pointAddress,
        profile.length * 3,
      );
      let previous = profile[0];
      let offset = 0;
      for (const point of profile) {
        const [radius, height] = point;
        const length = Math.hypot(radius - previous[0], height - previous[1]);
        if (
          !Number.isFinite(radius) ||
          !Number.isFinite(height) ||
          !Number.isFinite(length)
        )
          return;
        points[offset++] = radius;
        points[offset++] = height;
        points[offset++] = length;
        previous = point;
      }

      const address = wasm.revolve_profile(segments, halfTurn);
      if (
        tireRadius !== undefined &&
        !wasm.tire_profile_uvs(segments, tireRadius, halfTurn)
      )
        return;
      return copyMesh(
        address,
        vertexCount,
        wasm.profile_indices(),
        wasm.profile_index_count(),
      );
    },
  };
}

let objectModule;

async function compileObjectGeometry() {
  const response = await fetch(new URL("./objects.wasm", import.meta.url));
  if (!response.ok)
    throw new Error(`Object geometry WASM HTTP ${response.status}`);
  if (
    WebAssembly.compileStreaming &&
    response.headers.get("Content-Type") === "application/wasm"
  ) {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

export async function prepareObjectGeometry() {
  if (typeof WebAssembly === "undefined") return null;
  objectModule ??= compileObjectGeometry().catch(() => null);
  const module = await objectModule;
  return module ? createObjectGeometry(module) : null;
}
