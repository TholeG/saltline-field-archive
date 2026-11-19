use std::cell::RefCell;

const VERTICES: usize = 24;
const INDEX_COUNT: usize = 44 * 3;

struct BoxMesh {
    attributes: [f32; VERTICES * 8],
    indices: [u16; INDEX_COUNT],
}

thread_local! {
    static BOX_MESH: RefCell<BoxMesh> = const { RefCell::new(BoxMesh {
        attributes: [0.0; VERTICES * 8],
        indices: [0; INDEX_COUNT],
    }) };
}

fn corner_sign(corner: usize, axis: usize) -> f64 {
    if corner & (1 << axis) == 0 { -1.0 } else { 1.0 }
}

struct Faces<'a> {
    positions: &'a [[f64; 3]; VERTICES],
    indices: &'a mut [u16; INDEX_COUNT],
    count: usize,
}

impl Faces<'_> {
    fn triangle(&mut self, [a, b, c]: [usize; 3], outward: [f64; 3]) {
        let origin = self.positions[a];
        let [ux, uy, uz] = std::array::from_fn(|axis| self.positions[b][axis] - origin[axis]);
        let [vx, vy, vz] = std::array::from_fn(|axis| self.positions[c][axis] - origin[axis]);
        let winding = (uy * vz - uz * vy) * outward[0]
            + (uz * vx - ux * vz) * outward[1]
            + (ux * vy - uy * vx) * outward[2];
        let [second, third] = if winding >= 0.0 { [b, c] } else { [c, b] };
        self.indices[self.count..self.count + 3].copy_from_slice(&[
            a as u16,
            second as u16,
            third as u16,
        ]);
        self.count += 3;
    }

    fn quad(&mut self, [a, b, c, d]: [usize; 4], outward: [f64; 3]) {
        self.triangle([a, b, c], outward);
        self.triangle([a, c, d], outward);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn beveled_box(width: f64, height: f64, depth: f64, bevel: f64) -> *const f32 {
    BOX_MESH.with_borrow_mut(|mesh| {
        let half = [width / 2.0, height / 2.0, depth / 2.0];
        let bevel = half
            .iter()
            .fold(bevel, |bevel, extent| bevel.min(extent * 0.9));
        let mut positions = [[0.0; 3]; VERTICES];
        let (position_data, rest) = mesh.attributes.split_at_mut(VERTICES * 3);
        let (normal_data, uvs) = rest.split_at_mut(VERTICES * 3);

        // Each corner has one vertex on each of its three neighboring faces.
        for (vertex, position) in positions.iter_mut().enumerate() {
            let (corner, side) = (vertex / 3, vertex % 3);
            for axis in 0..3 {
                let sign = corner_sign(corner, axis);
                let inset = if axis == side { 0.0 } else { bevel };
                position[axis] = sign * (half[axis] - inset);
                position_data[vertex * 3 + axis] = position[axis] as f32;
                normal_data[vertex * 3 + axis] = if axis == side { sign as f32 } else { 0.0 };
            }
            let [x, y, z] = position.map(|value| value as f32);
            let uv = match side {
                0 => [z, y],
                1 => [x, z],
                _ => [x, y],
            };
            uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&uv);
        }

        let mut faces = Faces {
            positions: &positions,
            indices: &mut mesh.indices,
            count: 0,
        };
        for axis in 0..3 {
            let across = (axis + 1) % 3;
            let along = (axis + 2) % 3;
            for side in 0..2 {
                let base = side << axis;
                let corners = [
                    base,
                    base | (1 << across),
                    base | (1 << across) | (1 << along),
                    base | (1 << along),
                ];
                let mut outward = [0.0; 3];
                outward[axis] = corner_sign(base, axis);
                faces.quad(corners.map(|corner| corner * 3 + axis), outward);
            }
            for edge in 0..4 {
                let start = ((edge >> 1) << across) | ((edge & 1) << along);
                let end = start | (1 << axis);
                let mut outward = [0.0; 3];
                outward[across] = corner_sign(start, across);
                outward[along] = corner_sign(start, along);
                faces.quad(
                    [
                        start * 3 + across,
                        end * 3 + across,
                        end * 3 + along,
                        start * 3 + along,
                    ],
                    outward,
                );
            }
        }
        for corner in 0..8 {
            faces.triangle(
                [corner * 3, corner * 3 + 1, corner * 3 + 2],
                std::array::from_fn(|axis| corner_sign(corner, axis)),
            );
        }
        mesh.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn beveled_box_indices() -> *const u16 {
    BOX_MESH.with_borrow(|mesh| mesh.indices.as_ptr())
}
