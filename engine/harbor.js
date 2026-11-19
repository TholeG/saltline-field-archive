import { createHarborLayout } from "./harbor-layout.js";

const containerKinds = {
  20: 0,
  40: 1,
  hc: 2,
};
const containerNames = ["container20", "container40", "container40hc"];
const paints = [
  "red",
  "blue",
  "green",
  "yellow",
  "orange",
  "white",
  "grey",
  "teal",
];
const backdropNames = ["boat", "crane", "ship", "sky", "gulls"];
const relayModes = ["live", "cutting", "cut"];
const none = 0xffffffff;

function propKind(name) {
  if (name === "barrel") return 1;
  if (name === "gas_cylinders") return 2;
  if (name === "traffic_cone" || name === "jerry_can") return 3;
  return 0;
}

export function createHarbor(wasm) {
  const floats = (address, length) =>
    new Float64Array(wasm.memory.buffer, address, length);
  const table = (length) => {
    const address = wasm.harbor_scene_table(length);
    return new Uint32Array(wasm.memory.buffer, address, length);
  };

  function graph(root) {
    const nodes = [];
    const index = new Map();
    root.traverse((node) => {
      index.set(node, nodes.length);
      nodes.push(node);
    });
    const relationships = table(nodes.length * 3);
    for (const [i, node] of nodes.entries()) {
      relationships[3 * i] = index.get(node.parent) ?? none;
      relationships[3 * i + 1] = Number(!!node.userData?.noMerge);
    }
    wasm.harbor_dynamic_roots();
    return { nodes, relationships };
  }

  function applyMaterials(root, materials) {
    const meshes = [];
    const unique = [];
    const indices = new Map();
    root.traverse((node) => {
      if (!node.isMesh || Array.isArray(node.material)) return;
      meshes.push(node);
      if (!indices.has(node.material)) {
        indices.set(node.material, unique.length);
        unique.push(node.material);
      }
    });

    const knownNames = new Set(materials.names);
    const descriptors = new Uint32Array(unique.length * 4);
    for (const [index, material] of unique.entries()) {
      const flags =
        Number(!!material.isMeshStandardMaterial) |
        (Number(!!material.vertexColors) << 1) |
        (Number(!!material.transparent) << 2) |
        (Number(!!material.isMeshBasicMaterial) << 3) |
        (Number(!!material.name && knownNames.has(material.name)) << 4);
      const offset = 4 * index;
      descriptors[offset] = flags;
      descriptors[offset + 1] = none;
      if (!wasm.harbor_material_candidate(flags)) continue;
      try {
        const options = material.userData?.interior ? { interior: true } : {};
        if (materials.get(material.name, options) !== material) continue;
        const alternative = materials.get(material.name, {
          ...options,
          vertexColors: true,
        });
        descriptors[offset + 1] = indices.get(alternative) ?? none;
      } catch {}
    }

    const assignments = table(descriptors.length);
    assignments.set(descriptors);
    wasm.harbor_assign_materials(unique.length);
    for (const mesh of meshes) {
      const offset = indices.get(mesh.material) * 4;
      mesh.material = unique[assignments[offset + 2]];
      const shadows = assignments[offset + 3];
      mesh.castShadow = !!(shadows & 1);
      mesh.receiveShadow = !!(shadows & 2);
    }
  }

  return {
    ...createHarborLayout(wasm),
    slab(x, y, z, endX, endY, endZ) {
      return floats(wasm.harbor_slab(x, y, z, endX, endY, endZ), 6);
    },
    rod(from, to) {
      return floats(wasm.harbor_rod(...from, ...to), 8);
    },
    backdropPlan(only, showcase) {
      const selection = only ? backdropNames.indexOf(only) + 1 || none : 0;
      const flags = wasm.backdrop_plan(selection, showcase);
      const boatBase = floats(wasm.backdrop_boat_base(), 4).slice();
      const visible = (name) => !!(flags & (1 << backdropNames.indexOf(name)));
      return { visible, boatBase };
    },
    backdropFrame(time, delta, height, showcase) {
      return floats(wasm.backdrop_frame(time, delta, height, showcase), 7);
    },
    fanAngle: wasm.harbor_fan_angle,
    doorAngle: wasm.container_door_angle,
    containerVariant(model, options) {
      const [seed, doors, wear, color] = floats(
        wasm.container_variant(
          options.seed === undefined ? NaN : options.seed,
          options.doors || 0,
          options.wear === undefined ? 0.5 : options.wear,
          paints.indexOf(options.color),
          containerNames.indexOf(model),
        ),
        4,
      );
      return { seed, doors, wear, color: paints[color] };
    },
    relay(seed) {
      const address = wasm.relay_frame_buffer();
      const state = new Float64Array(7);
      wasm.relay_frame(seed, -2, 0);
      state.set(floats(address, state.length));

      return {
        setState(mode) {
          const command = relayModes.indexOf(mode);
          if (command < 0) return false;
          const buffer = floats(address, state.length);
          buffer.set(state);
          wasm.relay_frame(seed, command, 0);
          state.set(buffer);
          return true;
        },
        update(delta, apply) {
          const frame = floats(address, 10);
          frame.set(state);
          const level = wasm.relay_frame(seed, -1, delta || 0);
          state.set(frame.subarray(0, 7));
          apply(level, frame[7], frame[8], frame[9]);
        },
      };
    },
    bakeColliders(object, yaw, physics, options) {
      const local = object.userData.colliders;
      if (!local?.length) return;
      const address = wasm.harbor_placement_input(local.length * 6);
      const input = floats(address, local.length * 6);
      for (const [index, collider] of local.entries()) {
        input.set(collider.min, 6 * index);
        input.set(collider.max, 6 * index + 3);
      }
      floats(wasm.harbor_world_matrix(), 16).set(object.matrixWorld.elements);
      const length = wasm.harbor_bake_colliders(yaw) * 7;
      const bounds = floats(wasm.harbor_colliders(), length);
      for (let i = 0; i < length; i += 7) {
        physics.addBox(
          bounds.subarray(i, i + 3),
          bounds.subarray(i + 3, i + 6),
          { surface: local[bounds[i + 6]].surface || "metal", ...options },
        );
      }
    },
    propPlan(name, options, placement) {
      const flags =
        Number(!!placement.inert) |
        (Number(!!placement.noCol) << 1) |
        (Number(!!placement.dynamic) << 2) |
        (Number(options.color === "red") << 3) |
        (Number(!!options.lying) << 4) |
        (Number(!!options.open) << 5);
      return wasm.harbor_placement_kind(propKind(name), flags);
    },
    footprint(x, z, yaw, narrowHalfWidth) {
      return Array.from(
        floats(wasm.harbor_footprint(x, z, yaw, narrowHalfWidth), 4),
      );
    },
    containerStack(x, z, layers, yaw, jitter, random, place, metalRects) {
      if (layers.some(([kind]) => !(kind in containerKinds))) return;
      const length = layers.length * 12;
      const input = floats(wasm.harbor_placement_input(length), length);
      for (const [index, [kind, , options = {}]] of layers.entries()) {
        input.set(
          [
            containerKinds[kind],
            options.dx || 0,
            options.dz || 0,
            Number(!!options.flip),
            options.doors || 0,
            Number(!!options.reefer),
            index ? random() : 0,
            index ? random() : 0,
            random(),
            random(),
            options.wear ?? NaN,
            options.wear == null ? random() : 0,
          ],
          index * 12,
        );
      }
      // Prop construction can grow the module's memory while applying the plan.
      const plan = floats(
        wasm.harbor_stack(x, z, yaw, jitter),
        layers.length * 14 + 1,
      ).slice();
      const objs = [];
      for (const [index, [kind, color, options = {}]] of layers.entries()) {
        const i = index * 14;
        const [px, y, pz, angle, seed, wear, doors] = plan.subarray(i, i + 7);
        objs.push(
          place(
            containerNames[containerKinds[kind]],
            px,
            pz,
            angle,
            { color, seed, wear, doors },
            y,
          ),
        );
        metalRects.push(Array.from(plan.subarray(i + 7, i + 11)));
        if (options.reefer) {
          const [rx, ry, rz] = plan.subarray(i + 11, i + 14);
          place("reefer_unit", rx, rz, angle, { seed: index + 3 }, ry);
        }
      }
      return { top: plan[plan.length - 1], objs };
    },
    prepareLevel(staticRoot, dynamicRoot, materials) {
      staticRoot.updateMatrixWorld(true);
      const { nodes, relationships } = graph(staticRoot);
      for (const [index, node] of nodes.entries()) {
        if (relationships[index * 3 + 2] === 1 && node !== staticRoot)
          dynamicRoot.attach(node);
      }
      applyMaterials(staticRoot, materials);
    },
    mergeScene(root, cast, receive, { Group, Mesh, merge }) {
      root.updateMatrixWorld(true);
      // The original renderer keeps meshes under noMerge ancestors intact.
      let keepTree = false;
      for (let parent = root.parent; parent; parent = parent.parent) {
        if (parent.userData?.noMerge) keepTree = true;
      }
      const { nodes, relationships } = graph(root);
      const meshes = [];
      const materials = new Map();
      for (const [index, node] of nodes.entries()) {
        if (!node.isMesh) continue;
        const keep =
          keepTree ||
          relationships[index * 3 + 2] !== 0 ||
          node.isSkinnedMesh ||
          node.isInstancedMesh ||
          Array.isArray(node.material);
        let material = none;
        if (!keep) {
          const { uuid } = node.material;
          if (!materials.has(uuid)) materials.set(uuid, materials.size);
          material = materials.get(uuid);
        }
        meshes.push({ node, material, keep });
      }

      const descriptions = table(meshes.length * 5);
      for (const [i, { node, material, keep }] of meshes.entries()) {
        const shadows =
          Number(node.castShadow) | (Number(node.receiveShadow) << 1);
        descriptions.set(
          [material, shadows, node.layers.mask, Number(!!keep), 0],
          i * 5,
        );
      }
      const groups = Array.from(
        { length: wasm.harbor_merge_groups() },
        () => [],
      );
      const plan = descriptions.slice();
      const unchanged = [];
      for (const [i, { node }] of meshes.entries()) {
        const group = plan[i * 5 + 4];
        if (group === none) unchanged.push(node);
        else groups[group].push(node);
      }

      const merged = new Group();
      for (const nodes of groups) {
        const first = nodes[0];
        const geos = nodes.map((node) =>
          node.geometry.clone().applyMatrix4(node.matrixWorld),
        );
        const mesh = new Mesh(merge(geos), first.material);
        mesh.castShadow = first.castShadow && cast;
        mesh.receiveShadow = first.receiveShadow && receive;
        mesh.layers.mask = first.layers.mask;
        merged.add(mesh);
      }
      for (const node of unchanged) {
        node.updateMatrixWorld(true);
        const transform = node.matrixWorld.clone();
        merged.add(node);
        node.matrix.copy(transform);
        node.matrix.decompose(node.position, node.quaternion, node.scale);
      }
      return merged;
    },
  };
}
