const areas = ["yard", "gate", "workshop-props", "quay"];
const railMaterials = [
  "steel_dark",
  "steel_rust",
  "steel_worn",
  "steel_dark",
  "paint_yellow",
  "rubber",
  "steel_dark",
];

export function createHarborLayout(wasm) {
  const text = new TextDecoder();
  const layouts = new Map();
  const read = (address, count) =>
    new Float64Array(wasm.memory.buffer, address, count).slice();

  function fence(builder, x, z, endX, endZ, seed) {
    const count = wasm.harbor_fence_run(x, z, endX, endZ, seed);
    const plan = read(wasm.harbor_layout_commands(), count * 5 + 12);
    for (let i = 0; i < count; i++) {
      const [x, z, yaw, scale, seed] = plan.subarray(i * 5, i * 5 + 5);
      const posts = i === count - 1 ? "both" : "left";
      const panel = builder.place(
        "fence_panel",
        x,
        z,
        yaw,
        { seed, posts },
        0,
        { noCol: true },
      );
      panel.scale.x = scale;
    }
    const bounds = plan.subarray(count * 5);
    builder.col(...bounds.subarray(0, 6), "metal");
    builder.col(...bounds.subarray(6), "metal", { playerOnly: true });
  }

  function rails(builder) {
    const length = wasm.harbor_rail_plan();
    const plan = read(wasm.harbor_layout_commands(), length);
    for (let i = 0; i < length; i += 8) {
      const kind = plan[i];
      const bounds = plan.subarray(i + 1, i + 7);
      if (kind === 7) {
        builder.col(...bounds, "metal");
        continue;
      }
      const material = railMaterials[kind];
      const options = { bevel: plan[i + 7], cast: kind > 3 };
      if (kind === 3) builder.box(material, ...bounds, options);
      else builder.slab(material, ...bounds, options);
    }
  }

  return {
    buildLayout(area, builder, makeDetails) {
      let commands = layouts.get(area);
      if (!commands) {
        const section = areas.indexOf(area);
        const address = wasm.harbor_layout(section);
        const length = wasm.harbor_layout_length(section);
        const bytes = new Uint8Array(wasm.memory.buffer, address, length);
        commands = JSON.parse(text.decode(bytes));
        layouts.set(area, commands);
      }
      const details = makeDetails(builder);
      for (const [op, ...args] of commands) {
        if (op === "fence") fence(builder, ...args);
        else if (op === "rails") rails(builder);
        else (details[op] ?? builder[op])(...args);
      }
    },
    quayPoints() {
      return read(wasm.harbor_quay_points(), 6);
    },
  };
}
