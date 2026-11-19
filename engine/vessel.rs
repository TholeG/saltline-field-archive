use std::cell::RefCell;

use super::mesh_normals::flat_normals;

type Point = [f64; 3];

struct Hull {
    center_x: f64,
    half_beam: f64,
    bow_z: f64,
    length: f64,
    waterline: f64,
    deck: f64,
    bow_height: f64,
}

#[derive(Default)]
struct VesselMesh {
    input: Vec<f64>,
    samples: Vec<f64>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static VESSEL: RefCell<VesselMesh> = RefCell::new(VesselMesh::default());
}

fn difference(a: Point, b: Point) -> Point {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn cross([x, y, z]: Point, [u, v, w]: Point) -> Point {
    [y * w - z * v, z * u - x * w, x * v - y * u]
}

impl Hull {
    fn from_parameters(input: &[f64]) -> Self {
        Self {
            center_x: input[0],
            half_beam: input[1],
            bow_z: input[2],
            length: input[3],
            waterline: input[4],
            deck: input[5],
            bow_height: input[6],
        }
    }

    fn point(&self, station: f64, height: f64, side: f64) -> Point {
        let rake = (self.bow_height - height) * 0.22;
        let bow = (1.0 - station / 22.0).max(0.0);
        let z = self.bow_z - station - rake * bow;
        let elevation = ((height - self.waterline) / (self.deck - self.waterline)).clamp(0.0, 1.0);
        let bow_length = 17.0 - 8.5 * elevation.powf(1.3);
        let taper = station / bow_length;
        let form = if taper >= 1.0 {
            1.0
        } else {
            (taper * std::f64::consts::PI / 2.0).sin().powf(0.72)
        };
        let mut beam = 0.2 + (self.half_beam - 0.2) * form;
        let stern = (station - (self.length - 10.0)) / 10.0;
        if stern > 0.0 {
            beam *= 1.0 - 0.24 * stern * stern * (1.0 - 0.6 * elevation);
        }
        [self.center_x + side * beam, height, z]
    }

    fn normal(&self, station: f64, height: f64, side: f64) -> Point {
        let along = difference(
            self.point(station + 0.05, height, side),
            self.point((station - 0.05).max(0.0), height, side),
        );
        let upward = difference(
            self.point(station, height + 0.05, side),
            self.point(station, height - 0.05, side),
        );
        let [x, y, z] = cross(along, upward);
        let length = (x * x + y * y + z * z).sqrt();
        let inverse = if length == 0.0 { 1.0 } else { 1.0 / length };
        let normal = [x * inverse, y * inverse, z * inverse];
        if normal[0] * side + normal[2] * 0.25 < 0.0 {
            normal.map(|value| -value)
        } else {
            normal
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vessel_input(length: usize) -> *mut f64 {
    VESSEL.with_borrow_mut(|mesh| {
        mesh.input.resize(length, 0.0);
        mesh.input.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_samples() -> *const f64 {
    VESSEL.with_borrow_mut(|mesh| {
        let (parameters, stations) = mesh.input.split_at(7);
        let hull = Hull::from_parameters(parameters);
        mesh.samples.clear();
        for side in [-1.0, 1.0] {
            for pair in stations.chunks_exact(2) {
                mesh.samples.extend(hull.point(pair[0], pair[1], side));
                mesh.samples.extend(hull.normal(pair[0], pair[1], side));
            }
        }
        mesh.samples.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_panel(
    start: f64,
    end: f64,
    bottom: f64,
    top: f64,
    side: f64,
    offset: f64,
    columns: usize,
    rows: usize,
) -> *const f32 {
    VESSEL.with_borrow_mut(|mesh| {
        let hull = Hull::from_parameters(&mesh.input);
        let stride = columns + 1;
        let count = stride * (rows + 1);
        mesh.attributes.resize(count * 8, 0.0);
        mesh.indices.clear();
        let (positions, rest) = mesh.attributes.split_at_mut(count * 3);
        let (normals, uvs) = rest.split_at_mut(count * 3);
        for row in 0..=rows {
            for column in 0..=columns {
                let station = start + (end - start) * column as f64 / columns as f64;
                let height = bottom + (top - bottom) * row as f64 / rows as f64;
                let point = hull.point(station, height, side);
                let normal = hull.normal(station, height, side);
                let vertex = row * stride + column;
                let shifted = std::array::from_fn::<_, 3, _>(|axis| {
                    (point[axis] + normal[axis] * offset) as f32
                });
                positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&shifted);
                normals[vertex * 3..vertex * 3 + 3]
                    .copy_from_slice(&normal.map(|value| value as f32));
                let u = column as f64 / columns as f64;
                let u = if side < 0.0 { 1.0 - u } else { u };
                uvs[vertex * 2..vertex * 2 + 2]
                    .copy_from_slice(&[u as f32, (row as f64 / rows as f64) as f32]);
            }
        }
        for row in 0..rows {
            for column in 0..columns {
                let a = (row * stride + column) as u16;
                let c = a + stride as u16;
                mesh.indices.extend(if side > 0.0 {
                    [a, a + 1, c + 1, a, c + 1, c]
                } else {
                    [a, c + 1, a + 1, a, c, c + 1]
                });
            }
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_indices() -> *const u16 {
    VESSEL.with_borrow(|mesh| mesh.indices.as_ptr())
}

fn orient_triangle(output: &mut [f32], [a, b, c]: [Point; 3]) {
    let normal = cross(difference(b, a), difference(c, a));
    let [x, y, z] = std::array::from_fn(|axis| (a[axis] + b[axis] + c[axis]) * (1.0 / 3.0));
    let outward = if x.abs() > 3.39 {
        [x, 0.0, 0.0]
    } else {
        [0.0, y - 0.45, z]
    };
    let dot = normal[0] * outward[0] + normal[1] * outward[1] + normal[2] * outward[2];
    let [b, c] = if dot < 0.0 { [c, b] } else { [b, c] };
    for (vertex, point) in output.chunks_exact_mut(3).zip([a, b, c]) {
        vertex.copy_from_slice(&point.map(|value| value as f32));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dinghy_hull() -> *const f32 {
    VESSEL.with_borrow_mut(|mesh| {
        let sections = mesh.input.len() / 4;
        let count = 2 * ((sections - 1) * 4 + 2) * 3;
        mesh.attributes.resize(count * 8, 0.0);
        let (positions, rest) = mesh.attributes.split_at_mut(count * 3);
        let (normals, _) = rest.split_at_mut(count * 3);
        let mut offset = 0;
        for side in [-1.0, 1.0] {
            let rail = |x: f64, beam: f64, upper: f64| {
                [
                    x,
                    0.5 + (upper - 0.22_f64).max(0.0) * 0.6,
                    side * (beam + 0.1),
                ]
            };
            for pair in mesh.input.windows(8).step_by(4) {
                let [
                    x,
                    beam,
                    upper,
                    keel,
                    next_x,
                    next_beam,
                    next_upper,
                    next_keel,
                ] = pair.try_into().unwrap();
                let a = [x, keel, 0.0];
                let b = [next_x, next_keel, 0.0];
                let c = [x, upper, side * beam];
                let d = [next_x, next_upper, side * next_beam];
                let e = rail(x, beam, upper);
                let f = rail(next_x, next_beam, next_upper);
                for triangle in [[a, b, d], [a, d, c], [c, d, f], [c, f, e]] {
                    orient_triangle(&mut positions[offset..offset + 9], triangle);
                    offset += 9;
                }
            }
            let [x, beam, upper, keel] = mesh.input[..4].try_into().unwrap();
            let a = [x, keel, 0.0];
            let b = [x, 0.78, side * (beam + 0.1)];
            for triangle in [[a, [x, upper, side * beam], b], [a, b, [x, 0.78, 0.0]]] {
                orient_triangle(&mut positions[offset..offset + 9], triangle);
                offset += 9;
            }
        }
        flat_normals(positions, normals);
        mesh.attributes.as_ptr()
    })
}
