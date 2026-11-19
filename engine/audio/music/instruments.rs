use super::effects::{limit, upsample};
use crate::dsp::{Bands, Clip, Filter, Synth, coefficients, round};
use std::f64::consts::{PI, SQRT_2, TAU};

pub fn midi(note: f64) -> f64 {
    440.0 * 2.0_f64.powf((note - 69.0) / 12.0)
}

pub fn musical_noise(
    synth: &mut Synth,
    rate: f64,
    length: f64,
    release: f64,
    bands: Bands,
    attack: f64,
) -> Clip {
    let mut clip = Clip::new(rate, length);
    for sample in clip.iter_mut() {
        *sample = (synth.random() * 2.0 - 1.0) as f32;
    }
    clip.bands(bands);
    let damp = (-1.0 / (release * rate)).exp();
    let rise = if attack > 0.0 {
        (-1.0 / (attack * rate)).exp()
    } else {
        0.0
    };
    let (mut down, mut up, mut peak) = (1.0, 1.0, 0.0_f64);
    for sample in clip.iter_mut() {
        let wave = f64::from(*sample) * down * (1.0 - up);
        *sample = wave as f32;
        down *= damp;
        up *= rise;
        peak = peak.max(wave.abs());
    }
    if peak > 0.0 {
        clip.scale(1.0 / peak);
    }
    clip
}

pub fn drum_tone(
    rate: f64,
    length: f64,
    start: f64,
    end: f64,
    pitch_release: f64,
    release: f64,
    attack: f64,
) -> Clip {
    let mut clip = Clip::new(rate, length);
    let pitch_tail = (-1.0 / (pitch_release * rate)).exp();
    let decay = (-1.0 / (release * rate)).exp();
    let rise = (-1.0 / (attack * rate)).exp();
    let step = TAU * end / rate;
    let (mut phase, mut sweep, mut volume, mut onset) = (0.0, TAU * (start - end) / rate, 1.0, 1.0);
    for sample in clip.iter_mut() {
        phase += step + sweep;
        if phase > TAU {
            phase -= TAU;
        }
        *sample = (phase.sin() * volume * (1.0 - onset)) as f32;
        sweep *= pitch_tail;
        volume *= decay;
        onset *= rise;
    }
    clip
}

pub fn drum(synth: &mut Synth, pitch: f64, strength: f64) -> Clip {
    let rate = synth.rate;
    let mut clip = synth.silent(0.9);
    clip.mix(
        &drum_tone(
            rate,
            0.9,
            pitch * (1.5 + 0.6 * strength),
            pitch,
            0.016,
            0.16 + 0.1 * strength,
            0.0012,
        ),
        0.0,
        1.0,
    );
    clip.mix(
        &drum_tone(
            rate,
            0.35,
            pitch * 1.59 * 1.25,
            pitch * 1.59,
            0.02,
            0.07,
            0.001,
        ),
        0.0,
        0.3,
    );
    let overtones = [
        [pitch * 2.3, 0.05, 0.16],
        [pitch * 2.65, 0.04, 0.11],
        [pitch * 3.16, 0.03, 0.08],
        [pitch * 3.6, 0.025, 0.05],
    ];
    clip.mix(&synth.ring(0.25, &overtones), 0.0, 1.0);
    clip.mix(
        &musical_noise(synth, rate, 0.14, 0.026, Bands::band(520.0, 0.6), 0.0),
        0.0,
        0.2 + 0.32 * strength,
    );
    clip.mix(
        &musical_noise(synth, rate, 0.012, 0.0022, Bands::high(2200.0), 0.0),
        0.0,
        0.1 + 0.2 * strength,
    );
    limit(&mut clip, 1.2 + 0.5 * strength);
    clip
}

