use std::cell::RefCell;
use std::f64::consts::{PI, TAU};

use super::{approach, horizontal_length, mix, unit};

thread_local! {
    static PLAYER_INPUT: RefCell<[f64; 32]> = const { RefCell::new([0.0; 32]) };
    static PLAYER_RESULT: RefCell<[f64; 15]> = const { RefCell::new([0.0; 15]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn player_input_buffer() -> *mut f64 {
    PLAYER_INPUT.with_borrow_mut(|input| input.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn player_input_step(flags: u32) -> *const f64 {
    PLAYER_INPUT.with_borrow(|input| {
        let [
            dt,
            x,
            y,
            z,
            mut yaw,
            mut pitch,
            mut vx,
            mut vy,
            mut vz,
            current_speed,
            ads,
            mut coyote,
            mut jump_buffer,
            radius,
            scale_ads,
            pitch_limit,
            look_x,
            look_y,
            move_x,
            move_z,
            walk,
            run,
            crouch,
            ads_speed,
            ground_accel,
            ground_brake,
            air_accel,
            coyote_limit,
            buffer_limit,
            stand_height,
            crouch_height,
            jump_speed,
        ] = *input;
        let enabled = flags & 1 != 0;
        let mut grounded = flags & 2 != 0;
        let mut crouching = flags & 4 != 0;
        let was_sprinting = flags & 8 != 0;
        let wants_crouch = flags & 16 != 0;
        let wants_sprint = flags & 32 != 0;
        let firing = flags & 64 != 0;
        let wants_jump = flags & 128 != 0;

        let look_scale = mix(1.0, scale_ads, ads);
        if enabled {
            yaw -= look_x * look_scale;
            pitch = (pitch - look_y * look_scale).clamp(-pitch_limit, pitch_limit);
            if !(-4.0 * PI..=4.0 * PI).contains(&yaw) {
                yaw %= TAU;
            }
        }

        let can_stand = || {
            crate::physics::physics_has_clearance(
                x,
                z,
                radius * 0.9,
                y + crouch_height - 0.05,
                y + stand_height,
            )
        };
        if enabled && wants_crouch {
            crouching = true;
        } else if crouching && can_stand() {
            crouching = false;
        }
        let (steer, forward) = if enabled {
            (move_x, move_z)
        } else {
            (0.0, 0.0)
        };
        let moving = steer * steer + forward * forward > 0.01;
        let can_sprint = enabled
            && wants_sprint
            && forward > 0.55
            && steer.abs() < 0.6
            && !crouching
            && ads < 0.35
            && !firing;
        let sprinting = can_sprint
            && if grounded {
                current_speed > walk * 0.6
            } else {
                was_sprinting
            };
        let pace = if crouching {
            crouch
        } else if can_sprint {
            run
        } else {
            walk
        };
        let mut speed = mix(pace, pace.min(ads_speed), ads);
        if forward < 0.0 {
            speed *= mix(1.0, 0.82, -forward);
        }
        let sin = yaw.sin();
        let cos = yaw.cos();
        let desired_x = (steer * cos - forward * sin) * speed;
        let desired_z = (-steer * sin - forward * cos) * speed;
        let accel = match (grounded, moving) {
            (true, true) => ground_accel,
            (true, false) => ground_brake,
            (false, true) => air_accel,
            (false, false) => 0.6,
        };
        let (dx, dz) = if !grounded && !moving {
            (-vx, -vz)
        } else {
            (desired_x - vx, desired_z - vz)
        };
        let difference = horizontal_length(dx, dz);
        let max_step = accel * dt;
        if difference <= max_step || difference < 1e-6 {
            if grounded || moving {
                (vx, vz) = (desired_x, desired_z);
            }
        } else {
            vx += dx / difference * max_step;
            vz += dz / difference * max_step;
        }

        coyote = if grounded {
            coyote_limit
        } else {
            (coyote - dt).max(0.0)
        };
        jump_buffer = if enabled && wants_jump {
            buffer_limit
        } else {
            (jump_buffer - dt).max(0.0)
        };
        let jump = jump_buffer > 0.0 && coyote > 0.0 && (!crouching || can_stand());
        if jump {
            vy = jump_speed;
            coyote = 0.0;
            jump_buffer = 0.0;
            grounded = false;
            crouching = false;
        }
        let height = if crouching {
            crouch_height
        } else {
            stand_height
        };
        PLAYER_RESULT.with_borrow_mut(|result| {
            *result = [
                yaw,
                pitch,
                look_scale,
                vx,
                vy,
                vz,
                coyote,
                jump_buffer,
                height,
                sin,
                cos,
                f64::from(grounded as u8),
                f64::from(crouching as u8),
                f64::from(sprinting as u8),
                f64::from(jump as u8),
            ];
            result.as_ptr()
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn player_landing(
    on_ground: bool,
    was_grounded: bool,
    dt: f64,
    air_time: f64,
    fall_max: f64,
    fall_before_collision: f64,
    vertical_speed: f64,
    land_dip: f64,
) -> *const f64 {
    let speed = if on_ground && !was_grounded && air_time > 0.12 {
        0.0_f64.max(-fall_before_collision).max(fall_max)
    } else {
        -1.0
    };
    let shake = if speed > 7.5 {
        ((speed - 7.5) * 0.06).min(0.5)
    } else {
        0.0
    };
    let harm = if speed > 13.0 {
        (speed - 13.0) * 7.0
    } else {
        0.0
    };
    let (air_time, fall_max) = if on_ground {
        (0.0, 0.0)
    } else {
        (air_time + dt, fall_max.max(-vertical_speed))
    };
    PLAYER_RESULT.with_borrow_mut(|result| {
        result[..6].copy_from_slice(&[
            speed,
            (speed * land_dip).min(3.2),
            shake,
            harm,
            air_time,
            fall_max,
        ]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn player_gait(
    dt: f64,
    vx: f64,
    vz: f64,
    on_ground: bool,
    sprinting: bool,
    crouching: bool,
    walk_speed: f64,
    run_speed: f64,
    stride: f64,
    phase: f64,
    swing: f64,
    roll: f64,
    ads: f64,
    bob_ads: f64,
    bob_sprint: f64,
    bob_y: f64,
    bob_x: f64,
    strafe_roll: f64,
    cos: f64,
    sin: f64,
) -> *const f64 {
    let speed = horizontal_length(vx, vz);
    let mut next = phase;
    let mut step = false;
    if on_ground {
        if speed > 0.5 {
            next += speed * dt / stride;
            step = next.floor() > phase.floor();
        } else {
            let rounded = phase.floor() + f64::from((phase - phase.floor() >= 0.5) as u8);
            next = approach(phase, rounded, 8.0, dt);
        }
    }
    let scale = if on_ground {
        unit(speed / walk_speed)
            * if sprinting { bob_sprint } else { 1.0 }
            * mix(1.0, bob_ads, ads)
            * if crouching { 0.7 } else { 1.0 }
    } else {
        0.0
    };
    let swing = approach(swing, scale, 9.0, dt);
    let vertical = -(next * PI * 2.0).cos() * bob_y * swing;
    let horizontal = (next * PI).sin() * bob_x * swing;
    let strafe = vx * cos - vz * sin;
    let roll = approach(
        roll,
        -strafe / run_speed * strafe_roll * mix(1.0, 0.4, ads),
        8.0,
        dt,
    );
    PLAYER_RESULT.with_borrow_mut(|result| {
        result[..7].copy_from_slice(&[
            speed,
            next,
            f64::from(step as u8),
            swing,
            vertical,
            horizontal,
            roll,
        ]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn player_damage(health: f64, pulse: f64, amount: f64) -> *const f64 {
    PLAYER_RESULT.with_borrow_mut(|result| {
        result[..3].copy_from_slice(&[
            (health - amount).max(0.0),
            (pulse + 0.35 + amount / 40.0).min(1.0),
            0.22 + amount / 70.0,
        ]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn player_regenerate(
    health: f64,
    maximum: f64,
    timer: f64,
    delay: f64,
    rate: f64,
    dt: f64,
) -> f64 {
    if timer > delay && health < maximum {
        (health + rate * dt).min(maximum)
    } else {
        health
    }
}
