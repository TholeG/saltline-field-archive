// Box tessellation follows Three.js r180. See licenses/threejs-MIT.txt.
use std::cell::RefCell;

#[derive(Default)]
struct BoxGrid {
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static BOX_GRID: RefCell<BoxGrid> = RefCell::new(BoxGrid::default());
}

fn round_corner(
    mut position: [f64; 3],
    normal: &mut [f32; 3],
    half: [f64; 3],
    segments: [usize; 3],
    radius: f64,
) -> [f64; 3] {
    let mut outside_axes = 0;
    let mut center = [0.0; 3];
    let mut direction = [0.0; 3];
    for axis in 0..3 {
        let extent = half[axis];
        let core = extent - radius;
        let coordinate = &mut position[axis];
        if segments[axis] >= 3 && coordinate.abs() <= extent - 1e-6 {
            let step = 2.0 * extent / segments[axis] as f64;
            *coordinate = *coordinate * core / (extent - step);
        }
        outside_axes += usize::from(coordinate.abs() > extent - 1e-5);
        center[axis] = coordinate.min(core).max(-core);
        direction[axis] = *coordinate - center[axis];
    }
    let [x, y, z] = direction;
    let length_squared = x * x + y * y + z * z;
    if outside_axes >= 2 && length_squared > 1e-12 {
        let inverse = 1.0 / length_squared.sqrt();
        for axis in 0..3 {
            let component = direction[axis] * inverse;
            position[axis] = center[axis] + component * radius;
            normal[axis] = component as f32;
        }
    }
    position
}

fn snap_rounded_edges(
    mut position: [f64; 3],
    normal: &mut [f32; 3],
    half: [f64; 3],
    radius: f64,
) -> [f64; 3] {
    let mut outside_axes = 0;
    let mut center = [0.0; 3];
    let mut direction = [0.0; 3];
    for axis in 0..3 {
        let value = position[axis];
        let sign = if value == 0.0 { value } else { value.signum() };
        let core = half[axis] - radius;
        let on_boundary = value.abs() > half[axis] - 1e-5;
        position[axis] = sign * if on_boundary { half[axis] } else { core };
        outside_axes += usize::from(position[axis].abs() > half[axis] - 1e-5);
        center[axis] = position[axis].min(core).max(-core);
        direction[axis] = position[axis] - center[axis];
    }
    let [x, y, z] = direction;
    let length_squared = x * x + y * y + z * z;
    if outside_axes >= 2 && length_squared > 1e-10 {
        let inverse = 1.0 / length_squared.sqrt();
        for axis in 0..3 {
            let component = direction[axis] * inverse;
            position[axis] = center[axis] + component * radius;
            normal[axis] = component as f32;
        }
    }
    position
}

#[unsafe(no_mangle)]
pub extern "C" fn box_grid_mesh(
    width: f64,
    height: f64,
    depth: f64,
    width_segments: usize,
    height_segments: usize,
    depth_segments: usize,
    rounding: u32,
    radius: f64,
) -> *const f32 {
    BOX_GRID.with_borrow_mut(|mesh| {
        let dimensions = [width, height, depth];
        let half = dimensions.map(|size| size / 2.0);
        let segments = [width_segments, height_segments, depth_segments];
        let [nx, ny, nz] = segments.map(|n| n + 1);
        let count = 2 * (ny * nz + nz * nx + nx * ny);
        let radius = if rounding == 2 {
            half.iter().fold(radius, |r, extent| r.min(extent - 1e-4))
        } else {
            half.iter().fold(radius, |r, extent| r.min(extent * 0.95))
        };
        mesh.attributes.resize(count * 8, 0.0);
        mesh.indices.clear();
        let (positions, rest) = mesh.attributes.split_at_mut(count * 3);
        let (normals, uvs) = rest.split_at_mut(count * 3);
        let faces = [
            (2, 1, 0, -1.0, -1.0, 1.0),
            (2, 1, 0, 1.0, -1.0, -1.0),
            (0, 2, 1, 1.0, 1.0, 1.0),
            (0, 2, 1, 1.0, -1.0, -1.0),
            (0, 1, 2, 1.0, -1.0, 1.0),
            (0, 1, 2, -1.0, -1.0, -1.0),
        ];
        let mut vertex = 0;
        for (u, v, axis, u_sign, v_sign, side) in faces {
            let columns = segments[u];
            let rows = segments[v];
            let du = dimensions[u] / columns as f64;
            let dv = dimensions[v] / rows as f64;
            let base = vertex;
            for row in 0..=rows {
                for column in 0..=columns {
                    let mut point = [0.0; 3];
                    let mut normal = [0.0; 3];
                    point[u] = (column as f64 * du - half[u]) * u_sign;
                    point[v] = (row as f64 * dv - half[v]) * v_sign;
                    point[axis] = half[axis] * side;
                    // The original box positions pass through Float32 before rounding its corners.
                    point = point.map(|coordinate| f64::from(coordinate as f32));
                    normal[axis] = side as f32;
                    if rounding == 1 {
                        point = round_corner(point, &mut normal, half, segments, radius);
                    } else if rounding == 2 {
                        point = snap_rounded_edges(point, &mut normal, half, radius);
                    }
                    let [x, y, z] = point.map(|coordinate| coordinate as f32);
                    let [nx, ny, nz] = normal.map(f32::abs);
                    let uv = if nx >= ny && nx >= nz {
                        [z, y]
                    } else if ny >= nx && ny >= nz {
                        [x, z]
                    } else {
                        [x, y]
                    };
                    positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[x, y, z]);
                    normals[vertex * 3..vertex * 3 + 3].copy_from_slice(&normal);
                    uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&uv);
                    vertex += 1;
                }
            }
            let stride = columns + 1;
            for row in 0..rows {
                for column in 0..columns {
                    let a = (base + row * stride + column) as u16;
                    let b = a + stride as u16;
                    mesh.indices.extend([a, b, a + 1, b, b + 1, a + 1]);
                }
            }
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn box_grid_indices() -> *const u16 {
    BOX_GRID.with_borrow(|mesh| mesh.indices.as_ptr())
}
