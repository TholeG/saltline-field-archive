use std::cell::RefCell;

use super::mesh_normals::indexed_normals;

#[derive(Default)]
struct Extrusion {
    points: Vec<f64>,
    caps: Vec<u16>,
    edge_normals: Vec<[f64; 2]>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static EXTRUSION: RefCell<Extrusion> = RefCell::new(Extrusion::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn extrusion_points(length: usize) -> *mut f64 {
    EXTRUSION.with_borrow_mut(|mesh| {
        mesh.points.resize(length, 0.0);
        mesh.points.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn extrusion_caps(length: usize) -> *mut u16 {
    EXTRUSION.with_borrow_mut(|mesh| {
        mesh.caps.resize(length, 0);
        mesh.caps.as_mut_ptr()
    })
}

fn smooth(a: [f64; 2], b: [f64; 2], threshold: f64) -> [f64; 2] {
    if a[0] * b[0] + a[1] * b[1] <= threshold {
        return a;
    }
    let [x, y] = [a[0] + b[0], a[1] + b[1]];
    let magnitude = x.hypot(y);
    let divisor = if magnitude == 0.0 { 1.0 } else { magnitude };
    [x / divisor, y / divisor]
}

#[unsafe(no_mangle)]
pub extern "C" fn extrude_outline(depth: f64, threshold: f64, capped: bool) -> *const f32 {
    EXTRUSION.with_borrow_mut(|mesh| {
        let count = mesh.points.len() / 3;
        let vertices = count * if capped { 6 } else { 4 };
        let half = depth / 2.0;
        mesh.attributes.resize(vertices * 8, 0.0);
        mesh.indices.clear();
        mesh.edge_normals.clear();
        for edge in 0..count {
            let current = edge * 3;
            let next = (edge + 1) % count * 3;
            let [x, y] = [
                mesh.points[next] - mesh.points[current],
                mesh.points[next + 1] - mesh.points[current + 1],
            ];
            let length = mesh.points[current + 2];
            mesh.edge_normals.push([y / length, -x / length]);
        }

        let (positions, rest) = mesh.attributes.split_at_mut(vertices * 3);
        let (normals, uvs) = rest.split_at_mut(vertices * 3);
        let mut vertex = 0;
        let mut distance = 0.0;
        for edge in 0..count {
            let next = (edge + 1) % count;
            let previous = (edge + count - 1) % count;
            let base = vertex as u16;
            let end = distance + mesh.points[edge * 3 + 2];
            let normal = mesh.edge_normals[edge];
            let first = smooth(normal, mesh.edge_normals[previous], threshold);
            let second = smooth(normal, mesh.edge_normals[next], threshold);
            for (point, z, normal, u) in [
                (edge, half, first, distance),
                (edge, -half, first, distance),
                (next, -half, second, end),
                (next, half, second, end),
            ] {
                positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    mesh.points[point * 3] as f32,
                    mesh.points[point * 3 + 1] as f32,
                    z as f32,
                ]);
                normals[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    normal[0] as f32,
                    normal[1] as f32,
                    0.0,
                ]);
                uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&[u as f32, z as f32]);
                vertex += 1;
            }
            mesh.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            distance = end;
        }

        if capped {
            for sign in [1.0_f64, -1.0] {
                let base = vertex as u16;
                for point in mesh.points.chunks_exact(3) {
                    let (x, y) = (point[0] as f32, point[1] as f32);
                    positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                        x,
                        y,
                        (sign * half) as f32,
                    ]);
                    normals[vertex * 3..vertex * 3 + 3].copy_from_slice(&[0.0, 0.0, sign as f32]);
                    uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&[x, y]);
                    vertex += 1;
                }
                for face in mesh.caps.chunks_exact(3) {
                    let a = base + face[0];
                    let b = base + face[1];
                    let c = base + face[2];
                    mesh.indices
                        .extend(if sign > 0.0 { [a, b, c] } else { [a, c, b] });
                }
            }
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn sweep_ribbon(
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    double_sided: bool,
) -> *const f32 {
    EXTRUSION.with_borrow_mut(|mesh| {
        let count = mesh.points.len() / 4;
        let front = count * 2;
        let vertices = if double_sided { front * 2 } else { front };
        mesh.attributes.resize(vertices * 8, 0.0);
        mesh.indices.clear();
        let (positions, rest) = mesh.attributes.split_at_mut(vertices * 3);
        let (normals, uvs) = rest.split_at_mut(vertices * 3);
        let mut distance = 0.0;
        for (i, point) in mesh.points.chunks_exact(4).enumerate() {
            distance += point[3];
            positions[i * 6..i * 6 + 6].copy_from_slice(&[
                (point[0] - x) as f32,
                (point[1] - y) as f32,
                (point[2] - z) as f32,
                (point[0] + x) as f32,
                (point[1] + y) as f32,
                (point[2] + z) as f32,
            ]);
            uvs[i * 4..i * 4 + 4].copy_from_slice(&[
                distance as f32,
                0.0,
                distance as f32,
                width as f32,
            ]);
        }
        for segment in 0..count - 1 {
            let a = (segment * 2) as u16;
            mesh.indices.extend([a, a + 1, a + 3, a, a + 3, a + 2]);
        }
        indexed_normals(
            &positions[..front * 3],
            &mesh.indices,
            &mut normals[..front * 3],
        );
        if double_sided {
            positions.copy_within(..front * 3, front * 3);
            uvs.copy_within(..front * 2, front * 2);
            let (front_normals, back_normals) = normals.split_at_mut(front * 3);
            for (back, normal) in back_normals.iter_mut().zip(front_normals) {
                *back = -*normal;
            }
            let front = front as u16;
            let end = mesh.indices.len();
            for i in (0..end).step_by(3) {
                let [a, b, c] = [mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]];
                mesh.indices.extend([a + front, c + front, b + front]);
            }
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn extrusion_indices() -> *const u16 {
    EXTRUSION.with_borrow(|mesh| mesh.indices.as_ptr())
}
