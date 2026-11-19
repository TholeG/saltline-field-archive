use glam::Vec2;

pub const POWER: Vec2 = Vec2::new(-8.0, 2.15);
pub const WINCH: Vec2 = Vec2::new(14.5, -1.65);
pub const SIGNAL: Vec2 = Vec2::new(15.9, 1.62);
const START: Vec2 = Vec2::new(-11.2, 4.3);
const WALK_RADIUS: f32 = 0.24;
const OPERATE_DISTANCE: f32 = 1.65;
const ALIGN_TOLERANCE: f32 = 0.14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Power,
    Gate,
    Signal,
    Voyage,
    Complete,
}

impl Stage {
    pub fn station(self) -> Option<(Vec2, &'static str)> {
        match self {
            Self::Power => Some((POWER, "Power")),
            Self::Gate => Some((WINCH, "Winch")),
            Self::Signal => Some((SIGNAL, "Signal")),
            Self::Voyage | Self::Complete => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const fn new(x0: f32, z0: f32, x1: f32, z1: f32) -> Self {
        Self {
            min: Vec2::new(x0, z0),
            max: Vec2::new(x1, z1),
        }
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }
}

pub const WALKABLE: [Rect; 3] = [
    Rect::new(-15.5, -8.0, 0.0, 8.5),
    Rect::new(-2.0, -1.6, 14.9, 1.6),
    Rect::new(12.2, -3.0, 17.0, 3.0),
];

pub const SHEDS: [Rect; 3] = [
    Rect::new(-13.5, -5.9, -9.0, -2.0),
    Rect::new(-7.25, -6.8, -3.7, -3.8),
    Rect::new(-7.4, 4.6, -3.0, 7.2),
];

pub const CRATE_ROWS: [(f32, f32, usize); 3] = [(-13.8, -0.8, 2), (-4.5, -3.0, 3), (-1.7, -6.2, 2)];

fn clear_of(point: Vec2, area: Rect) -> bool {
    let closest = point.clamp(area.min, area.max);
    point.distance_squared(closest) > WALK_RADIUS * WALK_RADIUS
}

pub fn can_walk(point: Vec2) -> bool {
    [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
        .iter()
        .all(|direction| {
            WALKABLE
                .iter()
                .any(|area| area.contains(point + direction * WALK_RADIUS))
        })
        && SHEDS.iter().all(|shed| clear_of(point, *shed))
        && CRATE_ROWS.into_iter().all(|(x, z, crates)| {
            (0..crates).all(|i| {
                let x = x + i as f32 * 0.86;
                clear_of(point, Rect::new(x - 0.36, z - 0.38, x + 0.36, z + 0.38))
            })
        })
        && [POWER, WINCH, SIGNAL]
            .into_iter()
            .all(|station| point.distance_squared(station) >= 0.57 * 0.57)
}

#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub walk: Vec2,
    pub turn: f32,
    pub interact: bool,
}

pub struct World {
    pub player: Vec2,
    pub facing: f32,
    pub camera_angle: f32,
    pub stage: Stage,
    pub progress: f32,
    pub signal_angle: f32,
    pub voyage_time: f32,
    pub walking: bool,
}

impl World {
    pub fn new() -> Self {
        Self {
            player: START,
            facing: 1.7,
            camera_angle: 0.0,
            stage: Stage::Power,
            progress: 0.0,
            signal_angle: -0.76,
            voyage_time: 0.0,
            walking: false,
        }
    }

    pub fn near_station(&self) -> bool {
        self.stage.station().is_some_and(|(point, _)| {
            self.player.distance_squared(point) <= OPERATE_DISTANCE * OPERATE_DISTANCE
        })
    }

    pub fn alignment(&self) -> i8 {
        if self.signal_angle < -ALIGN_TOLERANCE {
            -1
        } else if self.signal_angle > ALIGN_TOLERANCE {
            1
        } else {
            0
        }
    }

    pub fn gate_opening(&self) -> f32 {
        match self.stage {
            Stage::Power => 0.0,
            Stage::Gate => self.progress,
            Stage::Signal | Stage::Voyage | Stage::Complete => 1.0,
        }
    }

    pub fn update(&mut self, dt: f32, controls: Controls) -> bool {
        self.camera_angle = (self.camera_angle + controls.turn).rem_euclid(std::f32::consts::TAU);
        let forward = Vec2::new(-self.camera_angle.sin(), -self.camera_angle.cos());
        let right = Vec2::new(-forward.y, forward.x);
        let direction = if self.stage == Stage::Signal && controls.interact && self.near_station() {
            let speed = 0.55 + 0.8 * (self.signal_angle.abs() / 0.55).min(1.0);
            self.signal_angle = (self.signal_angle + controls.walk.x * dt * speed).clamp(-1.2, 1.2);
            Vec2::ZERO
        } else {
            (controls.walk.x * right + controls.walk.y * forward).clamp_length_max(1.0)
        };
        let before = self.player;
        if direction.length_squared() > 0.0 {
            self.facing = direction.x.atan2(direction.y);
            let delta = direction * 5.6 * dt;
            let sideways = self.player + Vec2::new(delta.x, 0.0);
            if can_walk(sideways) {
                self.player.x = sideways.x;
            }
            let forward = self.player + Vec2::new(0.0, delta.y);
            if can_walk(forward) {
                self.player.y = forward.y;
            }
        }
        self.walking = before != self.player;

        if matches!(self.stage, Stage::Voyage | Stage::Complete) {
            self.voyage_time += dt;
            if self.stage == Stage::Voyage && self.voyage_time >= 4.4 {
                self.stage = Stage::Complete;
                return true;
            }
            return false;
        }

        let (duration, next) = match self.stage {
            Stage::Power => (0.55, Stage::Gate),
            Stage::Gate => (1.15, Stage::Signal),
            Stage::Signal => (0.6, Stage::Voyage),
            Stage::Voyage | Stage::Complete => unreachable!(),
        };
        if !controls.interact
            || !self.near_station()
            || (self.stage == Stage::Signal && self.alignment() != 0)
        {
            self.progress = (self.progress - dt * 1.8).max(0.0);
            return false;
        }
        self.progress += dt / duration;
        if self.progress >= 1.0 {
            self.stage = next;
            self.progress = 0.0;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoreline_and_sheds_block_movement_but_pier_links_both_sides() {
        assert!(can_walk(START));
        assert!(!can_walk(Vec2::new(0.3, 4.5)));
        assert!(!can_walk(Vec2::new(-10.0, -4.0)));
        assert!(!can_walk(Vec2::new(-13.8, -0.8)));
        for x in -11..16 {
            assert!(can_walk(Vec2::new(x as f32, 0.0)));
        }
    }

    #[test]
    fn gate_and_signal_require_order_range_and_alignment() {
        let mut world = World::new();
        let controls = Controls {
            interact: true,
            ..Default::default()
        };
        assert!(!world.update(2.0, controls));
        assert_eq!(world.progress, 0.0);
        world.player = Vec2::new(14.5, -0.5);
        world.update(2.0, controls);
        assert_eq!(world.stage, Stage::Power);
        world.player = POWER + Vec2::Y;
        world.update(0.6, controls);
        assert_eq!(world.stage, Stage::Gate);
        world.player = Vec2::new(14.5, -0.5);
        assert!(!world.update(0.5, controls));
        assert!(world.progress > 0.0);
        world.update(0.7, controls);
        assert_eq!(world.stage, Stage::Signal);
        assert_eq!(world.gate_opening(), 1.0);
        world.player = SIGNAL - Vec2::Y;
        world.update(1.0, controls);
        assert_eq!(world.stage, Stage::Signal);
        for _ in 0..60 {
            if world.alignment() == 0 {
                break;
            }
            world.update(
                1.0 / 60.0,
                Controls {
                    walk: Vec2::X,
                    ..controls
                },
            );
        }
        assert_eq!(world.alignment(), 0);
        assert_eq!(world.player, SIGNAL - Vec2::Y);
        world.update(0.6, controls);
        assert_eq!(world.stage, Stage::Voyage);
        assert!(!world.update(4.0, controls));
        assert!(world.update(0.5, controls));
        assert_eq!(world.stage, Stage::Complete);
    }
}
