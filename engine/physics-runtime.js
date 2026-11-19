import { createGameplay } from "./gameplay.js";

const BULLET_ONLY = 1;
const PLAYER_ONLY = 2;
const NO_NAV = 4;
const WATER = -2;

export function createPhysicsWorld(wasm, config, adapters) {
  const {
    width,
    height,
    minX,
    minZ,
    cell,
    padding,
    quay,
    ground,
    sea,
    stepHeight,
  } = config;
  const { Vector, toVector, bounds, groundSurface, random } = adapters;
  wasm.physics_init(
    width,
    height,
    minX,
    minZ,
    cell,
    padding,
    quay,
    ground,
    sea,
    stepHeight,
  );

  const boxes = [];
  let gridSnapshot;
  const read = (address, length) =>
    new Float64Array(wasm.memory.buffer, address, length);

  function registerBox({ min, max, bulletOnly, playerOnly, noNav }) {
    const flags =
      (bulletOnly ? BULLET_ONLY : 0) |
      (playerOnly ? PLAYER_ONLY : 0) |
      (noNav ? NO_NAV : 0);
    wasm.physics_add_box(min.x, min.y, min.z, max.x, max.y, max.z, flags);
  }

  function applyMovement(position, velocity, result) {
    const [x, y, z, vx, vy, vz, onGround, hitWall, hitCeil, box] = result;
    position.x = x;
    position.y = y;
    position.z = z;
    velocity.x = vx;
    velocity.y = vy;
    velocity.z = vz;
    const surface = onGround
      ? boxes[box]?.surface || groundSurface(x, z)
      : "concrete";
    return {
      onGround: !!onGround,
      hitWall: !!hitWall,
      hitCeil: !!hitCeil,
      surface,
    };
  }

  return {
    boxes,
    bounds,
    gameplay: createGameplay(wasm),
    addBox(a, b, options = {}) {
      const from = toVector(a);
      const to = toVector(b);
      const box = {
        min: new Vector(
          Math.min(from.x, to.x),
          Math.min(from.y, to.y),
          Math.min(from.z, to.z),
        ),
        max: new Vector(
          Math.max(from.x, to.x),
          Math.max(from.y, to.y),
          Math.max(from.z, to.z),
        ),
        surface: options.surface || "concrete",
        tag: options.tag || "",
        noNav: !!options.noNav,
        bulletOnly: !!options.bulletOnly,
        playerOnly: !!options.playerOnly,
      };
      registerBox(box);
      boxes.push(box);
      gridSnapshot = undefined;
      return box;
    },
    removeBox(box) {
      const index = boxes.indexOf(box);
      if (index < 0) return;
      wasm.physics_remove_box(index);
      boxes.splice(index, 1);
      gridSnapshot = undefined;
    },
    build() {
      // Synchronize any edits made directly to a collider since registration.
      wasm.physics_clear_boxes();
      for (const box of boxes) registerBox(box);
      gridSnapshot = undefined;
      wasm.nav_grid();
    },
    groundHeight(x, z, ceiling = 50) {
      return wasm.physics_ground_height(x, z, ceiling);
    },
    raycast(origin, direction, maxDistance = 500) {
      const address = wasm.physics_raycast(
        origin.x,
        origin.y,
        origin.z,
        direction.x,
        direction.y,
        direction.z,
        maxDistance,
      );
      if (!address) return null;
      const [dist, x, y, z, nx, ny, nz, index] = read(address, 8);
      const box = boxes[index] ?? null;
      return {
        dist,
        point: new Vector(x, y, z),
        normal: new Vector(nx, ny, nz),
        surface: box
          ? box.surface
          : index === WATER
            ? "water"
            : groundSurface(x, z),
        box,
      };
    },
    lineOfSight(a, b) {
      return !!wasm.physics_line_of_sight(a.x, a.y, a.z, b.x, b.y, b.z);
    },
    hasClearance(x, z, margin, minY, maxY) {
      return !!wasm.physics_has_clearance(x, z, margin, minY, maxY);
    },
    moveCapsule(
      position,
      velocity,
      time,
      radius = 0.32,
      height = 1.8,
      stepping = true,
    ) {
      const address = wasm.physics_move_capsule(
        position.x,
        position.y,
        position.z,
        velocity.x,
        velocity.y,
        velocity.z,
        time,
        radius,
        height,
        Number(!!stepping),
      );
      return applyMovement(position, velocity, read(address, 10));
    },
    movePlayer(
      position,
      velocity,
      time,
      radius,
      height,
      stepping,
      gravity,
      alive,
    ) {
      const address = wasm.physics_move_player(
        position.x,
        position.y,
        position.z,
        velocity.x,
        velocity.y,
        velocity.z,
        time,
        radius,
        height,
        Number(!!stepping),
        gravity,
        Number(alive),
      );
      const result = read(address, 11);
      const fallVelocity = result[10];
      return { ...applyMovement(position, velocity, result), fallVelocity };
    },
    nav: {
      cell,
      isWalkable(x, z) {
        return !!wasm.nav_is_walkable(x, z);
      },
      findPath(a, b) {
        const count = wasm.nav_find_path(a.x, a.z, b.x, b.z);
        if (count === 0) return null;
        const points = read(wasm.nav_waypoints(), count * 2);
        const path = [];
        for (let i = 0; i < points.length; i += 2) {
          path.push(new Vector(points[i], 0, points[i + 1]));
        }
        return path;
      },
      clearLine(a, b) {
        return !!wasm.nav_clear_line(a.x, a.z, b.x, b.z);
      },
      randomPoint(position, radius) {
        // Keep the game's random source and consume two draws per attempted point.
        for (let i = 0; i < 24; i++) {
          const address = wasm.nav_sample_point(
            position.x,
            position.z,
            radius,
            random(),
            random(),
          );
          if (address) {
            const [x, z] = read(address, 2);
            return new Vector(x, 0, z);
          }
        }
        const [x, z] = read(wasm.nav_nearest_point(position.x, position.z), 2);
        return new Vector(x, 0, z);
      },
      _grid() {
        // The snapshot stays readable even when another kernel grows WASM memory.
        if (!gridSnapshot) {
          const address = wasm.nav_grid();
          gridSnapshot = new Uint8Array(
            wasm.memory.buffer,
            address,
            width * height,
          ).slice();
        }
        return { blocked: gridSnapshot, w: width, h: height, minX, minZ, cell };
      },
    },
  };
}
