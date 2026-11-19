use super::{horizontal_length, unit};

#[unsafe(no_mangle)]
pub extern "C" fn mission_relay_distance(
    x: f64,
    y: f64,
    z: f64,
    bearing_x: f64,
    bearing_z: f64,
    target_x: f64,
    target_y: f64,
    target_z: f64,
    range: f64,
    facing: f64,
) -> f64 {
    let dx = target_x - x;
    let dz = target_z - z;
    let distance = horizontal_length(dx, dz);
    if distance > range
        || (target_y - y).abs() > 2.5
        || (distance > 0.9
            && (dx * bearing_x + dz * bearing_z)
                / (distance * horizontal_length(bearing_x, bearing_z).max(0.2))
                < facing)
    {
        -1.0
    } else {
        distance
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_cut_progress(
    progress: f64,
    dt: f64,
    cut_seconds: f64,
    active: bool,
) -> f64 {
    if active {
        progress + dt / cut_seconds
    } else {
        (progress - dt * 1.6).max(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_extraction_range(
    x: f64,
    y: f64,
    z: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    radius: f64,
) -> bool {
    horizontal_length(tx - x, tz - z) < radius && (ty - y).abs() < 3.5
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_rating(time: f64, shots: f64, hits: f64, retries: f64) -> u32 {
    let accuracy = if shots > 0.0 { unit(hits / shots) } else { 0.0 };
    let speed = if time < 270.0 {
        2
    } else if time < 400.0 {
        1
    } else {
        0
    };
    let precision = if accuracy >= 0.5 {
        2
    } else if accuracy >= 0.3 {
        1
    } else {
        0
    };
    speed + precision + u32::from(retries == 0.0)
}

use std::cell::RefCell;
use std::f64::consts::TAU;

thread_local! {
    static SPAWN_CANDIDATES: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
    static SPAWN_RESULT: RefCell<[f64; 5]> = const { RefCell::new([0.0; 5]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_candidates(count: usize) -> *mut f64 {
    SPAWN_CANDIDATES.with_borrow_mut(|data| {
        data.resize(count * 5, 0.0);
        data.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_pick_spawn(
    x: f64,
    z: f64,
    eye_x: f64,
    eye_y: f64,
    eye_z: f64,
    minimum: f64,
    now: f64,
    prefer_hidden: bool,
    flying: bool,
) -> i32 {
    SPAWN_CANDIDATES.with_borrow(|data| {
        let mut selected = -1;
        let mut best = -1e9;
        for (index, row) in data.chunks_exact(5).enumerate() {
            let [tx, ty, tz, random, last_used] = <[f64; 5]>::try_from(row).unwrap();
            let distance = horizontal_length(tx - x, tz - z);
            let mut score = random * 1.5;
            score += if distance >= minimum {
                2.0 + ((distance - minimum) / 30.0).min(1.0) * 0.5
            } else {
                -(minimum - distance) / 4.0
            };
            if prefer_hidden
                && !crate::physics::physics_line_of_sight(
                    tx,
                    ty + if flying { 0.0 } else { 1.0 },
                    tz,
                    eye_x,
                    eye_y,
                    eye_z,
                )
            {
                score += 2.5;
            }
            if now - last_used < 6.0 {
                score -= 3.0;
            }
            if score > best {
                best = score;
                selected = index as i32;
            }
        }
        selected
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_spawn_position(
    x: f64,
    y: f64,
    z: f64,
    random_x: f64,
    random_z: f64,
    anchored: bool,
    flying: bool,
) -> *const f64 {
    let mut position = [x, y, z];
    if anchored {
        let offset_x = (random_x - 0.5) * 2.4;
        let offset_z = (random_z - 0.5) * 2.4;
        if flying || crate::physics::nav_is_walkable(x + offset_x, z + offset_z) {
            position[0] += offset_x;
            position[2] += offset_z;
        }
    } else {
        let angle = random_x * TAU;
        position = [
            (x + angle.cos() * 26.0).clamp(-34.0, 26.0),
            0.0,
            (z + angle.sin() * 26.0).clamp(-26.0, 24.0),
        ];
    }
    SPAWN_RESULT.with_borrow_mut(|result| {
        result[..3].copy_from_slice(&position);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_holdout_config(wave: f64) -> *const f64 {
    SPAWN_RESULT.with_borrow_mut(|result| {
        *result = [
            (1.0 + 0.07 * (wave - 1.0)).min(2.5),
            (1.0 + 0.05 * (wave - 1.0)).min(2.0),
            (1.0 - 0.03 * (wave - 1.0)).max(0.6),
            (wave / 2.0).floor().min(5.0),
            (0.9 - 0.04 * wave).max(0.35),
        ];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_wave_types(
    wave: u32,
    gulls: usize,
    hounds: usize,
    holdout: bool,
) -> usize {
    SPAWN_CANDIDATES.with_borrow_mut(|data| {
        data.clear();
        if holdout {
            let wave = f64::from(wave);
            let count = (5.0 + 2.0 * wave).min(30.0) as usize;
            let hounds = (count as f64 * (0.2 + 0.03 * wave).min(0.45)).round() as usize;
            data.resize(count, 0.0);
            for hound in 0..hounds {
                let target = (1 + hound * count / hounds).min(count - 1);
                data[target] = 1.0;
            }
        } else {
            data.resize(gulls, 0.0);
            for hound in 0..hounds {
                data.insert((1 + hound * 2).min(data.len()), 1.0);
            }
        }
        data.len()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_wave_members() -> *const f64 {
    SPAWN_CANDIDATES.with_borrow(|data| data.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_holdout_tick(
    active: bool,
    waiting: bool,
    enemies: f64,
    wave: f64,
    remaining: f64,
    dt: f64,
) -> *const f64 {
    let mut remaining = remaining;
    let mut action = 0.0;
    if active {
        if !waiting && enemies == 0.0 {
            action = -1.0;
        }
    } else {
        remaining -= dt;
        if remaining <= 0.0 {
            action = -2.0;
        } else if wave > 0.0 && remaining < 3.05 && remaining.ceil() != (remaining + dt).ceil() {
            action = remaining.ceil();
        }
    }
    SPAWN_RESULT.with_borrow_mut(|result| {
        result[..2].copy_from_slice(&[remaining, action]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_intensity(
    dt: f64,
    alert: f64,
    hot: f64,
    alive: f64,
    pending: bool,
    all_cut: bool,
    nearby: bool,
) -> *const f64 {
    let alert = (alert - dt).max(0.0);
    let mut hot = (hot - dt).max(0.0);
    if hot > 0.0 && alive == 0.0 && !pending && hot < 24.0 {
        hot = 0.0;
    }
    let intensity = if all_cut || pending || (hot > 0.0 && alive > 0.0) {
        1.0
    } else if alert > 0.0 || (alive > 0.0 && nearby) {
        0.6
    } else {
        0.15
    };
    SPAWN_RESULT.with_borrow_mut(|result| {
        result[..3].copy_from_slice(&[alert, hot, intensity]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_kill_score(base: f64, wave: f64) -> f64 {
    base * (1.0 + 0.1 * (wave - 1.0))
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_queue_slot(now: f64, scheduled: f64, alive: usize, limit: usize) -> u32 {
    if now < scheduled {
        0
    } else if alive >= limit {
        1
    } else {
        2
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_queue_retry(tries: u32, spawned: bool, stub: bool) -> bool {
    !spawned && !stub && tries < 6
}

#[unsafe(no_mangle)]
pub extern "C" fn mission_pursuit(
    dt: f64,
    mut delay: f64,
    mut left: f64,
    interval: f64,
    alive: usize,
    minimum: usize,
    queued: bool,
) -> *const f64 {
    delay -= dt;
    let spawn = delay <= 0.0 && left > 0.0 && !queued && alive < minimum;
    if spawn {
        delay = interval;
        left -= 1.0;
    }
    let rally = delay <= 0.0;
    if rally {
        delay = 4.0;
    }
    SPAWN_RESULT.with_borrow_mut(|result| {
        result[..4].copy_from_slice(&[delay, left, f64::from(spawn as u8), f64::from(rally as u8)]);
        result.as_ptr()
    })
}
