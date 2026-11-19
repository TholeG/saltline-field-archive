use std::cell::RefCell;

#[derive(Clone, Copy, Default)]
struct Wave {
    direction_x: f64,
    direction_z: f64,
    amplitude: f64,
    wave_number: f64,
    frequency: f64,
}

impl Wave {
    fn spatial_phase(self, x: f64, z: f64) -> f64 {
        self.wave_number * (self.direction_x * x + self.direction_z * z)
    }
}

#[derive(Default)]
struct Water {
    waves: Vec<Wave>,
    parameters: Vec<f64>,
    vertices: Vec<f32>,
    attenuation: Vec<f64>,
    spatial_phases: Vec<(f64, f64)>,
    time_phases: Vec<(f64, f64)>,
}

thread_local! {
    static WATER: RefCell<Water> = RefCell::new(Water::default());
}

fn harbor_attenuation(x: f64, z: f64) -> f64 {
    let fraction = (((x - 30.0).hypot(z) - 50.0) / 170.0).clamp(0.0, 1.0);
    1.0 - fraction * fraction * (3.0 - 2.0 * fraction)
}

#[unsafe(no_mangle)]
pub extern "C" fn water_parameters(count: usize) -> *mut f64 {
    WATER.with_borrow_mut(|water| {
        water.parameters.resize(count * 5, 0.0);
        water.parameters.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn water_read_parameters() {
    WATER.with_borrow_mut(|water| {
        water.waves.clear();
        for values in water.parameters.chunks_exact(5) {
            water.waves.push(Wave {
                direction_x: values[0],
                direction_z: values[1],
                amplitude: values[2],
                wave_number: values[3],
                frequency: values[4],
            });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn water_height(x: f64, z: f64, time: f64, sea_level: f64) -> f64 {
    WATER.with_borrow(|water| {
        let height = water.waves.iter().fold(0.0, |height, wave| {
            height + wave.amplitude * (wave.spatial_phase(x, z) - wave.frequency * time).sin()
        });
        sea_level + height * harbor_attenuation(x, z)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn water_vertices(count: usize) -> *mut f32 {
    WATER.with_borrow_mut(|water| {
        water.vertices.resize(count * 3, 0.0);
        water.vertices.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn water_prepare_surface() {
    WATER.with_borrow_mut(|water| {
        water.attenuation.clear();
        water.spatial_phases.clear();
        water.time_phases.resize(water.waves.len(), (0.0, 0.0));
        for vertex in water.vertices.chunks_exact(3) {
            let (x, z) = (f64::from(vertex[0]), f64::from(vertex[2]));
            water.attenuation.push(harbor_attenuation(x, z));
            for wave in &water.waves {
                let (sine, cosine) = wave.spatial_phase(x, z).sin_cos();
                water
                    .spatial_phases
                    .push((wave.amplitude * sine, wave.amplitude * cosine));
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn water_update_surface(time: f64, sea_level: f64) -> *const f32 {
    WATER.with_borrow_mut(|water| {
        for (phase, wave) in water.time_phases.iter_mut().zip(&water.waves) {
            *phase = (wave.frequency * time).sin_cos();
        }
        let wave_count = water.waves.len();
        for (index, vertex) in water.vertices.chunks_exact_mut(3).enumerate() {
            let start = index * wave_count;
            let phases = &water.spatial_phases[start..start + wave_count];
            // sin(a - b) uses the cached spatial phase and the time phase shared by all vertices.
            let height = phases.iter().zip(&water.time_phases).fold(
                0.0,
                |height, (&(space_sin, space_cos), &(time_sin, time_cos))| {
                    height + (space_sin * time_cos - space_cos * time_sin)
                },
            );
            vertex[1] = ((sea_level + height * water.attenuation[index]) - sea_level) as f32;
        }
        water.vertices.as_ptr()
    })
}
