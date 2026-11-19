use std::cell::RefCell;

use crate::navigation::{GridGeometry, Navigation};

type Point = [f64; 3];

pub(super) struct Collider {
    pub min: Point,
    pub max: Point,
    pub bullet_only: bool,
    pub no_nav: bool,
    player_only: bool,
}

impl Collider {
    fn overlaps_footprint(&self, x: f64, z: f64, margin: f64) -> bool {
        !(x + margin < self.min[0]
            || x - margin > self.max[0]
            || z + margin < self.min[2]
            || z - margin > self.max[2])
    }
}

#[derive(Default)]
struct World {
    boxes: Vec<Collider>,
    nav: Navigation,
    quay: f64,
    ground: f64,
    sea: f64,
    step_height: f64,
    hit: [f64; 8],
    movement: [f64; 11],
    nav_point: [f64; 2],
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
}

struct Hit {
    distance: f64,
    normal: Point,
    surface: Surface,
}

enum Surface {
    Box(usize),
    Ground,
    Water,
}

struct CapsuleMovement {
    position: Point,
    velocity: Point,
    on_ground: bool,
    hit_wall: bool,
    hit_ceiling: bool,
    floor_box: Option<usize>,
}

impl World {
    fn navigation(&mut self) -> &mut Navigation {
        self.nav.prepare(&self.boxes, self.quay, self.step_height);
        &mut self.nav
    }

    fn ground_height(&self, x: f64, z: f64, ceiling: f64) -> f64 {
        let mut height = if x <= self.quay {
            self.ground
        } else {
            self.sea
        };
        for aabb in &self.boxes {
            if !aabb.bullet_only
                && x >= aabb.min[0]
                && x <= aabb.max[0]
                && z >= aabb.min[2]
                && z <= aabb.max[2]
                && aabb.max[1] <= ceiling
                && aabb.max[1] > height
            {
                height = aabb.max[1];
            }
        }
        height
    }