pub fn big_drum(synth: &mut Synth, strength: f64) -> Clip {
    let rate = synth.rate;
    let mut clip = synth.silent(1.6);
    clip.mix(
        &drum_tone(rate, 1.6, 124.0, 55.0, 0.045, 0.34 * strength, 0.002),
        0.0,
        1.0,
    );
    clip.mix(
        &musical_noise(synth, rate, 0.5, 0.09, Bands::low(150.0), 0.0),
        0.0,
        0.7,
    );
    clip.mix(
        &musical_noise(synth, rate, 0.16, 0.03, Bands::band(700.0, 0.5), 0.0),
        0.0,
        0.3,
    );
    clip.mix(
        &musical_noise(synth, rate, 0.02, 0.004, Bands::high(1800.0), 0.0),
        0.0,
        0.12,
    );
    limit(&mut clip, 1.5);
    clip
}

pub fn gong(synth: &mut Synth, frequency: f64, length: f64, strength: f64) -> Clip {
    let mut tones = Vec::with_capacity(19);
    for (partial, gain) in [
        (1.0, 1.0),
        (2.756, 0.7),
        (5.404, 0.5),
        (8.933, 0.32),
        (13.34, 0.2),
    ] {
        tones.push([
            frequency * partial * (1.0 + (synth.random() - 0.5) * 0.01),
            (length * 0.3) / (1.0 + (partial - 1.0) * 0.16),
            gain * strength.powf(partial.log2()),
        ]);
    }
    for _ in 0..14 {
        let overtone = 1.2 + 9.0 * synth.random().powf(1.4);
        tones.push([
            frequency * overtone,
            (length * 0.2) / (1.0 + (overtone - 1.0) * 0.3) * (0.5 + synth.random()),
            0.22 * synth.random() * strength.powf(overtone.log2()),
        ]);
    }
    let mut sound = synth.ring(length, &tones);
    sound.mix(
        &musical_noise(synth, synth.rate, 0.01, 0.0018, Bands::high(1500.0), 0.0),
        0.0,
        0.5 * strength,
    );
    sound.normalize(1.0);
    limit(&mut sound, 1.4);
    sound
}

pub fn hammer(synth: &mut Synth, frequency: f64, velocity: f64, length: f64) -> Clip {
    let rate = synth.rate / 2.0;
    let stiffness = 0.00032 * (frequency / 262.0).powf(1.2);
    let count = (0.46 * rate / frequency).floor().min(8.0) as usize;
    let mut tones = Vec::new();
    for harmonic in 1..=count {
        let harmonic = harmonic as f64;
        let partial = harmonic * frequency * (1.0 + stiffness * harmonic * harmonic).sqrt();
        let gain = harmonic.powf(-1.45)
            * (PI * harmonic / 7.6).sin().abs()
            * velocity.powf((harmonic - 1.0) * 0.3);
        let decay = length * 0.36 / (1.0 + 0.33 * (harmonic - 1.0));
        if harmonic <= 2.0 {
            tones.push([partial * 0.9996, decay, gain * 0.55]);
            tones.push([partial * 1.0006, decay * 0.82, gain * 0.45]);
        } else {
            tones.push([partial, decay, gain]);
        }
    }
    let strength = tones.iter().map(|v| v[2]).fold(0.0, f64::max);
    let mut sound = Clip::new(rate, length);
    for [frequency, release, gain] in tones {
        if frequency > rate * 0.46 || gain < strength * 0.001 {
            continue;
        }
        let step = TAU * frequency / rate;
        let a = (-1.0 / (release * rate)).exp();
        let b = 2.0 * a * step.cos();
        let c = a * a;
        let frames = (((strength * 0.001) / gain).ln() / a.ln()).ceil() as usize + 1;
        let (mut previous, mut current) = (-gain / a * step.sin(), 0.0);
        for sample in sound.iter_mut().take(frames) {
            *sample = (f64::from(*sample) + current) as f32;
            let next = b * current - c * previous;
            previous = current;
            current = next;
        }
    }
    let onset = round(0.007 * rate) as usize;
    for (i, s) in sound.iter_mut().take(onset).enumerate() {
        *s = (f64::from(*s) * (i as f64 / onset as f64)) as f32;
    }
    sound.mix(
        &musical_noise(synth, rate, 0.06, 0.014, Bands::low(420.0), 0.003),
        0.0,
        0.1 * velocity,
    );
    let fade = round(0.25 * rate) as usize;
    for (i, s) in sound.iter_mut().rev().take(fade).enumerate() {
        *s = (f64::from(*s) * (i as f64 / fade as f64)) as f32;
    }
    sound.scale(velocity);
    upsample(&sound, false)
}

