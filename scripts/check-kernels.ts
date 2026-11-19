import { Buffer } from "node:buffer";
import { runInNewContext } from "node:vm";
import { createObjectGeometry } from "../engine/objects.js";
import { patchGameKernels } from "../engine/patch-game";
import { createGameKernels } from "../engine/runtime.js";

const game = await Bun.file(new URL("../game.js", import.meta.url)).text();
const module = await WebAssembly.compile(
  await Bun.file(new URL("../ground/mask.wasm", import.meta.url)).arrayBuffer(),
);
const audioModule = await WebAssembly.compile(
  await Bun.file(
    new URL("../engine/audio.wasm", import.meta.url),
  ).arrayBuffer(),
);
const runtime = createGameKernels(module, () => audioModule);
await runtime.audio.prepare();
const geometry = createObjectGeometry(
  await WebAssembly.compile(
    await Bun.file(
      new URL("../engine/objects.wasm", import.meta.url),
    ).arrayBuffer(),
  ),
);
const builtHarborAreas: string[] = [];
const invoked = {
  water: 0,
  profiles: 0,
  boxes: 0,
  cylinders: 0,
  boxGrids: 0,
  sandbags: 0,
  cartons: 0,
  normalPasses: 0,
  tireProfiles: 0,
  corrugations: 0,
  surfaces: 0,
  tubes: 0,
  extrusions: 0,
  ribbons: 0,
  drapes: 0,
  hullSamples: 0,
  hullPanels: 0,
  dinghyHulls: 0,
  collars: 0,
  snappedBoxes: 0,
  meshBatches: 0,
  wearBatches: 0,
  transforms: 0,
  rebuiltNormals: 0,
  containerSides: 0,
  paths: 0,
  lines: 0,
  walkable: 0,
  samples: 0,
  sight: 0,
  clearance: 0,
  rays: 0,
  heights: 0,
  moves: 0,
  playerSteps: 0,
  synthesized: 0,
  filters: 0,
  sounds: 0,
};

function load(
  source: string,
  kernels?: typeof runtime,
  objects?: typeof geometry,
) {
  let navSeed = 0;
  let navDraws = 0;
  const math = Object.create(Math);
  math.random = () => {
    navDraws++;
    navSeed = (Math.imul(1664525, navSeed) + 1013904223) >>> 0;
    return navSeed / 4294967296;
  };
  const sandbox = {
    Math: math,
    resetNavRandom(seed: number) {
      navDraws = 0;
      navSeed = seed;
    },
    navRandomDraws: () => navDraws,
    saltlineKernels: kernels,
    saltlineGeometry: objects,
    window: { addEventListener() {} },
    document: { getElementById: () => null },
    console: { log() {}, warn() {}, error() {} },
    setTimeout,
    clearTimeout,
    performance,
    AbortController,
    URL,
    Float32Array,
    Uint16Array,
    Uint32Array,
  };
  return runInNewContext(
    source.replace(
      "cL().catch(n=>yT(n&&n.stack||n));",
      `
    function kernelTestMaterials(sceneVariants=false){
      const materials=new Map();
      const names=new Set();
      const get=(id,options={})=>{
        const key=sceneVariants
          ? id+"|"+!!options.interior+"|"+!!options.vertexColors
          : id;
        if(!materials.has(key)){
          const material=new ut.MeshStandardMaterial();
          if(sceneVariants){
            names.add(id);
            material.name=id;
            material.userData.interior=!!options.interior;
            material.vertexColors=!!options.vertexColors;
            material.transparent=id==="glass";
          }
          materials.set(key,material);
        }
        return materials.get(key);
      };
      const decal=id=>({material:get(id),aspect:2});
      const emissive=(color,intensity)=>get("emissive:"+color+":"+intensity);
      return {get,decal,label:decal,emissive,get names(){return [...names];}};
    }
    globalThis.kernelTests={
      createPhysics:()=>v_({level:{groundSurfaceAt:()=>"asphalt"}}),
      resetNavRandom,navRandomDraws,synthesize:K6,synthesizeSteps:J6,audioBank:Ir,
      water:Hw,waves:Og,seaLevel:Kn.seaY,profile:hs,box:tt,cylinder:vt,corrugate:sh,surface:Fa,tube:bn,
      plainBox:ot,roundedBox:uo,edgeRoundedBox:Ei,tire:yv,
      extrude:ko,ribbon:kr,
      shipPanel:fu,
      propNames:()=>Object.keys(Yi),
      transformProbe(mode){
        const mesh=uo(1,2,3,.025,3,4,5);
        if(mode==="positions")mesh.deleteAttribute("normal");
        mesh.computeBoundingBox();
        mesh.computeBoundingSphere();
        const matrix=new Je().makeRotationY(.21);
        matrix.scale(new K(.8,-1.3,mode==="singular"?0:.65));
        matrix.setPosition(.4,-2.2,0.8);
        if(mode==="projective")matrix.elements[3]=.075;
        mesh.applyMatrix4(matrix);
        return mesh;
      },
      batch(parts){
        return Jr(parts.map(({positions,indices,colors})=>{
          const mesh=s1(positions,null,new Float32Array(positions.length/3*2),indices);
          if(colors)mesh.setAttribute("color",new nt(new Float32Array(colors),3));
          return mesh;
        }));
      },
      smooth(positions,indices){return Cr(s1(positions,null,Array(positions.length/3*2).fill(0),indices));},
      buildProp(name, options){
        return uv[name]({mats:kernelTestMaterials()},options);
      },
      buildBackdrop(name,showcase){
        return kw({mats:kernelTestMaterials(),scene:new ut.Scene()},{only:name,showcase});
      },
      async buildLevel(seed){
        const game={mats:kernelTestMaterials(true),scene:new ut.Scene(),rng:su(seed)};
        game.props=fv(game);
        game.physics=v_(game);
        game.level=await Pg(game);
        return game;
      },
      createPlayerProbe(){
        const input={look:{dx:0,dy:0},move:{x:0,y:0},pressed:key=>key==="jump"&&input.jump};
        const game={input,events:BM(),camera:new ut.PerspectiveCamera()};
        game.physics=v_(game);
        game.physics.addBox([-36,1.28,-.9],[-33,2.2,.9],{surface:"metal"});
        const player=xE(game);
        const events=[];
        for(const name of ["jump","land","step","hit","dead","rescued"]){
          game.events.on("player:"+name,()=>events.push(name));
        }
        player.reset({pos:new K(-33.25,0,0),yaw:0});
        return {player,input,events,gameplay:game.physics.gameplay};
      }
    };
  `,
    ) + "\nkernelTests;",
    sandbox,
  );
}

