import { patchGameKernels } from "../engine/patch-game";

// Adapt the game at serving time so the archive and catalog still share the original bundle.
export function patchGameGround(source: string) {
  const start = "cL().catch(n=>yT(n&&n.stack||n));";
  const mask = "s=Dw(i),o=Hg(s,i";
  const detail = "{det:a,nrm:l}=Lw(r),c=Hg(a,r";
  if (
    [start, mask, detail].some((marker) => source.split(marker).length !== 2)
  ) {
    throw new Error("The original Saltline ground setup has changed.");
  }

  const audio = 'await r("audio","audio",()=>n.noaudio?Sr.audio():cE(t))';
  if (source.split(audio).length !== 2)
    throw new Error("The Saltline audio startup has changed.");
  const physics = "t.physics=v_(t)";
  if (source.split(physics).length !== 2)
    throw new Error("The Saltline startup has changed.");
  return patchGameKernels(source)
    .replace(mask, "s=await saltlineGroundMask(i),o=Hg(s,i")
    .replace(detail, "{det:a,nrm:l}=await saltlineGroundDetail(r),c=Hg(a,r")
    .replace(
      audio,
      'await r("audio","audio",async()=>{if(n.noaudio)return Sr.audio();await saltlineKernels?.audio.prepare();return cE(t)})',
    )
    .replace(
      physics,
      "t.physics=(await saltlineKernelsReady,await saltlineGeometryReady,v_(t))",
    )
    .replace(
      start,
      `
const saltlineGroundMode = new URLSearchParams(location.search).get("ground");
const saltlineGroundRuntime = saltlineGroundMode === "original"
  ? null
  : import("./ground/runtime.js").catch(() => null);
const saltlineGroundGenerator = saltlineGroundRuntime?.then(runtime => runtime?.prepareGroundTextures(saltlineGroundMode));
let saltlineKernels;
var saltlineGeometry;
const saltlineGeometryReady = saltlineGroundMode === "js" || saltlineGroundMode === "original"
  ? null
  : import("./engine/objects.js")
    .then(runtime => runtime.prepareObjectGeometry())
    .then(geometry => {
      saltlineGeometry = geometry;
      if(geometry)performance.mark("saltline:object-geometry:wasm");
    })
    .catch(() => {});
const saltlineKernelsReady = saltlineGroundMode === "js" ? null : saltlineGroundRuntime
  ?.then(runtime => runtime?.prepareGameKernels())
  .then(kernels => {
    saltlineKernels = kernels;
    if(kernels){
      performance.mark("saltline:game-kernels:wasm");
      if(!new URLSearchParams(location.search).get("noaudio"))kernels.audio.prepare();
    }
  })
  .catch(() => {});

async function saltlineGroundMask(resolution) {
  const generate = await saltlineGroundGenerator;
  if (generate) {
    return generate.mask(resolution, { bounds: Fi, basins: Pw, basinWidth: Iw, basinDepth: EP, waterline: wP });
  }
  const start = performance.now();
  const pixels = Dw(resolution);
  performance.measure("saltline:ground-mask:original", { start });
  return pixels;
}

async function saltlineGroundDetail(resolution) {
  const generate = await saltlineGroundGenerator;
  if (generate) return generate.detail(resolution);
  const start = performance.now();
  const pixels = Lw(resolution);
  performance.measure("saltline:ground-detail:original", { start });
  return pixels;
}

${start}`,
    );
}
