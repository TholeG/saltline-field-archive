import { gameplayChanges } from "./patch-gameplay";
import { audioChanges } from "./patch-audio";
import { harborChanges } from "./patch-harbor";

const createPhysics = "function v_(n){";
const playerClearance = "function F(){let J=n.physics&&n.physics.boxes;";
const playerGravity =
  ",A.vel.y-=Nt.gravity*J*.5,A.vel.y<-40&&(A.vel.y=-40);let ce=A.vel.y,P=n.physics.moveCapsule(A.pos,A.vel,J,zi.radius,o,l)||{};P.onGround||(A.vel.y-=Nt.gravity*J*.5);";
const corpseGravity =
  ",A.vel.y-=Nt.gravity*J;try{let G=n.physics.moveCapsule(A.pos,A.vel,J,zi.radius,.6,l);l=!!(G&&G.onGround)}catch{}";
const legacy =
  "function aT(n){let e=document.getElementById(oT);if(!e)return!1;";
const waterHeight = "function Hw(n,e,t){";
const profileGeometry = "function hs(n,e=16){";
const boxGeometry = 'function tt(n,e,t,i=.008,s="center"){';
const plainBoxGeometry = 'function ot(n,e,t,i="center"){';
const roundedBoxGeometry = 'function uo(n,e,t,i=.03,s=3,o=3,r=3,a="center"){';
const edgeRoundedBox = 'function Ei(n,e,t,i=.02,s="center"){';
const boxConstructor =
  "R0=class n extends pt{constructor(e=1,t=1,i=1,s=1,o=1,r=1){";
const boxParameters =
  'super(),this.type="BoxGeometry",this.parameters={width:e,height:t,depth:i,widthSegments:s,heightSegments:o,depthSegments:r};';
const profileTire = ",a=hs(r,i),l=a.attributes.uv,c=r.length,u=[0];";
const sandbagDeformation = "return fo(s,l=>{let c=l.x/(n/2)";
const cartonDeformation = "a&&vv(x,2),o&&(fo(x,m),Cr(x));";
const normalSmoothing = "function Cr(n){";
const outlineExtrusion = "function ko(n,e,{soft:t=.6,caps:i=!0}={}){";
const ribbonGeometry = "function kr(n,e,t=!0){";
const drapeStart = ",x=[],v=[],y=[],b=(N,Z)=>Z*(l+1)+N;";
const drapeEnd = "let R=S.attributes.normal,z=S.attributes.position,I=(N,Z)=>";
const cylinderGeometry = 'function vt(n,e,t,i=16,s=!1,o="center"){';
const cylinderConstructor =
  "io=class n extends pt{constructor(e=1,t=1,i=1,s=32,o=1,r=!1,a=0,l=Math.PI*2){";
const cylinderParameters =
  'super(),this.type="CylinderGeometry",this.parameters={radiusTop:e,radiusBottom:t,height:i,radialSegments:s,heightSegments:o,openEnded:r,thetaStart:a,thetaLength:l};';
const corrugatedProfile = "function sh(n,e,t,i,s,o,r=n){";
const sweptSurface =
  "function Fa(n,e,t,{closed:i=!1,center:s=null,inward:o=!1,caps:r=!1}={}){";
const tubeConstructor =
  "Ta=class n extends pt{constructor(e=new Cf(new K(-1,-1,0),new K(-1,1,0),new K(1,1,0)),t=64,i=1,s=8,o=!1){";
const tubeFrames =
  "let r=e.computeFrenetFrames(t,o);this.tangents=r.tangents,this.normals=r.normals,this.binormals=r.binormals;";
const tubeMesh = ",c=new Ta(r,Math.min(l,400),e,t,i),u=c.attributes.uv;";
const hullParameters = "[bt,xP,Io,Ln,ch,Ks,bP]";
const hullPanel = "function fu(n,e,t,i,s,o=.012,r=6,a=1){";
const hullPlates =
  "for(let ee of[-1,1])for(let le=0;le<p.length-1;le++){let xe=p[le]";
const hullPlatePoints =
  "[N0(xe,O[ue],ee),N0(j,q[ue],ee),N0(j,q[ue+1],ee),N0(xe,O[ue+1],ee)]";
