use std::cell::RefCell;
use std::f64::consts::FRAC_PI_2;

const LAYOUTS: [&str; 4] = [
    include_str!("harbor/yard.json"),
    include_str!("harbor/gate.json"),
    include_str!("harbor/workshop-props.json"),
    include_str!("harbor/quay.json"),
];

const QUAY_POINTS: [f64; 6] = [23.0, 0.0, 0.0, 29.0, 0.0, 20.0];

thread_local! {
    static COMMANDS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_layout(area: usize) -> *const u8 {
    LAYOUTS[area].as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_layout_length(area: usize) -> usize {
    LAYOUTS[area].len()
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_quay_points() -> *const f64 {
    QUAY_POINTS.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_layout_commands() -> *const f64 {
    COMMANDS.with_borrow(|commands| commands.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_fence_run(x: f64, z: f64, end_x: f64, end_z: f64, seed: f64) -> usize {
    let (dx, dz) = (end_x - x, end_z - z);
    let distance = dx.hypot(dz);
    let panels = (distance / 3.35).round().max(1.0) as usize;
    let scale = distance / (panels as f64 * 3.0);
    let yaw = if dx.abs() > dz.abs() { 0.0 } else { FRAC_PI_2 };
    COMMANDS.with_borrow_mut(|commands| {
        commands.clear();
        for index in 0..panels {
            let t = (index as f64 + 0.5) / panels as f64;
            commands.extend([x + dx * t, z + dz * t, yaw, scale, seed + index as f64]);
        }
        for (margin, height) in [(0.05, 2.4), (0.3, 7.0)] {
            commands.extend([
                x.min(end_x) - margin,
                0.0,
                z.min(end_z) - margin,
                x.max(end_x) + margin,
                height,
                z.max(end_z) + margin,
            ]);
        }
    });
    panels
}

// Each record contains its material/operation, six coordinates, and any bevel.
const TRACK: f64 = 0.0;
const RUST: f64 = 1.0;
const WORN: f64 = 2.0;
const FASTENER: f64 = 3.0;
const STOP: f64 = 4.0;
const RUBBER: f64 = 5.0;
const BASE: f64 = 6.0;
const COLLIDER: f64 = 7.0;

#[unsafe(no_mangle)]
pub extern "C" fn harbor_rail_plan() -> usize {
    COMMANDS.with_borrow_mut(|commands| {
        commands.clear();
        let mut record = |kind, bounds: [f64; 6], bevel| {
            commands.push(kind);
            commands.extend(bounds);
            commands.push(bevel);
        };
        let (start, end) = (-30.0, 28.0);
        for x in [18.5, 28.5] {
            record(
                TRACK,
                [x - 0.19, 0.0, start + 0.4, x + 0.19, 0.007, end - 0.4],
                0.0,
            );
            record(
                RUST,
                [x - 0.075, 0.0, start + 0.4, x + 0.075, 0.018, end - 0.4],
                0.0,
            );
            record(
                WORN,
                [x - 0.037, 0.0, start + 0.4, x + 0.037, 0.034, end - 0.4],
                0.008,
            );
            let mut z = start + 0.8;
            while z < end - 0.5 {
                for side in [-1.0, 1.0] {
                    record(
                        FASTENER,
                        [0.07, 0.022, 0.1, x + side * 0.115, 0.012, z],
                        0.0,
                    );
                }
                z += 0.9;
            }
            for (z, side) in [(start + 0.75, 1.0), (end - 0.75, -1.0)] {
                record(
                    STOP,
                    [x - 0.28, 0.0, z - 0.3, x + 0.28, 0.75, z + 0.3],
                    0.03,
                );
                record(
                    RUBBER,
                    [
                        x - 0.16,
                        0.3,
                        z + side * 0.3,
                        x + 0.16,
                        0.62,
                        z + side * 0.42,
                    ],
                    0.03,
                );
                record(
                    BASE,
                    [x - 0.34, 0.0, z - 0.38, x + 0.34, 0.04, z + 0.38],
                    0.0,
                );
                record(
                    COLLIDER,
                    [x - 0.28, 0.0, z - 0.36, x + 0.28, 0.75, z + 0.36],
                    0.0,
                );
            }
        }
        commands.len()
    })
}
