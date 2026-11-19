use super::{hash, to_byte};
use std::cell::RefCell;

thread_local! {
    static PIXELS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[inline]
fn smooth(fraction: f64) -> f64 {
    fraction * fraction * (3.0 - 2.0 * fraction)
}

#[inline]
fn interpolate(corners: &[f64], offset: usize, stride: usize, sx: f64, sz: f64) -> f64 {
    let a = corners[offset];
    let b = corners[offset + 1];
    let c = corners[offset + stride];
    let d = corners[offset + stride + 1];
    a + (b - a) * sx + (c - a) * sz + (a - b - c + d) * sx * sz
}

struct Octave {
    period: usize,
    stride: usize,
    corners: Vec<f64>,
    axis: Vec<(usize, f64)>,
    weight: f64,
}

struct PeriodicNoise {
    octaves: Vec<Octave>,
    total_weight: f64,
}

impl PeriodicNoise {
    fn new(resolution: usize, scale: usize, seed: u32, count: u32) -> Self {
        let mut octaves = Vec::with_capacity(count as usize);
        let mut total_weight = 0.0;
        let mut weight = 0.5;
        for octave in 0..count {
            let period = scale << octave;
            let stride = period + 1;
            let mut corners = Vec::with_capacity(stride * stride);

            // Duplicate the first row and column at the seam for contiguous lookups.
            for z in 0..=period {
                for x in 0..=period {
                    corners.push(hash(
                        (x % period) as i32,
                        (z % period) as i32,
                        seed + octave * 17,
                    ));
                }
            }
            let axis = (0..resolution)
                .map(|sample| {
                    let coordinate = (sample as f64 / resolution as f64) * period as f64;
                    let lattice = coordinate.floor();
                    (lattice as usize, smooth(coordinate - lattice))
                })
                .collect();
            octaves.push(Octave {
                period,
                stride,
                corners,
                axis,
                weight,
            });
            total_weight += weight;
            weight *= 0.5;
        }
        Self {
            octaves,
            total_weight,
        }
    }

    fn row(&self, z: usize, target: &mut [f64]) {
        target.fill(0.0);
        for octave in &self.octaves {
            let (iz, sz) = octave.axis[z];
            let row_offset = iz * octave.stride;
            for (sample, &(x, sx)) in target.iter_mut().zip(&octave.axis) {
                *sample += octave.weight
                    * interpolate(&octave.corners, row_offset + x, octave.stride, sx, sz);
            }
        }
        for sample in target {
            *sample /= self.total_weight;
        }
    }

    fn at(&self, mut x: f64, mut z: f64) -> f64 {
        let mut sum = 0.0;
        for octave in &self.octaves {
            let ix = x.floor();
            let iz = z.floor();
            let lattice_x = (ix as i32).rem_euclid(octave.period as i32) as usize;
            let lattice_z = (iz as i32).rem_euclid(octave.period as i32) as usize;
            let offset = lattice_z * octave.stride + lattice_x;
            sum += octave.weight
                * interpolate(
                    &octave.corners,
                    offset,
                    octave.stride,
                    smooth(x - ix),
                    smooth(z - iz),
                );
            x *= 2.0;
            z *= 2.0;
        }
        sum / self.total_weight
    }
}

struct Neighbors {
    minus_one: usize,
    plus_one: usize,
    minus_two: usize,
    plus_two: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn render_detail(resolution: usize) -> *const u8 {
    PIXELS.with_borrow_mut(|pixels| {
        let count = resolution * resolution;
        pixels.resize(count * 8, 0);
        let (detail, normals) = pixels.split_at_mut(count * 4);
        let mut fine_heights = vec![0.0f32; count];
        let mut rough_heights = vec![0.0f32; count];
        let fields = [
            (96, 3, 2),
            (40, 9, 2),
            (6, 21, 4),
            (8, 31, 2),
            (8, 37, 2),
            (28, 51, 3),
            (9, 71, 4),
        ]
        .map(|(scale, seed, count)| PeriodicNoise::new(resolution, scale, seed, count));
        let mut samples = fields.each_ref().map(|_| vec![0.0; resolution]);
        let ridges = PeriodicNoise::new(resolution, 5, 43, 3);
        let neighbors: Vec<_> = (0..resolution)
            .map(|i| Neighbors {
                minus_one: (i + resolution - 1) % resolution,
                plus_one: (i + 1) % resolution,
                minus_two: (i + resolution - 2) % resolution,
                plus_two: (i + 2) % resolution,
            })
            .collect();

        for row in 0..resolution {
            for (field, samples) in fields.iter().zip(&mut samples) {
                field.row(row, samples);
            }
            let [fine, coarse, patches, warp_x, warp_z, speckles, rough] = &samples;
            let z = row as f64 / resolution as f64;
            for col in 0..resolution {
                let index = row * resolution + col;
                let x = col as f64 / resolution as f64;
                let height = fine[col] * 0.6 + coarse[col] * 0.4;
                let ridge = ridges.at(
                    (x + (warp_x[col] - 0.5) * 0.25) * 5.0,
                    (z + (warp_z[col] - 0.5) * 0.25) * 5.0,
                );
                detail[index * 4] = to_byte(height);
                detail[index * 4 + 1] = to_byte(patches[col]);
                detail[index * 4 + 2] = to_byte(1.0 - (2.0 * ridge - 1.0).abs());
                detail[index * 4 + 3] = to_byte(speckles[col]);
                fine_heights[index] = height as f32;
                rough_heights[index] = rough[col] as f32;
            }
        }

        // The original stores heights in Float32Array before subtracting them as doubles.
        for (row, z) in neighbors.iter().enumerate() {
            let offset = row * resolution;
            for (col, x) in neighbors.iter().enumerate() {
                let pixel = (offset + col) * 4;
                let fine_x = (f64::from(fine_heights[offset + x.plus_one])
                    - f64::from(fine_heights[offset + x.minus_one]))
                    * 2.2;
                let fine_z = (f64::from(fine_heights[z.plus_one * resolution + col])
                    - f64::from(fine_heights[z.minus_one * resolution + col]))
                    * 2.2;
                let rough_x = (f64::from(rough_heights[offset + x.plus_two])
                    - f64::from(rough_heights[offset + x.minus_two]))
                    * 9.0;
                let rough_z = (f64::from(rough_heights[z.plus_two * resolution + col])
                    - f64::from(rough_heights[z.minus_two * resolution + col]))
                    * 9.0;
                normals[pixel] = to_byte(0.5 - fine_x);
                normals[pixel + 1] = to_byte(0.5 - fine_z);
                normals[pixel + 2] = to_byte(0.5 - rough_x);
                normals[pixel + 3] = to_byte(0.5 - rough_z);
            }
        }
        pixels.as_ptr()
    })
}