pub fn pluck(synth: &mut Synth, frequency: f64, strength: f64, length: f64, mute: f64) -> Clip {
    let rate = synth.rate;
    let mut clip = synth.silent(length);
    let period = rate / frequency;
    let damping = 0.5 - 0.22 * strength;
    let lag = (period - damping - 0.1).floor() as usize;
    let fraction = period - damping - lag as f64;
    let delay = (1.0 - fraction) / (1.0 + fraction);
    let sustain = (-period / (rate * 0.2 * 2.2)).exp();
    let mut excitation = vec![0.0_f32; lag];
    let (mut noise, mut mean) = (0.0, 0.0);
    let speed = 0.16 + 0.62 * strength;
    for sample in &mut excitation {
        noise += (synth.random() * 2.0 - 1.0 - noise) * speed;
        *sample = noise as f32;
        mean += noise;
    }
    mean /= lag as f64;
    let gap = (round(lag as f64 * 0.19) as usize).max(1);
    let attack: Vec<_> = excitation
        .iter()
        .enumerate()
        .map(|(i, &current)| {
            (f64::from(current) - mean - 0.55 * (f64::from(excitation[(i + gap) % lag]) - mean))
                as f32
        })
        .collect();
    let (mut prev_a, mut prev_b) = (0.0, 0.0);
    for index in 0..clip.len() {
        let previous = index.checked_sub(lag).map_or(0.0, |i| f64::from(clip[i]));
        let before = index
            .checked_sub(lag + 1)
            .map_or(0.0, |i| f64::from(clip[i]));
        let feedback = (1.0 - damping) * previous + damping * before;
        let interpolated = delay * feedback + prev_a - delay * prev_b;
        prev_a = feedback;
        prev_b = interpolated;
        clip[index] =
            (attack.get(index).copied().map_or(0.0, f64::from) + sustain * interpolated) as f32;
    }
    for (index, sample) in clip.iter_mut().enumerate() {
        let time = index as f64 / rate;
        *sample = (f64::from(*sample)
            * ((-time / mute).exp() * (index as f64 / (0.0012 * rate)).min(1.0)))
            as f32;
    }
    let click = musical_noise(synth, rate, 0.03, 0.007, Bands::low(500.0), 0.0);
    clip.mix(&click, 0.0, 0.12 * clip.peak() * strength);
    clip.normalize(1.0);
    let fade = round(0.02 * rate) as usize;
    for (index, sample) in clip.iter_mut().rev().take(fade).enumerate() {
        *sample = (f64::from(*sample) * (index as f64 / fade as f64)) as f32;
    }
    clip
}

pub fn saw(phase: &mut f64, step: f64) -> f64 {
    *phase += step;
    if *phase >= 1.0 {
        *phase -= 1.0;
    }
    let mut value = *phase + *phase - 1.0;
    if *phase < step {
        let t = *phase / step;
        value -= t + t - t * t - 1.0;
    } else if *phase > 1.0 - step {
        let t = (*phase - 1.0) / step;
        value -= t * t + t + t + 1.0;
    }
    value
}

#[derive(Clone, Copy)]
pub struct Brass {
    pub voices: usize,
    pub detune: f64,
    pub velocity: f64,
    pub attack: f64,
    pub release: f64,
    pub swell: [f64; 2],
    pub curve: f64,
    pub scoop: f64,
    pub brightness: f64,
    pub vibrato: f64,
}

impl Default for Brass {
    fn default() -> Self {
        Self {
            voices: 4,
            detune: 7.0,
            velocity: 0.8,
            attack: 0.045,
            release: 0.16,
            swell: [1.0, 1.0],
            curve: 1.6,
            scoop: -35.0,
            brightness: 8.0,
            vibrato: 1.0,
        }
    }
}

