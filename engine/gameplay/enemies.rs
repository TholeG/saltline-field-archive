use std::cell::RefCell;

use super::{horizontal_length, locomotion::angle_delta};

thread_local! {
    static AWARENESS: RefCell<[f64; 5]> = const { RefCell::new([0.0; 5]) };
    static GULL_ATTACK: RefCell<[f64; 6]> = const { RefCell::new([0.0; 6]) };
    static HOUND_ATTACK: RefCell<[f64; 10]> = const { RefCell::new([0.0; 10]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn gull_attack(
    dt: f64,
    cool: f64,
    mut timer: f64,
    flare: f64,
    mut shots: f64,
    burst: f64,
    gap: f64,
    turn: f64,
    distance: f64,
    range: f64,
    mut phase: u32,
    engaged: bool,
    can_shoot: bool,
) -> *const f64 {
    let cool = cool - dt;
    let mut flare = (flare - dt * 1.5).max(0.0);
    let mut action = 0u32;
    match phase {
        0 => {
            if engaged && can_shoot && cool <= 0.0 && turn.abs() < 0.35 && distance < range {
                action = 1;
            }
        }
        1 => {
            timer -= dt;
            flare = (flare + dt * 4.5).min(1.0);
            if !engaged {
                phase = 0;
                action = 2;
            } else if timer <= 0.0 {
                phase = 2;
                shots = burst;
                timer = 0.0;
                action = 3;
            }
        }
        2 => {
            timer -= dt;
            flare = 1.0;
            if timer <= 0.0 {
                timer = gap;
                shots -= 1.0;
                action = if shots <= 0.0 { 5 } else { 4 };
            }
        }
        _ => {}
    }
    GULL_ATTACK.with_borrow_mut(|result| {
        *result = [
            cool,
            timer,
            flare,
            shots,
            f64::from(phase),
            f64::from(action),
        ];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_can_lunge(
    phase: u32,
    distance: f64,
    range: f64,
    cool: f64,
    alive: bool,
) -> bool {
    phase <= 2 && distance < range && cool <= 0.0 && alive
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_move_chance(
    time: f64,
    distance: f64,
    min: f64,
    max: f64,
    seen: bool,
) -> bool {
    time > 1.4 && seen && distance > min - 1.0 && distance < max + 2.0
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_attack(
    dt: f64,
    distance: f64,
    min: f64,
    cool: f64,
    hidden: f64,
    aim: f64,
    turret: f64,
    mut timer: f64,
    mut flare: f64,
    mut shots: f64,
    burst: f64,
    gap: f64,
    mut phase: u32,
    seen: bool,
    alive: bool,
) -> *const f64 {
    let mut crouch = 0.0;
    let mut spring = 0.0;
    let mut request = false;
    let mut replan = false;
    let mut fire = false;
    let mut hit = false;
    let mut finished = false;
    match phase {
        1 | 2 => {
            timer += dt;
            replan = if seen {
                distance < min - 3.0 && timer > 0.8
            } else {
                hidden > 1.1 || timer > 3.0
            };
            request =
                seen && !replan && cool <= 0.0 && alive && angle_delta(aim, turret).abs() < 0.14;
        }
        3 => {
            timer -= dt;
            flare = (flare + dt * 5.0).min(1.0);
            crouch = 0.05;
            if timer <= 0.0 {
                phase = 4;
                shots = burst;
                timer = 0.0;
            }
        }
        4 => {
            timer -= dt;
            flare = 1.0;
            crouch = 0.05;
            if timer <= 0.0 {
                timer = gap;
                shots -= 1.0;
                fire = true;
            }
            finished = shots <= 0.0;
        }
        5 => {
            timer -= dt;
            crouch = 0.16;
            spring = 3.0 * dt;
            if timer <= 0.0 {
                phase = 6;
                timer = 0.36;
            }
        }
        6 => {
            timer -= dt;
            hit = distance < 1.35 && alive;
            if hit || timer <= 0.0 {
                phase = 7;
                timer = 0.55;
            }
        }
        7 => {
            timer -= dt;
            replan = timer <= 0.0;
        }
        _ => {}
    }
    HOUND_ATTACK.with_borrow_mut(|result| {
        *result = [
            timer,
            flare,
            shots,
            f64::from(phase),
            crouch,
            spring,
            f64::from(request as u8),
            f64::from(replan as u8),
            f64::from(fire as u8),
            f64::from(hit as u8 | (finished as u8) << 1),
        ];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_attack_slot(
    count: f64,
    tokens: f64,
    base: f64,
    crowd: f64,
    gap: f64,
    since_last: f64,
    since_other: f64,
    repeat_type: bool,
) -> bool {
    let limit = if count > 5.0 { crowd } else { base };
    let defer = repeat_type && since_other < 0.5 && since_last < gap * 2.2;
    tokens < limit && since_last >= gap && !defer
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_can_see(
    x: f64,
    y: f64,
    z: f64,
    facing_x: f64,
    facing_z: f64,
    range: f64,
    field: f64,
    engaged: bool,
) -> bool {
    let scale = x.abs().max(y.abs()).max(z.abs());
    let range_to_player = if scale == 0.0 {
        0.0
    } else {
        let (x, y, z) = (x / scale, y / scale, z / scale);
        (x * x + y * y + z * z).sqrt() * scale
    };
    if range_to_player >= range {
        return false;
    }
    let length = horizontal_length(x, z);
    let length = if length == 0.0 { 0.0001 } else { length };
    engaged || (x * facing_x + z * facing_z) / length > field || range_to_player < 3.5
}

#[unsafe(no_mangle)]
pub extern "C" fn enemy_awareness(
    dt: f64,
    mut seen: f64,
    mut hidden: f64,
    mut exposure: f64,
    mut mode_time: f64,
    sight: f64,
    memory: f64,
    exposure_up: f64,
    exposure_down: f64,
    mode: u32,
    visible: bool,
    player_alive: bool,
) -> *const f64 {
    if visible {
        seen += dt;
        hidden = 0.0;
        exposure = (exposure + dt * exposure_up).min(1.0);
    } else {
        hidden += dt;
        seen = (seen - dt * 0.6).max(0.0);
        exposure = (exposure - dt * exposure_down).max(0.0);
    }
    let transition = if mode == 2 {
        if hidden > memory || !player_alive {
            1
        } else {
            0
        }
    } else if visible && seen > if mode == 0 { sight } else { sight * 0.45 } {
        hidden = 0.0;
        2
    } else if mode != 0 {
        mode_time -= dt;
        if mode_time <= 0.0 { 3 } else { 0 }
    } else {
        0
    };
    AWARENESS.with_borrow_mut(|result| {
        *result = [seen, hidden, exposure, mode_time, f64::from(transition)];
        result.as_ptr()
    })
}
