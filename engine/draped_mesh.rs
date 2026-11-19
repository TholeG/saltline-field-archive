use std::cell::RefCell;

use super::mesh_normals::indexed_normals;
use super::soft_body::noise;

struct Edge {
    column: usize,
    row: usize,
    dx: f64,
    dz: f64,
}

#[derive(Default)]
struct Drape {
    input: Vec<f64>,
    boundary: Vec<Edge>,
    surface: Vec<[f64; 3]>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static DRAPE: RefCell<Drape> = RefCell::new(Drape::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn drape_input(length: usize) -> *mut f64 {
    DRAPE.with_borrow_mut(|mesh| {
        mesh.input.resize(length, 0.0);
        mesh.input.as_mut_ptr()
    })
}

fn make_boundary(edges: &mut Vec<Edge>, columns: usize, rows: usize) {
    edges.clear();
    for column in 0..columns {
        edges.push(Edge {
            column,
            row: 0,
            dx: 0.0,
            dz: -1.0,
        });
    }
    for row in 0..rows {
        edges.push(Edge {
            column: columns,
            row,
            dx: 1.0,
            dz: 0.0,
        });
    }
    for column in (1..=columns).rev() {
        edges.push(Edge {
            column,
            row: rows,
            dx: 0.0,
            dz: 1.0,
        });
    }
    for row in (1..=rows).rev() {
        edges.push(Edge {
            column: 0,
            row,
            dx: -1.0,
            dz: 0.0,
        });
    }
}

// Share the two-dimensional noise routine across all three cloth loops in the WASM build.
#[inline(never)]
fn cloth_noise(x: f64, z: f64) -> f64 {
    noise(x, z, 0.0)
}

fn height(x: f64, z: f64, seed: f64, supports: &[f64]) -> f64 {
    let mut y = 0.3_f64;
    for support in supports.chunks_exact(5) {
        let dx = (support[0] - x).max(0.0).max(x - support[1]);
        let dz = (support[2] - z).max(0.0).max(z - support[3]);
        let distance = dx.hypot(dz);
        y = y.max(support[4] - 0.95 * distance - 0.9 * distance * distance);
    }
    y + 0.016
        + 0.022 * cloth_noise(x * 5.0 + seed, z * 5.0)
        + 0.008 * (x * 23.0 + z * 7.0 + seed).sin()
        + 0.006 * (z * 31.0 - x * 5.0).sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn drape_mesh(
    columns: usize,
    rows: usize,
    skirt_rows: usize,
    supports: usize,
    min_x: f64,
    max_x: f64,
    min_z: f64,
    max_z: f64,
    limit_x: f64,
    limit_z: f64,
    strap_left: usize,
    strap_right: usize,
    uv_scale: f64,
    seed: f64,
) -> *const f32 {
    DRAPE.with_borrow_mut(|mesh| {
        let (supports, random) = mesh.input.split_at(supports * 5);
        make_boundary(&mut mesh.boundary, columns, rows);
        let stride = columns + 1;
        let top_vertices = stride * (rows + 1);
        let perimeter = mesh.boundary.len();
        let vertices = top_vertices + perimeter * skirt_rows;
        mesh.surface.clear();
        mesh.attributes.resize(vertices * 8, 0.0);
        mesh.indices.clear();
        let (positions, rest) = mesh.attributes.split_at_mut(vertices * 3);
        let (normals, uvs) = rest.split_at_mut(vertices * 3);
        let top_index = |column, row| row * stride + column;

        for row in 0..=rows {
            let z = min_z + (max_z - min_z) * row as f64 / rows as f64;
            for column in 0..=columns {
                let x = min_x + (max_x - min_x) * column as f64 / columns as f64;
                let y = height(x, z, seed, supports);
                let vertex = top_index(column, row);
                mesh.surface.push([x, y, z]);
                positions[vertex * 3..vertex * 3 + 3]
                    .copy_from_slice(&[x as f32, y as f32, z as f32]);
                uvs[vertex * 2..vertex * 2 + 2]
                    .copy_from_slice(&[(x * uv_scale) as f32, (z * uv_scale) as f32]);
            }
        }
        for row in 0..rows {
            for column in 0..columns {
                let a = top_index(column, row) as u16;
                let c = a + stride as u16;
                mesh.indices.extend([a, c, a + 1, a + 1, c, c + 1]);
            }
        }

        let peak = random[0] * perimeter as f64;
        for (index, edge) in mesh.boundary.iter().enumerate() {
            let corner =
                (edge.column == 0 || edge.column == columns) && (edge.row == 0 || edge.row == rows);
            let (dx, dz) = if corner {
                (
                    if edge.column == 0 { -0.707 } else { 0.707 },
                    if edge.row == 0 { -0.707 } else { 0.707 },
                )
            } else {
                (edge.dx, edge.dz)
            };
            let [x, y, z] = mesh.surface[top_index(edge.column, edge.row)];
            let along = index as f64;
            let distance = (along - peak).abs();
            let distance = distance.min(perimeter as f64 - distance);
            let hem = 0.17
                + 0.07 * (along * 0.55 + seed).sin()
                + 0.07 * cloth_noise(along * 0.31 + seed, 2.2)
                + 0.3 * (-distance * distance / 14.0).exp();
            let irregularity = random[index + 1] * 6.0;
            let under_strap =
                (edge.column == strap_left || edge.column == strap_right) && edge.dx == 0.0;
            for layer in 1..=skirt_rows {
                let fraction = layer as f64 / skirt_rows as f64;
                let height = y + (hem - y) * (0.25 * fraction + 0.75 * fraction * fraction)
                    - if layer == 1 { 0.02 } else { 0.0 };
                let fold = if under_strap {
                    0.035 * (std::f64::consts::PI * fraction).sin()
                } else {
                    0.0
                };
                let ripple = fraction
                    * (0.04 * (along * 1.7 + irregularity * 0.2).sin()
                        + 0.028 * (along * 0.83 + 1.3 + seed).sin())
                    + 0.02 * fraction * cloth_noise(along * 0.8, layer as f64 * 0.9 + seed)
                    - fold;
                let offset = (if layer == 1 {
                    0.028
                } else {
                    0.04 + 0.03 * fraction
                }) + ripple
                    + if corner { 0.012 * fraction } else { 0.0 };
                let vertex = top_vertices + index * skirt_rows + layer - 1;
                positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    (x + dx * offset).clamp(-limit_x, limit_x) as f32,
                    height as f32,
                    (z + dz * offset).clamp(-limit_z, limit_z) as f32,
                ]);
                let texture_distance = offset + (y - height);
                uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&[
                    ((x + dx * texture_distance) * uv_scale) as f32,
                    ((z + dz * texture_distance) * uv_scale) as f32,
                ]);
            }
        }
        let skirt_index = |edge: usize, layer: usize| {
            let point = &mesh.boundary[edge];
            if layer == 0 {
                top_index(point.column, point.row)
            } else {
                top_vertices + edge * skirt_rows + layer - 1
            }
        };
        for edge in 0..perimeter {
            let next = (edge + 1) % perimeter;
            for layer in 0..skirt_rows {
                let a = skirt_index(edge, layer) as u16;
                let b = skirt_index(next, layer) as u16;
                let c = skirt_index(edge, layer + 1) as u16;
                let d = skirt_index(next, layer + 1) as u16;
                mesh.indices.extend([a, b, c, b, d, c]);
            }
        }
        indexed_normals(positions, &mesh.indices, normals);
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn drape_indices() -> *const u16 {
    DRAPE.with_borrow(|mesh| mesh.indices.as_ptr())
}
