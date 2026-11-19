use super::super::dsp::{Clip, Filter, biquad, coefficients, round};
use std::f64::consts::TAU;

pub fn level(channels: &mut [Clip], target: f64) {
    let mut total = 0.0;
    let mut samples = 0;
    for channel in channels.iter() {
        for &sample in channel.iter() {
            total += f64::from(sample) * f64::from(sample);
        }
        samples += channel.len();
    }
    let mean = total / samples as f64;
    let gain = target / if mean == 0.0 { 1.0 } else { mean.sqrt() };
    for channel in channels {
        channel.scale(gain);
    }
}

pub fn limit(clip: &mut Clip, drive: f64) {
    let scale = (1.0 + drive * drive).sqrt() / drive;
    for sample in clip.iter_mut() {
        let x = f64::from(*sample) * drive;
        *sample = (x / (1.0 + x * x).sqrt() * scale) as f32;
    }
}

pub fn loop_filter(clip: &mut Clip, kind: Filter, cutoff: f64, q: f64, warmup: f64) {
    let coeff = coefficients(kind, cutoff, q, clip.rate);
    apply(clip, coeff, warmup);
}

fn apply(clip: &mut Clip, coeff: [f64; 5], warmup: f64) {
    let warmup = clip
        .len()
        .saturating_sub(round(warmup * clip.rate) as usize);
    let mut state = [0.0; 2];
    for &sample in &clip[warmup..] {
        biquad(f64::from(sample), coeff, &mut state);
    }
    for sample in clip.iter_mut() {
        *sample = biquad(f64::from(*sample), coeff, &mut state) as f32;
    }
}

pub fn loop_sweep(clip: &mut Clip, curve: impl Fn(f64) -> f64, q: f64) {
    let begin = clip.len().saturating_sub(round(0.4 * clip.rate) as usize);
    let length = clip.len() as f64;
    let rate = clip.rate;
    let mut coeff = coefficients(Filter::Low, curve(begin as f64 / length), q, rate);
    let mut state = [0.0; 2];
    for (index, &sample) in clip.iter().enumerate().skip(begin) {
        if index % 64 == 0 {
            coeff = coefficients(Filter::Low, curve(index as f64 / length), q, rate);
        }
        biquad(f64::from(sample), coeff, &mut state);
    }
    for (index, sample) in clip.iter_mut().enumerate() {
        if index % 64 == 0 {
            coeff = coefficients(Filter::Low, curve(index as f64 / length), q, rate);
        }
        *sample = biquad(f64::from(*sample), coeff, &mut state) as f32;
    }
}

pub const BRASS_FORMANTS: [(f64, f64, f64); 2] = [(520.0, 1.4, 0.35), (1350.0, 1.6, 0.22)];

pub fn formants(clip: &mut Clip, bands: &[(f64, f64, f64)], warmup: f64) {
    for &(frequency, q, strength) in bands {
        let boost = (1.0_f64 + strength).sqrt();
        let w = TAU * frequency / clip.rate;
        let alpha = w.sin() / (2.0 * q);
        let denominator = 1.0 + alpha / boost;
        let shared = -2.0 * w.cos() / denominator;
        let coeff = [
            (1.0 + alpha * boost) / denominator,
            shared,
            (1.0 - alpha * boost) / denominator,
            shared,
            (1.0 - alpha / boost) / denominator,
        ];
        apply(clip, coeff, warmup);
    }
}

pub fn chorus(channels: &mut [Clip], periods: f64, delay: f64, modulation: f64, level: f64) {
    for (side, signal) in channels.iter_mut().enumerate() {
        let source = signal.clone();
        let count = source.len() as f64;
        let offset = side as f64 * 2.1;
        let gain = level * if side == 0 { 1.0 } else { -1.0 };
        let rate = signal.rate;
        let delay_at = |frame: usize| {
            (delay
                + modulation * (TAU * periods * frame as f64 / count + offset).sin()
                + if side == 0 { 0.0 } else { 3.1 })
                * 0.001
                * rate
        };
        let (mut current, mut step) = (0.0, 0.0);
        for (index, sample) in signal.iter_mut().enumerate() {
            if index % 64 == 0 {
                current = delay_at(index);
                step = (delay_at(index + 64) - current) / 64.0;
            }
            let mut frame = index as f64 - current;
            current += step;
            if frame < 0.0 {
                frame += count;
            }
            let base = frame as usize;
            let fraction = frame - base as f64;
            let a = f64::from(source[base]);
            let b = f64::from(source[(base + 1) % source.len()]);
            *sample = (f64::from(*sample) + gain * (a + (b - a) * fraction)) as f32;
        }
    }
}