const reference = load(game);
const native = load(
  patchGameKernels(game),
  {
    water: {
      ...runtime.water,
      height(...args: Parameters<typeof runtime.water.height>) {
        invoked.water++;
        return runtime.water.height(...args);
      },
    },
    createPhysics(...args) {
      const world = runtime.createPhysics(...args);
      return {
        ...world,
        nav: {
          ...world.nav,
          findPath(...args) {
            invoked.paths++;
            return world.nav.findPath(...args);
          },
          clearLine(...args) {
            invoked.lines++;
            return world.nav.clearLine(...args);
          },
          isWalkable(...args) {
            invoked.walkable++;
            return world.nav.isWalkable(...args);
          },
          randomPoint(...args) {
            invoked.samples++;
            return world.nav.randomPoint(...args);
          },
        },
        groundHeight(...args) {
          invoked.heights++;
          return world.groundHeight(...args);
        },
        raycast(...args) {
          invoked.rays++;
          return world.raycast(...args);
        },
        moveCapsule(...args) {
          invoked.moves++;
          return world.moveCapsule(...args);
        },
        movePlayer(...args) {
          invoked.playerSteps++;
          return world.movePlayer(...args);
        },
        lineOfSight(...args) {
          invoked.sight++;
          return world.lineOfSight(...args);
        },
        hasClearance(...args) {
          invoked.clearance++;
          return world.hasClearance(...args);
        },
      };
    },
    audio: {
      ...runtime.audio,
      synthesize(...args: Parameters<typeof runtime.audio.synthesize>) {
        const result = runtime.audio.synthesize(...args);
        if (result) invoked.synthesized++;
        return result;
      },
      filter(...args: Parameters<typeof runtime.audio.filter>) {
        const applied = runtime.audio.filter(...args);
        if (applied) invoked.filters++;
        return applied;
      },
      finalize(...args: Parameters<typeof runtime.audio.finalize>) {
        const applied = runtime.audio.finalize(...args);
        if (applied) invoked.sounds++;
        return applied;
      },
    },
  },
  {
    harbor: {
      ...geometry.harbor,
      buildLayout(...args) {
        builtHarborAreas.push(args[0]);
        return geometry.harbor.buildLayout(...args);
      },
    },
    assembleMeshes(...args) {
      const mesh = geometry.assembleMeshes(...args);
      if (mesh) {
        invoked.meshBatches++;
        if (args[1]?.amount > 0 && mesh.colors) invoked.wearBatches++;
      }
      return mesh;
    },
    transformMesh(...args) {
      const applied = geometry.transformMesh(...args);
      if (applied) invoked.transforms++;
      return applied;
    },
    vertexNormals(...args) {
      const normals = geometry.vertexNormals(...args);
      if (normals) invoked.rebuiltNormals++;
      return normals;
    },
    paintContainerSide(...args) {
      const colors = geometry.paintContainerSide(...args);
      if (colors) invoked.containerSides++;
      return colors;
    },
    revolveProfile(...args) {
      const mesh = geometry.revolveProfile(...args);
      if (mesh) {
        if (args[3] === undefined) invoked.profiles++;
        else invoked.tireProfiles++;
      }
      return mesh;
    },
    beveledBox(...args) {
      const mesh = geometry.beveledBox(...args);
      if (mesh) invoked.boxes++;
      return mesh;
    },
    cylinder(...args) {
      const mesh = geometry.cylinder(...args);
      if (mesh) invoked.cylinders++;
      return mesh;
    },
    boxGrid(...args) {
      const mesh = geometry.boxGrid(...args);
      if (mesh) {
        if (args[5]) invoked.snappedBoxes++;
        else invoked.boxGrids++;
      }
      return mesh;
    },
    deformSandbag(...args) {
      const applied = geometry.deformSandbag(...args);
      if (applied) invoked.sandbags++;
      return applied;
    },
    deformCarton(...args) {
      const applied = geometry.deformCarton(...args);
      if (applied) invoked.cartons++;
      return applied;
    },
    smoothNormals(...args) {
      const normals = geometry.smoothNormals(...args);
      if (normals) invoked.normalPasses++;
      return normals;
    },
    corrugatedProfile(...args) {
      const profile = geometry.corrugatedProfile(...args);
      if (profile) invoked.corrugations++;
      return profile;
    },
    sweepSurface(...args) {
      const mesh = geometry.sweepSurface(...args);
      if (mesh) invoked.surfaces++;
      return mesh;
    },
    sweepTube(...args) {
      const mesh = geometry.sweepTube(...args);
      if (mesh) {
        if (args[7] === undefined) invoked.tubes++;
        else invoked.collars++;
      }
      return mesh;
    },
    extrudeOutline(...args) {
      const mesh = geometry.extrudeOutline(...args);
      if (mesh) invoked.extrusions++;
      return mesh;
    },
    sweepRibbon(...args) {
      const mesh = geometry.sweepRibbon(...args);
      if (mesh) invoked.ribbons++;
      return mesh;
    },
    drapeMesh(...args) {
      const mesh = geometry.drapeMesh(...args);
      if (mesh) invoked.drapes++;
      return mesh;
    },
    sampleHull(...args) {
      invoked.hullSamples++;
      return geometry.sampleHull(...args);
    },
    hullPanel(...args) {
      const mesh = geometry.hullPanel(...args);
      if (mesh) invoked.hullPanels++;
      return mesh;
    },
    dinghyHull(...args) {
      const mesh = geometry.dinghyHull(...args);
      if (mesh) invoked.dinghyHulls++;
      return mesh;
    },
  },
);

