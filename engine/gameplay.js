const gullPhases = ["idle", "tele", "burst"];
const houndPhases = [
  "move",
  "aim",
  "cool",
  "tele",
  "burst",
  "wind",
  "lunge",
  "recover",
];

export function createGameplay(wasm) {
  const read = (address, length) =>
    new Float64Array(wasm.memory.buffer, address, length);
  function assignHit(address, target, enemy) {
    if (!address) return false;
    const [distance, x, y, z, nx, ny, nz, weak] = read(address, 8);
    target.dist = distance;
    target.point.set(x, y, z);
    target.normal.set(nx, ny, nz);
    target.enemy = enemy;
    target.part = weak ? "weak" : "body";
    return true;
  }

  return {
    playerInput(
      player,
      input,
      tune,
      dt,
      coyote,
      bufferedJump,
      radius,
      jumpSpeed,
    ) {
      const state = read(wasm.player_input_buffer(), 32);
      state.set([
        dt,
        player.pos.x,
        player.pos.y,
        player.pos.z,
        player.yaw,
        player.pitch,
        player.vel.x,
        player.vel.y,
        player.vel.z,
        player.speed,
        player.ads,
        coyote,
        bufferedJump,
        radius,
        tune.adsLook,
        tune.pitchLimit,
        input.look.dx,
        input.look.dy,
        input.move.x,
        input.move.y,
        tune.walk,
        tune.sprint,
        tune.crouch,
        tune.ads,
        tune.accelGround,
        tune.decelGround,
        tune.accelAir,
        tune.coyote,
        tune.jumpBuffer,
        tune.standHeight,
        tune.crouchHeight,
        jumpSpeed,
      ]);
      const flags =
        Number(!!player.enabled) |
        (Number(!!player.onGround) << 1) |
        (Number(!!player.crouching) << 2) |
        (Number(!!player.sprinting) << 3) |
        (Number(!!input.crouch) << 4) |
        (Number(!!input.sprint) << 5) |
        (Number(!!input.fire) << 6) |
        (Number(player.enabled && !!input.pressed("jump")) << 7);
      const [
        yaw,
        pitch,
        lookScale,
        vx,
        vy,
        vz,
        grace,
        buffer,
        height,
        sin,
        cos,
        grounded,
        crouching,
        sprinting,
        jump,
      ] = read(wasm.player_input_step(flags), 15);
      player.yaw = yaw;
      player.pitch = pitch;
      player.lookScale = lookScale;
      player.vel.set(vx, vy, vz);
      player.onGround = !!grounded;
      player.crouching = !!crouching;
      player.sprinting = !!sprinting;
      return { grace, buffer, height, sin, cos, jump: !!jump };
    },
    landing(player, dt, grounded, airTime, peakFall, fallVelocity, dip) {
      return read(
        wasm.player_landing(
          Number(player.onGround),
          Number(grounded),
          dt,
          airTime,
          peakFall,
          fallVelocity,
          player.vel.y,
          dip,
        ),
        6,
      );
    },
    gait(player, tune, dt, phase, swing, roll, cos, sin) {
      const stride = player.crouching
        ? tune.stride.crouch
        : player.sprinting
          ? tune.stride.sprint
          : tune.stride.walk;
      return read(
        wasm.player_gait(
          dt,
          player.vel.x,
          player.vel.z,
          Number(player.onGround),
          Number(player.sprinting),
          Number(player.crouching),
          tune.walk,
          tune.sprint,
          stride,
          phase,
          swing,
          roll,
          player.ads,
          tune.bobAds,
          tune.bobSprint,
          tune.bobY,
          tune.bobX,
          tune.rollStrafe,
          cos,
          sin,
        ),
        7,
      );
    },
    damage(health, pulse, amount) {
      return read(wasm.player_damage(health, pulse, amount), 3);
    },
    regenerate: wasm.player_regenerate,
    hitsPlayer(origin, direction, player, limit) {
      return !!wasm.combat_hits_player(
        origin.x,
        origin.y,
        origin.z,
        direction.x,
        direction.y,
        direction.z,
        player.x,
        player.y,
        player.z,
        limit,
      );
    },
    playerSpread(target, spread, random) {
      const [x, y, z] = read(
        wasm.combat_player_spread(spread, random(), random()),
        3,
      );
      target.set(x, y, z);
    },
    enemySpread(direction, tune, enemy, movement, instability, random) {
      const { x, y, z } = direction;
      const [dx, dy, dz] = read(
        wasm.combat_enemy_spread(
          x,
          y,
          z,
          tune.spreadFar,
          tune.spreadNear,
          enemy.exposure,
          movement,
          instability,
          random(),
          random(),
          random(),
        ),
        3,
      );
      direction.set(dx, dy, dz);
    },
    hitDamage(damage, weak) {
      return wasm.combat_hit_damage(damage, Number(weak));
    },
    blastDamage(origin, target, height, radius, damage) {
      return wasm.combat_blast_damage(
        origin.x,
        origin.y,
        origin.z,
        target.x,
        target.y + height,
        target.z,
        radius,
        damage,
      );
    },
    hitGull(origin, direction, enemy, range, target) {
      return assignHit(
        wasm.combat_hit_gull(
          origin.x,
          origin.y,
          origin.z,
          direction.x,
          direction.y,
          direction.z,
          enemy.pos.x,
          enemy.pos.y,
          enemy.pos.z,
          range,
          target.dist,
        ),
        target,
        enemy,
      );
    },
    hitHound(origin, direction, enemy, hip, range, target) {
      const { headPos, pos, yaw } = enemy;
      return assignHit(
        wasm.combat_hit_hound(
          origin.x,
          origin.y,
          origin.z,
          direction.x,
          direction.y,
          direction.z,
          pos.x,
          pos.y,
          pos.z,
          headPos.x,
          headPos.y,
          headPos.z,
          yaw,
          hip,
          range,
          target.dist,
        ),
        target,
        enemy,
      );
    },
    boltHits(bolt, travel, player) {
      const { pos, crouching } = player;
      return !!wasm.combat_bolt_hits(
        bolt.pos.x,
        bolt.pos.y,
        bolt.pos.z,
        bolt.dir.x,
        bolt.dir.y,
        bolt.dir.z,
        travel,
        pos.x,
        pos.y,
        pos.z,
        Number(crouching),
      );
    },
    boltMotion(bolt, distance, dt) {
      const { pos, dir } = bolt;
      const [x, y, z, life, length] = read(
        wasm.combat_bolt_motion(
          pos.x,
          pos.y,
          pos.z,
          dir.x,
          dir.y,
          dir.z,
          bolt.life,
          bolt.len,
          distance,
          dt,
        ),
        5,
      );
      return { x, y, z, life, length };
    },
    fuseDamage(item, damage) {
      return read(
        wasm.combat_fuse_damage(item.hp, item.fuse, item.spec.fuse, damage),
        3,
      );
    },
    fuseStep(item, dt) {
      return read(wasm.combat_fuse_step(item.fuse, item.tick, dt), 3);
    },
    chainFuse: wasm.combat_chain_fuse,
    relayDistance(player, position, range, facing) {
      return wasm.mission_relay_distance(
        player.pos.x,
        player.pos.y,
        player.pos.z,
        player.forward.x,
        player.forward.z,
        position.x,
        position.y || 0,
        position.z,
        range,
        facing,
      );
    },
    cutProgress(progress, dt, seconds, active) {
      return wasm.mission_cut_progress(progress, dt, seconds, Number(active));
    },
    extractionRange(player, target) {
      const { pos, radius } = target;
      return !!wasm.mission_extraction_range(
        player.x,
        player.y,
        player.z,
        pos.x,
        pos.y,
        pos.z,
        radius || 2.5,
      );
    },
    rating: wasm.mission_rating,
    holdoutConfig(wave) {
      return read(wasm.mission_holdout_config(wave), 5);
    },
    waveTypes(wave, gulls, hounds, holdout) {
      const count = wasm.mission_wave_types(
        wave,
        gulls,
        hounds,
        Number(holdout),
      );
      return Array.from(read(wasm.mission_wave_members(), count), (type) =>
        type ? "hound" : "gull",
      );
    },
    chooseSpawn(
      candidates,
      from,
      eye,
      distance,
      used,
      now,
      preferHidden,
      flying,
      random,
    ) {
      const table = read(
        wasm.mission_candidates(candidates.length),
        candidates.length * 5,
      );
      for (const [index, candidate] of candidates.entries()) {
        const { x, y, z } = candidate.pos;
        table.set(
          [x, y, z, random(), used.get(candidate) ?? -Infinity],
          index * 5,
        );
      }
      const index = wasm.mission_pick_spawn(
        from.x,
        from.z,
        eye.x,
        eye.y,
        eye.z,
        distance,
        now,
        Number(preferHidden),
        Number(flying),
      );
      return candidates[index];
    },
    spawnPosition(position, anchored, flying, random, Vector) {
      const offsetX = random();
      const offsetZ = anchored ? random() : 0;
      const [x, y, z] = read(
        wasm.mission_spawn_position(
          position.x,
          position.y,
          position.z,
          offsetX,
          offsetZ,
          Number(anchored),
          Number(flying),
        ),
        3,
      );
      return new Vector(x, flying && y < 2.5 ? 6 + random() * 3 : y, z);
    },
    holdoutTick(run, enemies, dt) {
      return read(
        wasm.mission_holdout_tick(
          Number(run.waveOn),
          Number(!!run.queue.length),
          enemies,
          run.wave,
          run.breakT,
          dt,
        ),
        2,
      );
    },
    intensity(run, enemies, dt, nearby) {
      return read(
        wasm.mission_intensity(
          dt,
          run.alertT,
          run.waveHot,
          enemies,
          Number(!!run.queue.length),
          Number(run.allCut),
          Number(nearby),
        ),
        3,
      );
    },
    killScore: wasm.mission_kill_score,
    queueSlot: wasm.mission_queue_slot,
    queueRetry: wasm.mission_queue_retry,
    pursuit(run, tune, dt, alive) {
      return read(
        wasm.mission_pursuit(
          dt,
          run.pursuitT,
          run.pursuitLeft,
          tune.pursuitEvery,
          alive,
          tune.pursuitMinAlive,
          Number(!!run.queue.length),
        ),
        4,
      );
    },
    reloadAllowed(slot, reloading, switching) {
      return !!wasm.weapon_reload_allowed(
        slot.ammo,
        slot.reserve,
        slot.magSize,
        Number(!!reloading),
        Number(switching),
      );
    },
    reloadTransfer(slot) {
      return wasm.weapon_reload_transfer(slot.ammo, slot.reserve, slot.magSize);
    },
    reloadTime(reload, delta, forced) {
      return wasm.weapon_reload_time(
        reload.t,
        delta,
        Number(reload.fresh),
        forced ?? NaN,
      );
    },
    switchPhase(switching, tune, dt) {
      return read(
        wasm.weapon_switch(
          switching.t,
          dt,
          tune.holster,
          tune.draw,
          Number(switching.fresh),
          Number(switching.phase === "holster"),
        ),
        2,
      );
    },
    trigger(state, weapon, slot, dt, pressed, edge, switching) {
      const ready = !state.reload && !switching && state.sprintT < 0.45;
      const flags =
        Number(!!weapon.auto) |
        (Number(pressed) << 1) |
        (Number(edge) << 2) |
        (Number(ready) << 3) |
        (Number(state.dryDone) << 4);
      return read(
        wasm.weapon_trigger(
          dt,
          state.cooldown,
          state.fireQ,
          weapon.rpm,
          slot.ammo,
          slot.reserve,
          flags,
        ),
        7,
      );
    },
    attackSlot(group, enemy, count, tuning) {
      const other = enemy.type === "gull" ? "hound" : "gull";
      return !!wasm.enemy_attack_slot(
        count,
        group.tokens,
        tuning.maxTokens,
        tuning.maxTokensCrowd,
        tuning.tokenGap,
        group.t - group.tokenT,
        group.t - group.wantT[other],
        Number(group.lastType === enemy.type),
      );
    },
    canSee(enemy, target, range, field) {
      const { sensor, fwd } = enemy;
      return !!wasm.enemy_can_see(
        target.x - sensor.x,
        target.y - sensor.y,
        target.z - sensor.z,
        fwd.x,
        fwd.z,
        range,
        field,
        Number(enemy.mode === "engage"),
      );
    },
    awareness(enemy, dt, threshold, tune, player) {
      const mode =
        enemy.mode === "engage" ? 2 : enemy.mode === "patrol" ? 0 : 1;
      return read(
        wasm.enemy_awareness(
          dt,
          enemy.seeT,
          enemy.noSeeT,
          enemy.exposure,
          enemy.modeT,
          threshold,
          tune.loseSight,
          tune.exposureUp,
          tune.exposureDown,
          mode,
          Number(enemy.los),
          Number(player.alive !== false),
        ),
        5,
      );
    },
    fly(enemy, player, tune, dt, time, random) {
      const { pos, vel, avoid, home, lastKnown } = enemy;
      const target = player?.pos;
      const engaged = enemy.mode === "engage";
      const turnAround = engaged && player && enemy.flipT - dt <= 0;
      const input = read(wasm.gull_flight_input(), 42);
      input.set([
        dt,
        time,
        pos.x,
        pos.y,
        pos.z,
        vel.x,
        vel.y,
        vel.z,
        avoid.x,
        avoid.y,
        avoid.z,
        home.x,
        home.y,
        home.z,
        lastKnown.x,
        lastKnown.y,
        lastKnown.z,
        target?.x ?? 0,
        target?.y ?? 0,
        target?.z ?? 0,
        enemy.yaw,
        enemy.pitch,
        enemy.roll,
        enemy.phase,
        enemy.ceil,
        enemy.ceilT,
        enemy.flipT,
        enemy.orbitDir,
        enemy.orbitR,
        enemy.orbitA,
        enemy.feeler,
        enemy.noSeeT,
        enemy.alt,
        enemy.patrolR,
        tune.patrolSpeed,
        tune.maxSpeed,
        tune.accel,
        tune.rangeMin,
        tune.rangeMax,
        turnAround ? random() : 0,
        turnAround ? random() : 0,
        turnAround ? random() : 0,
      ]);
      const flags =
        Number(engaged) |
        (Number(enemy.mode === "investigate" || enemy.mode === "search") << 1) |
        (Number(enemy.hp < tune.hp * 0.5) << 2) |
        (Number(!!player) << 3) |
        (Number(!!enemy.hoverFixed) << 4) |
        (Number(enemy.atk !== "idle") << 5);
      const [
        x,
        y,
        z,
        vx,
        vy,
        vz,
        ax,
        ay,
        az,
        yaw,
        pitch,
        roll,
        ceiling,
        ceilingTime,
        flipTime,
        orbitDirection,
        orbitRadius,
        orbitPhase,
        feeler,
        accelerationY,
        speed,
        turn,
        cos,
        sin,
      ] = read(wasm.gull_flight_step(flags), 24);
      pos.set(x, y, z);
      vel.set(vx, vy, vz);
      avoid.set(ax, ay, az);
      enemy.yaw = yaw;
      enemy.pitch = pitch;
      enemy.roll = roll;
      enemy.ceil = ceiling;
      enemy.ceilT = ceilingTime;
      enemy.flipT = flipTime;
      enemy.orbitDir = orbitDirection;
      enemy.orbitR = orbitRadius;
      enemy.orbitA = orbitPhase;
      enemy.feeler = feeler;
      return { accelerationY, speed, turn, cos, sin };
    },
    houndGround(enemy, player, distance, group, tune, dt, lunging) {
      const { pos, push } = enemy;
      const target = player?.pos;
      const neighbors = group.filter(
        (other) => other !== enemy && other.type === "hound" && other.active,
      );
      const nearby = read(
        wasm.hound_neighbors(neighbors.length),
        neighbors.length * 2,
      );
      for (let index = 0; index < neighbors.length; index++) {
        nearby[index * 2] = neighbors[index].pos.x;
        nearby[index * 2 + 1] = neighbors[index].pos.z;
      }
      const input = read(wasm.hound_ground_input(), 22);
      input.set([
        dt,
        pos.x,
        pos.y,
        pos.z,
        enemy.yaw,
        enemy.wantYaw,
        enemy.yawRate,
        enemy.speed,
        enemy.maxSpeed,
        tune.turnRate,
        tune.accel,
        push.x,
        push.y,
        push.z,
        target?.x ?? 0,
        target?.z ?? 0,
        distance,
        enemy.feelT,
        enemy.stuckT,
        enemy.planT,
        enemy.phase,
        enemy.esT,
      ]);
      const flags =
        Number(lunging) |
        (Number(enemy.blocked) << 1) |
        (Number(!!player) << 2);
      const [
        x,
        y,
        z,
        yaw,
        yawRate,
        speed,
        previousSpeed,
        pushX,
        pushY,
        pushZ,
        feeler,
        blocked,
        stuckTime,
        planTime,
        replan,
      ] = read(wasm.hound_ground_step(flags), 15);
      enemy.yaw = yaw;
      enemy.yawRate = yawRate;
      enemy.speed = speed;
      enemy.prevSpeed = previousSpeed;
      push.set(pushX, pushY, pushZ);
      enemy.feelT = feeler;
      enemy.blocked = !!blocked;
      enemy.stuckT = stuckTime;
      enemy.planT = planTime;
      return { x, y, z, replan: !!replan };
    },
    houndFollow(enemy, speed) {
      const count = enemy.path?.length ?? 0;
      const points = read(wasm.hound_waypoints(count), count * 2);
      for (let index = 0; index < count; index++) {
        points[index * 2] = enemy.path[index].x;
        points[index * 2 + 1] = enemy.path[index].z;
      }
      const { pos, goal } = enemy;
      const [waypoint, yaw, maxSpeed, reached] = read(
        wasm.hound_follow(
          pos.x,
          pos.z,
          enemy.yaw,
          goal.x,
          goal.z,
          enemy.wp,
          speed,
        ),
        4,
      );
      enemy.wp = waypoint;
      enemy.maxSpeed = maxSpeed;
      if (reached) enemy.hasGoal = false;
      else enemy.wantYaw = yaw;
      return !!reached;
    },
    gullAttack(enemy, player, tune, dt, turn, engaged) {
      const canShoot = !!player?.alive && enemy.los;
      const distance =
        engaged && canShoot && enemy.atk === "idle"
          ? enemy.pos.distanceTo(player.pos)
          : Infinity;
      const [cool, timer, flare, shots, phase, action] = read(
        wasm.gull_attack(
          dt,
          enemy.cool,
          enemy.atkT,
          enemy.flare,
          enemy.shots,
          tune.burst,
          tune.burstGap,
          turn,
          distance,
          tune.fireRange,
          gullPhases.indexOf(enemy.atk),
          Number(engaged),
          Number(canShoot),
        ),
        6,
      );
      enemy.cool = cool;
      enemy.atkT = timer;
      enemy.flare = flare;
      enemy.shots = shots;
      enemy.atk = gullPhases[phase];
      return action;
    },
    houndCanLunge(enemy, player, distance, range) {
      return !!wasm.hound_can_lunge(
        houndPhases.indexOf(enemy.es),
        distance,
        range,
        enemy.lungeCd,
        Number(!!player.alive),
      );
    },
    houndPlan(enemy, player, tune, others, cover, nav, random) {
      const allies = others.filter(
        (other) =>
          other !== enemy &&
          other.alive &&
          other.type === "hound" &&
          other.mode === "engage",
      );
      const { pos } = enemy;
      const target = player.pos;
      const eye = player.eye || target;
      const positions = read(
        wasm.hound_plan(
          pos.x,
          pos.y,
          pos.z,
          target.x,
          target.y,
          target.z,
          eye.x,
          eye.y,
          eye.z,
          tune.rangeMin,
          tune.rangeMax,
          allies.length,
        ),
        allies.length * 2,
      );
      for (let i = 0; i < allies.length; i++) {
        const ally = allies[i];
        const heading = ally.es === "move" ? ally.firePos : ally.pos;
        positions[i * 2] = heading.x;
        positions[i * 2 + 1] = heading.z;
      }
      const consider = (x, y, z, bias) => {
        if (wasm.hound_can_fire_from(x, z))
          wasm.hound_evaluate_position(x, y, z, bias, random());
      };
      for (const point of cover ?? []) consider(point.x, point.y, point.z, -2);
      for (let i = 0; i < 7; i++) {
        const point = nav.randomPoint(target, tune.rangeMax);
        if (point) consider(point.x, point.y, point.z, 0);
      }
      for (let i = 0; i < 5; i++) {
        const [x, y, z] = read(wasm.hound_arc_point(random(), random()), 3);
        consider(x, y, z, 1);
      }
      const [x, y, z] = read(wasm.hound_fire_position(), 3);
      enemy.firePos.set(x, y, z);
    },
    houndMoveChance(enemy, distance, tune) {
      return !!wasm.hound_move_chance(
        enemy.moveT,
        distance,
        tune.rangeMin,
        tune.rangeMax,
        Number(enemy.los),
      );
    },
    houndAttack(enemy, player, tune, dt, distance, yaw) {
      const [
        timer,
        flare,
        shots,
        phase,
        crouch,
        spring,
        request,
        replan,
        fire,
        flags,
      ] = read(
        wasm.hound_attack(
          dt,
          distance,
          tune.rangeMin,
          enemy.cool,
          enemy.noSeeT,
          yaw - enemy.yaw,
          enemy.tyaw,
          enemy.esT,
          enemy.flare,
          enemy.shots,
          tune.burst,
          tune.burstGap,
          houndPhases.indexOf(enemy.es),
          Number(enemy.los),
          Number(!!player.alive),
        ),
        10,
      );
      enemy.flare = flare;
      enemy.shots = shots;
      const next = houndPhases[phase];
      return {
        timer,
        next,
        crouch,
        spring,
        request,
        replan,
        fire,
        hit: !!(flags & 1),
        finished: !!(flags & 2),
      };
    },
  };
}