const hullPlateNormals =
  "[Ba(xe,O[ue],ee),Ba(j,q[ue],ee),Ba(j,q[ue+1],ee),Ba(xe,O[ue+1],ee)]";
const dinghyStart =
  ",M=[],_=(U,k,C)=>{let A=k.clone().sub(U).cross(C.clone().sub(U))";
const dinghyEnd = ",l(o,T),l(i,Pe(-3.38,.2,-.86,2.1,.26,.86),u),";
const dinghyCollar =
  ",m=new Ta(d,60,.27,12,!1);{let U=m.attributes.uv,k=d.getLength();";
const mergeStart = "function Jr(n){";
const mergeAssembly = ",s=i.every(l=>l.index),o=i.every(l=>!l.index),r=s||o?";
const wearAssembly = "let u=Jr(c.geos);if(!c.raw&&i>0&&u.attributes.color){";
const transformAttributes =
  "applyMatrix4(e){let t=this.attributes.position;t!==void 0&&(t.applyMatrix4(e),t.needsUpdate=!0);let i=this.attributes.normal;if(i!==void 0){let o=new Qt().getNormalMatrix(e);i.applyNormalMatrix(o),i.needsUpdate=!0}";
const vertexNormals =
  'computeVertexNormals(){let e=this.index,t=this.getAttribute("position");if(t!==void 0){let i=this.getAttribute("normal");';
const containerPaintStart =
  "let xe=le.attributes.position,j=new Float32Array(xe.count*3);";
const containerPaintEnd =
  'le.setAttribute("color",new nt(j,3)),c.buckets.get(E0).geos.push(le)';

function applyChanges(source: string, changes: [string, string][]) {
  for (const [marker, replacement] of changes) {
    if (source.split(marker).length !== 2)
      throw new Error("Unbekannte Saltline-Schnittstelle: " + marker);
    source = source.replace(marker, () => replacement);
  }
  return source;
}

function patchDrapedGeometry(source: string) {
  const start = source.split(drapeStart);
  if (start.length !== 2)
    throw new Error("Die Saltline-Planengeometrie hat sich geändert.");
  const end = start[1].split(drapeEnd);
  if (end.length !== 2)
    throw new Error("Die Befestigung der Saltline-Plane hat sich geändert.");

  return `${start[0]};
const nativeDrape=saltlineGeometry?.drapeMesh(r,l,c,d,[u,f,h,p],[1.2,.7],[5,16],a,s,i);
const [S,b,w,_]=nativeDrape ? [
  saltlineSetMesh(new ut.BufferGeometry(),nativeDrape.mesh),
  nativeDrape.topVertex,nativeDrape.skirtVertex,nativeDrape.edgeIndices
] : (()=>{
  let ${drapeStart.slice(1)}${end[0]}
  return [S,b,w,_];
})();
${drapeEnd}${end[1]}`;
}

function patchDinghyGeometry(source: string) {
  const start = source.split(dinghyStart);
  if (start.length !== 2)
    throw new Error("Die Saltline-Bootsgeometrie hat sich geändert.");
  const end = start[1].split(dinghyEnd);
  if (end.length !== 2)
    throw new Error("Der Saltline-Bootsrumpf hat sich geändert.");
  return `${start[0]};
const nativeHull=saltlineGeometry?.dinghyHull(y,b,g,E);
const T=nativeHull ? saltlineSetMesh(new pt(),nativeHull) : (()=>{
  let ${dinghyStart.slice(1)}${end[0]};
  return T;
})();
${dinghyEnd.slice(1)}${end[1]}`;
}

