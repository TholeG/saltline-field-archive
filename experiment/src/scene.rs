use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};

use crate::world::{CRATE_ROWS, POWER, Rect, SHEDS, SIGNAL, Stage, WALKABLE, WINCH, World};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tint: [f32; 3],
    pub material: f32,
}

#[derive(Default)]
pub struct Mesh(pub Vec<Vertex>);

type Color = [f32; 3];
const STONE: Color = [0.39, 0.43, 0.46];
const DECK: Color = [0.59, 0.40, 0.27];
const PEACH: Color = [0.86, 0.38, 0.27];
const GOLD: Color = [1.0, 0.69, 0.26];
const TEAL: Color = [0.34, 0.77, 0.80];
const METAL: Color = [0.20, 0.28, 0.35];
const FOAM: Color = [0.85, 0.92, 0.86];
const CHANNEL_X: f32 = 20.7;

impl Mesh {
    fn triangle(&mut self, points: [Vec3; 3], color: Color, material: f32) {
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        let daylight = 0.71 + 0.29 * normal.dot(Vec3::new(-0.40, 0.78, 0.48)).max(0.0);
        let tint = color.map(|channel| {
            if material == 1.0 || material == 2.0 {
                channel
            } else {
                channel * daylight
            }
        });
        self.0.extend(points.map(|point| Vertex {
            position: point.to_array(),
            tint,
            material,
        }));
    }

    fn quad(&mut self, [a, b, c, d]: [Vec3; 4], color: Color, material: f32) {
        self.triangle([a, b, c], color, material);
        self.triangle([a, c, d], color, material);
    }

    fn block(&mut self, center: Vec3, size: Vec3, color: Color, material: f32) {
        self.oriented_block(center, size, 0.0, color, material);
    }

    fn oriented_block(&mut self, center: Vec3, size: Vec3, yaw: f32, color: Color, material: f32) {
        let (sin, cos) = yaw.sin_cos();
        let points = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ]
        .map(|p| {
            let [x, y, z] = (Vec3::from(p) * size * 0.5).to_array();
            center + Vec3::new(x * cos + z * sin, y, z * cos - x * sin)
        });
        for face in [
            [4, 7, 6, 5],
            [0, 1, 2, 3],
            [0, 4, 5, 1],
            [3, 2, 6, 7],
            [0, 3, 7, 4],
            [1, 5, 6, 2],
        ] {
            self.quad(face.map(|i| points[i]), color, material);
        }
    }

    fn column(&mut self, center: Vec3, radius: f32, height: f32, color: Color, material: f32) {
        let bottom = center - Vec3::Y * height * 0.5;
        let top = center + Vec3::Y * height * 0.5;
        for side in 0..10 {
            let edge = |n| {
                let angle = (n as f32) * std::f32::consts::TAU / 10.0;
                Vec3::new(angle.cos(), 0.0, angle.sin()) * radius
            };
            let a = edge(side);
            let b = edge(side + 1);
            self.quad([bottom + a, top + a, top + b, bottom + b], color, material);
            self.triangle([top, top + b, top + a], color, material);
        }
    }

    fn bar(&mut self, from: Vec3, to: Vec3, width: f32, color: Color) {
        let delta = to - from;
        if delta.y.abs() < 0.001 {
            self.oriented_block(
                (from + to) * 0.5,
                Vec3::new(width, width, delta.length()),
                delta.x.atan2(delta.z),
                color,
                0.0,
            );
        } else {
            // Upright diagonal braces in the X/Y plane.
            let side = Vec3::new(-delta.y, delta.x, 0.0).normalize_or_zero() * width * 0.5;
            let front = Vec3::Z * width * 0.5;
            for z in [-front, front] {
                self.quad(
                    [
                        from - side + z,
                        to - side + z,
                        to + side + z,
                        from + side + z,
                    ],
                    color,
                    0.0,
                );
            }
            self.quad(
                [
                    from + side - front,
                    to + side - front,
                    to + side + front,
                    from + side + front,
                ],
                color,
                0.0,
            );
            self.quad(
                [
                    from - side - front,
                    from - side + front,
                    to - side + front,
                    to - side - front,
                ],
                color,
                0.0,
            );
        }
    }
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn platform(mesh: &mut Mesh, area: Rect, height: f32, tint: Color, material: f32) {
    let size = area.max - area.min;
    let center = (area.max + area.min) * 0.5;
    mesh.block(
        v(center.x, -height * 0.5, center.y),
        v(size.x, height, size.y),
        tint,
        material,
    );
}

