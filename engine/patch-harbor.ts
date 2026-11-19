type Change = [marker: string, replacement: string];

const mergeScene = "function ig(n,{castShadow:e=!0,receiveShadow:t=!0}={}){";
const propCollision = "function T(C,A,F={}){let H=C.userData.colliders||[],";
const placeProp = "function w(C,A,F,H=0,N={},Z=0,Q={}){";
const placeSlab = "function d(C,A,F,H,N,Z,Q,W={}){";
const placeRod = "function v(C,A,F,H=.015,N={}){";
const containerStack = "function U(C,A,F,H=0,N=1){";
const stairsRect =
  "return i.push(Q?[C-1.5,A-.5,C+1.5,A+.5]:[C-.5,A-1.5,C+.5,A+1.5]),Z";
const catwalkRect = "return i.push([C-G,A-te,C+G,A+te]),W";
const containerOptions =
  "let i=T_[e],s=t.seed===void 0?0:Math.abs(Math.floor(t.seed)),o=s%kk,r=Math.round(ks.clamp(t.doors||0,0,1)*10)/10,a=Math.round(ks.clamp(t.wear===void 0?.5:t.wear,0,1)*2)/2,";
const containerPaint =
  "let u=c.get(l),f=t.color;pv.includes(f)||(f=t.seed===void 0?i.def:pv[(s*7+3)%pv.length]);";
const doorAngle = "let le=i*(ee>0?1:.88)*ks.degToRad(ee>0?112:100),";
const relayState = "if(ge.state=q,";
const relayMode = "ge.setState=ce=>{";
const relayTick = "ge.update=ce=>{ce=Math.min(ce||0,.1),";
const shedFan = "le.rotation.x+=$*1.7";
const backdropSetup =
  'let i=b=>!e||e===b,s=new At;s.name="backdrop";let o=bw(),r=new K(31.5,-1.6,21);t&&r.set(24,.34,15);';
const boatYaw = "a.root.rotation.y=-Math.PI/2,s.add(a.root)";
const backdropTick = "v=0;function y(b){";
const prepStart = ",t.staticRoot.updateMatrixWorld(!0);";
const prepEnd = "let f={count:0,update(){}};";
const yard = "function fP(n){";
const gate = "function hP(n){";
const quay = "function mw(n){";
const workshopStart = "let ue=Math.PI/2,de=($,se,fe,me,ge,ce=y,P)=>n.place(";
const workshopEnd = ',n.tube(t("rubber"),[[1.7';