pub fn brass(synth: &mut Synth, pitch: f64, time: f64, note: Brass) -> Clip {
    let downsample = pitch < 400.0;
    let rate = if downsample {
        synth.rate / 2.0
    } else {
        synth.rate
    };
    let mut clip = Clip::new(rate, time + note.release * 3.5);
    let blocks = clip.len().div_ceil(32) + 1;
    let [start_gain, end_gain] = note.swell;
    let full = 1.0 - (-time / note.attack).exp();
    let envelope = |t: f64| {
        if t < time {
            (1.0 - (-t / note.attack).exp())
                * (start_gain + (end_gain - start_gain) * (t / time).powf(note.curve))
        } else {
            full * end_gain * (-(t - time) / note.release).exp()
        }
    };
    let mut gain = Vec::with_capacity(blocks);
    let mut modulation = Vec::with_capacity(blocks);
    for block in 0..blocks {
        let t = (block * 32) as f64 / rate;
        gain.push(envelope(t) as f32);
        modulation.push(
            (2.0_f64.powf(note.scoop * (-t / 0.028).exp() / 1200.0)
                * (1.0 + 0.0035 * note.vibrato * (TAU * 5.2 * t).sin() * (t / 0.7).min(1.0)))
                as f32,
        );
    }
    for voice in 0..note.voices {
        let position = if note.voices > 1 {
            voice as f64 / (note.voices - 1) as f64 * 2.0 - 1.0
        } else {
            0.0
        };
        let cents = (position * note.detune + (synth.random() - 0.5) * 3.0) / 1200.0;
        let frequency = pitch * 2.0_f64.powf(cents) / rate;
        let mut phase = synth.random();
        for (block, chunk) in clip.chunks_mut(32).enumerate() {
            let step = frequency * f64::from(modulation[block]);
            for sample in chunk {
                *sample = (f64::from(*sample) + saw(&mut phase, step)) as f32;
            }
        }
    }
    let loudest = start_gain.max(end_gain);
    let filter = |block: usize| {
        coefficients(
            Filter::Low,
            (rate * 0.42).min(
                pitch
                    * (1.3
                        + note.brightness
                            * note.velocity
                            * (f64::from(gain[block.min(blocks - 1)]) / loudest).powf(1.4)),
            ),
            0.9,
            rate,
        )
    };
    let power = 0.8 / (note.voices as f64).sqrt();
    let mut state = [0.0; 2];
    let mut next = filter(0);
    for (block, chunk) in clip.chunks_mut(32).enumerate() {
        let mut coeff = next;
        next = filter(block + 1);
        let delta = std::array::from_fn::<_, 5, _>(|i| (next[i] - coeff[i]) / 32.0);
        let mut volume = f64::from(gain[block]);
        let slope = (f64::from(gain[block + 1]) - volume) / 32.0;
        for sample in chunk {
            let output = crate::dsp::biquad(f64::from(*sample) * power, coeff, &mut state);
            let driven = output * volume * 1.3;
            *sample = (driven / (1.0 + driven * driven).sqrt() * 0.8) as f32;
            volume += slope;
            for i in 0..5 {
                coeff[i] += delta[i];
            }
        }
    }
    let spit = musical_noise(
        synth,
        rate,
        0.05,
        0.014,
        Bands::band(2400.0_f64.min(pitch * 5.0), 0.8),
        0.004,
    );
    clip.mix(&spit, 0.0, 0.05 * note.velocity);
    let fade = clip.len().min(round(0.02 * rate) as usize);
    for (index, sample) in clip.iter_mut().rev().take(fade).enumerate() {
        *sample = (f64::from(*sample) * (index as f64 / fade as f64)) as f32;
    }
    if downsample {
        upsample(&clip, false)
    } else {
        clip
    }
}

