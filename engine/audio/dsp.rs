use std::f64::consts::{PI, TAU};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Clip {
    pub rate: f64,
    pub samples: Vec<f32>,
}

impl Deref for Clip {
    type Target = [f32];

    fn deref(&self) -> &[f32] {
        &self.samples
    }
}

impl DerefMut for Clip {
    fn deref_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }
}

pub fn round(value: f64) -> f64 {
    // Math.round breaks half ties towards positive infinity.
    let lower = value.floor();
    if value - lower < 0.5 {
        lower
    } else {
        lower + 1.0
    }
}

pub fn decay(seconds: f64, release: f64, attack: f64) -> f64 {
    let tail = (-seconds / release).exp();
    if attack > 0.0 {
        (1.0 - (-seconds / attack).exp()) * tail
    } else {
        tail
    }
}

pub fn glide(start: f64, end: f64, curve: f64) -> impl Fn(f64) -> f64 {
    move |t| start * (end / start).powf(t.powf(curve))
}

pub fn triangle(peak: f64, curve: f64) -> impl Fn(f64) -> f64 {
    move |t| {
        (if t < peak {
            t / peak
        } else {
            (1.0 - t) / (1.0 - peak)
        })
        .powf(curve)
    }
}

#[derive(Clone, Copy)]
pub enum Filter {
    Low,
    High,
    Band,
}

#[derive(Clone, Copy, Default)]
pub struct Bands {
    pub high: f64,
    pub low: f64,
    pub band: f64,
    pub q: f64,
}

impl Bands {
    pub const fn high(frequency: f64) -> Self {
        Self {
            high: frequency,
            low: 0.0,
            band: 0.0,
            q: 0.0,
        }
    }

    pub const fn low(frequency: f64) -> Self {
        Self {
            high: 0.0,
            low: frequency,
            band: 0.0,
            q: 0.0,
        }
    }

    pub const fn band(frequency: f64, q: f64) -> Self {
        Self {
            high: 0.0,
            low: 0.0,
            band: frequency,
            q,
        }
    }

    pub const fn range(high: f64, low: f64) -> Self {
        Self {
            high,
            low,
            band: 0.0,
            q: 0.0,
        }
    }
}

pub fn coefficients(kind: Filter, frequency: f64, q: f64, rate: f64) -> [f64; 5] {
    let w = TAU * frequency.min(rate * 0.47).max(10.0) / rate;
    let cos = w.cos();
    let alpha = w.sin() / (2.0 * q);
    let (b0, b1, b2) = match kind {
        Filter::Low => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0),
        Filter::High => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0),
        Filter::Band => (alpha, 0.0, -alpha),
    };
    let denominator = 1.0 + alpha;
    [
        b0 / denominator,
        b1 / denominator,
        b2 / denominator,
        -2.0 * cos / denominator,
        (1.0 - alpha) / denominator,
    ]
}

pub(super) fn biquad(input: f64, [b0, b1, b2, a1, a2]: [f64; 5], state: &mut [f64; 2]) -> f64 {
    let output = b0 * input + state[0];
    state[0] = b1 * input - a1 * output + state[1];
    state[1] = b2 * input - a2 * output;
    output
}

impl Clip {
    pub fn new(rate: f64, seconds: f64) -> Self {
        Self::with_length(rate, (round(rate * seconds) as usize).max(1))
    }

    pub fn with_length(rate: f64, length: usize) -> Self {
        Self {
            rate,
            samples: vec![0.0; length],
        }
    }

    pub fn mix(&mut self, input: &Clip, time: f64, gain: f64) {
        let start = round(time * self.rate) as isize;
        for (index, &sample) in input.iter().enumerate().skip((-start).max(0) as usize) {
            let destination = (start + index as isize) as usize;
            if destination >= self.len() {
                break;
            }
            self[destination] = (f64::from(self[destination]) + f64::from(sample) * gain) as f32;
        }
    }