fn shed(mesh: &mut Mesh, area: Rect, wall: Color, roof: Color, height: f32) {
    let center = (area.min + area.max) / 2.0;
    let size = area.max - area.min;
    let shadow = Vec2::new(0.75, 1.06);
    mesh.quad(
        [
            v(area.min.x, 0.009, area.min.y),
            v(area.min.x + shadow.x, 0.009, area.max.y + shadow.y),
            v(area.max.x + shadow.x, 0.009, area.max.y + shadow.y),
            v(area.max.x + shadow.x, 0.009, area.min.y + shadow.y),
        ],
        [0.39, 0.43, 0.39],
        4.0,
    );
    mesh.block(
        v(center.x, 0.10, center.y),
        v(size.x + 0.18, 0.20, size.y + 0.18),
        STONE,
        0.0,
    );
    mesh.block(
        v(center.x, height * 0.5, center.y),
        v(size.x, height, size.y),
        wall,
        0.0,
    );

    let x0 = area.min.x - 0.18;
    let x1 = area.max.x + 0.18;
    let z0 = area.min.y - 0.16;
    let z1 = area.max.y + 0.16;
    let peak = height + 0.74;
    let y = height * 0.995;
    mesh.quad(
        [
            v(x0, y, z0),
            v(x0, y, z1),
            v(center.x, peak, z1),
            v(center.x, peak, z0),
        ],
        roof,
        0.0,
    );
    mesh.quad(
        [
            v(center.x, peak, z0),
            v(center.x, peak, z1),
            v(x1, y, z1),
            v(x1, y, z0),
        ],
        roof,
        0.0,
    );
    mesh.triangle(
        [v(x0, y, z1), v(x1, y, z1), v(center.x, peak, z1)],
        wall,
        0.0,
    );
    mesh.triangle(
        [v(x1, y, z0), v(x0, y, z0), v(center.x, peak, z0)],
        wall,
        0.0,
    );
    for fraction in [0.23, 0.77] {
        let x = area.min.x + size.x * fraction;
        mesh.block(
            v(x, height * 0.57, area.max.y + 0.065),
            v(0.76, 0.6, 0.13),
            FOAM,
            0.0,
        );
        mesh.block(
            v(x, height * 0.57, area.max.y + 0.14),
            v(0.63, 0.46, 0.04),
            METAL,
            0.0,
        );
        mesh.block(
            v(x, height * 0.57, area.max.y + 0.17),
            v(0.50, 0.31, 0.035),
            [0.62, 0.75, 0.70],
            0.0,
        );
        mesh.block(
            v(x, height * 0.57, area.max.y + 0.19),
            v(0.045, 0.34, 0.02),
            FOAM,
            0.0,
        );
    }
    mesh.block(
        v(center.x, 0.70, area.max.y + 0.06),
        v(0.77, 1.4, 0.15),
        METAL,
        0.0,
    );
}

