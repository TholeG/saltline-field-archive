const baseAttributes = [
  ["position", 3],
  ["normal", 3],
  ["uv", 2],
];
const colorAttributes = [...baseAttributes, ["color", 3]];
const noIndex = 0xffffffff;
const floatGpuType = 1015;

export function createMeshBatcher(wasm) {
  return function assembleMeshes(parts, wear) {
    if (parts.length === 1 && !wear) return;
    const colored = Boolean(parts[0].attributes.color);
    const attributes = colored ? colorAttributes : baseAttributes;
    const vertexWidth = colored ? 11 : 8;
    let vertices = 0,
      indices = 0,
      indexedParts = 0,
      plainVertices = 0;
    for (const part of parts) {
      const count = part.attributes.position.count;
      if (part.morphTargetsRelative || Object.keys(part.morphAttributes).length)
        return;
      for (const [name, width] of attributes) {
        const attribute = part.attributes[name];
        if (
          !attribute ||
          attribute.itemSize !== width ||
          attribute.count !== count ||
          attribute.normalized ||
          attribute.isInterleavedBufferAttribute ||
          attribute.array.constructor !== Float32Array ||
          attribute.gpuType !== floatGpuType
        )
          return;
      }
      if (part.index) {
        const { array, normalized, itemSize } = part.index;
        if (
          normalized ||
          itemSize !== 1 ||
          !(array instanceof Uint16Array || array instanceof Uint32Array)
        )
          return;
        indices += array.length;
        indexedParts++;
      } else {
        plainVertices += count;
      }
      vertices += count;
    }
    const indexed = indexedParts === parts.length;
    const mixed = !indexed && indexedParts > 0;
    if (!indexed && !mixed && !(colored && wear?.amount > 0)) return;
    const outputVertices = mixed ? plainVertices + indices : vertices;
    if (
      !outputVertices ||
      Math.max(vertices, outputVertices) * vertexWidth > 16000000 ||
      indices > 16000000
    )
      return;

    const metadataAddress = wasm.batch_prepare(
      parts.length,
      vertices * vertexWidth,
      indices,
    );
    const attributeAddress = wasm.batch_attributes();
    const indexAddress = wasm.batch_indices();
    const buffer = wasm.memory.buffer;
    const metadata = new Uint32Array(buffer, metadataAddress, parts.length * 2);
    const values = new Float32Array(
      buffer,
      attributeAddress,
      vertices * vertexWidth,
    );
    const elements = new Uint32Array(buffer, indexAddress, indices);

    let firstVertex = 0,
      firstIndex = 0;
    for (const [partIndex, part] of parts.entries()) {
      const count = part.attributes.position.count;
      const indexArray = part.index?.array;
      metadata[partIndex * 2] = count;
      metadata[partIndex * 2 + 1] = indexArray?.length ?? noIndex;
      let componentOffset = 0;
      for (const [name, width] of attributes) {
        values.set(
          part.attributes[name].array,
          vertices * componentOffset + firstVertex * width,
        );
        componentOffset += width;
      }
      firstVertex += count;
      if (indexArray) {
        elements.set(indexArray, firstIndex);
        firstIndex += indexArray.length;
      }
    }

    const output = wasm.assemble_batch(
      vertices,
      outputVertices,
      colored,
      indexed,
      mixed,
      wear?.seed ?? 0,
      wear?.amount ?? 0,
    );
    if (!output) return;
    const resultBuffer = wasm.memory.buffer;
    const packed = new Float32Array(
      resultBuffer,
      output,
      outputVertices * vertexWidth,
    ).slice();
    let indexArray = null;
    if (indexed) {
      const result = new Uint32Array(resultBuffer, indexAddress, indices);
      indexArray = wasm.batch_large_indices()
        ? result.slice()
        : new Uint16Array(result);
    }
    return {
      positions: packed.subarray(0, outputVertices * 3),
      normals: packed.subarray(outputVertices * 3, outputVertices * 6),
      uvs: packed.subarray(outputVertices * 6, outputVertices * 8),
      colors: colored ? packed.subarray(outputVertices * 8) : null,
      indices: indexArray,
    };
  };
}