#[derive(Clone, Copy)]
pub struct Strings {
    pub voices: usize,
    pub detune: f64,
    pub attack: f64,
    pub release: f64,
    pub gain: f64,
    pub swell: [f64; 2],
    pub curve: f64,
    pub tail: f64,
    pub width: f64,
    pub pan: f64,
    pub drift: f64,
    pub tremolo: Option<(f64, f64)>,
}

impl Default for Strings {
    fn default() -> Self {
        Self {
            voices: 5,
            detune: 8.0,
            attack: 0.5,
            release: 0.6,
            gain: 1.0,
            swell: [1.0, 1.0],
            curve: 1.0,
            tail: 99.0,
            width: 0.75,
            pan: 0.0,
            drift: 0.0012,
            tremolo: None,
        }
    }
}

pub fn strings(
    channels: &mut [Clip],
    synth: &mut Synth,
    pitch: f64,
    start: f64,
    length: f64,
    note: Strings,
) {
    let rate = channels[0].rate;
    let frames = channels[0].len();
    let stereo = channels.len() == 2;
    let gain = note.gain / (note.voices as f64).sqrt();
    let [first_gain, last_gain] = note.swell;
    let tail = (note.release * 3.0).min(note.tail);
    let count = round((length + tail) * rate) as usize;
    let offset = (round(start * rate) as isize).rem_euclid(frames as isize) as usize;
    for voice in 0..note.voices {
        let lane = |index: usize| {
            if note.voices > 1 {
                index as f64 / (note.voices - 1) as f64 * 2.0 - 1.0
            } else {
                0.0
            }
        };
        let frequency = pitch
            * 2.0_f64.powf(
                (lane(voice) * note.detune + (synth.random() - 0.5) * note.detune * 0.4) / 1200.0,
            );
        let pan = (note.pan
            + lane((voice * 3 + 1) % note.voices) * if stereo { note.width } else { 0.0 })
        .clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * PI / 4.0;
        let left = if stereo {
            angle.cos() * gain * SQRT_2
        } else {
            gain
        };
        let right = if stereo {
            angle.sin() * gain * SQRT_2
        } else {
            0.0
        };
        let attack = note.attack * (0.65 + 0.7 * synth.random());
        let drift = note.drift * (0.5 + synth.random());
        let drift_rate = TAU * (0.07 + 0.2 * synth.random());
        let drift_start = synth.random() * TAU;
        let modulation = note
            .tremolo
            .map_or(0.0, |(rate, _)| TAU * rate * (0.93 + 0.14 * synth.random()));
        let phase = synth.random() * TAU;
        let depth = note.tremolo.map_or(0.0, |(_, depth)| depth);
        let envelope = |time: f64| {
            let mut amplitude = if time < attack {
                0.5 - 0.5 * (PI * time / attack).cos()
            } else {
                1.0
            };
            if time < length {
                amplitude *= if first_gain == last_gain {
                    first_gain
                } else {
                    first_gain + (last_gain - first_gain) * (time / length).powf(note.curve)
                };
            } else {
                let elapsed = time - length;
                amplitude *=
                    last_gain * (-elapsed / note.release).exp() * (1.0 - elapsed / tail).max(0.0);
            }
            if depth != 0.0 {
                amplitude *= 1.0 - depth * (0.5 - 0.5 * (modulation * time + phase).cos());
            }
            amplitude
        };
        let mut oscillator = synth.random();
        let mut next = envelope(0.0);
        let mut frame = offset;
        for block in (0..count).step_by(32) {
            let time = block as f64 / rate;
            let mut volume = next;
            next = envelope((block + 32) as f64 / rate);
            let change = (next - volume) / 32.0;
            let step = frequency * (1.0 + drift * (drift_rate * time + drift_start).sin()) / rate;
            for _ in 0..32.min(count - block) {
                let tone = saw(&mut oscillator, step) * volume;
                volume += change;
                channels[0][frame] = (f64::from(channels[0][frame]) + tone * left) as f32;
                if stereo {
                    channels[1][frame] = (f64::from(channels[1][frame]) + tone * right) as f32;
                }
                frame += 1;
                if frame == frames {
                    frame = 0;
                }
            }
        }
    }
}