pub fn stereo_cutoff([left, right]: &mut [Clip; 2], low: f64, side: f64) {
    let mut mid = Clip::with_length(left.rate, left.len());
    let mut sides = mid.clone();
    for index in 0..left.len() {
        let (l, r) = (f64::from(left[index]), f64::from(right[index]));
        mid[index] = (0.5 * (l + r)) as f32;
        sides[index] = (0.5 * (l - r)) as f32;
    }
    loop_filter(&mut mid, Filter::High, low, 0.707, 0.6);
    loop_filter(&mut sides, Filter::High, side, 0.707, 0.4);
    loop_filter(&mut sides, Filter::High, side, 0.707, 0.4);
    for index in 0..left.len() {
        let (m, s) = (f64::from(mid[index]), f64::from(sides[index]));
        left[index] = (m + s) as f32;
        right[index] = (m - s) as f32;
    }
}

pub fn mix_missing_tail(signal: &mut Clip, add: &Clip) {
    for (sample, &overlay) in signal.iter_mut().zip(add.iter()) {
        *sample = (f64::from(*sample) + f64::from(overlay)) as f32;
    }
}

pub fn upsample(source: &Clip, wrap: bool) -> Clip {
    let len = source.len() as isize;
    let mut out = Clip::with_length(source.rate * 2.0, 2 * len as usize);
    let sample = |index: isize| {
        if wrap {
            f64::from(source[index.rem_euclid(len) as usize])
        } else if index < 0 || index >= len {
            0.0
        } else {
            f64::from(source[index as usize])
        }
    };
    for index in 0..len {
        out[2 * index as usize] = source[index as usize];
        out[2 * index as usize + 1] = ((3.0 * (sample(index - 2) + sample(index + 3))
            - 25.0 * (sample(index - 1) + sample(index + 2))
            + 150.0 * (sample(index) + sample(index + 1)))
            / 256.0) as f32;
    }
    out
}

#[derive(Clone, Copy)]
pub struct Reverb {
    pub tail: f64,
    pub size: f64,
    pub damping: f64,
    pub pre_delay: f64,
    pub high_pass: f64,
}

pub const HALL: Reverb = Reverb {
    tail: 3.0,
    size: 1.45,
    damping: 0.3,
    pre_delay: 0.03,
    high_pass: 200.0,
};

struct Delay {
    frames: Vec<f32>,
    position: usize,
}

impl Delay {
    fn new(length: usize) -> Self {
        Self {
            frames: vec![0.0; length],
            position: 0,
        }
    }

    fn read(&self) -> f64 {
        f64::from(self.frames[self.position])
    }

    fn write(&mut self, value: f64) {
        self.frames[self.position] = value as f32;
        self.position += 1;
        if self.position == self.frames.len() {
            self.position = 0;
        }
    }

    fn diffuse(&mut self, input: f64) -> f64 {
        let delayed = self.read();
        let feedback = input + 0.6 * delayed;
        self.write(feedback);
        delayed - 0.6 * feedback
    }
}

pub fn reverb(input: &mut Clip, wet: f64, space: Reverb) {
    let rate = input.rate / 2.0;
    let pre_delay = (round(space.pre_delay * input.rate) as isize) & !1;
    let mut reflections = Clip::with_length(rate, input.len() / 2);
    let lengths = [
        0.0297, 0.0371, 0.0411, 0.0437, 0.0533, 0.0617, 0.0683, 0.0741,
    ]
    .map(|seconds| round(seconds * space.size * rate) as usize | 1);
    let mut network = lengths.map(Delay::new);
    let feedback = lengths.map(|length| 10.0_f64.powf(-3.0 * length as f64 / rate / space.tail));
    let damping = 1.0 - space.damping;
    let mut damped = [0.0; 8];
    let mut early = [0.0051, 0.0127].map(|seconds| Delay::new(round(seconds * rate) as usize | 1));
    for (index, sample) in reflections.iter_mut().enumerate() {
        let index = 2 * index as isize - pre_delay;
        let mut drive = 0.0;
        if index >= 3 && index < input.len() as isize - 3 {
            let center = index as usize;
            let s = |i: usize| f64::from(input[i]);
            drive = (16.0 * s(center) + 9.0 * (s(center - 1) + s(center + 1))
                - (s(center - 3) + s(center + 3)))
                * 0.0109375;
        }
        for delay in &mut early {
            drive = delay.diffuse(drive);
        }
        let delays: [f64; 8] = std::array::from_fn(|i| network[i].read());
        let shared = delays.iter().sum::<f64>() * 0.25;
        for i in 0..8 {
            damped[i] += (delays[i] - shared - damped[i]) * damping;
            network[i].write(damped[i] * feedback[i] + drive);
        }
        let [a, b, c, d, e, f, g, h] = delays;
        *sample = ((a - b + c - d + e - f + g - h) * 0.5) as f32;
    }
    reflections.filter(Filter::High, space.high_pass, 0.707);
    let s = |index: usize| f64::from(reflections[index]);
    for index in 2..reflections.len().saturating_sub(3) {
        input[2 * index] = (f64::from(input[2 * index]) + s(index) * wet) as f32;
        let interpolated = 3.0 * (s(index - 2) + s(index + 3))
            - 25.0 * (s(index - 1) + s(index + 2))
            + 150.0 * (s(index) + s(index + 1));
        input[2 * index + 1] =
            (f64::from(input[2 * index + 1]) + interpolated * (wet / 256.0)) as f32;
    }
}
