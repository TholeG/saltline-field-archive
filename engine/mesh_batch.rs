use std::cell::RefCell;

use super::soft_body::noise;

const NO_INDEX: u32 = u32::MAX;

#[derive(Default)]
struct Batch {
    parts: Vec<[u32; 2]>,
    attributes: Vec<f32>,
    indices: Vec<u32>,
    expanded: Vec<f32>,
    large_indices: bool,
}

thread_local! {
    static BATCH: RefCell<Batch> = RefCell::new(Batch::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn batch_prepare(parts: usize, attributes: usize, indices: usize) -> *mut u32 {
    BATCH.with_borrow_mut(|batch| {
        batch.parts.resize(parts, [0, NO_INDEX]);
        batch.attributes.resize(attributes, 0.0);
        batch.indices.resize(indices, 0);
        batch.large_indices = false;
        batch.parts.as_mut_ptr().cast()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn batch_attributes() -> *mut f32 {
    BATCH.with_borrow_mut(|batch| batch.attributes.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn batch_indices() -> *mut u32 {
    BATCH.with_borrow_mut(|batch| batch.indices.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn batch_large_indices() -> bool {
    BATCH.with_borrow(|batch| batch.large_indices)
}

fn copy_vertex(
    source: &[f32],
    destination: &mut [f32],
    input_vertices: usize,
    output_vertices: usize,
    source_vertex: usize,
    output_vertex: usize,
    widths: &[usize],
) {
    let mut component_offset = 0;
    for &width in widths {
        let input = input_vertices * component_offset + source_vertex * width;
        let output = output_vertices * component_offset + output_vertex * width;
        destination[output..output + width].copy_from_slice(&source[input..input + width]);
        component_offset += width;
    }
}

// Keep the shared noise implementation out of both hot vertex-color expressions.
#[inline(never)]
fn wear_noise(x: f64, y: f64, z: f64) -> f64 {
    noise(x, y, z)
}

fn apply_wear(attributes: &mut [f32], vertices: usize, seed: f64, amount: f64) {
    let (positions, rest) = attributes.split_at_mut(vertices * 3);
    let (normals, rest) = rest.split_at_mut(vertices * 3);
    let (_, colors) = rest.split_at_mut(vertices * 2);
    for ((position, normal), color) in positions
        .chunks_exact(3)
        .zip(normals.chunks_exact(3))
        .zip(colors.chunks_exact_mut(3))
    {
        let [x, y, z] = std::array::from_fn(|axis| f64::from(position[axis]));
        let broad = wear_noise(x * 1.9 + seed, y * 1.9, z * 1.9 + 5.3);
        let fine = wear_noise(x * 9.0 + 2.1, y * 9.0 + seed, z * 9.0);
        let upward = (f64::from(normal[1]) * 0.6 + 0.62).clamp(0.0, 1.0);
        let near_ground = (1.0 - y / 0.2).clamp(0.0, 1.0);
        let mut shade = (0.82 + 0.18 * broad) * (0.92 + 0.08 * fine) * (0.76 + 0.24 * upward);
        shade *= 1.0 - 0.42 * near_ground * near_ground * (0.55 + 0.45 * fine);
        shade = 1.0 + (shade - 1.0) * amount;
        let gains = [
            shade,
            shade * (1.0 - 0.05 * near_ground * amount),
            shade * (1.0 - 0.12 * near_ground * amount),
        ];
        for (channel, gain) in color.iter_mut().zip(gains) {
            *channel = (f64::from(*channel) * gain) as f32;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn assemble_batch(
    input_vertices: usize,
    output_vertices: usize,
    colored: bool,
    indexed: bool,
    mixed: bool,
    wear_seed: f64,
    wear_amount: f64,
) -> *const f32 {
    BATCH.with_borrow_mut(|batch| {
        let mut first_vertex = 0;
        let mut first_index = 0;

        // Checked before expanding any attributes so unsupported indices can fall back to JS.
        for &[vertices, indices] in &batch.parts {
            if indices == NO_INDEX {
                continue;
            }
            let end = first_index + indices as usize;
            if batch.indices[first_index..end]
                .iter()
                .any(|&i| i >= vertices)
            {
                return std::ptr::null();
            }
            first_index = end;
        }
        first_index = 0;

        let attributes = if mixed {
            let widths: &[usize] = if colored { &[3, 3, 2, 3] } else { &[3, 3, 2] };
            let components = if colored { 11 } else { 8 };
            batch.expanded.resize(output_vertices * components, 0.0);
            let mut output = 0;
            for &[vertices, indices] in &batch.parts {
                if indices == NO_INDEX {
                    for vertex in 0..vertices as usize {
                        copy_vertex(
                            &batch.attributes,
                            &mut batch.expanded,
                            input_vertices,
                            output_vertices,
                            first_vertex + vertex,
                            output,
                            widths,
                        );
                        output += 1;
                    }
                } else {
                    let end = first_index + indices as usize;
                    for &vertex in &batch.indices[first_index..end] {
                        copy_vertex(
                            &batch.attributes,
                            &mut batch.expanded,
                            input_vertices,
                            output_vertices,
                            first_vertex + vertex as usize,
                            output,
                            widths,
                        );
                        output += 1;
                    }
                    first_index = end;
                }
                first_vertex += vertices as usize;
            }
            &mut batch.expanded
        } else {
            if indexed {
                for &[vertices, indices] in &batch.parts {
                    let end = first_index + indices as usize;
                    for index in &mut batch.indices[first_index..end] {
                        *index += first_vertex as u32;
                        batch.large_indices |= *index >= 65535;
                    }
                    first_index = end;
                    first_vertex += vertices as usize;
                }
            }
            &mut batch.attributes
        };
        if colored && wear_amount > 0.0 {
            apply_wear(attributes, output_vertices, wear_seed, wear_amount);
        }
        attributes.as_ptr()
    })
}
