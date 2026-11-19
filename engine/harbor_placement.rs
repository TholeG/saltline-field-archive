use std::cell::RefCell;
use std::f64::consts::PI;

type Point = [f64; 3];

#[derive(Default)]
struct Placement {
    transform: [f64; 16],
    input: Vec<f64>,
    colliders: Vec<f64>,
    stack: Vec<f64>,
    footprint: [f64; 4],
    slab: [f64; 6],
    rod: [f64; 8],
}

thread_local! {
    static PLACEMENT: RefCell<Placement> = RefCell::new(Placement::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_placement_input(length: usize) -> *mut f64 {
    PLACEMENT.with_borrow_mut(|placement| {
        placement.input.resize(length, 0.0);
        placement.input.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_world_matrix() -> *mut f64 {
    PLACEMENT.with_borrow_mut(|placement| placement.transform.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_slab(
    x: f64,
    y: f64,
    z: f64,
    end_x: f64,
    end_y: f64,
    end_z: f64,
) -> *const f64 {
    PLACEMENT.with_borrow_mut(|placement| {
        placement.slab = [
            (end_x - x).abs(),
            (end_y - y).abs(),
            (end_z - z).abs(),
            (x + end_x) / 2.0,
            (y + end_y) / 2.0,
            (z + end_z) / 2.0,
        ];
        placement.slab.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_rod(
    x: f64,
    y: f64,
    z: f64,
    end_x: f64,
    end_y: f64,
    end_z: f64,
) -> *const f64 {
    let (dx, dy, dz) = (end_x - x, end_y - y, end_z - z);
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    let scale = if length == 0.0 { 1.0 } else { length.recip() };
    let (dx, dy, dz) = (dx * scale, dy * scale, dz * scale);
    let w = dy + 1.0;
    let [qx, qy, qz, qw] = if w < 1e-8 {
        [0.0, 0.0, 1.0, 0.0]
    } else {
        let unit = (dz * dz + dx * dx + w * w).sqrt().recip();
        [dz * unit, 0.0, -dx * unit, w * unit]
    };
    PLACEMENT.with_borrow_mut(|placement| {
        placement.rod = [
            length,
            (x + end_x) * 0.5,
            (y + end_y) * 0.5,
            (z + end_z) * 0.5,
            qx,
            qy,
            qz,
            qw,
        ];
        placement.rod.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_placement_kind(kind: u32, configuration: u32) -> u32 {
    let inert = configuration & 1 != 0;
    let skip_collision = configuration & 2 != 0;
    let dynamic = configuration & 4 != 0;
    // A separate gameplay system owns live explosives. Small props have no world collision.
    let live_barrel = kind == 1 && configuration & (8 | 16 | 32) == 8;
    if !inert && (live_barrel || kind == 2) {
        return 1;
    }
    (u32::from(dynamic) << 1) | (u32::from(!skip_collision && kind != 3) << 2)
}

struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    fn transformed(&self, matrix: &[f64; 16]) -> Bounds {
        let mut result = Bounds {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        };
        for x in [self.min[0], self.max[0]] {
            for y in [self.min[1], self.max[1]] {
                for z in [self.min[2], self.max[2]] {
                    let w = 1.0 / (matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15]);
                    for axis in 0..3 {
                        let value = (matrix[axis] * x
                            + matrix[axis + 4] * y
                            + matrix[axis + 8] * z
                            + matrix[axis + 12])
                            * w;
                        result.min[axis] = result.min[axis].min(value);
                        result.max[axis] = result.max[axis].max(value);
                    }
                }
            }
        }
        result
    }

    fn narrow_footprint(&mut self) {
        for axis in [0, 2] {
            let margin = (self.max[axis] - self.min[axis]) * 0.07;
            self.min[axis] += margin;
            self.max[axis] -= margin;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_bake_colliders(yaw: f64) -> usize {
    PLACEMENT.with_borrow_mut(|placement| {
        placement.colliders.clear();
        let diagonal = (2.0 * yaw).sin().abs() > 0.2;
        for (index, data) in placement.input.chunks_exact(6).enumerate() {
            let min = [data[0], data[1], data[2]];
            let max = [data[3], data[4], data[5]];
            let width = max[0] - min[0];
            let depth = max[2] - min[2];
            let major_axis = if width >= depth { 0 } else { 2 };
            let major_length = width.max(depth);
            let slices = if diagonal {
                (major_length / width.min(depth).max(0.25))
                    .round()
                    .clamp(1.0, 10.0) as u32
            } else {
                1
            };
            for slice in 0..slices {
                let mut local = Bounds { min, max };
                if slices > 1 {
                    let axis = major_axis;
                    local.min[axis] =
                        min[axis] + major_length * f64::from(slice) / f64::from(slices);
                    local.max[axis] =
                        min[axis] + major_length * f64::from(slice + 1) / f64::from(slices);
                }
                let mut world = local.transformed(&placement.transform);
                if world.max[1] - world.min[1] < 0.2 && world.max[1] < 0.3 {
                    continue;
                }
                if diagonal {
                    world.narrow_footprint();
                }
                placement.colliders.extend(world.min);
                placement.colliders.extend(world.max);
                placement.colliders.push(index as f64);
            }
        }
        placement.colliders.len() / 7
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_colliders() -> *const f64 {
    PLACEMENT.with_borrow(|placement| placement.colliders.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_footprint(x: f64, z: f64, yaw: f64, narrow_half_width: f64) -> *const f64 {
    PLACEMENT.with_borrow_mut(|placement| {
        let along_x = yaw.cos().abs() > 0.5;
        let (half_x, half_z) = if along_x {
            (1.5, narrow_half_width)
        } else {
            (narrow_half_width, 1.5)
        };
        placement.footprint = [x - half_x, z - half_z, x + half_x, z + half_z];
        placement.footprint.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_stack(x: f64, z: f64, yaw: f64, jitter: f64) -> *const f64 {
    PLACEMENT.with_borrow_mut(|placement| {
        placement.stack.clear();
        let along_x = yaw.cos().abs() > 0.5;
        let mut height = 0.0;
        for (index, layer) in placement.input.chunks_exact(12).enumerate() {
            let [
                kind,
                dx,
                dz,
                flipped,
                doors,
                reefer,
                rand_x,
                rand_z,
                rand_yaw,
                seed,
                wear,
                rand_wear,
            ] = <[f64; 12]>::try_from(layer).unwrap();
            let (length, box_height) = match kind as u32 {
                0 => (6.06, 2.59),
                1 => (12.19, 2.59),
                _ => (12.19, 2.9),
            };
            let offset_x = dx
                + if index == 0 {
                    0.0
                } else {
                    (-0.05 + (0.05 - -0.05) * rand_x) * jitter
                };
            let offset_z = dz
                + if index == 0 {
                    0.0
                } else {
                    (-0.04 + (0.04 - -0.04) * rand_z) * jitter
                };
            let angle = yaw
                + (-1.0 + 2.0 * rand_yaw) * 0.022 * jitter * if index == 0 { 1.0 } else { 0.6 }
                + if flipped != 0.0 { PI } else { 0.0 };
            let wear = if wear.is_nan() {
                0.2 + (0.85 - 0.2) * rand_wear
            } else {
                wear
            };
            let (half_x, half_z) = if along_x {
                (length / 2.0, 2.44 / 2.0)
            } else {
                (2.44 / 2.0, length / 2.0)
            };
            let (reefer_x, reefer_z) = if reefer != 0.0 {
                let spread = -(length / 2.0 + 0.2);
                (
                    x + offset_x + spread * angle.cos(),
                    z + offset_z - spread * angle.sin(),
                )
            } else {
                (0.0, 0.0)
            };
            placement.stack.extend_from_slice(&[
                x + offset_x,
                height,
                z + offset_z,
                angle,
                (1.0 + 99998.0 * seed).floor(),
                wear,
                doors,
                x + offset_x - half_x,
                z + offset_z - half_z,
                x + offset_x + half_x,
                z + offset_z + half_z,
                reefer_x,
                height + 0.14,
                reefer_z,
            ]);
            height += box_height;
        }
        placement.stack.push(height);
        placement.stack.as_ptr()
    })
}
