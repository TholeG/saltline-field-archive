use std::cell::RefCell;

const MIN_EDGE: f64 = 1e-6;

#[derive(Default)]
struct Profile {
    points: Vec<[f64; 3]>,
    ring: Vec<(f64, f64, f64)>,
    attributes: Vec<f32>,
    indices: Vec<u16>,
}

thread_local! {
    static PROFILE: RefCell<Profile> = RefCell::new(Profile::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn profile_points(count: usize) -> *mut f64 {
    PROFILE.with_borrow_mut(|profile| {
        profile.points.resize(count, [0.0; 3]);
        profile.points.as_mut_ptr().cast()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn revolve_profile(segments: usize, half_turn: f64) -> *const f32 {
    PROFILE.with_borrow_mut(|profile| {
        let vertex_count = profile.points.len() * (segments + 1);
        profile.attributes.resize(vertex_count * 8, 0.0);
        profile.indices.clear();
        profile.ring.clear();

        let max_radius = profile
            .points
            .iter()
            .fold(0.0_f64, |max, point| max.max(point[0]));
        for segment in 0..=segments {
            let angle = segment as f64 / segments as f64 * half_turn * 2.0;
            let (sine, cosine) = angle.sin_cos();
            profile.ring.push((cosine, sine, angle * max_radius));
        }

        let (positions, rest) = profile.attributes.split_at_mut(vertex_count * 3);
        let (normals, uvs) = rest.split_at_mut(vertex_count * 3);
        let stride = segments + 1;
        let mut distance = 0.0;
        for (row, &[radius, height, incoming_length]) in profile.points.iter().enumerate() {
            let (mut dx, mut dy) = (0.0, 0.0);
            if incoming_length > MIN_EDGE {
                let previous = profile.points[row - 1];
                dx += (radius - previous[0]) / incoming_length;
                dy += (height - previous[1]) / incoming_length;
                distance += incoming_length;
            }
            if let Some(&[next_radius, next_height, outgoing_length]) = profile.points.get(row + 1)
            {
                if outgoing_length > MIN_EDGE {
                    dx += (next_radius - radius) / outgoing_length;
                    dy += (next_height - height) / outgoing_length;
                }
                if outgoing_length >= MIN_EDGE {
                    for segment in 0..segments {
                        let a = (row * stride + segment) as u16;
                        let b = a + stride as u16;
                        if radius > MIN_EDGE {
                            profile.indices.extend([a, b, a + 1]);
                        }
                        if next_radius > MIN_EDGE {
                            profile.indices.extend([b, b + 1, a + 1]);
                        }
                    }
                }
            }
            let length = dx.hypot(dy);
            let divisor = if length == 0.0 { 1.0 } else { length };
            let radial_normal = dy / divisor;
            let normal_y = -dx / divisor;
            for (segment, &(cosine, sine, u)) in profile.ring.iter().enumerate() {
                let vertex = row * stride + segment;
                positions[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    (radius * cosine) as f32,
                    height as f32,
                    (radius * sine) as f32,
                ]);
                normals[vertex * 3..vertex * 3 + 3].copy_from_slice(&[
                    (radial_normal * cosine) as f32,
                    normal_y as f32,
                    (radial_normal * sine) as f32,
                ]);
                uvs[vertex * 2..vertex * 2 + 2].copy_from_slice(&[u as f32, distance as f32]);
            }
        }
        profile.attributes.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn profile_indices() -> *const u16 {
    PROFILE.with_borrow(|profile| profile.indices.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn profile_index_count() -> usize {
    PROFILE.with_borrow(|profile| profile.indices.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn tire_profile_uvs(segments: usize, radius: f64, half_turn: f64) -> bool {
    PROFILE.with_borrow_mut(|profile| {
        // The source tire contour defines these cross-section landmarks.
        const SIDEWALL: usize = 4;
        const TREAD_LEFT: usize = 8;
        const TREAD_RIGHT: usize = 9;
        let mut distances = [0.0; 20];
        if profile.points.len() != distances.len() {
            return false;
        }
        for (row, point) in profile.points.iter().enumerate().skip(1) {
            distances[row] = distances[row - 1] + point[2];
        }
        let center = (distances[TREAD_LEFT] + distances[TREAD_RIGHT]) / 2.0;
        let half_tread = center - distances[SIDEWALL];
        let sidewall_length = distances[SIDEWALL] - distances[0];
        if half_tread <= 0.0 || sidewall_length <= 0.0 {
            return false;
        }
        let circumference = (2.0 * half_turn * radius / 0.5).round().max(1.0) * 0.5;
        let stride = segments + 1;
        let uv_start = profile.points.len() * stride * 6;
        let uv = &mut profile.attributes[uv_start..];
        for (row, distance) in distances.into_iter().enumerate() {
            let from_center = if row >= distances.len() - 2 {
                -(half_tread + sidewall_length)
            } else {
                distance - center
            };
            let width = from_center.abs();
            let v = if width <= half_tread + 1e-6 {
                (width / half_tread) * 0.126
            } else {
                0.126 + ((width - half_tread) / sidewall_length).min(1.0) * 0.022
            };
            let v = if from_center == 0.0 {
                0.0
            } else {
                v.copysign(from_center)
            } as f32;
            for segment in 0..stride {
                let vertex = (row * stride + segment) * 2;
                uv[vertex] = (segment as f64 / segments as f64 * circumference) as f32;
                uv[vertex + 1] = v;
            }
        }
        true
    })
}
