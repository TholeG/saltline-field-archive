use std::cell::RefCell;

use super::mesh_normals::flat_normals;

type Point = [f64; 3];

#[derive(Clone, Copy)]
struct Vertex {
    position: Point,
    uv: [f64; 2],
}

#[derive(Default)]
struct Surface {
    input: Vec<f64>,
    vertices: Vec<Vertex>,
    attributes: Vec<f32>,
    profile: Vec<[f64; 2]>,
}

thread_local! {
    static SURFACE: RefCell<Surface> = RefCell::new(Surface::default());
}

fn subtract(a: Point, b: Point) -> Point {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn cross([ax, ay, az]: Point, [bx, by, bz]: Point) -> Point {
    [ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx]
}

fn orient(vertices: &mut [Vertex], center: Point, inward: bool) {
    let mut winding = 0.0;
    for triangle in vertices.chunks_exact(3) {
        let [a, b, c] = [
            triangle[0].position,
            triangle[1].position,
            triangle[2].position,
        ];
        let [x, y, z] = cross(subtract(b, a), subtract(c, a));
        let [ox, oy, oz] =
            std::array::from_fn(|axis| (a[axis] + b[axis] + c[axis]) / 3.0 - center[axis]);
        winding += x * ox + y * oy + z * oz;
    }
    if (winding < 0.0) != inward {
        for triangle in vertices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
}

fn mean(vertices: &[Vertex]) -> Point {
    let mut sum = [0.0; 3];
    for vertex in vertices {
        for (total, coordinate) in sum.iter_mut().zip(vertex.position) {
            *total += coordinate;
        }
    }
    sum.map(|component| component * (1.0 / vertices.len() as f64))
}

fn write_attributes(vertices: &[Vertex], attributes: &mut Vec<f32>) {
    attributes.resize(vertices.len() * 8, 0.0);
    let (positions, rest) = attributes.split_at_mut(vertices.len() * 3);
    let (normals, uvs) = rest.split_at_mut(vertices.len() * 3);
    for (i, vertex) in vertices.iter().enumerate() {
        for (output, component) in positions[i * 3..i * 3 + 3].iter_mut().zip(vertex.position) {
            *output = component as f32;
        }
        uvs[i * 2..i * 2 + 2].copy_from_slice(&vertex.uv.map(|component| component as f32));
    }
    flat_normals(positions, normals);
}

#[unsafe(no_mangle)]
pub extern "C" fn surface_input(length: usize) -> *mut f64 {
    SURFACE.with_borrow_mut(|surface| {
        surface.input.resize(length, 0.0);
        surface.input.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn sweep_surface(
    columns: usize,
    rows: usize,
    closed: bool,
    caps: bool,
    inward: bool,
    has_center: bool,
    center_x: f64,
    center_y: f64,
    center_z: f64,
) -> *const f32 {
    SURFACE.with_borrow_mut(|surface| {
        let edge_count = if closed { columns } else { columns - 1 };
        let (levels, rest) = surface.input.split_at(rows);
        let (distances, rest) = rest.split_at(edge_count + 1);
        let (projected, cap_data) = rest.split_at(columns * rows * 3);
        let position = |column: usize, row: usize| {
            let start = (row * columns + column) * 3;
            [projected[start], projected[start + 1], projected[start + 2]]
        };

        surface.vertices.clear();
        for row in 0..rows - 1 {
            for column in 0..edge_count {
                let next = (column + 1) % columns;
                let a = Vertex {
                    position: position(column, row),
                    uv: [distances[column], levels[row]],
                };
                let b = Vertex {
                    position: position(next, row),
                    uv: [distances[column + 1], levels[row]],
                };
                let c = Vertex {
                    position: position(next, row + 1),
                    uv: [distances[column + 1], levels[row + 1]],
                };
                let d = Vertex {
                    position: position(column, row + 1),
                    uv: [distances[column], levels[row + 1]],
                };
                surface.vertices.extend([a, b, c, a, c, d]);
            }
        }
        let center = if has_center {
            [center_x, center_y, center_z]
        } else {
            mean(&surface.vertices)
        };
        orient(&mut surface.vertices, center, inward);

        if caps {
            let (profile, values) = cap_data.split_at(columns * 2);
            for (cap, row) in [0, rows - 1].into_iter().enumerate() {
                let start = surface.vertices.len();
                let offset = 2 + cap * 3;
                let center_vertex = Vertex {
                    position: [values[offset], values[offset + 1], values[offset + 2]],
                    uv: [values[0], values[1]],
                };
                for column in 0..columns {
                    let next = (column + 1) % columns;
                    let a = Vertex {
                        position: position(column, row),
                        uv: [profile[column * 2], profile[column * 2 + 1]],
                    };
                    let b = Vertex {
                        position: position(next, row),
                        uv: [profile[next * 2], profile[next * 2 + 1]],
                    };
                    surface.vertices.extend([center_vertex, a, b]);
                }
                orient(&mut surface.vertices[start..], center, inward);
            }
        }
        write_attributes(&surface.vertices, &mut surface.attributes);
        surface.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn corrugated_profile(
    start: f64,
    end: f64,
    period: f64,
    depth: f64,
    flat: f64,
    slope: f64,
    origin: f64,
) -> usize {
    SURFACE.with_borrow_mut(|surface| {
        let valley = period - flat - 2.0 * slope;
        let sample = |position: f64| {
            let mut phase = (position - origin) % period;
            if phase < 0.0 {
                phase += period;
            }
            let offset = if phase <= flat {
                0.0
            } else if phase < flat + slope {
                -depth * (phase - flat) / slope
            } else if phase <= flat + slope + valley {
                -depth
            } else {
                -depth * (1.0 - (phase - flat - slope - valley) / slope)
            };
            [position, offset]
        };
        surface.profile.clear();
        surface.profile.push(sample(start));
        let first = ((start - origin) / period).floor() as i32 - 1;
        let last = ((end - origin) / period).ceil() as i32 + 1;
        for repeat in first..=last {
            let base = origin + f64::from(repeat) * period;
            for step in [0.0, flat, flat + slope, flat + slope + valley] {
                let position = base + step;
                if position > start + 1e-4 && position < end - 1e-4 {
                    surface.profile.push(sample(position));
                }
            }
        }
        surface.profile.push(sample(end));
        surface.profile.len()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn corrugated_points() -> *const f64 {
    SURFACE.with_borrow(|surface| surface.profile.as_ptr().cast())
}
