type Change = [marker: string, replacement: string];

function section(source: string, start: string, end: string) {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from + start.length);
  if (from < 0 || to < 0)
    throw new Error("Unbekannter Saltline-Spielablauf: " + start);
  return source.slice(from, to);
}

export function gameplayChanges(source: string): Change[] {
  const playerInput = section(
    source,
    "let oe=A.enabled;A.lookScale=",
    "\nlet ce,P;",
  );
  const landing = section(
    source,
    "let D=n.physics.bounds;if(D&&",
    "l=A.onGround,A.pos.y",
  );
  const gait = section(
    source,
    ",A.speed=Math.hypot(A.vel.x,A.vel.z);let B=A.crouching?",
    "Q(J),W(J)}function Z(J)",
  );
  const playerRay = section(
    source,
    "if(c&&c.alive){let u=a5.copy(c.pos);",
    'return a&&n.events.emit("impact"',
  );
  const gullRay = "_raycast(y,b,g,E){let T=y.x-d.pos.x,";
  const houndRay = "_raycast(k,C,A,F){let H=!1;";
  const boltContact =
    "C&&C.alive!==!1&&(JS.copy(H.pos).addScaledVector(H.dir,Q),nT.set(C.pos.x,C.pos.y+.35,C.pos.z),iT.set(C.pos.x,C.pos.y+(C.crouching?1:1.5),C.pos.z),jD(H.pos,JS,nT,iT)<.36*.36&&(W=!0))";
  const boltMotion =
    "H.pos.addScaledVector(H.dir,N),H.len=Math.min(1.5,H.len+N),";
  const chainFuse =
    "let E=.14+s()*.3+g*.03;b.fuse=b.fuse<0?E:Math.min(b.fuse,E)";
  const fuseDamage =
    "if(v.hp-=y,v.hp<=0){v.fuse=v.fuse<0?.06:Math.min(v.fuse,.06);return}v.fuse<0&&(v.fuse=v.spec.fuse,";
  const fuseStep =
    "if(b.fuse-=v,b.fuse<=0){m(b);continue}if(b.tick-=v,b.tick<=0){b.tick=.07;";
  const blastPlayer =
    "let l=a5.copy(a.pos).setY(a.pos.y+1).distanceTo(i);l<s&&n.physics.lineOfSight(i,a5.copy(a.pos).setY(a.pos.y+1))&&a.damage(o*.5*(1-l/s),i)";
  const weaponTrigger = section(
    source,
    "a.cooldown-=B;let Ue=!a.reload",
    "a.t-a.lastShot>.24",
  );
  const weaponSwitch = "function k(B){if(u.fresh";
  const playerSpread = section(
    source,
    "let he=E.spread,V=r()*Math.PI*2,",
    ",Z.copy(B.eye||X.position)",
  );
  const token = section(
    source,
    "let A=C>5?Bi.maxTokensCrowd",
    "},releaseToken(k)",
  );
  const awareness = section(
    source,
    "think(k,C,A){let F=n.player;",
    "},rally(k,C,A,F)",
  );
  const gullState = 'let E=d.mode==="engage",M=d.hp<gn.hp*.5';
  const gullRotation = "a.group.rotation.set(d.pitch,d.yaw,d.roll)";
  const gullFlight = section(source, gullState, ",l.body.position.y=");
  const legacyFlight = gullFlight
    .slice(gullState.length + 1, -(gullRotation.length + 1))
    .replace("let R=Math.hypot", "R=Math.hypot")
    .replace("let U=qS", "U=qS")
    .replace(
      "let k=Math.cos(d.yaw),C=Math.sin(d.yaw),A=",
      "k=Math.cos(d.yaw);C=Math.sin(d.yaw);let A=",
    )
    .replace("let Z=1-Math.exp", "Z=1-Math.exp");
  const gullAttack = section(
    source,
    "if(r.color.setRGB(d.lensC",
    "M&&(d.smokeT-=y",
  );
  const gullFire = section(
    source,
    "let q=d.muzzle.distanceTo(b.pos)",
    '}d.shots<=0&&(d.atk="idle"',
  );
  const gullSpread = section(source, "let de=(gn.spreadFar", ",n.fireBolt(");
  const houndSpread = section(
    source,
    "let F=(c0.spreadFar",
    ";let H=s.physics.raycast(g.muzzle",
  );
  const houndAttack = section(
    source,
    "else if(C){F=C.pos,H=C.pos;",
    "let W=O5(g.wantYaw,g.yaw)",
  );
  const houndGround = section(
    source,
    "let W=O5(g.wantYaw,g.yaw)",
    ",z(k),g.crouch",
  );
  const houndPlan = section(
    source,
    "let C=1/0,A=k.eye||k.pos,F=(Z,Q)=>",
    ",g.firePlayer.copy(k.pos)",
  );
  const follow = "g.planT-=k,g.planT<=0&&_();let A=g.goal;";
  const reinforcement = section(
    source,
    "function $(Y){let he=se();",
    "function se(){",
  );
  const spawn =
    "function O(Y,{minDist:he=xi.spawnFar,from:V=i.pos,preferHidden:ne=!0}={}){";
  const wave = section(
    source,
    "let V=cs.count(Y),ne=Math.round",
    "let Te=0,Re=0,Ue=cs.stagger(Y)",
  );
  const missionWave = section(
    source,
    "let he=xi.waves[Math.min",
    "let ne=0,ye=0;V.forEach",
  );
  const threat = section(
    source,
    ",g.alertT=Math.max(0,g.alertT-Y)",
    "return V!==g.intensity",
  );
  const relaySelect = "function me(){let Y=null,he=xi.interactRange;";
  const relayCut = "ne.t+=Y/xi.cutSeconds";
  const relayRelease = "ne.t=Math.max(0,ne.t-Y*1.6)";
  const extraction =
    "let ne=Math.hypot(V.pos.x-i.pos.x,V.pos.z-i.pos.z),ye=ne<(V.radius||2.5)&&Math.abs(V.pos.y-i.pos.y)<3.5;";
  const rating =
    "(g.time<270?2:g.time<400?1:0)+(he>=.5?2:he>=.3?1:0)+(g.retries===0?1:0)";
  const playerDamage =
    "A.health=Math.max(0,A.health-J),T=0,w=Math.min(1,w+.35+J/40),A.addShake(.22+J/70)";
  return [
    [
      playerInput,
      `const gameplay=n.physics.gameplay;
let ue,de;
if(gameplay){
  const motor=gameplay.playerInput(A,G,Nt,J,r,a,zi.radius,Pz);
  r=motor.grace;
  a=motor.buffer;
  o=motor.height;
  ue=motor.sin;
  de=motor.cos;
  if(motor.jump)n.events.emit("player:jump",{pos:A.pos});
}else{
  ${playerInput.replace("let ue=Math.sin(A.yaw),de=Math.cos(A.yaw);", "ue=Math.sin(A.yaw);de=Math.cos(A.yaw);")}
}`,
    ],
    [
      landing,
      `if(gameplay){
  const bounds=n.physics.bounds;
  if(bounds){
    A.pos.x=Hn(A.pos.x,bounds.min.x+.4,bounds.max.x-.4);
    A.pos.z=Hn(A.pos.z,bounds.min.z+.4,bounds.max.z-.4);
  }
  A.onGround=!!P.onGround;
  if(P.surface)L=P.surface;
  const [speed,dip,shake,damage,air,fall]=gameplay.landing(A,J,l,c,u,ce,Nt.landDip);
  if(speed>=0){
    x.v-=dip;
    if(shake)A.addShake(shake);
    n.events.emit("player:land",{speed,surface:H(),pos:A.pos});
    if(damage)A.damage(damage,null);
    f=.35;
  }
  c=air;u=fall;
  if(A.onGround&&A.pos.x<=Kn.quayX-1.2&&A.pos.y>-.5)C.copy(A.pos);
}else{
  ${landing}
}`,
    ],
    [
      gait,
      `;
if(gameplay){
  const [speed,phase,step,swing,bobY,bobX,roll]=gameplay.gait(A,Nt,J,f,h,m,de,ue);
  A.speed=speed;f=phase;h=swing;d=bobY;p=bobX;m=roll;
  if(step)n.events.emit("player:step",{surface:H(),sprint:A.sprinting,pos:A.pos});
  T+=J;A.health=gameplay.regenerate(A.health,A.maxHealth,T,Nt.regenDelay,Nt.regenRate,J);
}else{
  ${gait.slice(1, -1)};
}`,
    ],
    [
      playerDamage,
      `(()=>{
  const damage=n.physics.gameplay?.damage(A.health,w,J);
  if(damage){
    const [health,pulse,shake]=damage;
    A.health=health;
    w=pulse;
    A.addShake(shake);
  }else{
    A.health=Math.max(0,A.health-J);
    w=Math.min(1,w+.35+J/40);
    A.addShake(.22+J/70);
  }
  T=0;
})()`,
    ],
    [
      playerRay,
      `if(n.physics.gameplay){
  if(c?.alive&&n.physics.gameplay.hitsPlayer(i,s,c.pos,l)){c.damage(o,i);return true;}
}else{${playerRay}}`,
    ],
    [
      "c.enemy.damage(o*u,c.point,s,c.part)",
      'c.enemy.damage(n.physics.gameplay?.hitDamage(o,c.part==="weak")??o*u,c.point,s,c.part)',
    ],
    [
      'let c=l.pos.distanceTo(i);c<s&&n.physics.lineOfSight(i,a5.copy(l.pos).setY(l.pos.y+.5))&&l.damage(o*(1-c/s),l.pos,v1.copy(l.pos).sub(i).normalize(),"body")',
      `const damage=n.physics.gameplay?.blastDamage(i,l.pos,0,s,o);
if(damage!==undefined){
  if(!Number.isNaN(damage)&&n.physics.lineOfSight(i,a5.copy(l.pos).setY(l.pos.y+.5)))
    l.damage(damage,l.pos,v1.copy(l.pos).sub(i).normalize(),"body");
}else{
  let c=l.pos.distanceTo(i);
  if(c<s&&n.physics.lineOfSight(i,a5.copy(l.pos).setY(l.pos.y+.5)))l.damage(o*(1-c/s),l.pos,v1.copy(l.pos).sub(i).normalize(),"body");
}`,
    ],
    [
      blastPlayer,
      `const damage=n.physics.gameplay?.blastDamage(i,a.pos,1,s,o*.5);
if(damage!==undefined){
  if(!Number.isNaN(damage)&&n.physics.lineOfSight(i,a5.copy(a.pos).setY(a.pos.y+1)))a.damage(damage,i);
}else{${blastPlayer}}`,
    ],
    [
      gullRay,
      `_raycast(y,b,g,E){
const gameplay=s.physics.gameplay;
if(gameplay)return gameplay.hitGull(y,b,d,g,E);
let T=y.x-d.pos.x,`,
    ],
    [
      houndRay,
      `_raycast(k,C,A,F){
const gameplay=s.physics.gameplay;
if(gameplay)return gameplay.hitHound(k,C,g,g.pos.y+Yn.stand+g.sY.x-g.crouch,A,F);
let H=!1;`,
    ],
    [
      gullFlight,
      `${gullState};
let R,U,k,C,Z;
if(s.physics.gameplay){
  const motor=s.physics.gameplay.fly(d,b,gn,y,g,o);
  R=motor.speed;U=motor.turn;k=motor.cos;C=motor.sin;
  Bo.y=motor.accelerationY;
  Z=1-Math.exp(-7*y);
}else{
  let ${legacyFlight};
}
${gullRotation}`,
    ],
    [
      gullAttack,
      `function fireGullBolt(){
  ${gullFire
    .replace(
      gullSpread,
      `if(s.physics.gameplay){
    s.physics.gameplay.enemySpread(Qs,gn,d,n.playerSpeed*.45,M?1.5:1,o);
  }else{${gullSpread};}`,
    )
    .replace("},n.fireBolt(", "}\nn.fireBolt(")};
}
if(s.physics.gameplay){
  r.color.setRGB(d.lensC[0]*O,d.lensC[1]*O,d.lensC[2]*O);
  d.muzzle.set(0,-.12,.17).applyAxisAngle(W1,d.yaw).add(d.pos);
  const action=s.physics.gameplay.gullAttack(d,b,gn,y,U,E);
  if(action===1&&n.requestToken(d)){
    d.atk="tele";d.atkT=gn.telegraph;
    s.events.emit("enemy:telegraph",{enemy:d,type:"gull",time:gn.telegraph,pos:d.pos});
  }
  if(action===2)n.releaseToken(d);
  if(action===3)d.lead=.2+o()*.65;
  if(action>=4&&b)fireGullBolt();
  if(action===5){
    d.atk="idle";
    d.cool=gn.cooldown[0]+o()*(gn.cooldown[1]-gn.cooldown[0]);
    n.releaseToken(d);
  }
}else{
  ${gullAttack.replace(gullFire, "fireGullBolt()")}
}`,
    ],
    [
      houndSpread,
      `if(s.physics.gameplay){
  s.physics.gameplay.enemySpread(Ho,c0,g,n.playerSpeed*.3,1,o);
}else{${houndSpread};}`,
    ],
    [
      houndAttack,
      `else if(C&&s.physics.gameplay){
  F=C.pos;H=C.pos;
  const gameplay=s.physics.gameplay;
  const facing=Math.atan2(C.pos.x-g.pos.x,C.pos.z-g.pos.z);
  if(gameplay.houndCanLunge(g,C,Q,c0.lungeRange)){
    g.es="wind";g.esT=.28;
    n.releaseToken(g);
    s.events.emit("enemy:telegraph",{enemy:g,type:"hound",time:.28,pos:g.pos,melee:true});
  }
  if(g.es==="move"){
    g.moveT+=k;
    const reached=T(k,c0.runSpeed);
    if(reached||(gameplay.houndMoveChance(g,Q,c0)&&o()<k*.7)||g.moveT>7){
      g.es="aim";g.esT=0;g.hasGoal=false;
    }else if(C.pos.distanceToSquared(g.firePlayer)>49)w();
  }else{
    const phase=g.es;
    const attack=gameplay.houndAttack(g,C,c0,k,Q,facing);
    g.wantYaw=facing;N=attack.crouch;
    g.esT=attack.timer;g.sP.v+=attack.spring;
    if(phase==="wind"&&attack.next==="lunge"){
      g.speed=c0.lungeSpeed;g.yaw=facing;
    }
    if(phase==="lunge"){
      Z=true;g.maxSpeed=c0.lungeSpeed;g.speed=c0.lungeSpeed;
      if(attack.hit){
        C.damage(c0.lungeDamage,g.pos);
        s.events.emit("enemy:melee",{enemy:g,pos:g.pos});
        g.sP.v-=4;
      }
      if(attack.next==="recover"){g.lungeCd=c0.lungeCooldown;g.speed*=.3;}
    }
    g.es=attack.next;
    if(attack.request&&n.requestToken(g)){
      g.es="tele";g.esT=c0.telegraph;
      s.events.emit("enemy:telegraph",{enemy:g,type:"hound",time:c0.telegraph,pos:g.pos});
    }
    if(attack.fire)R(C);
    if(attack.finished){
      n.releaseToken(g);
      g.cool=c0.cooldown[0]+o()*(c0.cooldown[1]-c0.cooldown[0]);
      g.bursts++;
      if(g.bursts>=1+(o()<.4?1:0))w();
      else{g.es="cool";g.esT=0;}
    }else if(attack.replan)w();
  }
}${houndAttack}`,
    ],
    [
      houndGround + ",z(k)",
      `if(s.physics.gameplay){
  const motor=s.physics.gameplay.houndGround(g,C,Q,n.list,c0,k,Z);
  if(motor.replan){
    if(g.mode==="engage"&&g.es==="move")w();
    else if(g.mode==="patrol")g.hasGoal=false;
  }
  g.pos.set(motor.x,motor.y,motor.z);
}else{
  ${houndGround};
}
z(k)`,
    ],
    [
      follow,
      "g.planT-=k;if(g.planT<=0)_();if(s.physics.gameplay)return s.physics.gameplay.houndFollow(g,C);let A=g.goal;",
    ],
    [
      houndPlan + ",",
      `if(s.physics.gameplay)s.physics.gameplay.houndPlan(g,k,c0,n.list,s.level?.coverPoints,s.physics.nav,o);
else{${houndPlan};}`,
    ],
    [
      boltContact,
      `C&&C.alive!==!1&&(W=n.physics.gameplay
?n.physics.gameplay.boltHits(H,Q,C)
:(JS.copy(H.pos).addScaledVector(H.dir,Q),nT.set(C.pos.x,C.pos.y+.35,C.pos.z),
iT.set(C.pos.x,C.pos.y+(C.crouching?1:1.5),C.pos.z),jD(H.pos,JS,nT,iT)<.36*.36))`,
    ],
    [
      "H.life-=k;let Z=n.physics.raycast(H.pos,H.dir,N)",
      "const motion=n.physics.gameplay?.boltMotion(H,N,k);H.life=motion?.life??H.life-k;let Z=n.physics.raycast(H.pos,H.dir,N)",
    ],
    [
      boltMotion,
      `if(motion){H.pos.set(motion.x,motion.y,motion.z);H.len=motion.length;}
else{H.pos.addScaledVector(H.dir,N);H.len=Math.min(1.5,H.len+N);}`,
    ],
    [
      chainFuse,
      `if(n.physics.gameplay)b.fuse=n.physics.gameplay.chainFuse(b.fuse,g,s());
else{${chainFuse};}`,
    ],
    [
      playerSpread + ",",
      `if(n.physics.gameplay){
  n.physics.gameplay.playerSpread(C,E.spread,r);
  C.applyQuaternion(X.quaternion);
}else{${playerSpread};}`,
    ],
    [
      weaponSwitch,
      `function k(B){
const gameplay=n.physics.gameplay;
if(gameplay){
  const [time,action]=gameplay.switchPhase(u,oa,B);
  u.fresh=false;
  if(action===1){
    const previous=u.from;
    p.blend(.13);_(u.to);
    u.phase="draw";u.t=time;
    n.events.emit("weapon:switch",{from:previous.id,to:b.id,phase:"draw"});
    a.hol=1-oy(u.t/oa.draw,LS);
  }else{
    u.t=time;
    if(action===2){u.active=false;E.pending=null;a.hol=0;}
    else a.hol=u.phase==="holster"?CE(u.t/oa.holster):1-oy(u.t/oa.draw,LS);
  }
  return;
}
if(u.fresh`,
    ],
    [
      fuseDamage,
      `let lit=false;
const native=n.physics.gameplay?.fuseDamage(v,y);
if(native){
  [v.hp,v.fuse,lit]=native;
}else{
  v.hp-=y;
  if(v.hp<=0){v.fuse=v.fuse<0?.06:Math.min(v.fuse,.06);return;}
  if(v.fuse<0){v.fuse=v.spec.fuse;lit=true;}
}
lit&&(`,
    ],
    [
      fuseStep,
      `let spark=false;
const native=n.physics.gameplay?.fuseStep(b,v);
if(native){
  let action;
  [b.fuse,b.tick,action]=native;
  if(action===1){m(b);continue;}
  spark=action===2;
}else{
  b.fuse-=v;
  if(b.fuse<=0){m(b);continue;}
  b.tick-=v;
  if(b.tick<=0){b.tick=.07;spark=true;}
}
if(spark){`,
    ],
    [
      relaySelect,
      `${relaySelect}
const gameplay=n.physics.gameplay;
if(gameplay){
  for(const relay of g.relays){
    if(relay.cut)continue;
    const distance=gameplay.relayDistance(i,relay.o.pos,he,xi.interactFacing);
    if(distance>=0){Y=relay;he=distance;}
  }
  return Y;
}`,
    ],
    [
      relayCut,
      "ne.t=n.physics.gameplay?.cutProgress(ne.t,Y,xi.cutSeconds,true)??(ne.t+Y/xi.cutSeconds)",
    ],
    [
      relayRelease,
      "ne.t=n.physics.gameplay?.cutProgress(ne.t,Y,xi.cutSeconds,false)??Math.max(0,ne.t-Y*1.6)",
    ],
    [
      extraction,
      "let ye=n.physics.gameplay?.extractionRange(i.pos,V)??(Math.hypot(V.pos.x-i.pos.x,V.pos.z-i.pos.z)<(V.radius||2.5)&&Math.abs(V.pos.y-i.pos.y)<3.5);",
    ],
    [
      rating,
      `n.physics.gameplay?.rating(g.time,Y.shots,Y.hits,g.retries)??(${rating})`,
    ],
    [
      reinforcement,
      `function $(Y){
const gameplay=n.physics.gameplay;
if(!gameplay)return updateLegacyReinforcements(Y);
const alive=se();
for(let index=0;index<g.queue.length;index++){
  const entry=g.queue[index];
  const slot=gameplay.queueSlot(g.time,entry.at,alive,xi.maxAlive);
  if(!slot)continue;
  if(slot===1){entry.at=g.time+1;continue;}
  const spawned=!!q(entry.type,entry.pos,{state:"alert",target:i.pos});
  const stub=!!n.enemies.stub;
  if(!spawned&&!stub)entry.tries=(entry.tries||0)+1;
  if(gameplay.queueRetry(entry.tries||0,Number(spawned),Number(stub))){
    entry.at=g.time+1.5;
    continue;
  }
  g.queue.splice(index,1);
  a(n.enemies,"alert",i.pos,60);
  break;
}
if(g.allCut){
  const [delay,left,spawn,rally]=gameplay.pursuit(g,xi,Y,alive);
  g.pursuitT=delay;g.pursuitLeft=left;
  if(spawn){
    const type=n.rng()<0.6?"gull":"hound";
    g.queue.push({type,pos:O(type),at:g.time+0.5});
  }
  if(rally)a(n.enemies,"alert",i.pos,90);
}
}
${reinforcement.replace("function $(Y)", "function updateLegacyReinforcements(Y)")}`,
    ],
    [
      spawn,
      `${spawn}
const gameplay=n.physics.gameplay;
if(gameplay){
  const flying=Y==="gull";
  const anchor=gameplay.chooseSpawn(j(Y),V,i.eye,he,g.used,g.time,ne,flying,n.rng);
  if(anchor)g.used.set(anchor,g.time);
  return gameplay.spawnPosition(anchor?.pos||V,!!anchor,flying,n.rng,K);
}`,
    ],
    [
      "function x(){let Y=++g.wave,he=u();",
      "function x(){let Y=++g.wave,he=u();const config=n.physics.gameplay?.holdoutConfig(Y);const scales=config?Array.from(config):null;",
    ],
    [
      "let ze=cs.hp(Y),Ge=cs.dmg(Y),_e=cs.cool(Y);",
      "let [ze,Ge,_e]=scales??[cs.hp(Y),cs.dmg(Y),cs.cool(Y)];",
    ],
    [
      "xi.maxAlive+=cs.extraAlive(Y);",
      "xi.maxAlive+=scales?.[3]??cs.extraAlive(Y);",
    ],
    [
      wave,
      `let ye=n.physics.gameplay?.waveTypes(Y,0,0,true);
if(!ye){
  ${wave.replace("ye=[];", "entries=[];").replaceAll("ye.push", "entries.push").replace("ye[Math", "entries[Math")}
  ye=entries;
}
const V=ye.length;`,
    ],
    ["Ue=cs.stagger(Y);", "Ue=scales?.[4]??cs.stagger(Y);"],
    [
      missionWave,
      `let he=xi.waves[Math.min(Y,xi.waves.length-1)];
let V=n.physics.gameplay?.waveTypes(Y,he.gull,he.hound,false);
if(!V){
  V=[];
  for(let gull=0;gull<he.gull;gull++)V.push("gull");
  for(let hound=0;hound<he.hound;hound++)V.splice(Math.min(V.length,1+hound*2),0,"hound");
}`,
    ],
    [
      "function y(Y){g.waveOn?",
      `function y(Y){
const gameplay=n.physics.gameplay;
if(gameplay){
  const [remaining,action]=gameplay.holdoutTick(g,g.waveOn&&!g.queue.length?se():0,Y);
  g.breakT=remaining;
  if(action===-1)v();
  else if(action===-2)x();
  else if(action>0)o.message(\`Wave \${g.wave+1} in \${action}\`,1);
  return;
}
g.waveOn?`,
    ],
    [
      "g.score+=(cs.killScore[Y&&Y.type]||100)*(1+.1*(g.wave-1))",
      "g.score+=n.physics.gameplay?.killScore(cs.killScore[Y&&Y.type]||100,g.wave)??(cs.killScore[Y&&Y.type]||100)*(1+.1*(g.wave-1))",
    ],
    [
      threat,
      `;let V;
if(n.physics.gameplay){
  const enemies=se();
  const [alert,hot,intensity]=n.physics.gameplay.intensity(g,enemies,Y,enemies>0&&fe(26));
  g.alertT=alert;g.waveHot=hot;V=intensity;
}else{
  ${threat.slice(1).replace("let V=", "V=")}
}`,
    ],
    [
      weaponTrigger,
      `const gameplay=n.physics?.gameplay;
if(gameplay){
  const [start,cooldown,queue,shots,dry,reloadDelay,period]=gameplay.trigger(a,he,V,B,ye,Te,ne);
  a.cooldown=start;
  a.fireQ=queue;
  for(let shot=0;shot<shots;shot++){
    Q();
    if(he.auto)a.cooldown+=period;
  }
  if(dry){
    if(he.auto)a.dryDone=true;
    n.events.emit("weapon:dry",{weapon:Y.id});
    a.bump.v-=.08;
    if(reloadDelay>=0)a.autoReload=reloadDelay;
  }
  a.cooldown=cooldown;
}else{${weaponTrigger.slice(0, -1)};}
`,
    ],
    [
      "Math.min(X.magSize-X.ammo,X.reserve)",
      "(n.physics?.gameplay?.reloadTransfer(X)??Math.min(X.magSize-X.ammo,X.reserve))",
    ],
    [
      "return a.reload||u.active||B.ammo>=B.magSize||B.reserve<=0?",
      "return !(n.physics?.gameplay?.reloadAllowed(B,a.reload,u.active)??(!a.reload&&!u.active&&B.ammo<B.magSize&&B.reserve>0))?",
    ],
    [
      "l.reloadT!=null?$e.t=l.reloadT:$e.fresh?$e.fresh=!1:$e.t+=B",
      `n.physics?.gameplay?($e.t=n.physics.gameplay.reloadTime($e,B,l.reloadT),l.reloadT==null&&($e.fresh=false))
:l.reloadT!=null?$e.t=l.reloadT:$e.fresh?$e.fresh=!1:$e.t+=B`,
    ],
    [
      token,
      `const gameplay=n.physics.gameplay;
if(gameplay){
  if(!gameplay.attackSlot(f,k,C,Bi)){
    f.wantT[k.type]=f.t;
    return false;
  }
  f.tokens++;
  f.tokenT=f.t;
  f.lastType=k.type;
  k.hasToken=true;
  return true;
}
${token}`,
    ],
    [
      awareness,
      `think(k,C,A){
const gameplay=n.physics.gameplay;
if(gameplay){
  const player=n.player;
  if(k.passive||!player){k.los=false;return;}
  k.senseT-=C;
  if(k.senseT<=0){
    k.senseT=.11+e()*.08;
    const eye=player.eye||player.pos;
    k.los=player.alive!==false&&gameplay.canSee(k,eye,Bi.sightRange,$D)&&n.physics.lineOfSight(k.sensor,eye);
  }
  const [seen,hidden,exposure,remaining,transition]=gameplay.awareness(k,C,A,Bi,player);
  k.seeT=seen;k.noSeeT=hidden;k.exposure=exposure;k.modeT=remaining;
  if(k.los)k.lastKnown.copy(player.pos);
  if(transition===1){
    k.mode="search";
    k.modeT=Bi.searchTime+e()*3;
    f.releaseToken(k);
  }else if(transition===2){
    k.mode="engage";
    n.events.emit("enemy:alert",{enemy:k});
    f.rally(k.pos,18,player.pos,k);
  }else if(transition===3){
    k.mode="patrol";k.home.x=k.pos.x;k.home.z=k.pos.z;
  }
  return;
}
${awareness.slice("think(k,C,A){".length)}`,
    ],
  ];
}