pub fn harbor() -> Mesh {
    let mut mesh = Mesh::default();
    mesh.quad(
        [
            v(-220.0, -0.82, -220.0),
            v(-220.0, -0.82, 220.0),
            v(220.0, -0.82, 220.0),
            v(220.0, -0.82, -220.0),
        ],
        [0.23, 0.51, 0.56],
        1.0,
    );
    platform(&mut mesh, WALKABLE[0], 1.1, [0.49, 0.50, 0.44], 4.0);
    platform(&mut mesh, WALKABLE[1], 0.38, DECK, 3.0);
    platform(&mut mesh, WALKABLE[2], 0.49, [0.60, 0.44, 0.30], 3.0);
    for row in 0..9 {
        let z = row as f32 * 1.78 - 7.0;
        let tint = if row % 3 == 0 {
            STONE
        } else {
            [0.45, 0.46, 0.43]
        };
        mesh.block(v(-0.05, -0.46, z), v(0.48, 0.9, 1.55), tint, 0.0);
    }
    for plank in 0..42 {
        let x = plank as f32 * 0.4 - 1.8;
        let warmth = (plank % 5) as f32 * 0.018;
        mesh.block(
            v(x, 0.014, 0.0),
            v(0.388, 0.03, 3.17),
            DECK.map(|channel| channel + warmth),
            3.0,
        );
    }
    for x in [-1.0, 0.5, 2.5, 4.5, 6.5, 8.5, 10.5, 12.0, 15.9] {
        for z in [-1.4, 1.4] {
            mesh.column(v(x, -0.27, z), 0.12, 1.4, [0.36, 0.25, 0.21], 0.0);
            mesh.column(v(x, 0.45, z), 0.125, 0.13, FOAM, 0.0);
        }
    }
    for x in [0.1, 2.1, 4.1, 6.1, 8.1, 10.1, 12.1] {
        mesh.oriented_block(
            v(x, 0.039, 0.0),
            v(0.27, 0.013, 0.10),
            0.6,
            [0.87, 0.70, 0.46],
            0.0,
        );
        mesh.oriented_block(
            v(x, 0.039, 0.0),
            v(0.27, 0.013, 0.10),
            -0.6,
            [0.87, 0.70, 0.46],
            0.0,
        );
    }

    for (i, area) in SHEDS.into_iter().enumerate() {
        let walls = [[0.75, 0.45, 0.27], [0.41, 0.62, 0.62], [0.72, 0.66, 0.49]];
        shed(
            &mut mesh,
            area,
            walls[i],
            [0.28, 0.39, 0.43],
            1.85 + i as f32 * 0.13,
        );
    }
    for (x, z, rows) in CRATE_ROWS {
        for i in 0..rows {
            let x = x + i as f32 * 0.86;
            let tint = [0.52 + i as f32 * 0.045, 0.37, 0.23];
            mesh.block(v(x, 0.32, z), v(0.72, 0.64, 0.76), tint, 3.0);
            for side in [-0.26, 0.26] {
                mesh.block(
                    v(x + side, 0.34, z + 0.386),
                    v(0.07, 0.62, 0.04),
                    METAL,
                    0.0,
                );
            }
        }
    }
    for x in [-1.86, 10.65, 12.35, 15.55] {
        for z in [-1.32, 1.32] {
            mesh.column(v(x, 0.36, z), 0.075, 0.66, METAL, 0.0);
        }
    }
    for z in [-1.32, 1.32] {
        mesh.bar(v(10.65, 0.68, z), v(12.35, 0.68, z), 0.095, METAL);
        mesh.bar(v(10.65, 0.37, z), v(12.35, 0.37, z), 0.05, METAL);
    }
    for x in [-1.75, 5.4, 12.15] {
        mesh.block(v(x, -0.47, 0.0), v(0.3, 0.38, 3.46), METAL, 0.0);
    }

    mesh.block(v(POWER.x, 0.52, POWER.y), v(0.72, 1.04, 0.48), METAL, 0.0);
    mesh.block(
        v(POWER.x, 0.82, POWER.y + 0.25),
        v(0.49, 0.34, 0.05),
        [0.62, 0.65, 0.46],
        0.0,
    );
    for x in [POWER.x - 0.23, POWER.x + 0.23] {
        mesh.block(v(x, 0.07, POWER.y), v(0.19, 0.14, 0.61), STONE, 0.0);
    }
    mesh.block(v(SIGNAL.x, 0.15, SIGNAL.y), v(0.76, 0.30, 0.76), STONE, 0.0);
    mesh.column(v(SIGNAL.x, 0.90, SIGNAL.y), 0.15, 1.68, METAL, 0.0);
    mesh.column(v(SIGNAL.x, 1.30, SIGNAL.y), 0.19, 0.095, FOAM, 0.0);
    for i in 0..21 {
        let x = -14.6 + i as f32 * 0.67;
        mesh.block(
            v(x, 0.014, 2.22),
            v(0.31, 0.025, 0.055),
            [0.72, 0.68, 0.53],
            0.0,
        );
    }
    for (x, z) in [(-0.8, 2.9), (-9.0, 7.4), (-1.2, -3.2), (13.1, 2.35)] {
        mesh.column(v(x, 1.3, z), 0.082, 2.6, METAL, 0.0);
        mesh.block(v(x, 2.56, z + 0.16), v(0.15, 0.12, 0.42), METAL, 0.0);
        mesh.block(
            v(x, 2.48, z + 0.25),
            v(0.12, 0.075, 0.15),
            [0.99, 0.85, 0.60],
            2.0,
        );
    }
    for (x, z) in [
        (-13.8, 6.9),
        (-13.3, 7.2),
        (-12.9, 6.8),
        (-2.0, 5.5),
        (-2.4, 5.25),
        (-1.9, 5.1),
    ] {
        mesh.column(v(x, 0.30, z), 0.24, 0.6, [0.34, 0.48, 0.47], 0.0);
        for y in [0.16, 0.44] {
            mesh.column(v(x, y, z), 0.25, 0.043, STONE, 0.0);
        }
    }
    for side in [18.8, 22.6] {
        mesh.block(v(side, -0.28, -3.6), v(1.0, 1.15, 1.0), STONE, 0.0);
        mesh.block(v(side, 1.22, -3.6), v(0.35, 3.0, 0.40), PEACH, 0.0);
        mesh.block(v(side, 2.72, -3.6), v(0.55, 0.18, 0.60), FOAM, 0.0);
        mesh.column(v(side, 2.92, -3.6), 0.11, 0.24, GOLD, 2.0);
    }
    mesh.bar(v(18.8, 2.67, -3.6), v(22.6, 2.67, -3.6), 0.20, PEACH);
    mesh.bar(v(18.8, 2.09, -3.6), v(19.56, 2.67, -3.6), 0.14, PEACH);
    mesh.bar(v(22.6, 2.09, -3.6), v(21.84, 2.67, -3.6), 0.14, PEACH);
    mesh.block(v(WINCH.x, 0.20, WINCH.y), v(0.90, 0.40, 0.75), METAL, 0.0);
    mesh.column(v(WINCH.x, 0.76, WINCH.y), 0.12, 1.32, PEACH, 0.0);
    mesh.column(v(WINCH.x, 1.13, WINCH.y), 0.30, 0.13, PEACH, 0.0);
    mesh.block(
        v(WINCH.x, 0.027, WINCH.y + 0.77),
        v(1.68, 0.03, 0.80),
        [0.30, 0.34, 0.32],
        0.0,
    );

    // A breakwater gives the open channel a visible destination.
    for i in 0..16 {
        let x = -14.0 + i as f32 * 2.65;
        if (x - CHANNEL_X).abs() < 3.7 {
            continue;
        }
        let z = -17.0 - (i as f32 * 0.39).sin() * 2.5;
        let yaw = i as f32 * 0.72;
        let height = 0.8 + (i % 5) as f32 * 0.16;
        mesh.oriented_block(v(x, -0.54, z), v(3.2, height, 2.25), yaw, STONE, 4.0);
    }

    // Rock formations make each side of the channel recognizable during camera turns.
    for (x, z, size, yaw) in [
        (4.0, -11.0, 3.0, 0.6),
        (4.6, -10.6, 2.18, -0.3),
        (7.4, -13.2, 1.75, 0.22),
        (-16.8, -8.5, 2.5, 0.5),
        (24.5, 6.0, 1.25, 0.2),
        (24.8, 6.8, 2.1, -0.4),
    ] {
        mesh.oriented_block(
            v(x, -0.72, z),
            v(size, size * 0.61, size * 0.63),
            yaw,
            [0.43, 0.50, 0.45],
            0.0,
        );
        mesh.oriented_block(
            v(x + 0.16, -0.53, z - 0.12),
            v(size * 0.56, size * 0.68, size * 0.56),
            yaw + 0.32,
            STONE,
            0.0,
        );
    }
    mesh
}

