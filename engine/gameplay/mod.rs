mod combat;
mod enemies;
mod locomotion;
mod mission;
mod player;
mod tactics;
mod weapons;

pub fn unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

pub fn mix(start: f64, end: f64, value: f64) -> f64 {
    start + (end - start) * value
}

pub fn approach(start: f64, target: f64, rate: f64, delta: f64) -> f64 {
    target + (start - target) * (-rate * delta).exp()
}

pub fn horizontal_length(x: f64, z: f64) -> f64 {
    let scale = x.abs().max(z.abs());
    if scale == 0.0 || !scale.is_finite() {
        return scale;
    }
    let (x, z) = (x / scale, z / scale);
    (x * x + z * z).sqrt() * scale
}
