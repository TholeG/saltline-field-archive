use std::cell::RefCell;

use super::{horizontal_length, mix};

thread_local! {
    static DIRECTION: RefCell<Point> = const { RefCell::new([0.0; 3]) };
    static BOLT: RefCell<[f64; 5]> = const { RefCell::new([0.0; 5]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_hits_player(
    x: f64,
    y: f64,
    z: f64,
    direction_x: f64,
    direction_y: f64,
    direction_z: f64,
    px: f64,
    py: f64,
    pz: f64,
    limit: f64,
) -> bool {
    let distance = (px - x) * direction_x + (py + 1.0 - y) * direction_y + (pz - z) * direction_z;
    if distance <= 0.0 || distance >= limit {
        return false;
    }
    let x = x + direction_x * distance - px;
    let z = z + direction_z * distance - pz;
    let height = (y + direction_y * distance - (py + 0.95)).abs();
    horizontal_length(x, z) < 0.38 && height < 0.95
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_blast_damage(
    x: f64,
    y: f64,
    z: f64,
    target_x: f64,
    target_y: f64,
    target_z: f64,
    radius: f64,
    damage: f64,
) -> f64 {
    let (x, y, z) = (target_x - x, target_y - y, target_z - z);
    let distance = (x * x + y * y + z * z).sqrt();
    if distance < radius {
        damage * (1.0 - distance / radius)
    } else {
        f64::NAN
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_hit_damage(damage: f64, weak: bool) -> f64 {
    damage * if weak { 2.2 } else { 1.0 }
}

type Point = [f64; 3];

fn add(point: Point, vector: Point, factor: f64) -> Point {
    std::array::from_fn(|i| point[i] + vector[i] * factor)
}

fn difference(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalized(point: Point) -> Point {
    let distance = dot(point, point).sqrt();
    let inverse = if distance == 0.0 {
        1.0
    } else {
        distance.recip()
    };
    point.map(|coordinate| coordinate * inverse)
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_player_spread(spread: f64, angle: f64, radius: f64) -> *const f64 {
    let angle = angle * std::f64::consts::TAU;
    let radius = spread.tan() * radius.sqrt();
    DIRECTION.with_borrow_mut(|result| {
        *result = normalized([angle.cos() * radius, angle.sin() * radius, -1.0]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_enemy_spread(
    x: f64,
    y: f64,
    z: f64,
    far: f64,
    near: f64,
    exposure: f64,
    movement: f64,
    instability: f64,
    random_x: f64,
    random_y: f64,
    random_z: f64,
) -> *const f64 {
    let spread = mix(far, near, exposure) * (1.0 + movement) * instability;
    let perturbation = [random_x, random_y, random_z].map(|value| (value - 0.5) * 2.0);
    DIRECTION.with_borrow_mut(|result| {
        *result = normalized(add([x, y, z], perturbation, spread));
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_bolt_motion(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    life: f64,
    length: f64,
    distance: f64,
    dt: f64,
) -> *const f64 {
    let [x, y, z] = add([x, y, z], [dx, dy, dz], distance);
    BOLT.with_borrow_mut(|result| {
        *result = [x, y, z, life - dt, (length + distance).min(1.5)];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_chain_fuse(fuse: f64, distance: f64, offset: f64) -> f64 {
    let delay = 0.14 + offset * 0.3 + distance * 0.03;
    if fuse < 0.0 { delay } else { fuse.min(delay) }
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_check_enemy(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    range: f64,
) -> bool {
    let delta = [x, y, z];
    let projected = dot(delta, [dx, dy, dz]);
    !(projected < -1.5
        || projected > range + 1.5
        || dot(delta, delta) - projected * projected > 1.7)
}

thread_local! {
    static HIT: RefCell<[f64; 8]> = const { RefCell::new([0.0; 8]) };
    static BLAST: RefCell<[f64; 3]> = const { RefCell::new([0.0; 3]) };
}

fn hit_result(distance: f64, point: Point, normal: Point, weak: bool) -> *const f64 {
    HIT.with_borrow_mut(|hit| {
        let [x, y, z] = point;
        let [nx, ny, nz] = normal;
        *hit = [distance, x, y, z, nx, ny, nz, f64::from(weak as u8)];
        hit.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_hit_gull(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    px: f64,
    py: f64,
    pz: f64,
    range: f64,
    nearest: f64,
) -> *const f64 {
    let local = [x - px, (y - py + 0.03) * 2.3, z - pz];
    let direction = [dx, dy * 2.3, dz];
    let speed = dot(direction, direction);
    let center = dot(local, direction);
    let c = dot(local, local) - 0.5 * 0.5;
    let discriminant = center * center - speed * c;
    if discriminant < 0.0 {
        return std::ptr::null();
    }
    let distance = (-center - discriminant.sqrt()) / speed;
    if distance < 0.0 || distance > range || distance >= nearest {
        return std::ptr::null();
    }
    let point = add([x, y, z], [dx, dy, dz], distance);
    let [nx, ny, nz] = difference(point, [px, py, pz]);
    hit_result(distance, point, normalized([nx, ny * 2.3, nz]), false)
}

fn sphere_hit(
    origin: Point,
    direction: Point,
    center: Point,
    radius: f64,
    range: f64,
    nearest: f64,
) -> Option<(f64, Point, Point)> {
    let local = difference(origin, center);
    let projected = dot(local, direction);
    let outer = dot(local, local) - radius * radius;
    let discriminant = projected * projected - outer;
    if discriminant < 0.0 {
        return None;
    }
    let distance = -projected - discriminant.sqrt();
    if distance < 0.0 || distance > range || distance >= nearest {
        return None;
    }
    let point = add(origin, direction, distance);
    Some((distance, point, normalized(difference(point, center))))
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_hit_hound(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    px: f64,
    py: f64,
    pz: f64,
    head_x: f64,
    head_y: f64,
    head_z: f64,
    yaw: f64,
    hip: f64,
    range: f64,
    mut nearest: f64,
) -> *const f64 {
    let origin = [x, y, z];
    let direction = [dx, dy, dz];
    let head = [head_x, head_y, head_z];
    let mut weak = true;
    let mut hit = sphere_hit(origin, direction, head, 0.155, range, nearest);
    if let Some((distance, _, _)) = hit {
        nearest = distance;
    }
    let sin = yaw.sin();
    let cos = yaw.cos();
    for (height, radius) in [(hip + 0.03, 0.27), (py + (hip - py) * 0.45, 0.25)] {
        let start = [px - sin * 0.45, height, pz - cos * 0.45];
        let finish = [px + sin * 0.47, height, pz + cos * 0.47];
        let span = difference(finish, start);
        let from = difference(origin, start);
        let projected = dot(span, direction);
        let divisor = dot(span, span) - projected * projected;
        let along = if divisor > 1e-8 {
            (dot(span, from) - projected * dot(direction, from)) / divisor
        } else {
            0.0
        };
        let center = add(start, span, along.clamp(0.0, 1.0));
        if let Some(body) = sphere_hit(origin, direction, center, radius, range, nearest) {
            weak = false;
            nearest = body.0;
            hit = Some(body);
        }
    }
    match hit {
        Some((distance, point, normal)) => hit_result(distance, point, normal, weak),
        None => std::ptr::null(),
    }
}

fn segments_distance(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let travel = difference(b, a);
    let target = difference(d, c);
    let gap = difference(a, c);
    let a = dot(travel, travel);
    let b = dot(target, target);
    let c = dot(target, gap);
    let d = dot(travel, gap);
    let e = dot(travel, target);
    let (first, second) = if a <= 1e-9 && b <= 1e-9 {
        return dot(gap, gap);
    } else if a <= 1e-9 {
        (0.0, (c / b).clamp(0.0, 1.0))
    } else if b <= 1e-9 {
        ((-d / a).clamp(0.0, 1.0), 0.0)
    } else {
        let divisor = a * b - e * e;
        let first = if divisor > 1e-9 {
            ((e * c - d * b) / divisor).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let second = (e * first + c) / b;
        if second < 0.0 {
            ((-d / a).clamp(0.0, 1.0), 0.0)
        } else if second > 1.0 {
            (((e - d) / a).clamp(0.0, 1.0), 1.0)
        } else {
            (first, second)
        }
    };
    let vector = difference(
        add(gap, travel, first),
        target.map(|component| component * second),
    );
    dot(vector, vector)
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_bolt_hits(
    x: f64,
    y: f64,
    z: f64,
    dx: f64,
    dy: f64,
    dz: f64,
    travel: f64,
    px: f64,
    py: f64,
    pz: f64,
    crouching: bool,
) -> bool {
    let origin = [x, y, z];
    let end = add(origin, [dx, dy, dz], travel);
    let lower = [px, py + 0.35, pz];
    let upper = [px, py + if crouching { 1.0 } else { 1.5 }, pz];
    segments_distance(origin, end, lower, upper) < 0.36 * 0.36
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_fuse_damage(
    health: f64,
    fuse: f64,
    duration: f64,
    damage: f64,
) -> *const f64 {
    let health = health - damage;
    let (fuse, lit) = if health <= 0.0 {
        (if fuse < 0.0 { 0.06 } else { fuse.min(0.06) }, false)
    } else if fuse < 0.0 {
        (duration, true)
    } else {
        (fuse, false)
    };
    BLAST.with_borrow_mut(|result| {
        *result = [health, fuse, f64::from(lit as u8)];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn combat_fuse_step(fuse: f64, smoke_tick: f64, dt: f64) -> *const f64 {
    let fuse = fuse - dt;
    let mut tick = smoke_tick;
    let action = if fuse <= 0.0 {
        1
    } else {
        tick -= dt;
        if tick <= 0.0 {
            tick = 0.07;
            2
        } else {
            0
        }
    };
    BLAST.with_borrow_mut(|result| {
        *result = [fuse, tick, f64::from(action)];
        result.as_ptr()
    })
}