    pub fn mix_loop(&mut self, input: &Clip, time: f64, gain: f64) {
        let length = self.len();
        let start = (round(time * self.rate) as isize).rem_euclid(length as isize) as usize;
        for (index, &sample) in input.iter().enumerate() {
            let destination = (start + index) % length;
            self[destination] = (f64::from(self[destination]) + f64::from(sample) * gain) as f32;
        }
    }

    pub fn filter(&mut self, kind: Filter, frequency: f64, q: f64) {
        let coeff = coefficients(kind, frequency, q, self.rate);
        let mut state = [0.0; 2];
        for sample in self.iter_mut() {
            *sample = biquad(f64::from(*sample), coeff, &mut state) as f32;
        }
    }

    pub fn bands(&mut self, bands: Bands) {
        if bands.high != 0.0 {
            self.filter(Filter::High, bands.high, 0.707);
        }
        if bands.low != 0.0 {
            self.filter(Filter::Low, bands.low, 0.707);
        }
        if bands.band != 0.0 {
            self.filter(
                Filter::Band,
                bands.band,
                if bands.q == 0.0 { 1.0 } else { bands.q },
            );
        }
    }

    pub fn sweep(&mut self, kind: Filter, frequency: impl Fn(f64) -> f64, q: f64) {
        self.sweep_passes(kind, frequency, |_| q, 1);
    }

    pub fn sweep_passes(
        &mut self,
        kind: Filter,
        frequency: impl Fn(f64) -> f64,
        q: impl Fn(f64) -> f64,
        passes: usize,
    ) {
        let mut state = [0.0; 2];
        let count = self.len() as f64;
        let rate = self.rate;
        for pass in 0..passes {
            let mut coeff = [0.0; 5];
            for (index, sample) in self.iter_mut().enumerate() {
                if index % 32 == 0 {
                    let t = index as f64 / count;
                    coeff = coefficients(kind, frequency(t), q(t), rate);
                }
                let value = biquad(f64::from(*sample), coeff, &mut state);
                if pass + 1 == passes {
                    *sample = value as f32;
                }
            }
        }
    }

    pub fn envelope(&mut self, envelope: impl Fn(f64) -> f64) {
        let rate = self.rate;
        for (index, sample) in self.iter_mut().enumerate() {
            *sample = (f64::from(*sample) * envelope(index as f64 / rate)) as f32;
        }
    }

    pub fn block_envelope(&mut self, block: usize, envelope: impl Fn(f64) -> f64) {
        let rate = self.rate;
        let (mut gain, mut step) = (0.0, 0.0);
        for (index, sample) in self.iter_mut().enumerate() {
            if index % block == 0 {
                gain = envelope(index as f64 / rate);
                step = (envelope((index + block) as f64 / rate) - gain) / block as f64;
            }
            *sample = (f64::from(*sample) * gain) as f32;
            gain += step;
        }
    }

    pub fn peak(&self) -> f64 {
        self.iter()
            .fold(0.0_f64, |peak, &sample| peak.max(f64::from(sample.abs())))
    }

    pub fn scale(&mut self, gain: f64) {
        for sample in self.iter_mut() {
            *sample = (f64::from(*sample) * gain) as f32;
        }
    }

    pub fn normalize(&mut self, peak: f64) {
        let measured = self.peak();
        if measured > 0.0 {
            self.scale(peak / measured);
        }
    }

    pub fn saturate(&mut self, drive: f64) {
        let gain = 1.0 / drive.tanh();
        for sample in self.iter_mut() {
            *sample = ((f64::from(*sample) * drive).tanh() * gain) as f32;
        }
    }

    pub fn ceiling(&mut self, limit: f64) {
        for sample in self.iter_mut() {
            *sample = (limit * (f64::from(*sample) / limit).tanh()) as f32;
        }
    }

    pub fn mean_square(&self, start: usize, end: usize) -> f64 {
        let end = end.min(self.len());
        let sum = self[start.min(end)..end]
            .iter()
            .map(|&sample| f64::from(sample) * f64::from(sample))
            .sum::<f64>();
        sum / end.saturating_sub(start).max(1) as f64
    }

    pub fn rms(&self, start: usize, end: usize) -> f64 {
        self.mean_square(start, end).sqrt()
    }

