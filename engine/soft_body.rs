use std::cell::RefCell;

type Point = [f64; 3];
type Vertex = [f32; 3];

#[derive(Default)]
struct SoftMesh {
    positions: Vec<Vertex>,
    indices: Vec<u32>,
    ordered_keys: Vec<([i64; 3], usize)>,
    welds: Vec<usize>,
    sums: Vec<Option<Point>>,
    normals: Vec<Vertex>,
}

thread_local! {
    static SOFT_MESH: RefCell<SoftMesh> = RefCell::new(SoftMesh::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn soft_mesh_positions(vertices: usize) -> *mut f32 {
    SOFT_MESH.with_borrow_mut(|mesh| {
        mesh.positions.resize(vertices, [0.0; 3]);
        mesh.positions.as_mut_ptr().cast()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn soft_mesh_indices(count: usize) -> *mut u32 {
    SOFT_MESH.with_borrow_mut(|mesh| {
        mesh.indices.resize(count, 0);
        mesh.indices.as_mut_ptr()
    })
}

fn hash(x: i64, y: i64, z: i64) -> f64 {
    let mut seed = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    seed = (seed ^ (seed >> 13)).wrapping_mul(1274126177);
    f64::from(seed ^ (seed >> 16)) / 4294967296.0
}

pub(super) fn noise(x: f64, y: f64, z: f64) -> f64 {
    let base = [x.floor(), y.floor(), z.floor()];
    let [dx, dy, dz] = std::array::from_fn(|i| {
        let fraction = [x, y, z][i] - base[i];
        fraction * fraction * (3.0 - 2.0 * fraction)
    });
    let [x, y, z] = base.map(|coordinate| coordinate as i64);
    let lerp = |a, b, fraction| a + (b - a) * fraction;
    let layer = |z| {
        lerp(
            lerp(hash(x, y, z), hash(x + 1, y, z), dx),
            lerp(hash(x, y + 1, z), hash(x + 1, y + 1, z), dx),
            dy,
        )
    };
    lerp(layer(z), layer(z + 1), dz)
}

#[unsafe(no_mangle)]
pub extern "C" fn deform_sandbag(
    width: f64,
    height: f64,
    depth: f64,
    seed: f64,
    lean: f64,
    sag: f64,
) -> *const f32 {
    SOFT_MESH.with_borrow_mut(|mesh| {
        let half_height = height / 2.0;
        for position in &mut mesh.positions {
            let [mut x, mut y, mut z] = position.map(f64::from);
            let cx = x / (width / 2.0);
            let cy = y / half_height;
            let cz = z / (depth / 2.0);
            let distance = cx.abs();
            let lower_half_scale = if cy < 0.0 { 1.05 } else { 1.0 };
            z *= (1.0 - 0.22 * (distance * distance * distance.sqrt())) * lower_half_scale;
            y = (y + half_height) * (1.0 - 0.3 * (distance * distance * distance)) - half_height;
            if cy > 0.0 {
                y += 0.016
                    * (std::f64::consts::FRAC_PI_2 * cx).cos()
                    * (std::f64::consts::FRAC_PI_2 * cz).cos();
            }
            let above_base = cy > -0.9;
            y -= sag * 0.018 * distance * distance * f64::from(u8::from(above_base))
                + lean * x * 0.3;
            let variation = noise(x * 9.0 + seed, y * 9.0, z * 9.0 + seed) - 0.5;
            x += variation * 0.014;
            z += variation * 0.016;
            if above_base {
                y += variation * 0.014;
            }
            *position = [x as f32, y as f32, z as f32];
        }
        mesh.positions.as_ptr().cast()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn deform_carton(width: f64, height: f64, depth: f64, crush: f64) -> *const f32 {
    SOFT_MESH.with_borrow_mut(|mesh| {
        let pi = std::f64::consts::PI;
        for position in &mut mesh.positions {
            let [x, y, z] = position.map(f64::from);
            let level = (y / height).clamp(0.0, 1.0);
            let vertical_weight = level * level;
            let width_weight = 0.55 + 0.45 * (pi * x / width * 0.9 + 0.5).cos();
            let depth_weight = 0.8 + 0.2 * (pi * z / depth).cos();
            let displacement = (pi * level).sin();
            let sign = if x == 0.0 { x } else { x.signum() };
            *position = [
                (x + (0.12 * crush * height * level + 0.02 * crush * displacement * sign)) as f32,
                (y - crush * height * 0.32 * vertical_weight * width_weight * depth_weight) as f32,
                (z * (1.0 + 0.1 * crush * displacement)) as f32,
            ];
        }
        mesh.positions.as_ptr().cast()
    })
}

fn face_normal(a: Vertex, b: Vertex, c: Vertex) -> Point {
    let [ax, ay, az] = a.map(f64::from);
    let [bx, by, bz] = b.map(f64::from);
    let [cx, cy, cz] = c.map(f64::from);
    let (x, y, z) = (bx - ax, by - ay, bz - az);
    let (u, v, w) = (cx - ax, cy - ay, cz - az);
    [y * w - z * v, z * u - x * w, x * v - y * u]
}

#[unsafe(no_mangle)]
pub extern "C" fn smooth_mesh_normals(indexed: bool) -> *const f32 {
    SOFT_MESH.with_borrow_mut(|mesh| {
        let count = mesh.positions.len();
        mesh.ordered_keys.clear();
        for (i, position) in mesh.positions.iter().enumerate() {
            if position
                .iter()
                .any(|value| !value.is_finite() || value.abs() > 4e15)
            {
                return std::ptr::null();
            }
            let key = position.map(|value| (f64::from(value) * 2000.0 + 0.5).floor() as i64);
            mesh.ordered_keys.push((key, i));
        }
        mesh.ordered_keys.sort_unstable_by_key(|&(key, _)| key);
        mesh.welds.resize(count, 0);
        mesh.sums.clear();
        let mut previous = None;
        for &(key, vertex) in &mesh.ordered_keys {
            if previous != Some(key) {
                mesh.sums.push(None);
                previous = Some(key);
            }
            mesh.welds[vertex] = mesh.sums.len() - 1;
        }

        let triangles = if indexed { mesh.indices.len() } else { count } / 3;
        for face in 0..triangles {
            let vertices = std::array::from_fn(|i| {
                let index = face * 3 + i;
                if indexed {
                    mesh.indices[index] as usize
                } else {
                    index
                }
            });
            let [a, b, c] = vertices;
            if a >= count || b >= count || c >= count {
                return std::ptr::null();
            }
            let normal = face_normal(mesh.positions[a], mesh.positions[b], mesh.positions[c]);
            // Accumulate triangles in input order even when vertices share a position.
            for vertex in vertices {
                let sum = mesh.sums[mesh.welds[vertex]].get_or_insert([0.0; 3]);
                for (sum, component) in sum.iter_mut().zip(normal) {
                    *sum += component;
                }
            }
        }
        for sum in mesh.sums.iter_mut().flatten() {
            let [x, y, z] = *sum;
            let length = (x * x + y * y + z * z).sqrt();
            if length > 0.0 {
                *sum = sum.map(|value| value / length);
            }
        }
        // A vertex omitted by the index still has the original up-vector fallback.
        mesh.normals.clear();
        for &weld in &mesh.welds {
            let normal = mesh.sums[weld]
                .unwrap_or([0.0, 1.0, 0.0])
                .map(|value| value as f32);
            mesh.normals.push(normal);
        }
        mesh.normals.as_ptr().cast()
    })
}