export function harborChanges(source: string): Change[] {
  const start = source.indexOf(prepStart);
  const end = source.indexOf(prepEnd, start);
  if (start < 0 || end < 0)
    throw new Error("Die Saltline-Materialzuweisung hat sich geändert.");
  const levelPreparation = source.slice(start, end);
  const propsStart = source.indexOf(workshopStart);
  const propsEnd = source.indexOf(workshopEnd, propsStart);
  if (propsStart < 0 || propsEnd < 0)
    throw new Error("Die Saltline-Werkstatteinrichtung hat sich geändert.");
  const workshopProps = source.slice(propsStart, propsEnd + 1);
  return [
    [
      yard,
      `function saltlineBuildHarbor(area,builder){
  const harbor=saltlineGeometry?.harbor;
  if(!harbor)return false;
  harbor.buildLayout(area,builder,n=>({
    laneSign:(...args)=>a1(n,...args),
    cargo:(...args)=>Hl(n,...args),
    equipment:(...args)=>Ol(n,...args),
    junctionBox:(...args)=>Tv(n,...args),
    wheelChock:(...args)=>Vl(n,...args),
    beams:(...args)=>r1(n,...args),
    cable:(...args)=>na(n,...args),
    quayGuard:(...args)=>Sv(n,...args),
    torus:(args,yaw,material,options)=>n.add(new jn(...args).rotateY(yaw),material,options),
  }));
  return true;
}
${yard}
if(saltlineBuildHarbor("yard",n))return;`,
    ],
    [gate, `${gate}if(saltlineBuildHarbor("gate",n))return;`],
    [
      quay,
      `${quay}
if(saltlineBuildHarbor("quay",n)){
  const points=saltlineGeometry.harbor.quayPoints();
  return {relayPos:new K().fromArray(points),extraction:new K().fromArray(points,3)};
}`,
    ],
    [
      workshopProps,
      `if(!saltlineBuildHarbor("workshop-props",n)){
  ${workshopProps.slice(0, -1).replace(";return de(", ";de(")};
}
return `,
    ],
    [
      mergeScene,
      `${mergeScene}
if(saltlineGeometry?.harbor){
  return saltlineGeometry.harbor.mergeScene(n,e,t,{Group:At,Mesh:ht,merge:Jr});
}`,
    ],
    [
      placeSlab,
      `${placeSlab}
const slab=saltlineGeometry?.harbor.slab(A,F,H,N,Z,Q);
if(slab)return p(C,...slab,W);`,
    ],
    [
      placeRod,
      `${placeRod}
if(saltlineGeometry?.harbor){
  const [length,x,y,z,qx,qy,qz,qw]=saltlineGeometry.harbor.rod(A,F);
  const mesh=h(vt(H,H,length,N.seg||6),C,N);
  mesh.position.set(x,y,z);
  mesh.quaternion.set(qx,qy,qz,qw);
  return mesh;
}`,
    ],
    [
      propCollision,
      `function T(C,A,F={}){
if(saltlineGeometry?.harbor){
  saltlineGeometry.harbor.bakeColliders(C,A,n.physics,F.colOpts);
  return;
}
let H=C.userData.colliders||[],`,
    ],
    [
      placeProp,
      `${placeProp}
if(saltlineGeometry?.harbor){
  const plan=saltlineGeometry.harbor.propPlan(C,N,Q);
  const position={name:C,x:A,y:Z,z:F,yaw:H};
  if(plan&1){
    r.push({...position,opts:{...N}});
    o.push(position);
    return new At();
  }
  const object=n.props.build(C,N);
  object.position.set(A,Z,F);
  object.rotation.y=H;
  (plan&2?t:e).add(object);
  object.updateMatrixWorld(true);
  if(plan&4)T(object,H,Q);
  if(typeof object.userData.update==="function")s.push(delta=>object.userData.update(delta));
  o.push(position);
  return object;
}`,
    ],
    [
      containerStack,
      `${containerStack}
const stack=saltlineGeometry?.harbor.containerStack(C,A,F,H,N,a,w,i);
if(stack)return stack;`,
    ],
    [
      stairsRect,
      "return i.push(saltlineGeometry?.harbor.footprint(C,A,F,.5)??(Q?[C-1.5,A-.5,C+1.5,A+.5]:[C-.5,A-1.5,C+.5,A+1.5])),Z",
    ],
    [
      catwalkRect,
      "return i.push(saltlineGeometry?.harbor.footprint(C,A,F,.6)??[C-G,A-te,C+G,A+te]),W",
    ],
    [
      levelPreparation,
      `;if(saltlineGeometry?.harbor){
  saltlineGeometry.harbor.prepareLevel(t.staticRoot,t.dynRoot,n.mats);
}else{
  ${levelPreparation.slice(1)}
}`,
    ],
    [
      containerOptions,
      `const variant=saltlineGeometry?.harbor.containerVariant(e,t);
let i=T_[e],s=variant?.seed??(t.seed===void 0?0:Math.abs(Math.floor(t.seed))),
o=variant?.seed??s%kk,
r=variant?.doors??Math.round(ks.clamp(t.doors||0,0,1)*10)/10,
a=variant?.wear??Math.round(ks.clamp(t.wear===void 0?.5:t.wear,0,1)*2)/2,`,
    ],
    [
      containerPaint,
      "let u=c.get(l),f=variant?variant.color:t.color;if(!variant&&!pv.includes(f))f=t.seed===void 0?i.def:pv[(s*7+3)%pv.length];",
    ],
    [
      doorAngle,
      "let le=saltlineGeometry?.harbor.doorAngle(i,ee)??(i*(ee>0?1:.88)*ks.degToRad(ee>0?112:100)),",
    ],
    [
      relayState,
      `const relay=saltlineGeometry?.harbor.relay(t);
const relayColors=[gv,Dk,zk];
${relayState}`,
    ],
    [
      relayMode,
      `${relayMode}
if(relay){
  if(relay.setState(ce))q=ge.state=ce;
  return;
}`,
    ],
    [
      relayTick,
      `ge.update=ce=>{
if(relay){
  relay.update(ce,(level,color,rotor,door)=>{
    ge.level=level;
    O(j.copy(relayColors[color]),level);
    if(rotor)for(const mesh of xe)mesh.rotateY(rotor);
    if(door)for(const mesh of le)mesh.rotateY(door);
  });
  return;
}
ce=Math.min(ce||0,.1),`,
    ],
    [
      shedFan,
      "le.rotation.x=saltlineGeometry?.harbor.fanAngle(le.rotation.x,$)??(le.rotation.x+$*1.7)",
    ],
    [
      backdropSetup,
      `const backdrop=saltlineGeometry?.harbor.backdropPlan(e,t);
let i=b=>backdrop?backdrop.visible(b):!e||e===b,s=new At;
s.name="backdrop";
let o=bw(),r=backdrop?new K().fromArray(backdrop.boatBase):new K(31.5,-1.6,21);
if(t&&!backdrop)r.set(24,.34,15);`,
    ],
    [
      boatYaw,
      "a.root.rotation.y=backdrop?.boatBase[3]??-Math.PI/2,s.add(a.root)",
    ],
    [
      backdropTick,
      `${backdropTick}
if(backdrop){
  const [time,cranePitch,craneRoll,radar,boatY,boatPitch,boatRoll]=saltlineGeometry.harbor.backdropFrame(v,b,r.y,t);
  v=time;
  o.uTime.value=time;
  if(l){l.spreader.rotation.x=cranePitch;l.spreader.rotation.z=craneRoll;}
  if(c)c.radar.rotation.y=radar;
  if(a){a.root.position.y=boatY;a.hullGrp.rotation.x=boatPitch;a.hullGrp.rotation.z=boatRoll;}
  return;
}`,
    ],
  ];
}