function same(expected: unknown, actual: unknown, label: string) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${label}: ${JSON.stringify(actual)} != ${JSON.stringify(expected)}`,
    );
  }
}

let seed = 73425;
function random(min: number, max: number) {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return min + ((max - min) * seed) / 4294967296;
}

const waveVertices = new Float32Array(2000 * 3);
for (let i = 0; i < waveVertices.length; i += 3) {
  waveVertices[i] = random(-270, 330);
  waveVertices[i + 2] = random(-150, 150);
}
let maxHeightError = 0;
let maxVertexError = 0;
for (const time of [0, 0.5, 10, 33, 630, 3600]) {
  runtime.water.updateSurface(
    reference.waves,
    reference.seaLevel,
    waveVertices,
    time,
  );
  for (let i = 0; i < waveVertices.length; i += 3) {
    const x = waveVertices[i],
      z = waveVertices[i + 2];
    const expected = reference.water(x, z, time);
    maxHeightError = Math.max(
      maxHeightError,
      Math.abs(native.water(x, z, time) - expected),
    );
    maxVertexError = Math.max(
      maxVertexError,
      Math.abs(
        waveVertices[i + 1] - Math.fround(expected - reference.seaLevel),
      ),
    );
  }
}
if (!(maxHeightError <= 1e-12 && maxVertexError <= 1e-8)) {
  throw new Error(
    `Wasserhöhe weicht ab: ${maxHeightError} / ${maxVertexError} m`,
  );
}

function compareGeometry(expected: any, actual: any, label: string) {
  same(expected.type, actual.type, `${label}/Geometrietyp`);
  same(expected.parameters, actual.parameters, `${label}/Parameter`);
  same(expected.groups, actual.groups, `${label}/Materialgruppen`);
  same(expected.index?.count, actual.index?.count, `${label}/Indexanzahl`);
  for (let i = 0; i < (expected.index?.count ?? 0); i++) {
    same(expected.index.array[i], actual.index.array[i], `${label}/Index/${i}`);
  }
  same(
    Object.keys(expected.attributes).sort(),
    Object.keys(actual.attributes).sort(),
    `${label}/Attribute`,
  );
  for (const [key, reference] of Object.entries(expected.attributes) as [
    string,
    any,
  ][]) {
    const attribute = actual.attributes[key];
    same(
      [reference.itemSize, reference.count],
      [attribute.itemSize, attribute.count],
      `${label}/${key}/Format`,
    );
    for (let i = 0; i < reference.array.length; i++) {
      const a = reference.array[i];
      const b = attribute.array[i];
      if (a !== b && !(Number.isNaN(a) && Number.isNaN(b))) {
        throw new Error(
          `${label}/${key}/${i}: ${attribute.array[i]} != ${reference.array[i]}`,
        );
      }
    }
  }
}

const profile = [
  [0, 0],
  [0.1, 0],
  [0.1, 0],
  [0.1, 0.5],
  [0, 0.5],
];
compareGeometry(
  reference.profile(profile, 24),
  native.profile(profile, 24),
  "Profil mit doppeltem Punkt",
);
compareGeometry(
  reference.profile(profile, 1.5),
  native.profile(profile, 1.5),
  "JavaScript-Fallback",
);
for (const args of [
  [1, 2, 3],
  [0.4, 0.002, 0.6, 0.05, "bottom"],
  [1, 2, 3, 0],
  [1, 2, 3, -0.05],
  [0, 2, 3, 0.04],
]) {
  compareGeometry(
    reference.box(...args),
    native.box(...args),
    `Quader ${args}`,
  );
}
for (const args of [
  [0.1, 0.1, 0.4],
  [0.2, 0.4, 0.05, 9, false, "bottom"],
  [0, 0.2, 0.8, 11, false],
  [0.3, 0, 0.9, 5, false],
  [0.2, 0.2, 1.3, 5.5, true],
  [0, 0, 0.1, 3, true],
  [0.1, 0.1, 0, 3, false],
]) {
  compareGeometry(
    reference.cylinder(...args),
    native.cylinder(...args),
    `Zylinder ${args}`,
  );
}
for (const args of [
  [-6.2, 5.8, 0.275, 0.04, 0.064, 0.04],
  [-1.1, 1.1, 0.198, 0.02, 0.047, 0.031, 0.5],
  [1, 0.8, 0.18, 0.035, 0.04, 0.02],
]) {
  same(
    reference.corrugate(...args),
    native.corrugate(...args),
    `Wellprofil ${args}`,
  );
}

const corners = [
  [-1, -1],
  [1, -1],
  [1, 1],
  [-1, 1],
];
const levels = [-1, -0.3, 1];
const project = (a: number, b: number, row: number) => [a, b, row];
for (const options of [
  {},
  { closed: true, caps: true },
  { closed: true, center: [0, 0, 0], inward: true },
]) {
  compareGeometry(
    reference.surface(corners, levels, project, options),
    native.surface(corners, levels, project, options),
    `Profilfläche ${JSON.stringify(options)}`,
  );
}

const curve = [
  [0, 0, 0],
  [1, 0.2, 1],
  [2, -0.1, 0.2],
  [2.1, 1, 0.42],
];
for (const closed of [false, true]) {
  for (const tension of [0, 0.45]) {
    compareGeometry(
      reference.tube(curve, 0.017, 7, closed, tension),
      native.tube(curve, 0.017, 7, closed, tension),
      `Spline ${closed}/${tension}`,
    );
  }
}

const surfaces = new Set([
  "container20",
  "container40",
  "container40hc",
  "jersey_barrier",
]);
const tubes = new Set([
  "light_mast",
  "floodlight_stand",
  "jerry_can",
  "tarp_cargo",
  "toolbox_cart",
]);
for (const args of [
  [1, 2, 3],
  [0.7, 0.14, 0.33, "bottom"],
  [0, 0.5, 0.5],
]) {
  compareGeometry(
    reference.plainBox(...args),
    native.plainBox(...args),
    `Einfacher Quader ${args}`,
  );
}
for (const args of [
  [1, 2, 3],
  [0.7, 0.14, 0.33, 0.06, 4, 5, 4, "bottom"],
  [0.3, 0.05, 0.2, 0, 3, 2, 1],
  [0.6, 1, 1.1, 0.02, 2.5, 4, 3],
]) {
  compareGeometry(
    reference.roundedBox(...args),
    native.roundedBox(...args),
    `Gerundeter Quader ${args}`,
  );
}
for (const args of [
  [0.39, 0.25, 0.215, 20],
  [0.39, 0.25, 0.215, 16],
  [0.28, 0.16, 0.15, 12],
]) {
  compareGeometry(
    reference.tire(...args),
    native.tire(...args),
    `Reifen ${args}`,
  );
}

const smoothPositions = [
  0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0.0001, 0, 1, 0, 0, 0, 1,
];
for (const indices of [null, [0, 1, 2], [0, 1, 2, 3, 4, 5]]) {
  compareGeometry(
    reference.smooth(smoothPositions, indices),
    native.smooth(smoothPositions, indices),
    `Normalenglättung ${indices}`,
  );
}

const boxGrids = new Set([
  "pallet_stack",
  "cardboard_boxes",
  "ibc_tote",
  "dumpster",
  "fence_panel",
]);
const outline = [
  [-1, -1],
  [1, -1],
  [1, 0],
  [0, 0],
  [0, 1],
  [-1, 1],
  [-1, -1],
];
for (const options of [
  {},
  { caps: false, soft: 0 },
  { caps: true, soft: 1.2 },
]) {
  compareGeometry(
    reference.extrude(outline, 0.2, options),
    native.extrude(outline, 0.2, options),
    `Extrusion ${JSON.stringify(options)}`,
  );
}
const ribbonPath = [
  [0, 0, 0],
  [1, 0, 0],
  [1, 0, 0],
  [1.5, 0.3, 0.42],
];
for (const doubleSided of [false, true]) {
  compareGeometry(
    reference.ribbon(ribbonPath, [0, 0.02, 0.01], doubleSided),
    native.ribbon(ribbonPath, [0, 0.02, 0.01], doubleSided),
    `Band ${doubleSided}`,
  );
}

const extrusions = new Set([
  "pallet",
  "cable_reel",
  "jerry_can",
  "pipe_stack",
  "forklift",
  "dumpster",
]);
const ribbons = new Set([
  "crate_stack",
  "pipe_stack",
  "sandbags",
  "cardboard_boxes",
  "tarp_cargo",
  "dumpster",
]);
// The 19 logistics builders and the rack cover both native mesh assembly paths.
const wearObjects = new Set([
  "barrel",
  "pallet",
  "pallet_stack",
  "crate_wood",
  "crate_stack",
  "ibc_tote",
  "forklift",
  "cable_reel",
  "tire_stack",
  "dumpster",
  "generator",
  "jerry_can",
  "traffic_cone",
  "gas_cylinders",
  "tarp_cargo",
  "sandbags",
  "toolbox_cart",
  "cardboard_boxes",
  "pipe_stack",
]);
const batchObjects = new Set([...wearObjects, "shelf_rack"]);
const remainingObjects = new Set([
  "container20",
  "container40",
  "container40hc",
  "jersey_barrier",
  "light_mast",
  "fence_panel",
  "guard_booth",
  "gate_arm",
  "bollard_mooring",
  "bollard_post",
  "relay_node",
  "stairs_metal",
  "catwalk",
  "reefer_unit",
  "floodlight_stand",
  "sign_post",
]);
const objects = [...new Set([...batchObjects, ...remainingObjects])];
same(
  reference.propNames().sort(),
  [...objects].sort(),
  "Vollständige Liste der Hafenobjekte",
);

function listMeshes(root: any) {
  const meshes: any[] = [];
  root.updateMatrixWorld(true);
  root.traverse((node: any) => {
    if (node.isMesh) meshes.push(node);
  });
  return meshes;
}

function compareMeshes(
  expected: any,
  actual: any,
  id: string,
  compareAttributes = true,
) {
  const expectedMeshes = listMeshes(expected);
  const actualMeshes = listMeshes(actual);
  same(expectedMeshes.length, actualMeshes.length, `${id}/Meshanzahl`);
  const userData = (root: any) =>
    JSON.stringify(root.userData, (key, value) =>
      key === "uuid" ? undefined : value,
    );
  same(userData(expected), userData(actual), `${id}/Objektdaten`);
  for (const [index, mesh] of expectedMeshes.entries()) {
    const result = actualMeshes[index];
    same(
      mesh.matrixWorld.elements,
      result.matrixWorld.elements,
      `${id}/${index}/Transformation`,
    );
    if (compareAttributes)
      compareGeometry(mesh.geometry, result.geometry, `${id}/${index}`);
  }
}

for (const mode of ["positions", "normals", "singular", "projective"]) {
  const expected = reference.transformProbe(mode);
  const before = invoked.transforms;
  const actual = native.transformProbe(mode);
  if (invoked.transforms === before)
    throw new Error(`Transformation ${mode} ohne WASM`);
  compareGeometry(expected, actual, `Transformation ${mode}`);
  same(
    expected.attributes.position.version,
    actual.attributes.position.version,
    `${mode}/Positionsupdate`,
  );
  same(
    expected.attributes.normal?.version,
    actual.attributes.normal?.version,
    `${mode}/Normalenupdate`,
  );
  same(expected.boundingBox, actual.boundingBox, `${mode}/AABB`);
  same(expected.boundingSphere, actual.boundingSphere, `${mode}/Kugel`);
}

const triangle = [0, 0, 0, 1, 0, 0, 0, 1, 0];
const indexedTriangle = {
  positions: triangle,
  indices: [0, 1, 2],
  colors: triangle,
};
const flatTriangle = {
  positions: triangle.map((value) => value + 2),
  indices: null,
};
const longGeometry = {
  positions: new Float32Array(65535 * 3),
  indices: [0, 1, 2],
};
for (const parts of [
  [indexedTriangle, indexedTriangle],
  [indexedTriangle, flatTriangle, indexedTriangle],
  [cornerBatch(3), longGeometry, indexedTriangle],
]) {
  const before = invoked.meshBatches;
  const expected = reference.batch(parts);
  const result = native.batch(parts);
  if (invoked.meshBatches === before) throw new Error("Mesh-Stapel ohne WASM");
  same(
    expected.index?.array.constructor.name,
    result.index?.array.constructor.name,
    "Indexbreite",
  );
  compareGeometry(expected, result, "Mesh-Stapel");
}

function cornerBatch(size: number) {
  return {
    positions: triangle.map((value) => value * size),
    indices: [2, 1, 0],
  };
}

for (const args of [
  [1, 2, 3],
  [0.7, 0.14, 0.33, 0.06, "bottom"],
  [0.02, 1.2, 0.02, 0.03],
]) {
  compareGeometry(
    reference.edgeRoundedBox(...args),
    native.edgeRoundedBox(...args),
    `Kantenrundung ${args}`,
  );
}
for (const side of [-1, 1]) {
  compareGeometry(
    reference.shipPanel(0.1, 22, 0.1, 11, side),
    native.shipPanel(0.1, 22, 0.1, 11, side),
    `Rumpfbeplankung ${side}`,
  );
}

const backdrops = ["ship", "boat", "crane"];
for (const name of backdrops) {
  for (const showcase of [false, true]) {
    const expected = reference.buildBackdrop(name, showcase);
    const before = { ...invoked };
    const result = native.buildBackdrop(name, showcase);
    if (
      name === "ship" &&
      (before.hullSamples === invoked.hullSamples ||
        before.hullPanels === invoked.hullPanels)
    )
      throw new Error("Frachtschiff ohne WASM-Beplankung");
    if (
      name === "boat" &&
      (before.dinghyHulls === invoked.dinghyHulls ||
        before.collars === invoked.collars)
    )
      throw new Error("Schlauchboot ohne WASM-Rumpf oder -Schlauch");
    if (name === "crane" && before.snappedBoxes === invoked.snappedBoxes)
      throw new Error("Kran ohne WASM-Kantenrundung");
    const label = `${name}/${showcase ? "Katalog" : "Spiel"}`;
    same(expected.stats, result.stats, `${label}/Statistik`);
    compareMeshes(expected.root, result.root, label);
    expected.update(0.73);
    result.update(0.73);
    compareMeshes(expected.root, result.root, `${label}/Animation`, false);
  }
}

for (const id of objects) {
  for (const options of [
    { seed: 12, open: false, wear: 0, doors: 0, count: 4, courses: 3 },
    {
      seed: 31,
      open: true,
      lying: true,
      wear: 1,
      doors: 0.8,
      stripe: true,
      count: 5,
      courses: 7,
      state: "cut",
    },
  ]) {
    const expected = reference.buildProp(id, options);
    const before = { ...invoked };
    const result = native.buildProp(id, options);
    if (remainingObjects.has(id) && invoked.transforms === before.transforms)
      throw new Error(`Keine Rust-Transformation für ${id}`);
    if (
      id.startsWith("container") &&
      invoked.containerSides - before.containerSides !== 2
    )
      throw new Error(`Containerseiten ohne Rust-Farbberechnung für ${id}`);
    if (batchObjects.has(id) && invoked.meshBatches === before.meshBatches)
      throw new Error(`Kein Rust-Mesh-Zusammenbau für ${id}`);
    if (wearObjects.has(id) && invoked.wearBatches === before.wearBatches)
      throw new Error(`Keine Rust-Oberflächenalterung für ${id}`);
    if (surfaces.has(id) && invoked.surfaces === before.surfaces)
      throw new Error(`Keine WASM-Profilfläche für ${id}`);
    if (tubes.has(id) && invoked.tubes === before.tubes)
      throw new Error(`Kein WASM-Rohr für ${id}`);
    if (boxGrids.has(id) && invoked.boxGrids === before.boxGrids)
      throw new Error(`Kein WASM-Quader für ${id}`);
    if (extrusions.has(id) && invoked.extrusions === before.extrusions)
      throw new Error(`Kein WASM-Strangprofil für ${id}`);
    if (ribbons.has(id) && invoked.ribbons === before.ribbons)
      throw new Error(`Kein WASM-Band für ${id}`);
    if (id === "tarp_cargo" && invoked.drapes === before.drapes)
      throw new Error("Plane ohne WASM-Mesh");
    if (
      id === "sandbags" &&
      (invoked.sandbags === before.sandbags ||
        invoked.normalPasses === before.normalPasses)
    )
      throw new Error("Sandsäcke ohne WASM-Verformung und -Normalenglättung");
    if (id === "tire_stack" && invoked.tireProfiles === before.tireProfiles)
      throw new Error("Reifen ohne WASM-Profil und -UVs");
    if (id === "cardboard_boxes" && invoked.cartons === before.cartons)
      throw new Error("Kartons ohne WASM-Verformung");
    if (
      Object.entries(before).every(
        ([key, count]) => count === invoked[key as keyof typeof invoked],
      )
    )
      throw new Error(`Keine WASM-Geometrie für ${id}`);
    compareMeshes(expected, result, id);
    if (id === "relay_node") {
      for (const relay of [expected, result]) {
        relay.userData.setState("cutting");
        for (let frame = 0; frame < 12; frame++) relay.userData.update(1 / 60);
      }
      compareMeshes(expected, result, `${id}/Animation`, false);
    }
  }
}

function compareLevelPlacements(expected: any, actual: any, seed: number) {
  same(expected.level.stats, actual.level.stats, `Hafen/${seed}/Statistik`);
  same(
    expected.level.placed.map(({ name }: any) => name),
    actual.level.placed.map(({ name }: any) => name),
    `Hafen/${seed}/Objektreihenfolge`,
  );
  for (const [index, placement] of expected.level.placed.entries()) {
    const result = actual.level.placed[index];
    for (const axis of ["x", "y", "z", "yaw"])
      if (Math.abs(placement[axis] - result[axis]) > 1e-12)
        throw new Error(
          `Hafen/${seed}/${placement.name}/${index}/${axis} weicht ab.`,
        );
  }
  same(
    expected.physics.boxes.length,
    actual.physics.boxes.length,
    "Hafenkollisionen",
  );
  for (const [index, collider] of expected.physics.boxes.entries()) {
    const result = actual.physics.boxes[index];
    for (const corner of ["min", "max"]) {
      if (collider[corner].distanceTo(result[corner]) > 1e-12)
        throw new Error(`Hafen/${seed}/Collider ${index}/${corner} weicht ab.`);
    }
    same(
      [
        collider.surface,
        collider.tag,
        collider.noNav,
        collider.bulletOnly,
        collider.playerOnly,
      ],
      [
        result.surface,
        result.tag,
        result.noNav,
        result.bulletOnly,
        result.playerOnly,
      ],
      `Hafen/${seed}/Collider ${index}/Eigenschaften`,
    );
  }
  same(expected.level.explosives, actual.level.explosives, "Explosive Objekte");
  const label = (mesh: any) => [
    mesh.name,
    mesh.material.name,
    mesh.material.vertexColors,
    mesh.material.userData.interior,
    mesh.castShadow,
    mesh.receiveShadow,
    mesh.layers.mask,
    mesh.geometry.attributes.position.count,
    mesh.geometry.index?.count,
  ];
  same(
    listMeshes(expected.level.root).map(label),
    listMeshes(actual.level.root).map(label),
    `Hafen/${seed}/Meshgruppen und Materialien`,
  );
}

const expectedHarbor = await reference.buildLevel(4207);
const actualHarbor = await native.buildLevel(4207);
compareLevelPlacements(expectedHarbor, actualHarbor, 4207);
same(
  ["yard", "gate", "workshop-props", "quay"],
  builtHarborAreas,
  "Hafenaufbau in WASM",
);
compareMeshes(expectedHarbor.level.root, actualHarbor.level.root, "Hafen");
console.log(
  `${actualHarbor.level.placed.length} Platzierungen und ${actualHarbor.physics.boxes.length} Collider im Hafen verglichen.`,
);

const originalPlayer = reference.createPlayerProbe();
const rustPlayer = native.createPlayerProbe();
for (let frame = 0; frame < 420; frame++) {
  for (const { player, input } of [originalPlayer, rustPlayer]) {
    input.look.dx = frame < 240 ? 0 : 0.025;
    input.look.dy = frame < 100 ? 0 : -0.004;
    input.move.x = frame < 60 ? 0 : Math.sin(frame / 38) * 0.55;
    input.move.y = frame < 60 ? 0 : 1;
    input.ads = frame > 300;
    input.fire = frame > 160 && frame < 180;
    input.crouch = frame < 30 || frame > 200 && frame < 240;
    input.sprint = frame > 130 && frame < 220;
    input.jump = frame === 31 || frame === 108 || frame === 147;
    if (frame === 190 || frame === 290) player.damage(29, null);
    player.update(1 / 60);
  }
  const snapshot = ({ player }: typeof originalPlayer) => [
    ...player.pos.toArray(), ...player.vel.toArray(), player.health, player.yaw,
    player.pitch, player.speed, player.hurtPulse, player.lookScale,
  ];
  same(
    [originalPlayer.player.onGround, originalPlayer.player.crouching, originalPlayer.player.sprinting],
    [rustPlayer.player.onGround, rustPlayer.player.crouching, rustPlayer.player.sprinting],
    `Spielersteuerung/${frame}/Zustand`,
  );
  const expected = snapshot(originalPlayer);
  const actual = snapshot(rustPlayer);
  for (const [index, expectedValue] of expected.entries()) {
    if (Math.abs(expectedValue - actual[index]) > 1e-11)
      throw new Error(`Spielersteuerung/${frame}/${index}: ${expectedValue} != ${actual[index]}`);
  }
}
same(originalPlayer.events, rustPlayer.events, "Ereignisse der Spielersteuerung");
if (!rustPlayer.events.includes("jump") || !rustPlayer.events.includes("step") || !rustPlayer.events.includes("hit"))
  throw new Error("Spielersteuerung hat Sprung, Schritte oder Treffer ausgelassen.");
console.log("420 Spielerschritte mit Sprint, Ducken, Sprung und Schaden stimmen überein.");

const gameplay = rustPlayer.gameplay;
const switchTune = { holster: 0.2, draw: 0.3 };
const switching = { t: 0, fresh: true, phase: "holster" };
same([0, 0], Array.from(gameplay.switchPhase(switching, switchTune, 0.4)), "Frischer Waffenwechsel");
switching.fresh = false;
same([0.15, 1], Array.from(gameplay.switchPhase(switching, switchTune, 0.4)), "Übergang beim Waffenwechsel");
switching.phase = "draw";
same([0.4, 2], Array.from(gameplay.switchPhase(switching, switchTune, 0.4)), "Abgeschlossener Waffenwechsel");

const gull = { atk: "tele", atkT: 0.03, flare: 0, shots: 0, cool: 0 };
const gullTune = { burst: 3, burstGap: 0.16, fireRange: 28 };
const gullActions = [0.01, 0.03, 0.01, 0.17, 0.17].map(dt =>
  gameplay.gullAttack(gull, null, gullTune, dt, 0, true),
);
same([0, 3, 4, 4, 5], gullActions, "Gull: Vorwarnung und Feuerstoß");
same(0, gull.shots, "Gull: Schüsse im Feuerstoß");

const js = reference.createPhysics();
const wasm = native.createPhysics();
for (let i = 0; i < 160; i++) {
  const min = [random(-42, 21), random(-1, 5), random(-30, 28)];
  const max = min.map((value) => value + random(0.4, 2.4));
  const flags = {
    noNav: i % 7 === 0,
    playerOnly: i % 11 === 0,
    bulletOnly: i % 13 === 0,
    surface: String(i),
  };
  js.addBox(min, max, flags);
  wasm.addBox(min, max, flags);
}
function compareGrid(label: string) {
  const { blocked: expected, ...jsGrid } = js.nav._grid();
  const { blocked: actual, ...wasmGrid } = wasm.nav._grid();
  same(jsGrid, wasmGrid, label);
  if (!Buffer.from(expected).equals(Buffer.from(actual)))
    throw new Error(`${label}: Zellenraster weicht ab.`);
}

js.build();
wasm.build();
for (let pass = 0; pass < 2; pass++) {
  compareGrid(`Navigationsraster ${pass}`);
  for (let i = 0; i < 300; i++) {
    const start = { x: random(-46, 28), y: random(-2, 7), z: random(-31, 31) };
    const end = { x: random(-46, 28), y: 0, z: random(-31, 31) };
    const dir = {
      x: i % 9 === 0 ? 0 : random(-1, 1),
      y: i % 6 === 0 ? 0 : random(-1, 1),
      z: i % 10 === 0 ? 0 : random(-1, 1),
    };
    const ceiling = random(-2, 8);
    const path = (physics: typeof js) =>
      physics.nav.findPath(start, end)?.map((point: any) => point.toArray()) ??
      null;
    const ray = (physics: typeof js) => {
      const hit = physics.raycast(start, dir, 80);
      return (
        hit && [
          hit.dist,
          hit.point.toArray(),
          hit.normal.toArray(),
          hit.surface,
          physics.boxes.indexOf(hit.box),
        ]
      );
    };
    same(path(js), path(wasm), `Wegsuche ${pass}/${i}`);
    same(ray(js), ray(wasm), `Kollisionsstrahl ${pass}/${i}`);
    same(
      js.lineOfSight(start, end),
      wasm.lineOfSight(start, end),
      `Sichtlinie ${pass}/${i}`,
    );
    same(
      js.nav.clearLine(start, end),
      wasm.nav.clearLine(start, end),
      `Gerader Weg ${pass}/${i}`,
    );
    same(
      js.nav.isWalkable(start.x, start.z),
      wasm.nav.isWalkable(start.x, start.z),
      `Begehbarkeit ${pass}/${i}`,
    );
    const margin = 0.32 * 0.9;
    const minY = start.y + 1.15;
    const maxY = start.y + 1.8;
    const clear = !js.boxes.some(
      ({ min, max, bulletOnly }: any) =>
        !bulletOnly &&
        min.y < maxY &&
        max.y > minY &&
        start.x + margin >= min.x &&
        start.x - margin <= max.x &&
        start.z + margin >= min.z &&
        start.z - margin <= max.z,
    );
    same(
      clear,
      wasm.hasClearance(start.x, start.z, margin, minY, maxY),
      `Kopffreiheit ${pass}/${i}`,
    );
    const sampleSeed = 73425 + pass * 300 + i;
    reference.resetNavRandom(sampleSeed);
    native.resetNavRandom(sampleSeed);
    const expected = js.nav.randomPoint(start, 3.5);
    const actual = wasm.nav.randomPoint(start, 3.5);
    same(
      reference.navRandomDraws(),
      native.navRandomDraws(),
      `Zufallsziehungen ${pass}/${i}`,
    );
    if (expected.distanceTo(actual) > 1e-12)
      throw new Error(`Zufallswegpunkt ${pass}/${i} weicht ab.`);
    same(
      js.groundHeight(start.x, start.z, ceiling),
      wasm.groundHeight(start.x, start.z, ceiling),
      `Bodenhöhe ${pass}/${i}`,
    );
    const duration = random(0.012, 0.12);
    const velocity = { x: dir.x * 8, y: dir.y * 8, z: dir.z * 8 };
    const move = (physics: typeof js) => {
      const p = { ...start },
        v = { ...velocity };
      return [
        p,
        v,
        physics.moveCapsule(p, v, duration, 0.32, 1.8, i % 3 !== 0),
      ];
    };
    same(move(js), move(wasm), `Kapselbewegung ${pass}/${i}`);
    const gravity = 15.8;
    const alive = i % 2 === 0;
    const expectedPosition = { ...start };
    const expectedVelocity = { ...velocity };
    expectedVelocity.y -= alive ? gravity * duration * 0.5 : gravity * duration;
    if (alive && expectedVelocity.y < -40) expectedVelocity.y = -40;
    const fallVelocity = expectedVelocity.y;
    const expectedStep = js.moveCapsule(
      expectedPosition,
      expectedVelocity,
      duration,
      0.32,
      1.8,
      i % 3 !== 0,
    );
    if (alive && !expectedStep.onGround)
      expectedVelocity.y -= gravity * duration * 0.5;
    const position = { ...start };
    const playerVelocity = { ...velocity };
    const step = wasm.movePlayer(
      position,
      playerVelocity,
      duration,
      0.32,
      1.8,
      i % 3 !== 0,
      gravity,
      alive,
    );
    same(
      [expectedPosition, expectedVelocity, { ...expectedStep, fallVelocity }],
      [position, playerVelocity, step],
      `Spielerbewegung mit Gravitation ${pass}/${i}`,
    );
  }
  for (const physics of [js, wasm]) {
    physics.removeBox(physics.boxes[31]);
    physics.addBox([-28, 0, -12], [-24, 4, 3], { surface: "replacement" });
  }
}

for (const physics of [js, wasm]) {
  const added = physics.addBox([23, 0, 23], [24, 0.3, 24]);
  added.max.y = 1.5;
  physics.build();
  same(
    false,
    physics.nav.isWalkable(23.5, 23.5),
    "Geänderter Collider nach Build",
  );
}
compareGrid("Navigationsraster nach erneutem Build");
const separateWorld = native.createPhysics();
separateWorld.addBox([-50, 0, -40], [32, 12, 35]);
same(false, separateWorld.nav.isWalkable(23.5, 23.5), "Separater Collider");
same(12, separateWorld.groundHeight(23.5, 23.5), "Separater Boden");
same(1.5, wasm.groundHeight(23.5, 23.5), "Erhaltener Boden");
same(
  null,
  separateWorld.nav.findPath({ x: 0, z: 0 }, { x: -3, z: 2 }),
  "Vollständig blockierter Weg",
);
compareGrid("Unabhängige Navigationsraster");

let variants = 0;
let sounds = 0;
let samples = 0;
let differingSamples = 0;
let maxSampleError = 0;
const bytes = (channel: Float32Array) =>
  Buffer.from(channel.buffer, channel.byteOffset, channel.byteLength);
function comparePcm(
  expected: any,
  actual: any,
  label: string,
  tolerance: number,
) {
  same(
    [
      expected.sr,
      expected.loop,
      expected.ch.map((channel: Float32Array) => channel.length),
    ],
    [
      actual.sr,
      actual.loop,
      actual.ch.map((channel: Float32Array) => channel.length),
    ],
    `${label}/Audioformat`,
  );
  for (const [index, channel] of expected.ch.entries()) {
    const result = actual.ch[index];
    if (bytes(channel).equals(bytes(result))) continue;
    for (let i = 0; i < channel.length; i++) {
      const delta = Math.abs(channel[i] - result[i]);
      if (!Number.isFinite(delta) || delta > tolerance)
        throw new Error(
          `${label}/${index}/${i}: ${channel[i]} != ${result[i]}`,
        );
      if (!Object.is(channel[i], result[i])) differingSamples++;
      maxSampleError = Math.max(maxSampleError, delta);
    }
  }
}
for (const [name, spec] of Object.entries(reference.audioBank) as [
  string,
  { n: number },
][]) {
  for (let variant = 0; variant < spec.n; variant++) {
    const expected = reference.synthesize(name, variant);
    const result = native.synthesize(name, variant);
    comparePcm(expected, result, `${name}/${variant}`, 2e-8);
    for (const channel of expected.ch) samples += channel.length;
    variants++;
  }
  sounds++;
}
same(variants, invoked.synthesized, "Alle Klangvarianten aus Rust");
for (const name of ["ir_outdoor", "ir_shed"]) {
  for (const rate of [44100, 48000, 96000]) {
    const steps = native.synthesizeSteps(name, 0, rate);
    const output = steps.step();
    same(output === steps.step(), true, `${name}/${rate}/wiederholter Schritt`);
    comparePcm(
      reference.synthesize(name, 0, rate),
      output,
      `${name}/${rate}`,
      2e-8,
    );
  }
}
// A sample rate outside the Web Audio range takes the JS synthesis path.
comparePcm(
  reference.synthesize("ir_shed", 0, 6000),
  native.synthesize("ir_shed", 0, 6000),
  "Audio-Fallback",
  0,
);
if (Object.values(invoked).some((count) => count === 0))
  throw new Error("Ein WASM-Kernel wurde in der Prüfung nicht aufgerufen.");
console.log(
  `${invoked.moves} Kapselbewegungen, ${invoked.rays} Kollisionsstrahlen, ${invoked.heights} Bodenhöhen, ${invoked.sight} Sichtlinien und ${invoked.clearance} Abfragen zur Kopffreiheit in WASM geprüft.`,
);
console.log(
  `${invoked.playerSteps} Spielerbewegungen mit Gravitation in WASM geprüft.`,
);
console.log(
  `${invoked.paths} A*-Wege, ${invoked.lines} geradlinige Wege, ${invoked.walkable} begehbare Zellen und ${invoked.samples} Zufallswegpunkte in WASM geprüft.`,
);
console.log(
  `${sounds} Klänge, ${variants} Varianten und ${samples} PCM-Samples aus Rust. ${differingSamples} Werte mit max. ${maxSampleError.toExponential(2)} Abweichung inkl. Impulsantworten bei weiteren Sampleraten.`,
);
console.log(
  `${invoked.water} Wasserhöhen verglichen (max. ${maxHeightError.toExponential(2)} m Abweichung; Vorschau, Float32: ${maxVertexError.toExponential(2)} m).`,
);
console.log(
  `${objects.length} Objekte in je zwei Varianten verglichen. Mesh-Daten stimmen überein.`,
);
console.log(
  `${invoked.profiles} Rotationsprofile, ${invoked.boxes} gefaste Quader, ${invoked.cylinders} Zylinder, ${invoked.corrugations} Wellprofile, ${invoked.surfaces} Profilflächen und ${invoked.tubes} Rohre in WASM.`,
);
console.log(
  `${invoked.boxGrids} einfache und gerundete Quader, ${invoked.tireProfiles} Reifenprofile, ${invoked.sandbags} Sandsack- und ${invoked.cartons} Kartonverformungen sowie ${invoked.normalPasses} Normalenglättungen in WASM.`,
);
console.log(
  `${invoked.extrusions} Profil-Extrusionen, ${invoked.ribbons} Bänder und ${invoked.drapes} Planen in WASM.`,
);
console.log(
  `${backdrops.length} Hafenmodelle im Spiel und im Katalog verglichen. ${invoked.hullSamples} Rumpfgitter, ${invoked.hullPanels} Rumpfpaneele, ${invoked.dinghyHulls} Bootsrümpfe, ${invoked.collars} Bootsschläuche und ${invoked.snappedBoxes} Kantenrundungen in WASM.`,
);
console.log(
  `${batchObjects.size} Modelle mit Rust-Mesh-Zusammenbau nachgewiesen, davon ${wearObjects.size} mit prozeduraler Alterung in Rust. ${invoked.meshBatches} Mesh-Stapel, ${invoked.wearBatches} Alterungsdurchläufe.`,
);
console.log(
  `${remainingObjects.size} weitere Modelle mit Rust-Transformation, ${invoked.transforms} Transformationen, ${invoked.rebuiltNormals} Normalenberechnungen und ${invoked.containerSides} farbige Containerseiten.`,
);