    pub fn peak_rms(&self, width: f64, step: f64) -> f64 {
        let width = round(width * self.rate) as usize;
        let step = round(step * self.rate).max(1.0) as usize;
        (0..=self.len().saturating_sub(width))
            .step_by(step)
            .map(|start| self.rms(start, start + width))
            .fold(0.0, f64::max)
    }

    pub fn comb(&mut self, delay: f64, feedback: f64, wet: f64) {
        let lag = (round(delay * self.rate) as usize).max(1);
        let mut echo = vec![0.0_f32; self.len()];
        for index in 0..echo.len() {
            let delayed = index.checked_sub(lag).map_or(0.0, |i| f64::from(echo[i]));
            echo[index] = (f64::from(self[index]) + delayed * feedback) as f32;
        }
        let attenuation = 1.0 - feedback * 0.5;
        for (sample, echo) in self.iter_mut().zip(echo) {
            *sample =
                (f64::from(*sample) * (1.0 - wet) + f64::from(echo) * attenuation * wet) as f32;
        }
    }

    pub fn seamless(mut self, output_length: usize) -> Self {
        let overlap = self.len().saturating_sub(output_length);
        for index in 0..overlap.min(output_length) {
            let t = (index as f64 + 0.5) / overlap as f64;
            let angle = t * PI / 2.0;
            self[index] = (f64::from(self[index]) * angle.sin()
                + f64::from(self[output_length + index]) * angle.cos())
                as f32;
        }
        self.samples.resize(output_length, 0.0);
        self
    }

    pub fn fade(&mut self, attack: f64, release: f64) {
        let attack = (round(attack * self.rate) as usize).min(self.len() / 2);
        let release = (round(release * self.rate) as usize).min(self.len() / 2);
        for (index, sample) in self.iter_mut().take(attack).enumerate() {
            *sample = (f64::from(*sample) * (index as f64 / attack as f64)) as f32;
        }
        for (index, sample) in self.iter_mut().rev().take(release).enumerate() {
            let t = index as f64 / release as f64;
            *sample = (f64::from(*sample) * (t * t)) as f32;
        }
    }
}

pub struct Synth {
    pub rate: f64,
    state: u32,
}

impl Synth {
    pub fn new(rate: f64, seed: u32) -> Self {
        Self { rate, state: seed }
    }

