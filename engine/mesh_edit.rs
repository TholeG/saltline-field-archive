use std::cell::RefCell;

use super::mesh_normals::{flat_normals, indexed_normals};

#[derive(Default)]
struct MeshEdit {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    matrices: [f64; 25],
}

thread_local! {
    static MESH: RefCell<MeshEdit> = RefCell::new(MeshEdit::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn edit_mesh_buffer(length: usize, index_count: usize) -> *mut f32 {
    MESH.with_borrow_mut(|mesh| {
        mesh.vertices.resize(length, 0.0);
        mesh.indices.resize(index_count, 0);
        mesh.vertices.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn edit_mesh_matrices() -> *mut f64 {
    MESH.with_borrow_mut(|mesh| mesh.matrices.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn edit_mesh_indices() -> *mut u32 {
    MESH.with_borrow_mut(|mesh| mesh.indices.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn transform_mesh(position_length: usize) {
    MESH.with_borrow_mut(|mesh| {
        let (positions, normals) = mesh.vertices.split_at_mut(position_length);
        let (transform, normal_matrix) = mesh.matrices.split_at(16);
        for point in positions.chunks_exact_mut(3) {
            let (x, y, z) = (
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            );
            let w = 1.0 / (transform[3] * x + transform[7] * y + transform[11] * z + transform[15]);
            for i in 0..3 {
                point[i] = ((transform[i] * x
                    + transform[i + 4] * y
                    + transform[i + 8] * z
                    + transform[i + 12])
                    * w) as f32;
            }
        }
        for direction in normals.chunks_exact_mut(3) {
            let (x, y, z) = (
                f64::from(direction[0]),
                f64::from(direction[1]),
                f64::from(direction[2]),
            );
            let [nx, ny, nz] = std::array::from_fn(|i| {
                normal_matrix[i] * x + normal_matrix[i + 3] * y + normal_matrix[i + 6] * z
            });
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            let inverse = if length > 0.0 { 1.0 / length } else { 1.0 };
            direction.copy_from_slice(&[
                (nx * inverse) as f32,
                (ny * inverse) as f32,
                (nz * inverse) as f32,
            ]);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn rebuild_mesh_normals(vertex_count: usize, indexed: bool) -> bool {
    MESH.with_borrow_mut(|mesh| {
        let (positions, normals) = mesh.vertices.split_at_mut(vertex_count * 3);
        if indexed {
            if mesh
                .indices
                .iter()
                .any(|&index| index as usize >= vertex_count)
            {
                return false;
            }
            indexed_normals(positions, &mesh.indices, normals);
        } else {
            flat_normals(positions, normals);
        }
        true
    })
}
