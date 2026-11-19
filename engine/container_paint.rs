use std::cell::RefCell;

#[derive(Default)]
struct PanelPaint {
    positions: Vec<f32>,
    details: Vec<f64>,
    colors: Vec<f32>,
}

thread_local! {
    static PAINT: RefCell<PanelPaint> = RefCell::new(PanelPaint::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn panel_paint_inputs(vertices: usize, details: usize) -> *mut f32 {
    PAINT.with_borrow_mut(|paint| {
        paint.positions.resize(vertices * 3, 0.0);
        paint.details.resize(details, 0.0);
        paint.positions.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn panel_paint_details() -> *mut f64 {
    PAINT.with_borrow_mut(|paint| paint.details.as_mut_ptr())
}

fn smooth(start: f64, end: f64, value: f64) -> f64 {
    let point = ((value - start) / (end - start)).clamp(0.0, 1.0);
    point * point * (3.0 - 2.0 * point)
}

fn noise(x: f64, y: f64) -> f64 {
    let hash = |x, y| {
        let value = f64::sin(x * 127.1 + y * 311.7) * 43758.5453;
        value - value.floor()
    };
    let (column, row) = (x.floor(), y.floor());
    let x = smooth(0.0, 1.0, x - column);
    let y = smooth(0.0, 1.0, y - row);
    let a = hash(column, row);
    let b = hash(column + 1.0, row);
    let c = hash(column, row + 1.0);
    let d = hash(column + 1.0, row + 1.0);
    a + (b - a) * x + (c - a) * y + (a - b - c + d) * x * y
}

fn tint(color: &mut [f64; 3], tint: &[f64], strength: f64) {
    for (channel, factor) in color.iter_mut().zip(tint) {
        *channel += (*channel * factor - *channel) * strength;
    }
}

struct Paint<'a> {
    seed: f64,
    wear: f64,
    side_depth: f64,
    roof: f64,
    upper_limit: f64,
    base_tint: &'a [f64],
    streak_tint: &'a [f64],
    dents: &'a [f64],
    streaks: &'a [f64],
}

impl Paint<'_> {
    fn vertex(&self, [x, y, z]: [f64; 3]) -> [f32; 3] {
        let mut shade =
            1.0 + (noise(x * 0.9 + self.seed, y * 1.7 + z * 0.4) - 0.5) * 0.16 * (0.5 + self.wear);
        shade *= 0.94 + 0.06 * noise(x * 6.1 + z * 3.3, y * 5.0 + self.seed);
        let dirt = smooth(0.62, 0.02, y)
            * (0.45 + 0.5 * self.wear)
            * (0.75 + 0.5 * noise(x * 2.3 + z, self.seed));
        let mut color = [shade; 3];
        tint(&mut color, self.base_tint, dirt.min(1.0));
        let groove = smooth(0.008, 0.034, self.side_depth - z.abs());
        color[0] *= 1.0 - 0.17 * groove;
        color[1] *= 1.0 - 0.18 * groove;
        color[2] *= 1.0 - 0.18 * groove;

        let below_roof = smooth(self.roof - 0.75, self.upper_limit, y);
        if below_roof > 0.0 {
            for streak in self.streaks.chunks_exact(3) {
                let distance = (x - streak[0]).abs() / streak[1];
                if distance < 1.0 {
                    tint(
                        &mut color,
                        self.streak_tint,
                        below_roof
                            * below_roof
                            * streak[2]
                            * (1.0 - distance * distance)
                            * (0.45 + 0.55 * self.wear),
                    );
                }
            }
        }
        let mut depth = 0.0;
        for dent in self.dents.chunks_exact(4) {
            let distance = (x - dent[0]).hypot((y - dent[1]) * 0.9) / dent[2];
            if distance < 1.0 {
                let weight = 1.0 - distance * distance;
                depth += dent[3] * weight * weight;
            }
        }
        if depth > 0.004 {
            tint(&mut color, &[0.66, 0.46, 0.36], (depth * 22.0).min(0.8));
        }
        color.map(|channel| channel as f32)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn paint_container_panel(
    seed: f64,
    wear: f64,
    side_depth: f64,
    roof: f64,
    upper_limit: f64,
    dents: usize,
) -> *const f32 {
    PAINT.with_borrow_mut(|panel| {
        let (tints, details) = panel.details.split_at(6);
        let (dents, streaks) = details.split_at(dents * 4);
        let paint = Paint {
            seed,
            wear,
            side_depth,
            roof,
            upper_limit,
            base_tint: &tints[..3],
            streak_tint: &tints[3..],
            dents,
            streaks,
        };
        panel.colors.resize(panel.positions.len(), 0.0);
        // Adjacent triangles share vertices even though the panel uses a non-indexed mesh.
        let mut recent = [None; 64];
        for (position, color) in panel
            .positions
            .chunks_exact(3)
            .zip(panel.colors.chunks_exact_mut(3))
        {
            let key = [
                position[0].to_bits(),
                position[1].to_bits(),
                position[2].to_bits(),
            ];
            let hash = key[0].wrapping_mul(2654435761)
                ^ key[1].wrapping_mul(2246822519)
                ^ key[2].wrapping_mul(3266489917);
            let slot = &mut recent[(hash >> 26) as usize];
            let cached = match *slot {
                Some((previous, color)) if previous == key => color,
                _ => {
                    let color = paint.vertex([
                        f64::from(position[0]),
                        f64::from(position[1]),
                        f64::from(position[2]),
                    ]);
                    *slot = Some((key, color));
                    color
                }
            };
            color.copy_from_slice(&cached);
        }
        panel.colors.as_ptr()
    })
}
