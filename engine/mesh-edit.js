function isPackedVector(attribute) {
  return (
    attribute.itemSize === 3 &&
    !attribute.normalized &&
    !attribute.isInterleavedBufferAttribute &&
    attribute.array instanceof Float32Array
  );
}

function overlaps(a, b) {
  return (
    a.buffer === b.buffer &&
    a.byteOffset < b.byteOffset + b.byteLength &&
    b.byteOffset < a.byteOffset + a.byteLength
  );
}

export function createMeshEditor(wasm) {
  return {
    transformMesh(position, normal, transform, normalMatrix) {
      if (
        !position ||
        position.count < 24 ||
        !isPackedVector(position) ||
        (normal &&
          (!isPackedVector(normal) ||
            normal.count !== position.count ||
            overlaps(position.array, normal.array)))
      )
        return false;
      const length = position.array.length;
      const total = length + (normal?.array.length ?? 0);
      const address = wasm.edit_mesh_buffer(total, 0);
      const data = new Float32Array(wasm.memory.buffer, address, total);
      const matrices = new Float64Array(
        wasm.memory.buffer,
        wasm.edit_mesh_matrices(),
        25,
      );
      data.set(position.array);
      if (normal) {
        data.set(normal.array, length);
        matrices.set(normalMatrix, 16);
      }
      matrices.set(transform);
      wasm.transform_mesh(length);
      position.array.set(data.subarray(0, length));
      if (normal) normal.array.set(data.subarray(length));
      return true;
    },
    vertexNormals(position, index, normal) {
      if (
        !position ||
        position.count < 48 ||
        !isPackedVector(position) ||
        (normal &&
          (!isPackedVector(normal) ||
            normal.count !== position.count ||
            overlaps(position.array, normal.array))) ||
        (index &&
          (index.normalized ||
            index.itemSize !== 1 ||
            !(
              index.array instanceof Uint16Array ||
              index.array instanceof Uint32Array
            ))) ||
        (index?.count ?? position.count) % 3 !== 0
      )
        return;
      const length = position.array.length;
      const address = wasm.edit_mesh_buffer(length * 2, index?.count ?? 0);
      const buffer = wasm.memory.buffer;
      const data = new Float32Array(buffer, address, length * 2);
      data.set(position.array);
      if (index)
        new Uint32Array(buffer, wasm.edit_mesh_indices(), index.count).set(
          index.array,
        );
      if (!wasm.rebuild_mesh_normals(position.count, Boolean(index))) return;
      if (normal) {
        normal.array.set(data.subarray(length));
        return normal.array;
      }
      return data.slice(length);
    },
  };
}