export function patchGameGeometry(source: string) {
  const changes: [string, string][] = [
    [
      profileGeometry,
      `function saltlineSetMesh(geometry,mesh){
  geometry.setAttribute("position",new nt(mesh.positions,3));
  geometry.setAttribute("normal",new nt(mesh.normals,3));
  if(mesh.uvs)geometry.setAttribute("uv",new nt(mesh.uvs,2));
  if(mesh.colors)geometry.setAttribute("color",new nt(mesh.colors,3));
  if(mesh.indices)geometry.setIndex(new nt(mesh.indices,1));
  if(mesh.groups)for(const group of mesh.groups)geometry.addGroup(...group);
  return geometry;
}
${profileGeometry}
const mesh=saltlineGeometry?.revolveProfile(n,e,Vt);
if(mesh)return saltlineSetMesh(new pt(),mesh);`,
    ],
    [
      boxGeometry,
      `${boxGeometry}
const mesh=saltlineGeometry?.beveledBox(n,e,t,i);
if(mesh){
  const geometry=saltlineSetMesh(new pt(),mesh);
  if(s==="bottom")geometry.translate(0,e/2,0);
  return geometry;
}`,
    ],
    [
      boxConstructor + boxParameters,
      `R0=class n extends pt{constructor(e=1,t=1,i=1,s=1,o=1,r=1,saltlineMesh){
${boxParameters}
if(saltlineMesh){saltlineSetMesh(this,saltlineMesh);return;}`,
    ],
    [
      plainBoxGeometry,
      `${plainBoxGeometry}
const mesh=saltlineGeometry?.boxGrid(n,e,t,[1,1,1]);
if(mesh){
  const geometry=new R0(n,e,t,1,1,1,mesh);
  if(i==="bottom")geometry.translate(0,e/2,0);
  return geometry;
}`,
    ],
    [
      roundedBoxGeometry,
      `${roundedBoxGeometry}
const mesh=saltlineGeometry?.boxGrid(n,e,t,[s,o,r],i);
if(mesh){
  const geometry=new R0(n,e,t,s,o,r,mesh);
  if(a==="bottom")geometry.translate(0,e/2,0);
  return geometry;
}`,
    ],
    [
      edgeRoundedBox,
      `${edgeRoundedBox}
const mesh=saltlineGeometry?.boxGrid(n,e,t,[3,3,3],i,true);
if(mesh){
  const geometry=new R0(n,e,t,3,3,3,mesh);
  if(s==="bottom")geometry.translate(0,e/2,0);
  return geometry;
}`,
    ],
    [
      profileTire,
      `;
const mesh=saltlineGeometry?.revolveProfile(r,i,Vt,n);
if(mesh)return saltlineSetMesh(new pt(),mesh);
let a=hs(r,i),l=a.attributes.uv,c=r.length,u=[0];`,
    ],
    [
      sandbagDeformation,
      `if(saltlineGeometry?.deformSandbag(s.attributes.position.array,n,e,t,o,r,a))return Cr(s);
${sandbagDeformation}`,
    ],
    [
      cartonDeformation,
      `a&&vv(x,2);
if(o){
  if(!saltlineGeometry?.deformCarton(x.attributes.position.array,e,t,i,o))fo(x,m);
  Cr(x);
}`,
    ],
    [
      normalSmoothing,
      `${normalSmoothing}
const normals=saltlineGeometry?.smoothNormals(n.attributes.position.array,n.index?.array);
if(normals){n.setAttribute("normal",new nt(normals,3));return n;}`,
    ],
    [
      outlineExtrusion,
      `${outlineExtrusion}
const triangulate=i ? ()=>As.triangulateShape(n.map(([x,y])=>new We(x,y)),[]) : undefined;
const mesh=saltlineGeometry?.extrudeOutline(n,e,t,triangulate);
if(mesh)return saltlineSetMesh(new pt(),mesh);`,
    ],
    [
      ribbonGeometry,
      `${ribbonGeometry}
const mesh=saltlineGeometry?.sweepRibbon(n,e,t);
if(mesh){
  const geometry=saltlineSetMesh(new pt(),mesh);
  if(!t)geometry.attributes.normal.needsUpdate=true;
  return geometry;
}`,
    ],
    [
      cylinderConstructor + cylinderParameters,
      `io=class n extends pt{constructor(e=1,t=1,i=1,s=32,o=1,r=!1,a=0,l=Math.PI*2,saltlineMesh){
${cylinderParameters}
if(saltlineMesh){saltlineSetMesh(this,saltlineMesh);return;}`,
    ],
    [
      cylinderGeometry,
      `${cylinderGeometry}
const mesh=saltlineGeometry?.cylinder(n,e,t,i,s);
if(mesh){
  const geometry=new io(n,e,t,i,1,s,0,Math.PI*2,mesh);
  if(o==="bottom")geometry.translate(0,t/2,0);
  return geometry;
}`,
    ],
    [
      corrugatedProfile,
      `${corrugatedProfile}
const profile=saltlineGeometry?.corrugatedProfile(n,e,t,i,s,o,r);
if(profile)return profile;`,
    ],
    [
      sweptSurface,
      `${sweptSurface}
const mesh=saltlineGeometry?.sweepSurface(n,e,t,i,s,o,r);
if(mesh){
  const geometry=saltlineSetMesh(new pt(),mesh);
  geometry.attributes.normal.needsUpdate=true;
  return geometry;
}`,
    ],
    [
      tubeConstructor,
      tubeConstructor.replace("o=!1){", "o=!1,saltlineBuildTube){"),
    ],
    [
      tubeFrames,
      `${tubeFrames}
const mesh=saltlineBuildTube?.(e,t,i,s,o,r);
if(mesh){saltlineSetMesh(this,mesh);return;}`,
    ],
    [
      tubeMesh,
      `;
let nativeTube;
const buildTube=saltlineGeometry && ((curve,rows,radius,segments,closed,frames)=>
  nativeTube=saltlineGeometry.sweepTube(curve,rows,radius,segments,closed,frames,a));
const c=new Ta(r,Math.min(l,400),e,t,i,buildTube);
if(nativeTube)return c;
const u=c.attributes.uv;`,
    ],
    [
      hullPanel,
      `${hullPanel}
const mesh=saltlineGeometry?.hullPanel(${hullParameters},n,e,t,i,s,o,r,a);
if(mesh)return saltlineSetMesh(new pt(),mesh);`,
    ],
    [
      hullPlates,
      `const nativePlates=saltlineGeometry?.sampleHull(p.map(station=>[station,m(station)]),${hullParameters});
const hullVector=(side,row,layer,normal)=>new K().fromArray(nativePlates,
  (((side>0?p.length:0)+row)*(x+1)+layer)*6+(normal?3:0));
${hullPlates}`,
    ],
    [
      hullPlatePoints,
      `nativePlates ? [hullVector(ee,le,ue,false),hullVector(ee,le+1,ue,false),hullVector(ee,le+1,ue+1,false),hullVector(ee,le,ue+1,false)] : ${hullPlatePoints}`,
    ],
    [
      hullPlateNormals,
      `nativePlates ? [hullVector(ee,le,ue,true),hullVector(ee,le+1,ue,true),hullVector(ee,le+1,ue+1,true),hullVector(ee,le,ue+1,true)] : ${hullPlateNormals}`,
    ],
    [
      dinghyCollar,
      `;
let nativeCollar;
const buildCollar=saltlineGeometry && ((curve,rows,radius,segments,closed,frames)=>
  nativeCollar=saltlineGeometry.sweepTube(curve,rows,radius,segments,closed,frames,curve.getLength(),1.7));
const m=new Ta(d,60,.27,12,!1,buildCollar);
if(!nativeCollar){let U=m.attributes.uv,k=d.getLength();`,
    ],
    [mergeStart, "function Jr(n,saltlineWear){"],
    [
      mergeAssembly,
      `;
const mesh=saltlineGeometry?.assembleMeshes(i,saltlineWear);
if(mesh){
  if(saltlineWear&&mesh.colors)saltlineWear.applied=true;
  return saltlineSetMesh(new pt(),mesh);
}
let ${mergeAssembly.slice(1)}`,
    ],
    [
      wearAssembly,
      `const saltlineWear=saltlineGeometry&&!c.raw&&i>0 ? {seed:r,amount:i} : undefined;
let u=Jr(c.geos,saltlineWear);
if(!saltlineWear?.applied&&!c.raw&&i>0&&u.attributes.color){`,
    ],
    [
      transformAttributes,
      `applyMatrix4(e){
const t=this.attributes.position,i=this.attributes.normal;
const normalMatrix=i!==undefined ? new Qt().getNormalMatrix(e) : undefined;
const transformed=saltlineGeometry?.transformMesh(t,i,e.elements,normalMatrix?.elements);
if(t!==undefined){
  if(!transformed)t.applyMatrix4(e);
  t.needsUpdate=true;
}
if(i!==undefined){
  if(!transformed)i.applyNormalMatrix(normalMatrix);
  i.needsUpdate=true;
}`,
    ],
    [
      vertexNormals,
      `${vertexNormals}
const nativeNormals=saltlineGeometry?.vertexNormals(t,e,i);
if(nativeNormals){
  if(i===undefined){i=new nt(nativeNormals,3);this.setAttribute("normal",i);}
  i.needsUpdate=true;
  return;
}`,
    ],
    [
      containerPaintStart,
      `let xe=le.attributes.position;
let j=saltlineGeometry?.paintContainerSide(xe.array,{
  seed:S,wear:s,sideDepth:d,roof:r,upperLimit:y,side:ee,
  baseTint:T,streakTint:w,dents:b,streaks:M
});
if(!j){
  j=new Float32Array(xe.count*3);`,
    ],
    [containerPaintEnd, `}${containerPaintEnd}`],
  ];
  return patchDinghyGeometry(
    patchDrapedGeometry(
      applyChanges(source, [...changes, ...harborChanges(source)]),
    ),
  );
}