    fn raycast(&self, origin: Point, direction: Point, mut closest: f64) -> Option<Hit> {
        let mut hit = None;
        if direction[1] < -1e-6 {
            let ground = (self.ground - origin[1]) / direction[1];
            if ground > 0.0 && ground < closest {
                let (distance, surface) = if origin[0] + direction[0] * ground <= self.quay {
                    (ground, Surface::Ground)
                } else {
                    ((self.sea - origin[1]) / direction[1], Surface::Water)
                };
                if distance > 0.0 && distance < closest {
                    closest = distance;
                    hit = Some(Hit {
                        distance,
                        normal: [0.0, 1.0, 0.0],
                        surface,
                    });
                }
            }
        }

        let inverse = direction.map(|value| 1.0 / value);
        'boxes: for (index, aabb) in self.boxes.iter().enumerate() {
            if aabb.player_only {
                continue;
            }
            let (mut entry, mut exit) = (0.0, closest);
            let (mut axis, mut sign) = (None, 0.0);
            for component in 0..3 {
                let origin = origin[component];
                let (min, max) = (aabb.min[component], aabb.max[component]);
                if direction[component] == 0.0 {
                    if origin < min || origin > max {
                        continue 'boxes;
                    }
                    continue;
                }
                let mut near = (min - origin) * inverse[component];
                let mut far = (max - origin) * inverse[component];
                let mut normal = -1.0;
                if near > far {
                    std::mem::swap(&mut near, &mut far);
                    normal = 1.0;
                }
                if near > entry {
                    entry = near;
                    axis = Some(component);
                    sign = normal;
                }
                if far < exit {
                    exit = far;
                }
                if entry > exit {
                    continue 'boxes;
                }
            }
            if let Some(axis) = axis {
                closest = entry;
                let mut normal = [0.0; 3];
                normal[axis] = sign;
                hit = Some(Hit {
                    distance: entry,
                    normal,
                    surface: Surface::Box(index),
                });
            }
        }
        hit
    }

    fn line_of_sight(&self, [x, y, z]: Point, [end_x, end_y, end_z]: Point) -> bool {
        let (dx, dy, dz) = (end_x - x, end_y - y, end_z - z);
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        if distance < 1e-4 {
            return true;
        }
        let inverse_distance = 1.0 / distance;
        let direction = [
            dx * inverse_distance,
            dy * inverse_distance,
            dz * inverse_distance,
        ];
        self.raycast([x, y, z], direction, distance - 0.05)
            .is_none()
    }

    fn move_capsule(
        &self,
        [mut x, original_y, mut z]: Point,
        [mut vx, mut vy, mut vz]: Point,
        time: f64,
        radius: f64,
        height: f64,
        stepping: bool,
    ) -> CapsuleMovement {
        let step_height = if stepping { self.step_height } else { 0.08 };
        let travel_x = vx * time;
        let travel_z = vz * time;
        // Normalize before squaring, as Math.hypot does for two components.
        let scale = travel_x.abs().max(travel_z.abs());
        let distance = if scale == 0.0 {
            0.0
        } else {
            let x = travel_x / scale;
            let z = travel_z / scale;
            (x * x + z * z).sqrt() * scale
        };
        let step_count = (distance / (radius * 0.6)).ceil();
        let steps = if step_count.is_finite() {
            step_count.max(1.0) as usize
        } else {
            0
        };
        let delta_x = travel_x / steps as f64;
        let delta_z = travel_z / steps as f64;
        let radius_sq = radius * radius;
        let mut hit_wall = false;
        for _ in 0..steps {
            x += delta_x;
            z += delta_z;
            for _ in 0..3 {
                let mut adjusted = false;
                for aabb in self.boxes.iter() {
                    if aabb.bullet_only
                        || aabb.max[1] <= original_y + step_height
                        || aabb.min[1] >= original_y + height
                    {
                        continue;
                    }
                    let mut dx = x - x.min(aabb.max[0]).max(aabb.min[0]);
                    let mut dz = z - z.min(aabb.max[2]).max(aabb.min[2]);
                    let distance_sq = dx * dx + dz * dz;
                    if distance_sq >= radius_sq {
                        continue;
                    }
                    if distance_sq > 1e-10 {
                        let distance = distance_sq.sqrt();
                        let overlap = radius - distance;
                        dx /= distance;
                        dz /= distance;
                        x += dx * overlap;
                        z += dz * overlap;
                        let projection = vx * dx + vz * dz;
                        if projection < 0.0 {
                            vx -= dx * projection;
                            vz -= dz * projection;
                        }
                    } else {
                        let (left, right, near, far) = (
                            x - aabb.min[0],
                            aabb.max[0] - x,
                            z - aabb.min[2],
                            aabb.max[2] - z,
                        );
                        let side = left.min(right).min(near).min(far);
                        if side == left {
                            x = aabb.min[0] - radius;
                            if vx > 0.0 {
                                vx = 0.0;
                            }
                        } else if side == right {
                            x = aabb.max[0] + radius;
                            if vx < 0.0 {
                                vx = 0.0;
                            }
                        } else if side == near {
                            z = aabb.min[2] - radius;
                            if vz > 0.0 {
                                vz = 0.0;
                            }
                        } else {
                            z = aabb.max[2] + radius;
                            if vz < 0.0 {
                                vz = 0.0;
                            }
                        }
                    }
                    adjusted = true;
                    hit_wall = true;
                }
                if !adjusted {
                    break;
                }
            }
        }

        let mut y = original_y + vy * time;
        let margin = radius * 0.7;
        let mut floor = if x <= self.quay + margin {
            self.ground
        } else {
            -50.0
        };
        let mut floor_box = None;
        let mut ceiling = f64::INFINITY;
        let max_floor = original_y + step_height + 1e-4;
        let min_ceiling = original_y + height - 0.05;
        for (index, aabb) in self.boxes.iter().enumerate() {
            if aabb.bullet_only || !aabb.overlaps_footprint(x, z, margin) {
                continue;
            }
            if aabb.max[1] <= max_floor && aabb.max[1] > floor {
                floor = aabb.max[1];
                floor_box = Some(index);
            }
            if aabb.min[1] >= min_ceiling && aabb.min[1] < ceiling {
                ceiling = aabb.min[1];
            }
        }
        let mut on_ground = false;
        let mut hit_ceiling = false;
        if vy > 0.0 && y + height > ceiling {
            y = ceiling - height;
            vy = 0.0;
            hit_ceiling = true;
        } else if y <= floor {
            y = floor;
            if vy < 0.0 {
                vy = 0.0;
            }
            on_ground = true;
        } else if stepping && vy <= 0.0 && y - floor < step_height {
            y = floor;
            vy = 0.0;
            on_ground = true;
        }
        CapsuleMovement {
            position: [x, y, z],
            velocity: [vx, vy, vz],
            on_ground,
            hit_wall,
            hit_ceiling,
            floor_box,
        }
    }

    fn save_movement(&mut self, movement: CapsuleMovement, fall_velocity: f64) -> *const f64 {
        let [x, y, z] = movement.position;
        let [vx, vy, vz] = movement.velocity;
        self.movement = [
            x,
            y,
            z,
            vx,
            vy,
            vz,
            movement.on_ground as u8 as f64,
            movement.hit_wall as u8 as f64,
            movement.hit_ceiling as u8 as f64,
            movement.floor_box.map_or(-1.0, |index| index as f64),
            fall_velocity,
        ];
        self.movement.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_init(
    width: usize,
    height: usize,
    min_x: f64,
    min_z: f64,
    cell: f64,
    padding: f64,
    quay: f64,
    ground: f64,
    sea: f64,
    step_height: f64,
) {
    let geometry = GridGeometry {
        width,
        height,
        min_x,
        min_z,
        cell,
        padding,
    };
    WORLD.with_borrow_mut(|world| {
        *world = World {
            nav: Navigation::new(geometry),
            quay,
            ground,
            sea,
            step_height,
            ..World::default()
        };
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_add_box(
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
    flags: u32,
) {
    WORLD.with_borrow_mut(|world| {
        world.boxes.push(Collider {
            min: [min_x, min_y, min_z],
            max: [max_x, max_y, max_z],
            bullet_only: flags & 1 != 0,
            player_only: flags & 2 != 0,
            no_nav: flags & 4 != 0,
        });
        world.nav.invalidate();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_remove_box(index: usize) {
    WORLD.with_borrow_mut(|world| {
        world.boxes.remove(index);
        world.nav.invalidate();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_clear_boxes() {
    WORLD.with_borrow_mut(|world| {
        world.boxes.clear();
        world.nav.invalidate();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_ground_height(x: f64, z: f64, ceiling: f64) -> f64 {
    WORLD.with_borrow(|world| world.ground_height(x, z, ceiling))
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_has_clearance(
    x: f64,
    z: f64,
    margin: f64,
    min_y: f64,
    max_y: f64,
) -> bool {
    WORLD.with_borrow(|world| {
        !world.boxes.iter().any(|aabb| {
            !aabb.bullet_only
                && aabb.overlaps_footprint(x, z, margin)
                && aabb.min[1] < max_y
                && aabb.max[1] > min_y
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_raycast(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    distance: f64,
) -> *const f64 {
    WORLD.with_borrow_mut(|world| {
        let Some(hit) = world.raycast([x, y, z], [dx, dy, dz], distance) else {
            return std::ptr::null();
        };
        let [nx, ny, nz] = hit.normal;
        let distance = hit.distance;
        let surface = match hit.surface {
            Surface::Box(index) => index as f64,
            Surface::Ground => -1.0,
            Surface::Water => -2.0,
        };
        world.hit = [
            distance,
            x + dx * distance,
            y + dy * distance,
            z + dz * distance,
            nx,
            ny,
            nz,
            surface,
        ];
        world.hit.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_line_of_sight(
    x: f64,
    y: f64,
    z: f64,
    end_x: f64,
    end_y: f64,
    end_z: f64,
) -> bool {
    WORLD.with_borrow(|world| world.line_of_sight([x, y, z], [end_x, end_y, end_z]))
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_move_capsule(
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    time: f64,
    radius: f64,
    height: f64,
    stepping: bool,
) -> *const f64 {
    WORLD.with_borrow_mut(|world| {
        let movement = world.move_capsule([x, y, z], [vx, vy, vz], time, radius, height, stepping);
        world.save_movement(movement, vy)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn physics_move_player(
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    time: f64,
    radius: f64,
    height: f64,
    stepping: bool,
    gravity: f64,
    alive: bool,
) -> *const f64 {
    WORLD.with_borrow_mut(|world| {
        let half_gravity = gravity * time * 0.5;
        let mut fall_velocity = if alive {
            vy - half_gravity
        } else {
            vy - gravity * time
        };
        if alive && fall_velocity < -40.0 {
            fall_velocity = -40.0;
        }
        let mut movement = world.move_capsule(
            [x, y, z],
            [vx, fall_velocity, vz],
            time,
            radius,
            height,
            stepping,
        );
        if alive && !movement.on_ground {
            movement.velocity[1] -= half_gravity;
        }
        world.save_movement(movement, fall_velocity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_grid() -> *const u8 {
    WORLD.with_borrow_mut(|world| world.navigation().grid().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_is_walkable(x: f64, z: f64) -> bool {
    WORLD.with_borrow_mut(|world| world.navigation().is_walkable([x, z]))
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_clear_line(x: f64, z: f64, end_x: f64, end_z: f64) -> bool {
    WORLD.with_borrow_mut(|world| world.navigation().world_line([x, z], [end_x, end_z]))
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_find_path(x: f64, z: f64, end_x: f64, end_z: f64) -> usize {
    WORLD.with_borrow_mut(|world| {
        let nav = world.navigation();
        if nav.world_path([x, z], [end_x, end_z]) {
            nav.waypoints().len()
        } else {
            0
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_waypoints() -> *const [f64; 2] {
    WORLD.with_borrow(|world| world.nav.waypoints().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_sample_point(
    x: f64,
    z: f64,
    radius: f64,
    angle: f64,
    distance: f64,
) -> *const f64 {
    WORLD.with_borrow_mut(|world| {
        let Some(point) = world.navigation().sample([x, z], radius, angle, distance) else {
            return std::ptr::null();
        };
        world.nav_point = point;
        world.nav_point.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn nav_nearest_point(x: f64, z: f64) -> *const f64 {
    WORLD.with_borrow_mut(|world| {
        world.nav_point = world.navigation().nearest_point([x, z]);
        world.nav_point.as_ptr()
    })
}

pub(crate) fn ray_probe(
    origin: [f64; 3],
    direction: [f64; 3],
    distance: f64,
) -> Option<(f64, [f64; 3])> {
    WORLD.with_borrow(|world| {
        world
            .raycast(origin, direction, distance)
            .map(|hit| (hit.distance, hit.normal))
    })
}