    pub fn random(&mut self) -> f64 {
        self.state = self.state.wrapping_add(1831565813);
        let state = self.state;
        let mut hash = (state ^ (state >> 15)).wrapping_mul(1 | state);
        hash = hash.wrapping_add((hash ^ (hash >> 7)).wrapping_mul(61 | hash)) ^ hash;
        f64::from(hash ^ (hash >> 14)) / 4294967296.0
    }

    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.random()
    }

    pub fn fork(&mut self) -> Self {
        Self::new(self.rate, (self.random() * 1e9).floor() as u32)
    }

    pub fn silent(&self, seconds: f64) -> Clip {
        Clip::new(self.rate, seconds)
    }

    pub fn noise(&mut self, seconds: f64) -> Clip {
        self.noise_frames((round(seconds * self.rate) as usize).max(1))
    }

    pub fn noise_frames(&mut self, length: usize) -> Clip {
        let mut clip = Clip::with_length(self.rate, length);
        for sample in clip.iter_mut() {
            *sample = (self.random() * 2.0 - 1.0) as f32;
        }
        clip
    }

    pub fn loop_noise(
        &mut self,
        length: f64,
        overlap: f64,
        filter: impl FnOnce(&mut Clip),
    ) -> Clip {
        let mut clip = self.noise(length + overlap);
        filter(&mut clip);
        clip.seamless(round(length * self.rate) as usize)
    }

    pub fn burst(&mut self, length: f64, release: f64, bands: Bands, attack: f64) -> Clip {
        let mut clip = self.noise(length);
        clip.bands(bands);
        clip.envelope(|t| decay(t, release, attack));
        clip.normalize(1.0);
        clip
    }

    pub fn ring(&mut self, length: f64, modes: &[[f64; 3]]) -> Clip {
        self.resonance(length, modes, true)
    }

    pub fn resonance(&mut self, length: f64, modes: &[[f64; 3]], random_phase: bool) -> Clip {
        let mut clip = self.silent(length);
        for &[frequency, release, gain] in modes {
            if frequency > self.rate * 0.46 {
                continue;
            }
            let angle = TAU * frequency / self.rate;
            let phase = if random_phase {
                self.random() * TAU
            } else {
                0.0
            };
            let attenuation = (-1.0 / (release * self.rate)).exp();
            let recurrence = 2.0 * attenuation * angle.cos();
            let attenuation_squared = attenuation * attenuation;
            let count = ((1e-5 / gain.max(1e-5)).ln() / attenuation.ln()).ceil() as usize + 1;
            let mut before = gain / attenuation * (phase - angle).sin();
            let mut current = gain * phase.sin();
            for sample in clip.iter_mut().take(count) {
                *sample = (f64::from(*sample) + current) as f32;
                let next = recurrence * current - attenuation_squared * before;
                before = current;
                current = next;
            }
        }
        let fade = clip.len().min(round(0.0004 * self.rate) as usize);
        for (index, sample) in clip.iter_mut().take(fade).enumerate() {
            *sample = (f64::from(*sample) * (index as f64 / fade as f64)) as f32;
        }
        clip
    }

    pub fn click(&mut self, modes: &[[f64; 3]], options: Click) -> Clip {
        let longest = modes
            .iter()
            .fold(0.004_f64, |release, mode| release.max(mode[1]));
        let mut clip = self.silent((longest * 7.0 + 0.004).min(0.5));
        let modes: Vec<_> = modes
            .iter()
            .map(|&[frequency, release, gain]| {
                [
                    frequency * (1.0 + (self.random() * 2.0 - 1.0) * options.jitter),
                    release,
                    gain,
                ]
            })
            .collect();
        clip.mix(
            &self.ring(clip.len() as f64 / self.rate, &modes),
            0.0,
            options.ring,
        );
        if options.tick != 0.0 {
            let tick = self.burst(0.006, options.tick_release, Bands::high(options.high), 0.0);
            clip.mix(&tick, 0.0, options.tick);
        }
        clip
    }

    pub fn knock(
        &self,
        length: f64,
        start: f64,
        end: f64,
        pitch_decay: f64,
        release: f64,
        attack: f64,
    ) -> Clip {
        let mut clip = self.silent(length);
        let mut phase = 0.0;
        for (index, sample) in clip.iter_mut().enumerate() {
            let t = index as f64 / self.rate;
            phase += TAU * (end + (start - end) * (-t / pitch_decay).exp()) / self.rate;
            *sample = (phase.sin() * (-t / release).exp() * (1.0 - (-t / attack).exp())) as f32;
        }
        clip
    }

    pub fn chirp(&self, length: f64, start: f64, end: f64, window: bool) -> Clip {
        let mut clip = self.silent(length);
        let mut phase = 0.0;
        let count = clip.len() as f64;
        for (index, sample) in clip.iter_mut().enumerate() {
            let t = index as f64 / count;
            phase += TAU * start * (end / start).powf(t) / self.rate;
            *sample = (phase.sin() * if window { (PI * t).sin() } else { 1.0 }) as f32;
        }
        clip
    }

    pub fn pressure(&self, length: f64) -> Clip {
        let mut clip = self.silent(length * 12.0);
        for (index, sample) in clip.iter_mut().enumerate() {
            let t = index as f64 / self.rate / length;
            *sample = ((1.0 - t) * (-t).exp()) as f32;
        }
        clip
    }
}

#[derive(Clone, Copy)]
pub struct Click {
    pub tick: f64,
    pub high: f64,
    pub tick_release: f64,
    pub jitter: f64,
    pub ring: f64,
}

impl Default for Click {
    fn default() -> Self {
        Self {
            tick: 0.7,
            high: 2500.0,
            tick_release: 0.0009,
            jitter: 0.02,
            ring: 0.6,
        }
    }
}

pub fn decibels(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}
