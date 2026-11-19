use std::cell::RefCell;
use std::f64::consts::PI;

thread_local! {
    static BACKDROP_FRAME: RefCell<[f64; 7]> = const { RefCell::new([0.0; 7]) };
    static RELAY_FRAME: RefCell<[f64; 10]> = const { RefCell::new([0.0; 10]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn backdrop_frame(
    clock: f64,
    delta: f64,
    height: f64,
    showcase: bool,
) -> *const f64 {
    let time = clock + delta;
    let heave = if showcase {
        0.0
    } else {
        0.05 * (time * 1.15).sin() + 0.025 * (time * 2.3 + 1.3).sin()
    };
    BACKDROP_FRAME.with_borrow_mut(|frame| {
        *frame = [
            time,
            0.012 * (time * 0.52).sin() + 0.004 * (time * 1.31 + 1.0).sin(),
            0.009 * (time * 0.41 + 1.1).sin(),
            time * 1.9,
            height + heave,
            0.05 * (time * 0.9 + 0.4).sin() + 0.016 * (time * 2.1).sin(),
            0.02 * (time * 1.3 + 2.0).sin(),
        ];
        frame.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_fan_angle(angle: f64, delta: f64) -> f64 {
    angle + delta * 1.7
}

#[unsafe(no_mangle)]
pub extern "C" fn container_door_angle(opening: f64, side: f64) -> f64 {
    let (scale, degrees) = if side > 0.0 {
        (1.0, 112.0)
    } else {
        (0.88, 100.0)
    };
    opening * scale * (degrees * (PI / 180.0))
}

const LIVE: f64 = 0.0;
const CUTTING: f64 = 1.0;
const CUT: f64 = 2.0;

struct Relay {
    mode: f64,
    time: f64,
    time_since_cut: f64,
    rotor_speed: f64,
    door_angle: f64,
    door_speed: f64,
    door_target: f64,
}

impl Relay {
    fn load(state: &[f64; 10]) -> Self {
        Self {
            mode: state[0],
            time: state[1],
            time_since_cut: state[2],
            rotor_speed: state[3],
            door_angle: state[4],
            door_speed: state[5],
            door_target: state[6],
        }
    }

    fn change_mode(&mut self, mode: f64) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        if mode == CUT {
            self.time_since_cut = 0.0;
            self.door_target = 1.32;
            self.door_speed = 2.5;
        }
        if mode == LIVE {
            self.door_target = 0.0;
        }
    }

    fn update(&mut self, dt: f64) -> (u32, f64, f64, f64) {
        let dt = dt.min(0.1);
        self.time += dt;
        let (color, light, target_speed) = match self.mode {
            LIVE => (0, 5.5 + 3.0 * (self.time * 2.2).sin(), 0.7),
            CUTTING => {
                let bright = (self.time * 2.0 * PI * 5.5).sin() > 0.0;
                let target_speed = 2.6 + (self.time * 9.0).sin() * 1.5;
                if bright {
                    (1, 9.0, target_speed)
                } else {
                    (0, 0.5, target_speed)
                }
            }
            _ => {
                self.time_since_cut += dt;
                let time = self.time_since_cut;
                let light = if time < 0.3 {
                    9.0
                } else if time < 2.6 {
                    let fade = 1.0 - (time - 0.3) / 2.3;
                    let flicker = (time * 37.0).sin() * (time * 13.3).sin() > -0.2;
                    7.0 * fade * fade * if flicker { 1.0 } else { 0.08 }
                } else {
                    0.0
                };
                (2, light, 0.0)
            }
        };

        let response = if self.mode == CUT { 1.6 } else { 3.0 };
        self.rotor_speed += (target_speed - self.rotor_speed) * (dt * response).min(1.0);
        let rotor_rotation = if self.rotor_speed.abs() > 0.001 {
            self.rotor_speed * dt
        } else {
            0.0
        };

        let mut door_rotation = 0.0;
        if (self.door_target - self.door_angle).abs() > 1e-4 || self.door_speed.abs() > 0.001 {
            self.door_speed +=
                (-(self.door_angle - self.door_target) * 30.0 - self.door_speed * 3.2) * dt;
            let mut next_angle = self.door_angle + self.door_speed * dt;
            if next_angle < 0.0 {
                next_angle = 0.0;
                self.door_speed = self.door_speed.abs() * 0.3;
            }
            if next_angle > 1.9 {
                next_angle = 1.9;
                self.door_speed = -self.door_speed.abs() * 0.4;
            }
            door_rotation = -(next_angle - self.door_angle);
            self.door_angle = next_angle;
        }
        (color, light, rotor_rotation, door_rotation)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn relay_frame_buffer() -> *mut f64 {
    RELAY_FRAME.with_borrow_mut(|frame| frame.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn relay_frame(initial_seed: i32, command: i32, dt: f64) -> f64 {
    RELAY_FRAME.with_borrow_mut(|frame| {
        if command == -2 {
            *frame = [0.0; 10];
            frame[1] = f64::from(initial_seed % 7) * 0.9;
            frame[3] = 0.7;
            return 0.0;
        }
        let mut relay = Relay::load(frame);
        if command >= 0 {
            relay.change_mode(f64::from(command));
            frame[..7].copy_from_slice(&[
                relay.mode,
                relay.time,
                relay.time_since_cut,
                relay.rotor_speed,
                relay.door_angle,
                relay.door_speed,
                relay.door_target,
            ]);
            return 0.0;
        }
        let (color, light, rotor_rotation, door_rotation) = relay.update(dt);
        *frame = [
            relay.mode,
            relay.time,
            relay.time_since_cut,
            relay.rotor_speed,
            relay.door_angle,
            relay.door_speed,
            relay.door_target,
            f64::from(color),
            rotor_rotation,
            door_rotation,
        ];
        light
    })
}
