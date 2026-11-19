pub mod cues;
mod effects;
mod instruments;
pub mod loops;
pub mod percussion;

const BAR: f64 = 2.5;
const BEAT: f64 = 0.625;
const STEP: f64 = 0.15625;
const FOUNDATIONS: [f64; 4] = [57.0, 58.0, 58.0, 57.0];
const LOW_CHORDS: [f64; 4] = [65.0, 65.0, 67.0, 64.0];
