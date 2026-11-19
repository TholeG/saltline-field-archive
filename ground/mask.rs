use std::cell::RefCell;

#[path = "../engine/gameplay/mod.rs"]
mod gameplay;
#[path = "../engine/kernels.rs"]
mod kernels;
#[path = "../engine/navigation.rs"]
mod navigation;
#[path = "../engine/physics.rs"]
mod physics;
#[path = "../engine/water.rs"]
mod water;

mod detail;

#[derive(Default)]
struct Buffers {
    basins: Vec<f64>,
    pixels: Vec<u8>,
}

thread_local! {
    static BUFFERS: RefCell<Buffers> = RefCell::new(Buffers::default());
}

struct Column {
    x: f64,
    in_aisle: bool,
    gutter_weight: f64,
    lane_weight: f64,
    inland: f64,
}

struct Row {
    z: f64,
    gutter: f64,
    lane: f64,
    active_basins: Vec<(usize, f64)>,
}

#[inline]
fn smoothstep(min: f64, max: f64, value: f64) -> f64 {
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn hash(x: i32, z: i32, seed: u32) -> f64 {
    let mut bits = (x as u32).wrapping_mul(374761393)
        ^ (z as u32).wrapping_mul(668265263)
        ^ seed.wrapping_mul(1274126177);
    bits = (bits ^ (bits >> 13)).wrapping_mul(1274126177);
    f64::from(bits ^ (bits >> 16)) / 4294967296.0
}

#[inline]
fn value_noise(x: f64, z: f64, seed: u32) -> f64 {
    let ix = x.floor();
    let iz = z.floor();
    let dx = x - ix;
    let dz = z - iz;
    let sx = dx * dx * (3.0 - 2.0 * dx);
    let sz = dz * dz * (3.0 - 2.0 * dz);
    let (ix, iz) = (ix as i32, iz as i32);
    let a = hash(ix, iz, seed);
    let b = hash(ix + 1, iz, seed);
    let c = hash(ix, iz + 1, seed);
    let d = hash(ix + 1, iz + 1, seed);
    a + (b - a) * sx + (c - a) * sz + (a - b - c + d) * sx * sz
}

#[inline]
fn fractal<const OCTAVES: u32>(mut x: f64, mut z: f64, seed: u32) -> f64 {
    let mut sum = 0.0;
    let mut weight = 0.5;
    let mut total_weight = 0.0;
    for octave in 0..OCTAVES {
        sum += weight * value_noise(x, z, seed + octave * 17);
        total_weight += weight;
        x *= 2.0;
        z *= 2.0;
        weight *= 0.5;
    }
    sum / total_weight
}

#[inline]
fn gaussian(value: f64) -> f64 {
    (-value * value).exp()
}

#[inline]
fn to_byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn basin_buffer(count: usize) -> *mut f64 {
    BUFFERS.with_borrow_mut(|buffers| {
        buffers.basins.resize(count * 5, 0.0);
        buffers.basins.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn render_mask(
    resolution: usize,
    x0: f64,
    z0: f64,
    size: f64,
    basin_width: f64,
    basin_depth: f64,
    waterline: f64,
) -> *const u8 {
    BUFFERS.with_borrow_mut(|buffers| {
        let basin_count = buffers.basins.len() / 5;
        let mut basin_x = vec![0.0; resolution * basin_count];
        let mut columns = Vec::with_capacity(resolution);
        let mut rows = Vec::with_capacity(resolution);
        let step = size / resolution as f64;
        for index in 0..resolution {
            let x = x0 + (index as f64 + 0.5) * step;
            let z = z0 + (index as f64 + 0.5) * step;
            let mut active_basins = Vec::new();
            for (basin, params) in buffers.basins.chunks_exact(5).enumerate() {
                let dx = (x - params[0]) / (params[2] * basin_width);
                let dz = (z - params[1]) / (params[3] * basin_width);
                let offset = index * basin_count + basin;
                basin_x[offset] = dx * dx;
                let distance_z = dz * dz;
                if distance_z < 9.0 {
                    active_basins.push((basin, distance_z));
                }
            }
            let in_aisle = x > -37.0 && x < 29.0;
            let (gutter_weight, lane_weight) = if in_aisle {
                let texture = 0.5 + 0.5 * value_noise(x / 6.0, 3.1, 5);
                let drain = fractal::<2>(x / 9.0, 7.7, 19);
                let gate = smoothstep(0.14, 0.34, drain)
                    * smoothstep(-27.0, -24.0, x)
                    * (1.0 - smoothstep(14.0, 17.0, x));
                (
                    0.22 * (0.55 + 0.45 * texture) * gate,
                    0.09 * texture * (1.0 - smoothstep(0.3, 0.6, drain)),
                )
            } else {
                (0.0, 0.0)
            };
            columns.push(Column {
                x,
                in_aisle,
                gutter_weight,
                lane_weight,
                inland: 0.25 * smoothstep(28.6, 29.8, x),
            });
            rows.push(Row {
                z,
                active_basins,
                gutter: gaussian((z - 4.9) / 0.5) + gaussian((z - 7.1) / 0.5),
                lane: gaussian((z - 15.4) / 0.5) + gaussian((z - 17.2) / 0.5),
            });
        }

        let strengths: Vec<_> = buffers
            .basins
            .chunks_exact(5)
            .map(|params| params[4] + basin_depth)
            .collect();
        buffers.pixels.resize(resolution * resolution * 4, 0);
        let mut pixels = buffers.pixels.chunks_exact_mut(4);
        for Row {
            z,
            gutter,
            lane,
            active_basins,
        } in &rows
        {
            let z = *z;
            for (col, column) in columns.iter().enumerate() {
                let x = column.x;
                let mut height = 0.5
                    + (fractal::<4>(x / 8.5, z / 8.5, 11) - 0.47) * 0.3
                    + (fractal::<3>(x / 2.4, z / 2.4, 23) - 0.5) * 0.14;
                let distances_x = &basin_x[col * basin_count..][..basin_count];
                for &(basin, distance_z) in active_basins {
                    let distance = distances_x[basin] + distance_z;
                    if distance < 9.0 {
                        height -= strengths[basin] * gaussian(distance);
                    }
                }
                if column.in_aisle {
                    height -= column.gutter_weight * gutter + column.lane_weight * lane;
                }
                height += column.inland;
                let pixel = pixels.next().unwrap();
                pixel[0] = to_byte(0.5 + (height - waterline) * 2.5);
                pixel[1] = to_byte(
                    0.5 + (fractal::<3>(x / 5.5, z / 5.5, 41) - 0.5) * 1.5 + (0.5 - height) * 0.9,
                );
                pixel[2] = to_byte(
                    smoothstep(0.6, 0.78, fractal::<4>(x / 1.4, z / 1.4, 77))
                        * smoothstep(0.47, 0.62, fractal::<2>(x / 9.0, z / 9.0, 91)),
                );
                pixel[3] = to_byte(
                    fractal::<3>(x / 22.0, z / 0.9, 57) * 0.6
                        + fractal::<3>(x / 13.0, z / 13.0, 63) * 0.4,
                );
            }
        }
        buffers.pixels.as_ptr()
    })
}