pub fn actors(mesh: &mut Mesh, world: &World, time: f32) {
    mesh.0.clear();
    let a = world.facing;
    let px = world.player.x;
    let pz = world.player.y;
    let stride = if world.walking {
        (time * 13.0).sin() * 0.10
    } else {
        0.0
    };
    let bob = stride.abs() * 0.32;
    mesh.column(v(px, 0.040, pz), 0.35, 0.01, [0.29, 0.32, 0.31], 0.0);
    let offset = |x: f32, y, z: f32| {
        v(
            px + x * a.cos() + z * a.sin(),
            y + bob,
            pz + z * a.cos() - x * a.sin(),
        )
    };
    for side in [-1.0, 1.0] {
        mesh.oriented_block(
            offset(0.114 * side, 0.23, stride * side),
            v(0.155, 0.38, 0.19),
            a,
            METAL,
            0.0,
        );
        mesh.oriented_block(
            offset(0.114 * side, 0.09, 0.051 + stride * side),
            v(0.159, 0.13, 0.24),
            a,
            [0.22, 0.27, 0.29],
            0.0,
        );
        mesh.oriented_block(
            offset(0.27 * side, 0.60, -stride * side * 0.6),
            v(0.13, 0.37, 0.18),
            a,
            PEACH,
            0.0,
        );
    }
    mesh.oriented_block(offset(0.0, 0.64, 0.0), v(0.41, 0.49, 0.28), a, PEACH, 0.0);
    mesh.oriented_block(offset(0.0, 0.56, 0.0), v(0.43, 0.054, 0.29), a, GOLD, 0.0);
    mesh.oriented_block(
        offset(0.0, 0.99, 0.0),
        v(0.26, 0.23, 0.23),
        a,
        [0.87, 0.69, 0.49],
        0.0,
    );
    mesh.column(offset(0.0, 1.12, 0.0), 0.174, 0.08, GOLD, 0.0);
    mesh.column(offset(0.0, 1.17, 0.0), 0.138, 0.10, GOLD, 0.0);

    let powered = world.stage != Stage::Power;
    let cleared = matches!(world.stage, Stage::Voyage | Stage::Complete);
    let lamps = if cleared { TEAL } else { GOLD };
    let power_light = if powered { TEAL } else { PEACH };
    mesh.block(
        v(POWER.x - 0.10, 0.84, POWER.y + 0.28),
        v(0.11, 0.10, 0.04),
        power_light,
        2.0,
    );
    let lever = if powered { -0.32 } else { 0.32 };
    mesh.bar(
        v(POWER.x + 0.13, 0.75, POWER.y + 0.28),
        v(POWER.x + 0.13 + lever * 0.42, 0.92, POWER.y + 0.28),
        0.06,
        METAL,
    );

    let bearing = std::f32::consts::FRAC_PI_2 + world.signal_angle;
    let head = v(SIGNAL.x, 1.70, SIGNAL.y);
    let lamp_axis = v(bearing.sin(), 0.0, bearing.cos());
    mesh.oriented_block(head, v(0.37, 0.32, 0.55), bearing, METAL, 0.0);
    mesh.oriented_block(
        head + lamp_axis * 0.29,
        v(0.30, 0.25, 0.06),
        bearing,
        lamps,
        2.0,
    );

    mesh.column(v(WINCH.x, 1.31, WINCH.y), 0.10, 0.23, lamps, 2.0);
    let gate_y = 0.39 + world.gate_opening() * 1.90;
    mesh.block(v(CHANNEL_X, gate_y, -3.6), v(3.6, 0.23, 0.23), lamps, 2.0);
    for x in [19.2, 19.8, 20.4, 21.0, 21.6, 22.2] {
        mesh.block(v(x, gate_y - 0.30, -3.6), v(0.08, 0.57, 0.095), METAL, 0.0);
    }

    let elapsed = world.voyage_time;
    let boat = Vec2::new(CHANNEL_X, 2.7 - (elapsed * 2.1).min(34.0));
    let float = (time * 2.1).sin() * 0.028;
    let b = |x, y, z| v(boat.x + x, y + float, boat.y + z);
    mesh.block(b(0.0, -0.59, 0.0), v(1.68, 0.53, 2.78), PEACH, 0.0);
    mesh.block(b(0.0, -0.28, 0.0), v(1.62, 0.12, 2.74), FOAM, 0.0);
    mesh.triangle(
        [
            b(-0.84, -0.28, -1.39),
            b(0.84, -0.28, -1.39),
            b(0.0, -0.28, -2.03),
        ],
        FOAM,
        0.0,
    );
    mesh.triangle(
        [
            b(-0.84, -0.59, -1.39),
            b(-0.84, -0.28, -1.39),
            b(0.0, -0.28, -2.03),
        ],
        PEACH,
        0.0,
    );
    mesh.triangle(
        [
            b(0.84, -0.28, -1.39),
            b(0.84, -0.59, -1.39),
            b(0.0, -0.28, -2.03),
        ],
        PEACH,
        0.0,
    );
    mesh.block(
        b(0.0, 0.12, 0.28),
        v(1.16, 0.68, 1.26),
        [0.90, 0.86, 0.69],
        0.0,
    );
    mesh.block(b(0.0, 0.50, 0.28), v(1.24, 0.13, 1.32), METAL, 0.0);
    mesh.block(
        b(0.0, 0.27, -0.36),
        v(1.01, 0.19, 0.03),
        [0.53, 0.75, 0.74],
        0.0,
    );
    for side in [-0.6, 0.6] {
        for end in [-0.12, 0.58] {
            mesh.block(
                b(side, 0.29, end),
                v(0.035, 0.26, 0.39),
                [0.42, 0.66, 0.69],
                0.0,
            );
        }
        mesh.column(
            b(side, 0.52, -0.30),
            0.043,
            0.12,
            if side < 0.0 { PEACH } else { TEAL },
            2.0,
        );
    }
    mesh.column(b(0.0, 0.77, 0.45), 0.042, 0.51, METAL, 0.0);
    mesh.column(b(0.0, 1.08, 0.45), 0.075, 0.10, lamps, 2.0);

    if elapsed > 0.0 {
        mesh.bar(
            b(-0.51, -0.80, 1.1),
            b(-1.63, -0.80, 4.1),
            0.035,
            [0.62, 0.83, 0.82],
        );
        mesh.bar(
            b(0.51, -0.80, 1.1),
            b(1.63, -0.80, 4.1),
            0.035,
            [0.62, 0.83, 0.82],
        );
    }
}
