use super::horizontal_length;
use crate::physics::{nav_is_walkable, physics_ground_height, ray_probe};
use std::cell::RefCell;
use std::f64::consts::{PI, TAU};

thread_local! {
    static FLIGHT: RefCell<[f64; 42]> = const { RefCell::new([0.0; 42]) };
    static FLIGHT_RESULT: RefCell<[f64; 24]> = const { RefCell::new([0.0; 24]) };
    static GROUND: RefCell<[f64; 22]> = const { RefCell::new([0.0; 22]) };
    static GROUND_RESULT: RefCell<[f64; 15]> = const { RefCell::new([0.0; 15]) };
    static NEIGHBORS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
    static WAYPOINTS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
    static FOLLOW_RESULT: RefCell<[f64; 4]> = const { RefCell::new([0.0; 4]) };
}

pub fn angle_delta(target: f64, angle: f64) -> f64 {
    let mut delta = (target - angle) % TAU;
    if delta > PI {
        delta -= TAU;
    } else if delta < -PI {
        delta += TAU;
    }
    delta
}

fn length([x, y, z]: [f64; 3]) -> f64 {
    (x * x + y * y + z * z).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn gull_flight_input() -> *mut f64 {
    FLIGHT.with_borrow_mut(|input| input.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn gull_flight_step(flags: u32) -> *const f64 {
    FLIGHT.with_borrow(|input| {
        let [
            dt,
            time,
            x,
            y,
            z,
            vx,
            vy,
            vz,
            avoid_x,
            avoid_y,
            avoid_z,
            home_x,
            home_y,
            home_z,
            seen_x,
            seen_y,
            seen_z,
            player_x,
            player_y,
            player_z,
            mut yaw,
            mut pitch,
            mut roll,
            phase,
            mut ceiling,
            mut ceiling_timer,
            mut flip_timer,
            mut orbit_direction,
            mut orbit_radius,
            mut orbit_phase,
            feeler,
            unseen,
            altitude,
            patrol_radius,
            patrol_speed,
            max_speed,
            accel,
            min_radius,
            max_radius,
            new_flip,
            flip_chance,
            new_radius,
        ] = *input;
        let engaged = flags & 1 != 0;
        let searching = flags & 2 != 0;
        let hurt = flags & 4 != 0;
        let has_player = flags & 8 != 0;
        let fixed_height = flags & 16 != 0;
        let attacking = flags & 32 != 0;

        let mut position = [x, y, z];
        let mut velocity = [vx, vy, vz];
        let mut avoid = [avoid_x, avoid_y, avoid_z];
        let mut speed_limit = patrol_speed;
        let (target_x, target_z, mut target_y) = if engaged && has_player {
            speed_limit = max_speed;
            flip_timer -= dt;
            if flip_timer <= 0.0 {
                flip_timer = 3.0 + new_flip * 5.0;
                if flip_chance < 0.6 {
                    orbit_direction = -orbit_direction;
                }
                orbit_radius = (orbit_radius + (new_radius - 0.5) * 6.0)
                    .clamp(min_radius + 1.0, max_radius - 2.0);
            }
            if unseen > 1.5 {
                orbit_phase = (z - seen_z).atan2(x - seen_x);
                (seen_x, seen_z, seen_y + altitude + 1.0)
            } else {
                let orbit_speed = if attacking { 0.9 } else { 3.6 };
                orbit_phase += orbit_direction * orbit_speed / orbit_radius * dt;
                (
                    player_x + orbit_phase.cos() * orbit_radius,
                    player_z + orbit_phase.sin() * orbit_radius,
                    player_y + altitude,
                )
            }
        } else if searching {
            speed_limit = 5.5;
            orbit_phase += 0.5 * dt;
            (
                seen_x + orbit_phase.cos() * 4.5,
                seen_z + orbit_phase.sin() * 4.5,
                seen_y + altitude.min(5.5),
            )
        } else {
            orbit_phase += 0.28 * dt * orbit_direction;
            (
                home_x + orbit_phase.cos() * patrol_radius,
                home_z + orbit_phase.sin() * patrol_radius,
                if fixed_height { altitude } else { home_y },
            )
        };
        target_y = target_y.min(ceiling);
        if !fixed_height {
            target_y = target_y.max(2.2);
        }
        let mut force = [
            (target_x - x) * 2.2 - vx * 2.4,
            (target_y - y) * 5.0 - vy * 3.6,
            (target_z - z) * 2.2 - vz * 2.4,
        ];
        let feeler = (feeler + 1.0) % 3.0;
        let speed = horizontal_length(vx, vz);
        if feeler == 0.0 && speed > 0.6 {
            let distance = 2.2 + speed * 0.5;
            if let Some((hit, normal)) =
                ray_probe(position, [vx / speed, 0.0, vz / speed], distance)
            {
                let proximity = 1.0 - hit / distance;
                avoid = normal.map(|axis| axis * (26.0 * proximity));
                avoid[1] += 9.0 * proximity;
                if proximity > 0.5 {
                    orbit_direction = -orbit_direction;
                    flip_timer = 2.5;
                }
            }
        } else if feeler == 1.0 {
            if let Some((hit, _)) = ray_probe(position, [0.0, -1.0, 0.0], 2.4) {
                if !fixed_height {
                    avoid[1] += (2.4 - hit) * 10.0;
                }
            }
        } else if feeler == 2.0 {
            ceiling_timer -= dt * 3.0;
            if ceiling_timer <= 0.0 {
                ceiling_timer = 0.35;
                ceiling =
                    ray_probe(position, [0.0, 1.0, 0.0], 4.0).map_or(1e9, |(hit, _)| y + hit - 1.3);
            }
        }
        let damping = (-6.0 * dt).exp();
        for i in 0..3 {
            force[i] += avoid[i];
            avoid[i] *= damping;
        }
        if hurt {
            force[0] += (time * 7.3 + phase).sin() * 5.0;
            force[2] += (time * 6.1 + phase).cos() * 5.0;
            force[1] += (time * 9.7).sin() * 3.0;
        }
        let strength = length(force);
        if strength > accel {
            force = force.map(|axis| axis * (accel / strength));
        }
        for i in 0..3 {
            velocity[i] += force[i] * dt;
        }
        let velocity_length = length(velocity);
        if velocity_length > speed_limit {
            velocity = velocity.map(|axis| axis * (speed_limit / velocity_length));
        }
        for axis in 0..3 {
            let speed = velocity[axis];
            if speed.abs() < 0.02 {
                continue;
            }
            let sign = if speed > 0.0 { 1.0 } else { -1.0 };
            let clearance = if axis == 1 { 0.35 } else { 0.62 };
            let mut direction = [0.0; 3];
            direction[axis] = sign;
            if let Some((hit, _)) = ray_probe(position, direction, speed.abs() * dt + clearance) {
                velocity[axis] = -speed * 0.25;
                avoid[axis] -= sign * 8.0;
                if hit < clearance {
                    position[axis] -= sign * (clearance - hit);
                }
            }
        }
        for i in 0..3 {
            position[i] += velocity[i] * dt;
        }
        if !fixed_height {
            let floor = physics_ground_height(position[0], position[2], position[1] + 0.5) + 1.1;
            if position[1] < floor {
                position[1] = floor;
                if velocity[1] < 0.0 {
                    velocity[1] = 0.0;
                }
            }
        }
        let mut facing = yaw;
        if (engaged || searching) && has_player {
            let (target_x, target_z) = if engaged && unseen < 1.5 {
                (player_x, player_z)
            } else {
                (seen_x, seen_z)
            };
            facing = (target_x - position[0]).atan2(target_z - position[2]);
        } else if speed > 0.4 {
            facing = velocity[0].atan2(velocity[2]);
        }
        let turn = angle_delta(facing, yaw);
        yaw += turn.clamp(-3.2 * dt, 3.2 * dt);
        let cos = yaw.cos();
        let sin = yaw.sin();
        let mut target_pitch = ((force[0] * sin + force[2] * cos) * 0.04).clamp(-0.42, 0.42);
        let mut target_roll = (-(force[0] * cos - force[2] * sin) * 0.04).clamp(-0.42, 0.42);
        if hurt {
            target_pitch += (time * 11.0 + phase).sin() * 0.1;
            target_roll += (time * 8.7 + phase * 2.0).cos() * 0.13;
        }
        let easing = 1.0 - (-7.0 * dt).exp();
        pitch += (target_pitch - pitch) * easing;
        roll += (target_roll - roll) * easing;
        FLIGHT_RESULT.with_borrow_mut(|result| {
            let [x, y, z] = position;
            let [vx, vy, vz] = velocity;
            let [ax, ay, az] = avoid;
            *result = [
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
                ceiling_timer,
                flip_timer,
                orbit_direction,
                orbit_radius,
                orbit_phase,
                feeler,
                force[1],
                speed,
                turn,
                cos,
                sin,
            ];
            result.as_ptr()
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_ground_input() -> *mut f64 {
    GROUND.with_borrow_mut(|input| input.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_neighbors(count: usize) -> *mut f64 {
    NEIGHBORS.with_borrow_mut(|neighbors| {
        neighbors.resize(count * 2, 0.0);
        neighbors.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_waypoints(count: usize) -> *mut f64 {
    WAYPOINTS.with_borrow_mut(|points| {
        points.resize(count * 2, 0.0);
        points.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_follow(
    x: f64,
    z: f64,
    yaw: f64,
    goal_x: f64,
    goal_z: f64,
    mut index: usize,
    speed: f64,
) -> *const f64 {
    let (target_x, target_z) = WAYPOINTS.with_borrow(|points| {
        let count = points.len() / 2;
        if count == 0 {
            return (goal_x, goal_z);
        }
        while index < count - 1
            && horizontal_length(points[index * 2] - x, points[index * 2 + 1] - z) < 0.55
        {
            index += 1;
        }
        let offset = index.min(count - 1) * 2;
        (points[offset], points[offset + 1])
    });
    let distance = horizontal_length(goal_x - x, goal_z - z);
    let reached = distance < 0.45;
    let target_yaw = (target_x - x).atan2(target_z - z);
    let error = angle_delta(target_yaw, yaw).abs();
    let speed = if reached || error > 1.1 {
        0.0
    } else {
        (speed * (1.0 - error * 0.5)).min(0.4 + distance * 2.6)
    };
    FOLLOW_RESULT.with_borrow_mut(|result| {
        *result = [index as f64, target_yaw, speed, f64::from(reached as u8)];
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hound_ground_step(flags: u32) -> *const f64 {
    GROUND.with_borrow(|input| {
        let [
            dt,
            x,
            y,
            z,
            mut yaw,
            desired_yaw,
            mut yaw_rate,
            mut speed,
            max_speed,
            turn_rate,
            accel,
            mut push_x,
            push_y,
            mut push_z,
            player_x,
            player_z,
            player_distance,
            mut feeler,
            mut stuck_time,
            mut plan_time,
            phase,
            attack_time,
        ] = *input;
        let lunging = flags & 1 != 0;
        let mut blocked = flags & 2 != 0;
        let has_player = flags & 4 != 0;
        let error = angle_delta(desired_yaw, yaw);
        let desired_turn =
            (error * 6.0).clamp(-turn_rate, turn_rate) * if lunging { 2.0 } else { 1.0 };
        yaw_rate += (desired_turn - yaw_rate) * (1.0 - (-10.0 * dt).exp());
        yaw += yaw_rate * dt;
        let prev_speed = speed;
        let acceleration = if max_speed > speed {
            accel
        } else {
            -accel * 1.4
        };
        if !lunging {
            speed = (speed + acceleration * dt).clamp(0.0, max_speed.max(0.0));
            if acceleration < 0.0 && speed < max_speed {
                speed = max_speed;
            }
        }
        let (sin, cos) = (yaw.sin(), yaw.cos());
        NEIGHBORS.with_borrow(|neighbors| {
            for point in neighbors.chunks_exact(2) {
                let (dx, dz) = (x - point[0], z - point[1]);
                let distance = horizontal_length(dx, dz);
                if distance < 1.7 && distance > 0.001 {
                    let push = (1.7 - distance) * 2.2 / distance;
                    push_x += dx * push * dt * 6.0;
                    push_z += dz * push * dt * 6.0;
                }
            }
        });
        if has_player && !lunging && player_distance < 1.25 && player_distance > 0.001 {
            let push = (1.25 - player_distance) * 3.0 / player_distance;
            push_x += (x - player_x) * push * dt * 6.0;
            push_z += (z - player_z) * push * dt * 6.0;
        }
        let damping = (-5.0 * dt).exp();
        push_x *= damping;
        push_z *= damping;
        let mut next_x = x + (sin * speed + push_x) * dt;
        let mut next_z = z + (cos * speed + push_z) * dt;
        feeler -= dt;
        if feeler <= 0.0 && speed > 0.3 {
            feeler = 0.12;
            blocked = ray_probe([x, y + 0.55, z], [sin, 0.0, cos], 0.75 + speed * 0.12).is_some();
        } else if speed <= 0.3 {
            blocked = false;
        }
        let mut replan = false;
        if !nav_is_walkable(next_x, next_z) || (blocked && (!lunging || attack_time < 0.3)) {
            if blocked {
                next_x = x + push_x * dt;
                next_z = z + push_z * dt;
                yaw_rate += if phase > 3.14 { 1.0 } else { -1.0 } * 4.0 * dt;
            } else if nav_is_walkable(next_x, z) {
                next_z = z;
            } else if nav_is_walkable(x, next_z) {
                next_x = x;
            } else {
                (next_x, next_z) = (x, z);
            }
            speed *= 0.6;
            stuck_time += dt;
            if stuck_time > 0.5 {
                stuck_time = 0.0;
                plan_time = 0.0;
                replan = true;
            }
        }
        let next_y =
            y + (physics_ground_height(next_x, next_z, y + 0.6) - y) * (1.0 - (-12.0 * dt).exp());
        GROUND_RESULT.with_borrow_mut(|result| {
            *result = [
                next_x,
                next_y,
                next_z,
                yaw,
                yaw_rate,
                speed,
                prev_speed,
                push_x,
                push_y * damping,
                push_z,
                feeler,
                f64::from(blocked as u8),
                stuck_time,
                plan_time,
                f64::from(replan as u8),
            ];
            result.as_ptr()
        })
    })
}