// Inserts optional numeric kernels at the original adapters, where the game objects stay in JS.
export function patchGameAudio(source: string) {
  return applyChanges(source, audioChanges);
}

export function patchGameKernels(source: string) {
  const changes: [string, string][] = [
    [
      waterHeight,
      `${waterHeight}
const nativeHeight=saltlineKernels?.water.height(Og,Kn.seaY,n,e,t);
if(nativeHeight!==undefined)return nativeHeight;`,
    ],
    [
      createPhysics,
      `${createPhysics}
if(saltlineKernels){
  const world=saltlineKernels.createPhysics({
    width:f0,height:co,minX:Ll,minZ:ru,cell:$s,padding:t1,
    quay:Kn.quayX,ground:Kn.groundY,sea:Kn.seaY,stepHeight:hg,
  },{
    Vector:K,toVector:x_,random:()=>Math.random(),
    bounds:{min:Kn.min.clone(),max:Kn.max.clone()},
    groundSurface:(x,z)=>n.level&&n.level.groundSurfaceAt?n.level.groundSurfaceAt(x,z):"concrete",
  });
  performance.mark("saltline:physics-world:wasm");
  return world;
}`,
    ],
    [
      playerClearance,
      `function F(){
if(n.physics?.hasClearance){
  return n.physics.hasClearance(A.pos.x,A.pos.z,zi.radius*.9,A.pos.y+Nt.crouchHeight-.05,A.pos.y+Nt.standHeight);
}
let J=n.physics&&n.physics.boxes;`,
    ],
    [
      playerGravity,
      `;
let ce,P;
if(n.physics.movePlayer){
  P=n.physics.movePlayer(A.pos,A.vel,J,zi.radius,o,l,Nt.gravity,true);
  ce=P.fallVelocity;
}else{
  A.vel.y-=Nt.gravity*J*.5;
  if(A.vel.y<-40)A.vel.y=-40;
  ce=A.vel.y;
  P=n.physics.moveCapsule(A.pos,A.vel,J,zi.radius,o,l)||{};
  if(!P.onGround)A.vel.y-=Nt.gravity*J*.5;
}`,
    ],
    [
      corpseGravity,
      `;
if(n.physics.movePlayer){
  try{
    l=n.physics.movePlayer(A.pos,A.vel,J,zi.radius,.6,l,Nt.gravity,false).onGround;
  }catch{}
}else{
  A.vel.y-=Nt.gravity*J;
  try{
    const movement=n.physics.moveCapsule(A.pos,A.vel,J,zi.radius,.6,l);
    l=!!movement?.onGround;
  }catch{}
}`,
    ],
    [
      legacy,
      `${legacy}if(e.dataset.src){location.assign(e.dataset.src);return!0}`,
    ],
  ];
  const patched = applyChanges(source, changes);
  return patchGameGeometry(patchGameAudio(applyChanges(patched, gameplayChanges(patched))));
}
