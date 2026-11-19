// Tube tessellation follows Three.js r180. See licenses/threejs-MIT.txt.
use std::cell::RefCell;

#[derive(Default)]
struct Tube {
    frames: Vec<f64>,
    ring: Vec<(f64, f64, f32)>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static TUBE: RefCell<Tube> = RefCell::new(Tube::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn tube_frames(rows: usize) -> *mut f64 {
    TUBE.with_borrow_mut(|tube| {
        tube.frames.resize(rows * 9, 0.0);
        tube.frames.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn sweep_tube(
    rows: usize,
    radial_segments: usize,
    radius: f64,
    length: f64,
    closed: bool,
    uv_scale: f64,
) -> *const f32 {
    TUBE.with_borrow_mut(|tube| {
        let stride = radial_segments + 1;
        let vertex_count = (rows + 1) * stride;
        tube.attributes.resize(vertex_count * 8, 0.0);
        tube.indices.clear();
        if tube.ring.len() != stride {
            tube.ring.clear();
            for segment in 0..=radial_segments {
                let fraction = segment as f64 / radial_segments as f64;
                let (sine, cosine) = (fraction * std::f64::consts::TAU).sin_cos();
                tube.ring.push((-cosine, sine, fraction as f32));
            }
        }
        let (positions, rest) = tube.attributes.split_at_mut(vertex_count * 3);
        let (normals, uvs) = rest.split_at_mut(vertex_count * 3);
        for row in 0..=rows {
            let frame = if closed && row == rows { 0 } else { row };
            let start = frame * 9;
            let [px, py, pz, nx, ny, nz, bx, by, bz] =
                std::array::from_fn(|i| tube.frames[start + i]);
            let u = f64::from((row as f64 / rows as f64) as f32) * length;
            for (segment, &(cosine, sine, v)) in tube.ring.iter().enumerate() {
                let [x, y, z] = [
                    cosine * nx + sine * bx,
                    cosine * ny + sine * by,
                    cosine * nz + sine * bz,
                ];
                let magnitude = (x * x + y * y + z * z).sqrt();
                let inverse = if magnitude == 0.0 {
                    1.0
                } else {
                    1.0 / magnitude
                };
                let [x, y, z] = [x * inverse, y * inverse, z * inverse];
                let vertex = row * stride + segment;
                positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    (px + radius * x) as f32,
                    (py + radius * y) as f32,
                    (pz + radius * z) as f32,
                ]);
                normals[vertex * 3..vertex * 3 + 3]
                    .copy_from_slice(&[x as f32, y as f32, z as f32]);
                let v = if uv_scale.is_nan() {
                    f64::from(v) * std::f64::consts::PI * 2.0 * radius
                } else {
                    f64::from(v) * uv_scale
                };
                uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&[u as f32, v as f32]);
            }
        }

        for row in 0..rows {
            for segment in 0..radial_segments {
                let a = (row * stride + segment) as u16;
                let b = a + stride as u16;
                tube.indices.extend([a, b, a + 1, b, b + 1, a + 1]);
            }
        }
        tube.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tube_indices() -> *const u16 {
    TUBE.with_borrow(|tube| tube.indices.as_ptr())
}
