// Match the per-face Float32 accumulations of Three.js BufferGeometry.
#[inline(never)]
pub(super) fn indexed_normals<T: Copy + Into<u32>>(
    positions: &[f32],
    indices: &[T],
    normals: &mut [f32],
) {
    normals.fill(0.0);
    let point = |index: T| {
        let start = index.into() as usize * 3;
        std::array::from_fn::<_, 3, _>(|axis| f64::from(positions[start + axis]))
    };
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [point(triangle[0]), point(triangle[1]), point(triangle[2])];
        let [x, y, z] = std::array::from_fn(|axis| c[axis] - b[axis]);
        let [u, v, w] = std::array::from_fn(|axis| a[axis] - b[axis]);
        let face = [y * w - z * v, z * u - x * w, x * v - y * u];
        for index in triangle {
            let start = (*index).into() as usize * 3;
            for (value, contribution) in normals[start..start + 3].iter_mut().zip(face) {
                *value = (f64::from(*value) + contribution) as f32;
            }
        }
    }
    for normal in normals.chunks_exact_mut(3) {
        let [x, y, z] = std::array::from_fn(|axis| f64::from(normal[axis]));
        let length = (x * x + y * y + z * z).sqrt();
        if length > 0.0 {
            let inverse = 1.0 / length;
            normal.copy_from_slice(&[
                (x * inverse) as f32,
                (y * inverse) as f32,
                (z * inverse) as f32,
            ]);
        }
    }
}

pub(super) fn flat_normals(positions: &[f32], normals: &mut [f32]) {
    for (triangle, output) in positions.chunks_exact(9).zip(normals.chunks_exact_mut(9)) {
        let [ax, ay, az, bx, by, bz, cx, cy, cz] = triangle.try_into().unwrap();
        let [ax, ay, az, bx, by, bz, cx, cy, cz] =
            [ax, ay, az, bx, by, bz, cx, cy, cz].map(f64::from);
        let [x, y, z] = [cx - bx, cy - by, cz - bz];
        let [u, v, w] = [ax - bx, ay - by, az - bz];
        // Three.js rounds the cross product before normalizing it.
        let [nx, ny, nz] = [y * w - z * v, z * u - x * w, x * v - y * u]
            .map(|component| f64::from(component as f32));
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        let inverse = if length == 0.0 { 1.0 } else { 1.0 / length };
        let normal = [nx, ny, nz].map(|component| (component * inverse) as f32);
        for vertex in output.chunks_exact_mut(3) {
            vertex.copy_from_slice(&normal);
        }
    }
}
