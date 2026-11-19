// Cylinder tessellation follows Three.js r180. See licenses/threejs-MIT.txt.
use std::cell::RefCell;
use std::f64::consts::TAU;

#[derive(Default)]
struct Cylinder {
    ring: Vec<(f32, f64, f64)>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static CYLINDER: RefCell<Cylinder> = RefCell::new(Cylinder::default());
}

struct Vertices<'a> {
    positions: &'a mut [f32],
    normals: &'a mut [f32],
    uvs: &'a mut [f32],
    count: usize,
}

impl Vertices<'_> {
    fn push(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) {
        let position_start = self.count * 3;
        let uv_start = self.count * 2;
        self.positions[position_start..position_start + 3].copy_from_slice(&position);
        self.normals[position_start..position_start + 3].copy_from_slice(&normal);
        self.uvs[uv_start..uv_start + 2].copy_from_slice(&uv);
        self.count += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cylinder_mesh(
    top_radius: f64,
    bottom_radius: f64,
    height: f64,
    segments: usize,
    open: bool,
) -> *const f32 {
    CYLINDER.with_borrow_mut(|mesh| {
        let half_height = height / 2.0;
        let stride = segments + 1;
        let caps = [(top_radius, 1.0), (bottom_radius, -1.0)];
        let cap_count = caps
            .iter()
            .filter(|(radius, _)| !open && *radius > 0.0)
            .count();
        let vertex_count = 2 * stride + cap_count * (segments + stride);
        mesh.attributes.resize(vertex_count * 8, 0.0);
        mesh.indices.clear();
        if mesh.ring.len() != stride {
            mesh.ring.clear();
            for segment in 0..=segments {
                let fraction = segment as f64 / segments as f64;
                let (sine, cosine) = (fraction * TAU).sin_cos();
                mesh.ring.push((fraction as f32, sine, cosine));
            }
        }

        let (positions, other) = mesh.attributes.split_at_mut(vertex_count * 3);
        let (normals, uvs) = other.split_at_mut(vertex_count * 3);
        let mut vertices = Vertices {
            positions,
            normals,
            uvs,
            count: 0,
        };
        let taper = bottom_radius - top_radius;
        let slope = taper / height;
        let circumference = std::f64::consts::PI * (top_radius + bottom_radius);
        for row in [0.0, 1.0] {
            let radius = row * taper + top_radius;
            let y = (-row * height + half_height) as f32;
            for &(fraction, sine, cosine) in &mesh.ring {
                let length = (sine * sine + slope * slope + cosine * cosine).sqrt();
                let inverse = 1.0 / length;
                let normal = [sine * inverse, slope * inverse, cosine * inverse].map(|n| n as f32);
                let x = (radius * sine) as f32;
                let z = (radius * cosine) as f32;
                let uv = if f64::from(normal[1]).abs() > 0.9 {
                    [x, z]
                } else {
                    [(f64::from(fraction) * circumference) as f32, y]
                };
                vertices.push([x, y, z], normal, uv);
            }
        }
        for segment in 0..segments as u16 {
            let lower = segment + stride as u16;
            if top_radius > 0.0 {
                mesh.indices.extend([segment, lower, segment + 1]);
            }
            if bottom_radius > 0.0 {
                mesh.indices.extend([lower, lower + 1, segment + 1]);
            }
        }

        for (radius, sign) in caps {
            if open || radius == 0.0 {
                continue;
            }
            let center = vertices.count as u16;
            let rim = center + segments as u16;
            let y = (half_height * sign) as f32;
            let normal = [0.0, sign as f32, 0.0];
            for _ in 0..segments {
                vertices.push([0.0, y, 0.0], normal, [0.0, 0.0]);
            }
            for &(_, sine, cosine) in &mesh.ring {
                let x = (radius * sine) as f32;
                let z = (radius * cosine) as f32;
                vertices.push([x, y, z], normal, [x, z]);
            }
            for segment in 0..segments as u16 {
                let edge = rim + segment;
                let center = center + segment;
                let indices = if sign > 0.0 {
                    [edge, edge + 1, center]
                } else {
                    [edge + 1, edge, center]
                };
                mesh.indices.extend(indices);
            }
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn cylinder_indices() -> *const u16 {
    CYLINDER.with_borrow(|mesh| mesh.indices.as_ptr())
}
