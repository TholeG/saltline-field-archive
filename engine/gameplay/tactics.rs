use std::cell::RefCell;

use super::horizontal_length;
use crate::physics::{nav_is_walkable, physics_ground_height, physics_line_of_sight};

struct FiringPlan {
    hound: [f64; 3],
    player: [f64; 3],
    eye: [f64; 3],
    min_range: f64,
    max_range: f64,
    score: f64,
    best: [f64; 3],
    arc_point: [f64; 3],
    allies: Vec<f64>,
}

thread_local! {
    static PLAN: RefCell<FiringPlan> = const { RefCell::new(FiringPlan {
        hound: [0.0; 3],
        player: [0.0; 3],
        eye: [0.0; 3],
        min_range: 0.0,
        max_range: 0.0,
        score: f64::INFINITY,
        best: [0.0; 3],
        arc_point: [0.0; 3],
        allies: Vec::new(),
    }) };
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_plan(
    x: f64,
    y: f64,
    z: f64,
    px: f64,
    py: f64,
    pz: f64,
    eye_x: f64,
    eye_y: f64,
    eye_z: f64,
    min_range: f64,
    max_range: f64,
    count: usize,
) -> *mut f64 {
    PLAN.with_borrow_mut(|plan| {
        plan.hound = [x, y, z];
        plan.player = [px, py, pz];
        plan.eye = [eye_x, eye_y, eye_z];
        plan.min_range = min_range;
        plan.max_range = max_range;
        plan.score = f64::INFINITY;
        plan.best = plan.hound;
        plan.allies.resize(count * 2, 0.0);
        plan.allies.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_can_fire_from(x: f64, z: f64) -> bool {
    PLAN.with_borrow(|plan| {
        let [px, _, pz] = plan.player;
        let range = horizontal_length(x - px, z - pz);
        range >= plan.min_range && range <= plan.max_range && nav_is_walkable(x, z)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_evaluate_position(x: f64, y: f64, z: f64, bias: f64, random: f64) {
    PLAN.with_borrow_mut(|plan| {
        let [hx, _, hz] = plan.hound;
        let [px, _, pz] = plan.player;
        let mut score = horizontal_length(x - hx, z - hz) * 0.9 + bias + random * 3.0;
        let (dx, dz) = (x - px, z - pz);
        let range = horizontal_length(dx, dz);
        for ally in plan.allies.chunks_exact(2) {
            let (ax, az) = (ally[0] - px, ally[1] - pz);
            let alignment = (dx * ax + dz * az) / (range * horizontal_length(ax, az) + 1e-4);
            if alignment > 0.86 {
                score += 14.0 * (alignment - 0.86) / 0.14;
            }
            if horizontal_length(x - ally[0], z - ally[1]) < 3.0 {
                score += 16.0;
            }
        }
        if score >= plan.score {
            return;
        }
        let [eye_x, eye_y, eye_z] = plan.eye;
        if !physics_line_of_sight(x, y + 0.95, z, eye_x, eye_y, eye_z) {
            score += 22.0;
        }
        if score < plan.score {
            plan.score = score;
            plan.best = [x, y, z];
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_arc_point(direction: f64, radius: f64) -> *const f64 {
    PLAN.with_borrow_mut(|plan| {
        let [hx, _, hz] = plan.hound;
        let [px, py, pz] = plan.player;
        let angle = (hz - pz).atan2(hx - px) + (direction - 0.5) * 2.6;
        let radius = plan.min_range + 1.0 + radius * (plan.max_range - plan.min_range - 2.0);
        let x = px + angle.cos() * radius;
        let z = pz + angle.sin() * radius;
        plan.arc_point = [x, physics_ground_height(x, z, py + 1.0), z];
        plan.arc_point.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_fire_position() -> *const f64 {
    PLAN.with_borrow(|plan| plan.best.as_ptr())
}
